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
    pub template: TemplateSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct SingBoxSettings {
    pub binary: PathBuf,
    pub config_path: PathBuf,
    pub api_listen: String,
}

impl Default for SingBoxSettings {
    fn default() -> Self {
        Self {
            binary: PathBuf::from("sing-box"),
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
    pub enabled: bool,
    pub mode: ProxyMode,
    pub tproxy_port: u16,
    pub dns_port: u16,
    pub mixed_port: u16,
    pub routing_mark: u32,
    pub table_id: u32,
    pub tun_interface: String,
    pub bypass_cn_ips: bool,
    pub bypass_reserved_ips: bool,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: ProxyMode::Tproxy,
            tproxy_port: 7893,
            dns_port: 1053,
            mixed_port: 7890,
            routing_mark: 0x67890,
            table_id: 100,
            tun_interface: "tun0".to_string(),
            bypass_cn_ips: true,
            bypass_reserved_ips: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSettings {
    pub enabled: bool,
    pub template_dir: PathBuf,
    pub files: Vec<String>,
    pub direct_domains: Vec<String>,
}

impl Default for TemplateSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            template_dir: PathBuf::from("./templates"),
            files: vec![
                "log.json".to_string(),
                "experimental.json".to_string(),
                "dns.json".to_string(),
                "inbounds.json".to_string(),
                "outbounds.json".to_string(),
                "route.json".to_string(),
            ],
            direct_domains: vec![
                "sensorsdata.cn".to_string(),
                "courier.push.apple.com".to_string(),
                "opencode.ai".to_string(),
            ],
        }
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
