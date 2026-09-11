use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;
use tracing::info;

use crate::config::{NetworkMode, NetworkSettings};
use crate::error::{CantoError, Result};
use crate::network::{TunCapture, resolve_tun_capture};

const MIXED_IN: &str = "mixed-in";
const TPROXY_IN: &str = "tproxy-in";
const DNS_IN: &str = "dns-in";
const TUN_IN: &str = "tun-in";
const TUN_IFACE: &str = "canto";
const TUN_ADDR: &str = "172.19.0.1/30";
const CNIP_TAG: &str = "cnip";
const DEFAULT_DIRECT: &str = "直连";

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

/// Overlays gateway inbounds and routing fields onto a complete source config.
///
/// Keeps dns, outbounds, endpoints, experimental, and log untouched.
pub fn apply_runtime_overlay(config: Value, network: &NetworkSettings) -> Result<Value> {
    apply_runtime_overlay_with_capture(config, network, None)
}

pub fn apply_runtime_overlay_with_capture(
    mut config: Value,
    network: &NetworkSettings,
    capture: Option<&TunCapture>,
) -> Result<Value> {
    if !config.is_object() {
        return Err(CantoError::Config(
            "sing-box source must be a JSON object".to_string(),
        ));
    }
    network.validate()?;

    match network.mode {
        NetworkMode::Tproxy => apply_tproxy_overlay(&mut config, network)?,
        NetworkMode::Tun => {
            let owned = match capture {
                Some(_) => None,
                None => Some(resolve_tun_capture(network, false)?),
            };
            let capture = capture.or(owned.as_ref()).expect("tun capture");
            apply_tun_overlay(&mut config, network, capture)?;
        }
    }
    Ok(config)
}

fn apply_tproxy_overlay(config: &mut Value, network: &NetworkSettings) -> Result<()> {
    config["inbounds"] = tproxy_inbounds(network);
    ensure_route_object(config)?;
    config["route"]["default_mark"] = json!(network.routing_mark);
    config["route"]["auto_detect_interface"] = json!(false);
    insert_dns_in_hijack_if_missing(config)?;
    Ok(())
}

