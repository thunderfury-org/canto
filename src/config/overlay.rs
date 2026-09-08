use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use tracing::info;

use crate::config::NetworkSettings;
use crate::error::{CantoError, Result};

const MIXED_IN: &str = "mixed-in";
const TPROXY_IN: &str = "tproxy-in";
const DNS_IN: &str = "dns-in";

/// Loads a complete sing-box JSON object from a local source file.
pub fn load_source(path: &Path) -> Result<Value> {
    if path.as_os_str().is_empty() {
        return Err(CantoError::Config(
            "sing-box source is required ([singbox].source in canto.toml)".to_string(),
        ));
    }

    let content = fs::read_to_string(path).map_err(|e| {
        CantoError::Config(format!(
            "Failed to read source config '{}': {e}",
            path.display()
        ))
    })?;

    let value: Value = serde_json::from_str(&content).map_err(|e| {
        CantoError::Config(format!(
            "Source config '{}' is not valid JSON: {e}",
            path.display()
        ))
    })?;

    if !value.is_object() {
        return Err(CantoError::Config(format!(
            "Source config '{}' must be a JSON object",
            path.display()
        )));
    }

    Ok(value)
}

/// Writes pretty-printed JSON to `output_path`, creating parent directories as needed.
pub fn write_runtime_config(value: &Value, output_path: &Path) -> Result<()> {
    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| {
            CantoError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to create directory '{}': {e}", parent.display()),
            ))
        })?;
    }

    let mut formatted = serde_json::to_string_pretty(value)?;
    if !formatted.ends_with('\n') {
        formatted.push('\n');
    }
    fs::write(output_path, formatted)?;
    info!(
        "Wrote runtime sing-box configuration to {}",
        output_path.display()
    );
    Ok(())
}

/// Overlays gateway inbounds and routing-mark fields onto a complete source config.
///
/// Keeps dns, outbounds, endpoints, experimental, and log untouched.
pub fn apply_runtime_overlay(mut config: Value, network: &NetworkSettings) -> Result<Value> {
    if !config.is_object() {
        return Err(CantoError::Config(
            "sing-box source must be a JSON object".to_string(),
        ));
    }

    config["inbounds"] = tproxy_inbounds(network);
    ensure_route_object(&mut config)?;
    config["route"]["default_mark"] = json!(network.routing_mark);
    config["route"]["auto_detect_interface"] = json!(false);
    insert_hijack_dns_if_missing(&mut config)?;
    Ok(config)
}

fn tproxy_inbounds(network: &NetworkSettings) -> Value {
    json!([
        {
            "type": "mixed",
            "tag": MIXED_IN,
            "listen": "0.0.0.0",
            "listen_port": network.mixed_port
        },
        {
            "type": "tproxy",
            "tag": TPROXY_IN,
            "listen": "::",
            "listen_port": network.tproxy_port
        },
        {
            "type": "direct",
            "tag": DNS_IN,
            "listen": "0.0.0.0",
            "listen_port": network.dns_port
        }
    ])
}

fn ensure_route_object(config: &mut Value) -> Result<()> {
    match config.get("route") {
        Some(Value::Object(_)) => Ok(()),
        Some(_) => Err(CantoError::Config(
            "source route must be a JSON object when present".to_string(),
        )),
        None => {
            config["route"] = json!({});
            Ok(())
        }
    }
}

fn insert_hijack_dns_if_missing(config: &mut Value) -> Result<()> {
    let already_present = config
        .get("route")
        .and_then(|route| route.get("rules"))
        .and_then(Value::as_array)
        .is_some_and(|rules| rules.iter().any(is_dns_in_hijack));

    if already_present {
        return Ok(());
    }

    let rules = route_rules_mut(config)?;
    rules.insert(
        0,
        json!({
            "inbound": [DNS_IN],
            "action": "hijack-dns"
        }),
    );
    Ok(())
}

fn route_rules_mut(config: &mut Value) -> Result<&mut Vec<Value>> {
    let route = config
        .get_mut("route")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| CantoError::Config("route must be a JSON object".to_string()))?;

    if !route.get("rules").is_some_and(Value::is_array) {
        route.insert("rules".to_string(), Value::Array(Vec::new()));
    }

    route
        .get_mut("rules")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| CantoError::Config("route.rules must be an array".to_string()))
}

