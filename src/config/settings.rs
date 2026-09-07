use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::error::{CantoError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    pub canto: CantoSettings,
    pub singbox: SingBoxSettings,
    pub network: NetworkSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CantoSettings {
    pub work_dir: PathBuf,
}

impl Default for CantoSettings {
    fn default() -> Self {
        Self {
            work_dir: PathBuf::from("./run"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SingBoxSettings {
    pub binary: PathBuf,
    pub source: PathBuf,
    pub config_path: PathBuf,
}

impl Default for SingBoxSettings {
    fn default() -> Self {
        Self {
            binary: PathBuf::from("sing-box"),
            source: PathBuf::from(".data/config-with-tailscale.json"),
            config_path: PathBuf::from("./run/config.json"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_tproxy_port")]
    pub tproxy_port: u16,
    #[serde(default = "default_dns_port")]
    pub dns_port: u16,
    #[serde(default = "default_mixed_port")]
    pub mixed_port: u16,
    #[serde(default = "default_fwmark")]
    pub fwmark: u32,
    #[serde(default = "default_routing_mark")]
    pub routing_mark: u32,
    #[serde(default)]
    pub lan_cidrs: Vec<String>,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            tproxy_port: 7893,
            dns_port: 1053,
            mixed_port: 7890,
            fwmark: default_fwmark(),
            routing_mark: 0x67890,
            lan_cidrs: Vec::new(),
        }
    }
}

fn default_fwmark() -> u32 {
    0x67891
}

fn default_true() -> bool {
    true
}

fn default_tproxy_port() -> u16 {
    7893
}

fn default_dns_port() -> u16 {
    1053
}

fn default_mixed_port() -> u16 {
    7890
}

fn default_routing_mark() -> u32 {
    0x67890
}

impl NetworkSettings {
    /// Returns whether nftables and policy routing should be applied.
    ///
    /// `--no-network` and `enabled = false` skip capture. Capture is always tproxy.
    pub fn should_apply_capture(&self, no_network: bool) -> Result<bool> {
        if no_network || !self.enabled {
            return Ok(false);
        }
        if self.fwmark == self.routing_mark {
            return Err(CantoError::Config(
                "network.fwmark and network.routing_mark must be different".to_string(),
            ));
        }
        Ok(true)
    }
}

impl Settings {
    /// Load settings from an explicit file or standard search paths
    pub fn load(custom_path: Option<&Path>) -> Result<Self> {
        if let Some(path) = custom_path {
            return Self::from_file(path);
        }

        // Standard lookup order
        let candidate_paths = [
            PathBuf::from("canto.toml"),
            PathBuf::from("/etc/canto/canto.toml"),
        ];

        for path in &candidate_paths {
            if path.exists() {
                info!("Loading configuration from {}", path.display());
                return Self::from_file(path);
            }
        }

        // User config directory lookup
        if let Some(config_dir) = dirs::config_dir() {
            let user_canto_toml = config_dir.join("canto").join("canto.toml");
            if user_canto_toml.exists() {
                info!("Loading configuration from {}", user_canto_toml.display());
                return Self::from_file(&user_canto_toml);
            }
        }

        info!("No configuration file found; using defaults");
        Ok(Self::default())
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|e| {
            CantoError::Config(format!(
                "Failed to read configuration file at '{}': {e}",
                path.display()
            ))
        })?;

        let settings: Self = toml::from_str(&content).map_err(|e| {
            CantoError::Config(format!(
                "Failed to parse TOML configuration at '{}': {e}",
                path.display()
            ))
        })?;

        Ok(settings)
    }

    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string_pretty(self)
            .map_err(|e| CantoError::Config(format!("Failed to serialize settings to TOML: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_apply_capture_modes() {
        let mut network = NetworkSettings::default();
        assert!(network.should_apply_capture(false).unwrap());
        assert!(!network.should_apply_capture(true).unwrap());

        network.enabled = false;
        assert!(!network.should_apply_capture(false).unwrap());

        network.enabled = true;
        network.fwmark = network.routing_mark;
        let err = network.should_apply_capture(false).unwrap_err().to_string();
        assert!(err.contains("must be different"));
    }

    #[test]
    fn test_ignores_removed_legacy_toml_keys() {
        let settings: Settings = toml::from_str(
            r#"
[canto]
work_dir = "./run"
log_level = "debug"

[singbox]
binary = "sing-box"
source = "./upstream.json"
config_path = "./run/config.json"
api_listen = "127.0.0.1:9090"

[network]
enabled = true
mode = "tun"
tproxy_port = 7893
dns_port = 1053
mixed_port = 7890
fwmark = 424081
routing_mark = 424080
tun_interface = "tun0"
bypass_cn_ips = false
bypass_reserved_ips = false
lan_cidrs = ["192.168.100.0/24"]
"#,
        )
        .unwrap();

        assert_eq!(settings.canto.work_dir, PathBuf::from("./run"));
        assert_eq!(settings.singbox.source, PathBuf::from("./upstream.json"));
        assert!(settings.network.enabled);
        assert_eq!(settings.network.tproxy_port, 7893);
        assert_eq!(
            settings.network.lan_cidrs,
            vec!["192.168.100.0/24".to_string()]
        );
        assert!(settings.network.should_apply_capture(false).unwrap());
    }
}
