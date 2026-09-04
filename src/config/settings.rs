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
    pub log_level: String,
}

impl Default for CantoSettings {
    fn default() -> Self {
        Self {
            work_dir: PathBuf::from("./run"),
            log_level: "info".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SingBoxSettings {
    pub binary: PathBuf,
    pub source: PathBuf,
    pub config_path: PathBuf,
    pub api_listen: String,
}

impl Default for SingBoxSettings {
    fn default() -> Self {
        Self {
            binary: PathBuf::from("sing-box"),
            source: PathBuf::from(".data/config-with-tailscale.json"),
            config_path: PathBuf::from("./run/config.json"),
            api_listen: "127.0.0.1:9090".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ProxyMode {
    #[default]
    Tproxy,
    Tun,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub mode: ProxyMode,
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
    #[serde(default = "default_tun_interface")]
    pub tun_interface: String,
    #[serde(default = "default_true")]
    pub bypass_cn_ips: bool,
    #[serde(default = "default_true")]
    pub bypass_reserved_ips: bool,
    #[serde(default)]
    pub lan_cidrs: Vec<String>,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: ProxyMode::Tproxy,
            tproxy_port: 7893,
            dns_port: 1053,
            mixed_port: 7890,
            fwmark: default_fwmark(),
            routing_mark: 0x67890,
            tun_interface: "tun0".to_string(),
            bypass_cn_ips: true,
            bypass_reserved_ips: true,
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

fn default_tun_interface() -> String {
    "tun0".to_string()
}

impl NetworkSettings {
    /// Returns whether nftables and policy routing should be applied.
    ///
    /// `mode = "tun"` is rejected. `--no-network`, `enabled = false`, and `mode = "none"` skip capture.
    pub fn should_apply_capture(&self, no_network: bool) -> Result<bool> {
        if no_network || !self.enabled || self.mode == ProxyMode::None {
            return Ok(false);
        }
        if self.mode == ProxyMode::Tun {
            return Err(CantoError::Config(
                "v1 only supports tproxy; network.mode = \"tun\" is not implemented".to_string(),
            ));
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
        network.mode = ProxyMode::None;
        assert!(!network.should_apply_capture(false).unwrap());

        network.mode = ProxyMode::Tun;
        let err = network.should_apply_capture(false).unwrap_err().to_string();
        assert!(err.contains("tproxy"));

        network.mode = ProxyMode::Tproxy;
        network.fwmark = network.routing_mark;
        let err = network.should_apply_capture(false).unwrap_err().to_string();
        assert!(err.contains("must be different"));
    }

    #[test]
    fn test_singbox_settings_fill_missing_api_listen() {
        let settings: Settings = toml::from_str(
            r#"
[singbox]
binary = "sing-box"
source = "./upstream.json"
config_path = "./run/config.json"
"#,
        )
        .unwrap();
        assert_eq!(settings.singbox.api_listen, "127.0.0.1:9090");
    }

    #[test]
    fn test_network_settings_fill_missing_unused_fields() {
        let settings: Settings = toml::from_str(
            r#"
[network]
enabled = true
mode = "tproxy"
tproxy_port = 7893
dns_port = 1053
mixed_port = 7890
fwmark = 424081
routing_mark = 424080
lan_cidrs = ["192.168.100.0/24"]
"#,
        )
        .unwrap();
        assert_eq!(settings.network.tun_interface, "tun0");
        assert!(settings.network.bypass_cn_ips);
        assert!(settings.network.enabled);
        assert_eq!(settings.network.tproxy_port, 7893);
    }
}