fn apply_tun_overlay(
    config: &mut Value,
    network: &NetworkSettings,
    capture: &TunCapture,
) -> Result<()> {
    if network.bypass_cn && !has_cnip_rule_set(config) {
        return Err(CantoError::Config(
            "network.bypass_cn requires a route.rule_set with tag 'cnip'".to_string(),
        ));
    }

    config["inbounds"] = tun_inbounds(network, capture);
    ensure_route_object(config)?;
    if let Some(route) = config.get_mut("route").and_then(Value::as_object_mut) {
        route.remove("default_mark");
    }
    config["route"]["auto_detect_interface"] = json!(true);
    insert_tun_pre_sniff_rules(config, network)?;
    Ok(())
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

fn tun_inbounds(network: &NetworkSettings, capture: &TunCapture) -> Value {
    let mut tun = Map::new();
    tun.insert("type".to_string(), json!("tun"));
    tun.insert("tag".to_string(), json!(TUN_IN));
    tun.insert("interface_name".to_string(), json!(TUN_IFACE));
    tun.insert("address".to_string(), json!([TUN_ADDR]));
    tun.insert("auto_route".to_string(), json!(true));
    tun.insert("auto_redirect".to_string(), json!(capture.auto_redirect));
    tun.insert("strict_route".to_string(), json!(false));
    if !capture.include_interface.is_empty() {
        tun.insert(
            "include_interface".to_string(),
            json!(capture.include_interface),
        );
    }
    if !capture.exclude_interface.is_empty() {
        tun.insert(
            "exclude_interface".to_string(),
            json!(capture.exclude_interface),
        );
    }

    json!([
        {
            "type": "mixed",
            "tag": MIXED_IN,
            "listen": capture.mixed_listen,
            "listen_port": network.mixed_port
        },
        Value::Object(tun)
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

fn insert_dns_in_hijack_if_missing(config: &mut Value) -> Result<()> {
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

fn insert_tun_pre_sniff_rules(config: &mut Value, network: &NetworkSettings) -> Result<()> {
    let direct = direct_tag(config);
    let mut injected = Vec::new();

    if !route_rules(config).iter().any(is_port_53_hijack) {
        injected.push(json!({
            "port": 53,
            "action": "hijack-dns"
        }));
    }

    if network.bypass_cn {
        let outbound = route_rules(config)
            .iter()
            .find(|rule| is_cnip_rule(rule))
            .and_then(|rule| rule.get("outbound"))
            .and_then(Value::as_str)
            .unwrap_or(&direct)
            .to_string();
        injected.push(json!({
            "rule_set": [CNIP_TAG],
            "action": "bypass",
            "outbound": outbound
        }));
    }

    if !network.udp && !route_rules(config).iter().any(is_udp_bypass) {
        injected.push(json!({
            "network": "udp",
            "action": "bypass",
            "outbound": direct
        }));
    }

    if let Some(ports) = network.ports.ports()
        && !route_rules(config)
            .iter()
            .any(|rule| is_ports_invert_bypass(rule, &ports))
    {
        injected.push(json!({
            "port": ports,
            "invert": true,
            "action": "bypass",
            "outbound": direct
        }));
    }

    let rules = route_rules_mut(config)?;
    if network.bypass_cn {
        rules.retain(|rule| !is_cnip_rule(rule));
    }
    let sniff_at = rules.iter().position(is_sniff_rule).unwrap_or(0);
    for (offset, rule) in injected.into_iter().enumerate() {
        rules.insert(sniff_at + offset, rule);
    }
    Ok(())
}

fn route_rules(config: &Value) -> &[Value] {
    config
        .get("route")
        .and_then(|route| route.get("rules"))
        .and_then(Value::as_array)
        .map(|rules| rules.as_slice())
        .unwrap_or(&[])
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

fn has_cnip_rule_set(config: &Value) -> bool {
    config
        .get("route")
        .and_then(|route| route.get("rule_set"))
        .and_then(Value::as_array)
        .is_some_and(|sets| {
            sets.iter()
                .any(|set| set.get("tag").and_then(Value::as_str) == Some(CNIP_TAG))
        })
}

fn direct_tag(config: &Value) -> String {
    if let Some(outbound) = route_rules(config)
        .iter()
        .find(|rule| is_cnip_rule(rule))
        .and_then(|rule| rule.get("outbound"))
        .and_then(Value::as_str)
    {
        return outbound.to_string();
    }
    if let Some(tag) = config
        .get("outbounds")
        .and_then(Value::as_array)
        .and_then(|outbounds| {
            outbounds.iter().find_map(|outbound| {
                if outbound.get("type").and_then(Value::as_str) == Some("direct") {
                    outbound.get("tag").and_then(Value::as_str)
                } else {
                    None
                }
            })
        })
    {
        return tag.to_string();
    }
    DEFAULT_DIRECT.to_string()
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

fn is_port_53_hijack(rule: &Value) -> bool {
    if rule.get("action").and_then(Value::as_str) != Some("hijack-dns") {
        return false;
    }
    match rule.get("port") {
        Some(Value::Number(port)) => port.as_u64() == Some(53),
        Some(Value::Array(ports)) => ports.len() == 1 && ports[0].as_u64() == Some(53),
        _ => false,
    }
}

fn is_sniff_rule(rule: &Value) -> bool {
    rule.get("action").and_then(Value::as_str) == Some("sniff")
}

fn rule_set_tags(rule: &Value) -> Vec<&str> {
    match rule.get("rule_set") {
        Some(Value::String(tag)) => vec![tag.as_str()],
        Some(Value::Array(tags)) => tags.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    }
}

fn is_cnip_rule(rule: &Value) -> bool {
    let tags = rule_set_tags(rule);
    tags.len() == 1 && tags[0] == CNIP_TAG
}

#[cfg_attr(not(test), allow(dead_code))]
fn is_cnip_bypass(rule: &Value) -> bool {
    is_cnip_rule(rule) && rule.get("action").and_then(Value::as_str) == Some("bypass")
}

fn is_udp_bypass(rule: &Value) -> bool {
    rule.get("action").and_then(Value::as_str) == Some("bypass")
        && rule.get("network").and_then(Value::as_str) == Some("udp")
        && rule.get("port").is_none()
}

fn is_ports_invert_bypass(rule: &Value, ports: &[u16]) -> bool {
    if rule.get("action").and_then(Value::as_str) != Some("bypass") {
        return false;
    }
    if rule.get("invert").and_then(Value::as_bool) != Some(true) {
        return false;
    }
    match rule.get("port") {
        Some(Value::Array(values)) => {
            let parsed: Vec<u16> = values
                .iter()
                .filter_map(|value| value.as_u64().map(|n| n as u16))
                .collect();
            parsed == ports
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{NetworkMode, NetworkSettings, PortsFilter};
    use crate::network::TunCapture;
    use serde_json::json;

    fn tun_network() -> NetworkSettings {
        NetworkSettings {
            bypass_cn: false,
            ..NetworkSettings::default()
        }
    }

    fn tproxy_network() -> NetworkSettings {
        NetworkSettings {
            mode: NetworkMode::Tproxy,
            bypass_cn: false,
            ..NetworkSettings::default()
        }
    }

    fn overlay_tun(source: Value) -> Value {
        apply_runtime_overlay(source, &tun_network()).unwrap()
    }

    fn overlay_tproxy(source: Value) -> Value {
        apply_runtime_overlay(source, &tproxy_network()).unwrap()
    }

    fn cnip_source() -> Value {
        json!({
            "inbounds": [{
                "type": "tun",
                "tag": "tun-in",
                "interface_name": "sing-box-utun",
                "address": ["192.168.255.1/30"],
                "auto_route": true,
                "strict_route": true
            }],
            "outbounds": [{ "tag": "直连", "type": "direct" }],
            "route": {
                "rule_set": [{
                    "tag": "cnip",
                    "type": "inline",
                    "rules": [{ "ip_cidr": ["1.2.3.3/32"] }]
                }],
                "rules": [
                    { "ip_cidr": ["192.168.5.0/24"], "outbound": "ts-ep" },
                    { "action": "sniff" },
                    { "rule_set": ["private"], "outbound": "直连" },
                    { "rule_set": ["cnip"], "outbound": "直连" },
                    { "rule_set": ["cn"], "outbound": "直连" }
                ]
            }
        })
    }

    #[test]
    fn test_replaces_desktop_tun_with_gateway_tun() {
        let capture = TunCapture {
            mixed_listen: "192.168.100.1".to_string(),
            include_interface: vec!["veth-lan-gw".to_string()],
            exclude_interface: vec!["docker0".to_string()],
            auto_redirect: true,
        };
        let config = apply_runtime_overlay_with_capture(
            cnip_source(),
            &NetworkSettings::default(),
            Some(&capture),
        )
        .unwrap();
        let inbounds = config["inbounds"].as_array().unwrap();
        assert_eq!(inbounds.len(), 2);
        assert_eq!(inbounds[0]["tag"], MIXED_IN);
        assert_eq!(inbounds[0]["listen"], "192.168.100.1");
        assert_eq!(inbounds[1]["tag"], TUN_IN);
        assert_eq!(inbounds[1]["type"], "tun");
        assert_eq!(inbounds[1]["interface_name"], TUN_IFACE);
        assert_eq!(inbounds[1]["address"][0], TUN_ADDR);
        assert_eq!(inbounds[1]["auto_route"], true);
        assert_eq!(inbounds[1]["auto_redirect"], true);
        assert_eq!(inbounds[1]["strict_route"], false);
        assert_eq!(inbounds[1]["include_interface"][0], "veth-lan-gw");
        assert_eq!(inbounds[1]["exclude_interface"][0], "docker0");
        assert!(config["route"].get("default_mark").is_none());
        assert_eq!(config["route"]["auto_detect_interface"], true);
        assert!(
            inbounds
                .iter()
                .all(|inbound| inbound["tag"] != "dns-in" && inbound["type"] != "tproxy")
        );
    }

    #[test]
    fn test_moves_cnip_bypass_before_sniff() {
        let config = apply_runtime_overlay(cnip_source(), &NetworkSettings::default()).unwrap();
        let rules = config["route"]["rules"].as_array().unwrap();
        let sniff = rules.iter().position(is_sniff_rule).unwrap();
        let cnip = rules.iter().position(is_cnip_bypass).unwrap();
        assert!(cnip < sniff);
        assert!(rules.iter().any(|rule| {
            rule.get("rule_set")
                .and_then(Value::as_array)
                .is_some_and(|tags| tags.iter().any(|tag| tag == "cn"))
                && rule.get("outbound") == Some(&json!("直连"))
                && rule.get("action").is_none()
        }));
        assert_eq!(rules.iter().filter(|rule| is_cnip_rule(rule)).count(), 1);
        assert!(rules.iter().any(is_port_53_hijack));
        assert!(rules.iter().any(is_udp_bypass));
        assert!(
            rules
                .iter()
                .any(|rule| is_ports_invert_bypass(rule, &PortsFilter::Common.ports().unwrap()))
        );
    }

    #[test]
    fn test_bypass_cn_false_leaves_cnip_outbound() {
        let config = overlay_tun(cnip_source());
        let rules = config["route"]["rules"].as_array().unwrap();
        assert!(!rules.iter().any(is_cnip_bypass));
        assert!(
            rules.iter().any(|rule| {
                is_cnip_rule(rule) && rule.get("outbound") == Some(&json!("直连"))
            })
        );
    }

    #[test]
    fn test_missing_cnip_rule_set_errors_when_bypass_cn() {
        let err = apply_runtime_overlay(
            json!({ "outbounds": [{ "tag": "直连", "type": "direct" }] }),
            &NetworkSettings::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("cnip"), "{err}");
    }

    #[test]
    fn test_local_only_disables_auto_redirect() {
        let network = NetworkSettings {
            lan: false,
            docker: false,
            ..tun_network()
        };
        let config = apply_runtime_overlay(
            json!({ "outbounds": [{ "tag": "直连", "type": "direct" }] }),
            &network,
        )
        .unwrap();
        assert_eq!(config["inbounds"][0]["listen"], "127.0.0.1");
        assert_eq!(config["inbounds"][1]["auto_redirect"], false);
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

        let config = overlay_tproxy(source);
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
    fn test_tproxy_with_bypass_cn_is_rejected() {
        let network = NetworkSettings {
            bypass_cn: true,
            ..tproxy_network()
        };
        let err = apply_runtime_overlay(json!({}), &network)
            .unwrap_err()
            .to_string();
        assert!(err.contains("bypass_cn"), "{err}");
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

        let config = overlay_tun(source);
        assert_eq!(config["log"]["level"], "warn");
        assert_eq!(config["dns"]["final"], "dns_direct");
        assert_eq!(config["outbounds"][0]["tag"], "直连");
        assert_eq!(config["endpoints"][0]["tag"], "ts-ep");
        assert_eq!(
            config["experimental"]["clash_api"]["external_controller"],
            "127.0.0.1:9090"
        );
        assert_eq!(config["route"]["final"], "默认策略");
        assert!(
            config["route"]["rules"]
                .as_array()
                .unwrap()
                .iter()
                .any(|rule| rule.get("protocol") == Some(&json!("dns")))
        );
    }

    #[test]
    fn test_writes_default_mark_and_disables_auto_detect() {
        let config = overlay_tproxy(json!({
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
        let config = overlay_tproxy(json!({
            "outbounds": [{ "tag": "直连", "type": "direct" }]
        }));

        assert_eq!(config["route"]["default_mark"], 0x67890);
        assert_eq!(config["route"]["auto_detect_interface"], false);
        assert_eq!(config["route"]["rules"][0]["action"], "hijack-dns");
        assert_eq!(config["route"]["rules"][0]["inbound"][0], DNS_IN);
    }

    #[test]
    fn test_inserts_hijack_dns_at_head_when_missing() {
        let config = overlay_tproxy(json!({
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
        let config = overlay_tproxy(json!({
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
        let overlayed = apply_runtime_overlay(source, &tun_network()).unwrap();
        write_runtime_config(&overlayed, &path).unwrap();

        let roundtrip: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(roundtrip["inbounds"][0]["tag"], MIXED_IN);
        assert_eq!(roundtrip["inbounds"][1]["tag"], TUN_IN);
        assert_eq!(roundtrip["outbounds"][0]["tag"], "直连");
        assert!(roundtrip["route"].get("default_mark").is_none());
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
