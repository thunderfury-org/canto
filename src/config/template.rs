use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::error::{CantoError, Result};

pub struct TemplateEngine {
    direct_domains: Vec<String>,
}

impl TemplateEngine {
    pub fn new(direct_domains: Vec<String>) -> Self {
        Self { direct_domains }
    }

    /// Recursively deep merges `override_val` into `base`.
    /// When both values are Objects, keys are merged recursively.
    /// In all other cases, `override_val` replaces `base`.
    pub fn deep_merge(base: &mut Value, override_val: Value) {
        match (base, override_val) {
            (Value::Object(base_map), Value::Object(override_map)) => {
                for (key, val) in override_map {
                    Self::deep_merge(base_map.entry(key).or_insert(Value::Null), val);
                }
            }
            (base_slot, override_val) => {
                *base_slot = override_val;
            }
        }
    }

    /// Loads multiple JSON template files and deep merges them in order
    pub fn merge_files(&self, template_files: &[PathBuf]) -> Result<Value> {
        let mut merged = Value::Object(serde_json::Map::new());

        for path in template_files {
            let content = fs::read_to_string(path).map_err(|e| {
                CantoError::Config(format!(
                    "Failed to read template file '{}': {e}",
                    path.display()
                ))
            })?;

            let json_val: Value = serde_json::from_str(&content).map_err(|e| {
                CantoError::Config(format!(
                    "Failed to parse JSON template in '{}': {e}",
                    path.display()
                ))
            })?;

            Self::deep_merge(&mut merged, json_val);
        }

        self.apply_transformations(&mut merged)?;
        Ok(merged)
    }

    /// Injects shared direct domains into route and DNS anchor rules
    fn apply_transformations(&self, config: &mut Value) -> Result<()> {
        if self.direct_domains.is_empty() {
            return Ok(());
        }

        let domain_values: Vec<Value> = self
            .direct_domains
            .iter()
            .map(|d| Value::String(d.clone()))
            .collect();

        // 1. Injects into route rules with outbound == "直连" and domain_suffix == []
        if let Some(rules) = config
            .get_mut("route")
            .and_then(|r| r.get_mut("rules"))
            .and_then(|rules| rules.as_array_mut())
        {
            for rule in rules {
                if let Some(outbound) = rule.get("outbound").and_then(|o| o.as_str()) {
                    let is_direct = outbound == "直连" || outbound == "direct";
                    let is_empty_domain_suffix = rule
                        .get("domain_suffix")
                        .and_then(|ds| ds.as_array())
                        .map(|arr| arr.is_empty())
                        .unwrap_or(false);

                    if is_direct && is_empty_domain_suffix {
                        rule["domain_suffix"] = Value::Array(domain_values.clone());
                    }
                }
            }
        }

        // 2. Injects into dns rules with server == "dns_direct" and domain_suffix == []
        if let Some(dns_rules) = config
            .get_mut("dns")
            .and_then(|d| d.get_mut("rules"))
            .and_then(|rules| rules.as_array_mut())
        {
            for rule in dns_rules {
                let is_direct_dns = rule
                    .get("server")
                    .and_then(|s| s.as_str())
                    .map(|s| s == "dns_direct")
                    .unwrap_or(false);

                let is_empty_domain_suffix = rule
                    .get("domain_suffix")
                    .and_then(|ds| ds.as_array())
                    .map(|arr| arr.is_empty())
                    .unwrap_or(false);

                if is_direct_dns && is_empty_domain_suffix {
                    rule["domain_suffix"] = Value::Array(domain_values.clone());
                }
            }
        }

        Ok(())
    }

    /// Writes JSON value to output file with 2-space indentation
    pub fn write_to_file(value: &Value, output_path: &Path) -> Result<()> {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                CantoError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create directory '{}': {e}", parent.display()),
                ))
            })?;
        }

        let formatted = serde_json::to_string_pretty(value)?;
        fs::write(output_path, formatted)?;
        info!(
            "Generated sing-box configuration at {}",
            output_path.display()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deep_merge_objects() {
        let mut base = json!({
            "log": {
                "level": "info",
                "timestamp": true
            },
            "inbounds": [
                {"type": "mixed", "listen_port": 7890}
            ]
        });

        let override_val = json!({
            "log": {
                "level": "debug"
            },
            "experimental": {
                "clash_api": {
                    "external_controller": "127.0.0.1:9090"
                }
            }
        });

        TemplateEngine::deep_merge(&mut base, override_val);

        assert_eq!(base["log"]["level"], "debug");
        assert_eq!(base["log"]["timestamp"], true);
        assert_eq!(
            base["experimental"]["clash_api"]["external_controller"],
            "127.0.0.1:9090"
        );
        assert_eq!(base["inbounds"][0]["listen_port"], 7890);
    }

    #[test]
    fn test_direct_domains_injection() {
        let engine = TemplateEngine::new(vec!["test.domain.com".to_string()]);
        let mut config = json!({
            "route": {
                "rules": [
                    {
                        "outbound": "直连",
                        "domain_suffix": []
                    }
                ]
            },
            "dns": {
                "rules": [
                    {
                        "server": "dns_direct",
                        "domain_suffix": []
                    }
                ]
            }
        });

        engine.apply_transformations(&mut config).unwrap();

        assert_eq!(
            config["route"]["rules"][0]["domain_suffix"][0],
            "test.domain.com"
        );
        assert_eq!(
            config["dns"]["rules"][0]["domain_suffix"][0],
            "test.domain.com"
        );
    }
}
