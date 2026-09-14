use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::error::{CantoError, Result};

pub const COMMON_PORTS: &[u16] = &[
    22, 53, 80, 123, 143, 194, 443, 465, 587, 853, 993, 995, 8080, 8443, 9443,
];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PortsFilter {
    #[default]
    Common,
    All,
    Custom(Vec<u16>),
}

impl PortsFilter {
    pub fn ports(&self) -> Option<Vec<u16>> {
        match self {
            Self::Common => Some(COMMON_PORTS.to_vec()),
            Self::All => None,
            Self::Custom(ports) => Some(ports.clone()),
        }
    }
}

impl Serialize for PortsFilter {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Common => serializer.serialize_str("common"),
            Self::All => serializer.serialize_str("all"),
            Self::Custom(ports) => ports.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for PortsFilter {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PortsFilterVisitor;

        impl<'de> Visitor<'de> for PortsFilterVisitor {
            type Value = PortsFilter;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(r#""common", "all", or a list of port numbers (1-65535)"#)
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<PortsFilter, E>
            where
                E: de::Error,
            {
                match value.to_ascii_lowercase().as_str() {
                    "common" => Ok(PortsFilter::Common),
                    "all" => Ok(PortsFilter::All),
                    other => Err(de::Error::custom(format!(
                        "invalid ports filter '{other}'; expected 'common', 'all', or an array of ports"
                    ))),
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<PortsFilter, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut ports = Vec::new();
                while let Some(port) = seq.next_element::<u16>()? {
                    if port == 0 {
                        return Err(de::Error::custom("port number 0 is invalid"));
                    }
                    if !ports.contains(&port) {
                        ports.push(port);
                    }
                }
                if ports.is_empty() {
                    return Err(de::Error::custom("ports list must not be empty"));
                }
                ports.sort_unstable();
                Ok(PortsFilter::Custom(ports))
            }
        }

        deserializer.deserialize_any(PortsFilterVisitor)
    }
}

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
    pub source: String,
    pub config_path: PathBuf,
    #[serde(default = "default_refresh_interval_secs")]
    pub refresh_interval_secs: u64,
}

impl Default for SingBoxSettings {
    fn default() -> Self {
        Self {
            binary: PathBuf::from("sing-box"),
            source: String::from(".data/config-with-tailscale.json"),
            config_path: PathBuf::from("./run/config.json"),
            refresh_interval_secs: default_refresh_interval_secs(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkMode {
    #[default]
    Tun,
    Tproxy,
}

impl NetworkMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tun => "tun",
            Self::Tproxy => "tproxy",
        }
    }

    pub fn is_tun(self) -> bool {
        matches!(self, Self::Tun)
    }
}

impl Serialize for NetworkMode {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for NetworkMode {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.to_ascii_lowercase().as_str() {
            "tun" => Ok(Self::Tun),
            "tproxy" => Ok(Self::Tproxy),
            other => Err(de::Error::custom(format!(
                "invalid network.mode '{other}'; expected 'tun' or 'tproxy'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub mode: NetworkMode,
    #[serde(default = "default_true")]
    pub bypass_cn: bool,
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

    // Scope controls
    #[serde(default = "default_true")]
    pub lan: bool,
    #[serde(default = "default_true")]
    pub local: bool,
    #[serde(default = "default_false")]
    pub docker: bool,

    // Traffic controls
    #[serde(default = "default_true")]
    pub tcp: bool,
    #[serde(default = "default_false")]
    pub udp: bool,
    #[serde(default)]
    pub ports: PortsFilter,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: NetworkMode::Tun,
            bypass_cn: true,
            tproxy_port: 7893,
            dns_port: 1053,
            mixed_port: 7890,
            fwmark: default_fwmark(),
            routing_mark: 0x67890,
            lan_cidrs: Vec::new(),
            lan: true,
            local: true,
            docker: false,
            tcp: true,
            udp: false,
            ports: PortsFilter::Common,
        }
    }
}

fn default_fwmark() -> u32 {
    0x67891
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
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

fn default_refresh_interval_secs() -> u64 {
    24 * 60 * 60
}

impl NetworkSettings {
    pub fn validate(&self) -> Result<()> {
        if self.mode == NetworkMode::Tun && !self.local {
            return Err(CantoError::Config(
                "network.local = false is only supported with network.mode = \"tproxy\""
                    .to_string(),
            ));
        }
        if self.mode == NetworkMode::Tproxy && self.bypass_cn {
            return Err(CantoError::Config(
                "network.bypass_cn requires TUN + auto_redirect; set network.mode = \"tun\" or bypass_cn = false".to_string(),
            ));
        }
        if self.mode == NetworkMode::Tproxy && self.fwmark == self.routing_mark {
            return Err(CantoError::Config(
                "network.fwmark and network.routing_mark must be different".to_string(),
            ));
        }
        if !self.lan && !self.local && !self.docker {
            return Err(CantoError::Config(
                "at least one of network.lan, network.local, or network.docker must be enabled when network.enabled is true".to_string(),
            ));
        }
        if !self.tcp && !self.udp {
            return Err(CantoError::Config(
                "at least one of network.tcp or network.udp must be enabled when network.enabled is true".to_string(),
            ));
        }
        Ok(())
    }

    /// Returns whether network capture should be applied.
    ///
    /// `--no-network` and `enabled = false` skip capture. TUN path only enables
    /// forwarding and leftover cleanup; tproxy still installs nftables.
    pub fn should_apply_capture(&self, no_network: bool) -> Result<bool> {
        if no_network || !self.enabled {
            return Ok(false);
        }
        self.validate()?;
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

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct TestPortsWrapper {
        ports: PortsFilter,
    }

    #[test]
    fn test_should_apply_capture_modes() {
        let mut network = NetworkSettings::default();
        assert!(network.should_apply_capture(false).unwrap());
        assert!(!network.should_apply_capture(true).unwrap());

        network.enabled = false;
        assert!(!network.should_apply_capture(false).unwrap());

        network.enabled = true;
        network.mode = NetworkMode::Tproxy;
        network.bypass_cn = false;
        network.fwmark = network.routing_mark;
        let err = network.should_apply_capture(false).unwrap_err().to_string();
        assert!(err.contains("must be different"));

        network.fwmark = 424081;
        network.lan = false;
        network.local = false;
        network.docker = false;
        let err = network.should_apply_capture(false).unwrap_err().to_string();
        assert!(err.contains("network.lan"));

        network.lan = true;
        network.tcp = false;
        network.udp = false;
        let err = network.should_apply_capture(false).unwrap_err().to_string();
        assert!(err.contains("network.tcp"));
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
        assert_eq!(settings.singbox.source, "./upstream.json");
        assert_eq!(settings.singbox.refresh_interval_secs, 24 * 60 * 60);
        assert!(settings.network.enabled);
        assert_eq!(settings.network.tproxy_port, 7893);
        assert_eq!(
            settings.network.lan_cidrs,
            vec!["192.168.100.0/24".to_string()]
        );
        assert!(settings.network.lan);
        assert!(settings.network.local);
        assert!(!settings.network.docker);
        assert!(settings.network.tcp);
        assert!(!settings.network.udp);
        assert_eq!(settings.network.ports, PortsFilter::Common);
        assert_eq!(settings.network.mode, NetworkMode::Tun);
        assert!(settings.network.bypass_cn);
        assert!(settings.network.should_apply_capture(false).unwrap());
    }

    #[test]
    fn test_rejects_invalid_mode_and_tun_local_false() {
        let err = toml::from_str::<Settings>(
            r#"
[network]
mode = "redirect"
"#,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("invalid network.mode"), "{err}");

        let network = NetworkSettings {
            local: false,
            ..NetworkSettings::default()
        };
        let err = network.validate().unwrap_err().to_string();
        assert!(err.contains("network.local = false"), "{err}");

        let network = NetworkSettings {
            mode: NetworkMode::Tproxy,
            bypass_cn: true,
            ..NetworkSettings::default()
        };
        let err = network.validate().unwrap_err().to_string();
        assert!(err.contains("bypass_cn"), "{err}");
    }

    #[test]
    fn test_parses_custom_scope_and_ports_config() {
        let settings: Settings = toml::from_str(
            r#"
[network]
lan = true
local = false
docker = false
tcp = true
udp = true
ports = [80, 443, 8080]
"#,
        )
        .unwrap();
        assert!(settings.network.lan);
        assert!(!settings.network.local);
        assert!(!settings.network.docker);
        assert!(settings.network.tcp);
        assert!(settings.network.udp);
        assert_eq!(
            settings.network.ports,
            PortsFilter::Custom(vec![80, 443, 8080])
        );
        assert_eq!(settings.network.ports.ports(), Some(vec![80, 443, 8080]));
    }

    #[test]
    fn test_ports_filter_serialization_roundtrip() {
        let common = TestPortsWrapper {
            ports: PortsFilter::Common,
        };
        let toml_common = toml::to_string(&common).unwrap();
        assert!(toml_common.contains(r#"ports = "common""#));
        let parsed_common: TestPortsWrapper = toml::from_str(&toml_common).unwrap();
        assert_eq!(parsed_common.ports, PortsFilter::Common);

        let all = TestPortsWrapper {
            ports: PortsFilter::All,
        };
        let toml_all = toml::to_string(&all).unwrap();
        assert!(toml_all.contains(r#"ports = "all""#));
        let parsed_all: TestPortsWrapper = toml::from_str(&toml_all).unwrap();
        assert_eq!(parsed_all.ports, PortsFilter::All);

        let custom = TestPortsWrapper {
            ports: PortsFilter::Custom(vec![80, 443]),
        };
        let toml_custom = toml::to_string(&custom).unwrap();
        let parsed_custom: TestPortsWrapper = toml::from_str(&toml_custom).unwrap();
        assert_eq!(parsed_custom.ports, PortsFilter::Custom(vec![80, 443]));
    }

    #[test]
    fn test_ports_filter_rejects_invalid_values() {
        let err = toml::from_str::<TestPortsWrapper>(r#"ports = "invalid""#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("invalid ports filter"));

        let err = toml::from_str::<TestPortsWrapper>("ports = []")
            .unwrap_err()
            .to_string();
        assert!(err.contains("ports list must not be empty"));

        let err = toml::from_str::<TestPortsWrapper>("ports = [0, 80]")
            .unwrap_err()
            .to_string();
        assert!(err.contains("port number 0 is invalid"));
    }

    #[test]
    fn test_parses_url_source_and_refresh_interval() {
        let settings: Settings = toml::from_str(
            r#"
[singbox]
source = "https://config.example/source.json"
refresh_interval_secs = 3600
"#,
        )
        .unwrap();
        assert_eq!(
            settings.singbox.source,
            "https://config.example/source.json"
        );
        assert_eq!(settings.singbox.refresh_interval_secs, 3600);
    }
}