fn is_dns_in_hijack(rule: &Value) -> bool {
    let is_hijack = rule
        .get("action")
        .and_then(Value::as_str)
        .is_some_and(|action| action == "hijack-dns");
    if !is_hijack {
        return false;
    }

    match rule.get("inbound") {
        Some(Value::String(tag)) => tag == DNS_IN,
        Some(Value::Array(tags)) => tags.iter().any(|tag| tag.as_str() == Some(DNS_IN)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NetworkSettings;
    use serde_json::json;

    fn overlay(source: Value) -> Value {
        apply_runtime_overlay(source, &NetworkSettings::default()).unwrap()
    }

    #[test]
    fn test_replaces_tun_inbound_with_tproxy_set() {
        let source = json!({
            "inbounds": [
                {
                    "type": "tun",
                    "tag": "tun-in",
                    "auto_route": true
                }
            ],
            "outbounds": [{ "tag": "直连", "type": "direct" }]
        });

        let config = overlay(source);
        let inbounds = config["inbounds"].as_array().unwrap();
        assert_eq!(inbounds.len(), 3);
        assert_eq!(inbounds[0]["tag"], MIXED_IN);
        assert_eq!(inbounds[0]["type"], "mixed");
        assert_eq!(inbounds[0]["listen_port"], 7890);
        assert_eq!(inbounds[1]["tag"], TPROXY_IN);
        assert_eq!(inbounds[1]["type"], "tproxy");
        assert_eq!(inbounds[1]["listen"], "::");
        assert_eq!(inbounds[1]["listen_port"], 7893);
        assert!(inbounds[1].get("network").is_none());
        assert_eq!(inbounds[2]["tag"], DNS_IN);
        assert_eq!(inbounds[2]["type"], "direct");
        assert_eq!(inbounds[2]["listen_port"], 1053);
        assert!(
            config["inbounds"]
                .as_array()
                .unwrap()
                .iter()
                .all(|inbound| inbound["tag"] != "tun-in")
        );
    }

    #[test]
    fn test_preserves_non_inbound_fields() {
        let source = json!({
            "log": { "level": "warn" },
            "dns": { "final": "dns_direct" },
            "inbounds": [{ "type": "tun", "tag": "tun-in" }],
            "outbounds": [{ "tag": "直连", "type": "direct" }],
            "endpoints": [{ "type": "tailscale", "tag": "ts-ep" }],
            "experimental": {
                "clash_api": {
                    "external_controller": "127.0.0.1:9090"
                }
            },
            "route": {
                "final": "默认策略",
                "rules": [{ "protocol": "dns", "action": "hijack-dns" }]
            }
        });

        let config = overlay(source);
        assert_eq!(config["log"]["level"], "warn");
        assert_eq!(config["dns"]["final"], "dns_direct");
        assert_eq!(config["outbounds"][0]["tag"], "直连");
        assert_eq!(config["endpoints"][0]["tag"], "ts-ep");
        assert_eq!(
            config["experimental"]["clash_api"]["external_controller"],
            "127.0.0.1:9090"
        );
        assert_eq!(config["route"]["final"], "默认策略");
        assert_eq!(config["route"]["rules"][1]["protocol"], "dns");
    }

    #[test]
    fn test_writes_default_mark_and_disables_auto_detect() {
        let config = overlay(json!({
            "inbounds": [],
            "route": {
                "final": "默认策略",
                "auto_detect_interface": true
            }
        }));

        assert_eq!(config["route"]["default_mark"], 0x67890);
        assert_eq!(config["route"]["auto_detect_interface"], false);
        assert_eq!(config["route"]["final"], "默认策略");
    }

    #[test]
    fn test_creates_route_when_missing() {
        let config = overlay(json!({
            "outbounds": [{ "tag": "直连", "type": "direct" }]
        }));

        assert_eq!(config["route"]["default_mark"], 0x67890);
        assert_eq!(config["route"]["auto_detect_interface"], false);
        assert_eq!(config["route"]["rules"][0]["action"], "hijack-dns");
        assert_eq!(config["route"]["rules"][0]["inbound"][0], DNS_IN);
    }

    #[test]
    fn test_inserts_hijack_dns_at_head_when_missing() {
        let config = overlay(json!({
            "route": {
                "rules": [
                    { "protocol": "bittorrent", "outbound": "直连" }
                ]
            }
        }));

        let rules = config["route"]["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0]["action"], "hijack-dns");
        assert_eq!(rules[0]["inbound"][0], DNS_IN);
        assert_eq!(rules[1]["protocol"], "bittorrent");
    }

    #[test]
    fn test_does_not_duplicate_existing_dns_in_hijack() {
        let config = overlay(json!({
            "route": {
                "rules": [
                    {
                        "inbound": ["dns-in", "other-in"],
                        "action": "hijack-dns"
                    }
                ]
            }
        }));

        let rules = config["route"]["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0]["inbound"][0], DNS_IN);
    }

    #[test]
    fn test_load_and_write_same_path_does_not_clobber_source_before_read() {
        let path = std::env::temp_dir().join(format!(
            "canto_overlay_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &path,
            r#"{"inbounds":[{"type":"tun","tag":"tun-in"}],"outbounds":[{"tag":"直连","type":"direct"}]}"#,
        )
        .unwrap();

        let source = load_source(&path).unwrap();
        let overlayed = apply_runtime_overlay(source, &NetworkSettings::default()).unwrap();
        write_runtime_config(&overlayed, &path).unwrap();

        let roundtrip: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(roundtrip["inbounds"][0]["tag"], MIXED_IN);
        assert_eq!(roundtrip["outbounds"][0]["tag"], "直连");
        assert_eq!(roundtrip["route"]["default_mark"], 0x67890);
    }

    #[test]
    fn test_load_source_rejects_missing_and_non_object() {
        let err = load_source(Path::new("")).unwrap_err().to_string();
        assert!(err.contains("source is required"));

        let missing = load_source(Path::new("/tmp/canto-does-not-exist.json"))
            .unwrap_err()
            .to_string();
        assert!(missing.contains("Failed to read source config"));

        let path =
            std::env::temp_dir().join(format!("canto_overlay_array_{}.json", std::process::id()));
        fs::write(&path, "[1, 2, 3]").unwrap();
        let err = load_source(&path).unwrap_err().to_string();
        fs::remove_file(&path).ok();
        assert!(err.contains("must be a JSON object"));
    }
}
