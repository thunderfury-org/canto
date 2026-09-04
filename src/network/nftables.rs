#[cfg(target_os = "linux")]
use std::process::Command;
use tracing::debug;
#[cfg(target_os = "linux")]
use tracing::{info, warn};

use crate::config::NetworkSettings;
#[cfg(target_os = "linux")]
use crate::error::CantoError;
use crate::error::Result;

pub const TABLE_NAME: &str = "canto";
pub const TABLE_FAMILY: &str = "inet";

pub struct NftablesManager<'a> {
    settings: &'a NetworkSettings,
}

impl<'a> NftablesManager<'a> {
    pub fn new(settings: &'a NetworkSettings) -> Self {
        Self { settings }
    }

    /// Generates the complete nftables configuration for LAN + local tproxy.
    pub fn generate_ruleset(&self) -> String {
        let tproxy_port = self.settings.tproxy_port;
        let dns_port = self.settings.dns_port;
        let mixed_port = self.settings.mixed_port;
        let fwmark = self.settings.fwmark;
        let routing_mark = self.settings.routing_mark;
        let lan_cidrs = self.lan_cidrs().join(",\n            ");

        format!(
            r#"table inet {TABLE_NAME} {{
    set reserved_ipv4 {{
        type ipv4_addr
        flags interval
        elements = {{
            0.0.0.0/8,
            10.0.0.0/8,
            100.64.0.0/10,
            127.0.0.0/8,
            169.254.0.0/16,
            172.16.0.0/12,
            192.0.0.0/24,
            192.0.2.0/24,
            192.88.99.0/24,
            192.168.0.0/16,
            198.51.100.0/24,
            203.0.113.0/24,
            224.0.0.0/4,
            240.0.0.0/4
        }}
    }}

    set lan_ipv4 {{
        type ipv4_addr
        flags interval
        elements = {{
            {lan_cidrs}
        }}
    }}

    chain dns_prerouting {{
        type nat hook prerouting priority -110; policy accept;
        meta nfproto ipv6 return
        meta mark {routing_mark} return
        meta mark {fwmark} return
        ip saddr != @lan_ipv4 return
        meta l4proto {{ tcp, udp }} th dport 53 redirect to :{dns_port}
    }}

    chain dns_output {{
        type nat hook output priority -110; policy accept;
        meta nfproto ipv6 return
        meta mark {routing_mark} return
        meta mark {fwmark} return
        meta l4proto {{ tcp, udp }} th dport 53 redirect to :{dns_port}
    }}

    chain tproxy_prerouting {{
        type filter hook prerouting priority mangle - 10; policy accept;
        meta nfproto ipv6 return
        meta mark {routing_mark} return
        meta mark {fwmark} return
        ip saddr != @lan_ipv4 return
        ip daddr @reserved_ipv4 return
        meta l4proto {{ tcp, udp }} th dport 53 return
        meta l4proto {{ tcp, udp }} tproxy to :{tproxy_port} meta mark set {fwmark} accept
    }}

    chain tproxy_output {{
        type route hook output priority mangle - 10; policy accept;
        meta nfproto ipv6 return
        meta mark {routing_mark} return
        meta mark {fwmark} return
        ip daddr @reserved_ipv4 return
        meta l4proto {{ tcp, udp }} th dport 53 return
        meta l4proto {{ tcp, udp }} meta mark set {fwmark}
    }}

    chain tproxy_mark_out {{
        type filter hook prerouting priority mangle; policy accept;
        meta nfproto ipv6 return
        meta mark {fwmark} meta l4proto {{ tcp, udp }} tproxy to :{tproxy_port} accept
    }}

    chain input_protect {{
        type filter hook input priority filter; policy accept;
        iif "lo" accept
        ip saddr @lan_ipv4 accept
        tcp dport {{ {mixed_port}, {tproxy_port}, {dns_port} }} reject
        udp dport {{ {mixed_port}, {tproxy_port}, {dns_port} }} reject
    }}
}}
"#
        )
    }

    fn lan_cidrs(&self) -> Vec<String> {
        if self.settings.lan_cidrs.is_empty() {
            crate::network::lan::fallback_lan_cidrs()
        } else {
            self.settings.lan_cidrs.clone()
        }
    }

    /// Applies the transparent proxy ruleset
    pub fn apply(&self) -> Result<()> {
        let rules = self.generate_ruleset();

        #[cfg(target_os = "linux")]
        {
            info!("Applying nftables ruleset for table 'inet {TABLE_NAME}'");
            self.flush()?;

            let mut child = Command::new("nft")
                .arg("-f")
                .arg("-")
                .stdin(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| CantoError::Network(format!("Failed to execute nft command: {e}")))?;

            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin.write_all(rules.as_bytes())?;
            }

            let output = child.wait_with_output()?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(CantoError::CommandFailed {
                    command: "nft -f -".to_string(),
                    code: output.status.code().unwrap_or(-1),
                    stderr: stderr.to_string(),
                });
            }

            info!("nftables ruleset applied successfully");
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            debug!("Non-Linux system detected. Generated nftables rules:\n{rules}");
            debug!("Skipping nftables application on non-Linux OS");
            Ok(())
        }
    }

    /// Flushes and removes the canto table from nftables
    pub fn flush(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            debug!("Deleting nftables table 'inet {TABLE_NAME}'");
            let output = Command::new("nft")
                .args(["delete", "table", TABLE_FAMILY, TABLE_NAME])
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    info!("Removed nftables table 'inet {TABLE_NAME}'");
                }
                Ok(_) => {
                    debug!("Table 'inet {TABLE_NAME}' did not exist or already removed");
                }
                Err(e) => {
                    warn!("Failed to invoke nft delete: {e}");
                }
            }
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            debug!("Skipping nftables cleanup on non-Linux OS");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NetworkSettings;

    fn ruleset() -> String {
        NftablesManager::new(&NetworkSettings::default()).generate_ruleset()
    }

    #[test]
    fn test_generates_lan_and_local_tproxy_rules() {
        let rules = ruleset();
        let defaults = NetworkSettings::default();
        let fwmark = defaults.fwmark;
        let routing_mark = defaults.routing_mark;

        assert_ne!(fwmark, routing_mark);
        assert!(rules.contains("set reserved_ipv4"));
        assert!(rules.contains("chain tproxy_prerouting"));
        assert!(rules.contains("chain tproxy_output"));
        assert!(rules.contains("chain tproxy_mark_out"));
        assert!(rules.contains("set lan_ipv4"));
        assert!(rules.contains("ip saddr != @lan_ipv4 return"));
        assert!(rules.contains("ip daddr @reserved_ipv4 return"));
        assert!(rules.contains(&format!("meta mark {routing_mark} return")));
        assert!(rules.contains(&format!("meta mark {fwmark} return")));
        assert!(rules.contains(&format!(
            "meta l4proto {{ tcp, udp }} tproxy to :7893 meta mark set {fwmark} accept"
        )));
        assert!(rules.contains(&format!(
            "meta l4proto {{ tcp, udp }} meta mark set {fwmark}"
        )));
        assert!(rules.contains(&format!(
            "meta mark {fwmark} meta l4proto {{ tcp, udp }} tproxy to :7893 accept"
        )));
        assert!(!rules.contains(&format!(
            "meta mark {routing_mark} meta l4proto {{ tcp, udp }} tproxy to :7893 accept"
        )));
        assert!(!rules.contains(&format!("meta mark set {routing_mark}")));
    }

    #[test]
    fn test_generates_private_and_local_dns_redirect() {
        let rules = ruleset();
        let defaults = NetworkSettings::default();

        assert!(rules.contains("chain dns_prerouting"));
        assert!(rules.contains("chain dns_output"));
        assert!(rules.contains("type nat hook prerouting priority -110"));
        assert!(rules.contains("type nat hook output priority -110"));
        assert!(!rules.contains("priority dstnat"));
        assert!(!rules.contains("255.255.255.255/32"));
        assert!(rules.contains("redirect to :1053"));
        assert!(rules.contains(&format!("meta mark {} return", defaults.routing_mark)));
        assert!(rules.contains(&format!("meta mark {} return", defaults.fwmark)));
    }

    #[test]
    fn test_rejects_wan_access_to_proxy_ports() {
        let rules = ruleset();
        assert!(rules.contains("chain input_protect"));
        assert!(rules.contains("ip saddr @lan_ipv4 accept"));
        assert!(rules.contains("tcp dport { 7890, 7893, 1053 } reject"));
        assert!(rules.contains("udp dport { 7890, 7893, 1053 } reject"));
    }

    #[test]
    fn test_uses_configured_lan_cidrs_as_source_allowlist() {
        let settings = NetworkSettings {
            lan_cidrs: vec!["192.168.5.0/24".to_string()],
            ..NetworkSettings::default()
        };
        let rules = NftablesManager::new(&settings).generate_ruleset();
        assert!(rules.contains("192.168.5.0/24"));
        assert!(rules.contains("ip saddr != @lan_ipv4 return"));
        assert!(!rules.contains("ip saddr != @reserved_ipv4 return"));
    }

    #[test]
    fn test_skips_ipv6_and_does_not_bypass_loopback() {
        let rules = ruleset();
        assert!(rules.contains("meta nfproto ipv6 return"));
        assert!(!rules.contains("iif \"lo\" return"));
        assert!(!rules.contains("iif 'lo' return"));
        assert!(rules.contains("iif \"lo\" accept"));
    }
}
