#[cfg(target_os = "linux")]
use std::process::Command;
use tracing::debug;
#[cfg(target_os = "linux")]
use tracing::{info, warn};

use crate::config::{NetworkMode, NetworkSettings, PortsFilter};
#[cfg(target_os = "linux")]
use crate::error::CantoError;
use crate::error::Result;

pub const TABLE_NAME: &str = "canto";
pub const TABLE_FAMILY: &str = "inet";
/// Default docker bridge only. Do not glob `br-*`; OpenWrt LAN is `br-lan`.
const DOCKER_IIFNAME: &str = "docker0";

pub struct NftablesManager<'a> {
    settings: &'a NetworkSettings,
}

impl<'a> NftablesManager<'a> {
    pub fn new(settings: &'a NetworkSettings) -> Self {
        Self { settings }
    }

    pub fn dump(&self) -> String {
        if self.settings.mode == NetworkMode::Tun {
            return "# canto TUN path has no nftables ruleset\n".to_string();
        }
        self.generate_ruleset()
    }

    /// Generates the complete nftables configuration based on network settings.
    pub fn generate_ruleset(&self) -> String {
        let tproxy_port = self.settings.tproxy_port;
        let dns_port = self.settings.dns_port;
        let mixed_port = self.settings.mixed_port;
        let fwmark = self.settings.fwmark;
        let routing_mark = self.settings.routing_mark;
        let lan_cidrs = self.lan_cidrs().join(",\n            ");
        let docker_iif = DOCKER_IIFNAME;

        let proxy_ports_set = match self.settings.ports.ports() {
            Some(ports) => {
                let elements = ports
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    r#"    set proxy_ports {{
        type inet_service
        elements = {{ {elements} }}
    }}

"#
                )
            }
            None => String::new(),
        };

        let proto_match = match (self.settings.tcp, self.settings.udp) {
            (true, true) => Some("meta l4proto { tcp, udp }"),
            (true, false) => Some("meta l4proto tcp"),
            (false, true) => Some("meta l4proto udp"),
            (false, false) => None,
        };

        let port_match = match self.settings.ports {
            PortsFilter::All => "",
            _ => " th dport @proxy_ports",
        };

        let capture_prerouting_rule = match proto_match {
            Some(proto) => {
                format!(
                    "{proto}{port_match} tproxy to :{tproxy_port} meta mark set {fwmark} accept"
                )
            }
            None => "return".to_string(),
        };

        let capture_output_rule = match proto_match {
            Some(proto) => format!("{proto}{port_match} meta mark set {fwmark}"),
            None => "return".to_string(),
        };

        let mark_out_rule = match proto_match {
            Some(proto) if self.settings.local => {
                format!("meta mark {fwmark} {proto} tproxy to :{tproxy_port} accept")
            }
            _ => "return".to_string(),
        };

        let (dns_prerouting_extra, dns_prerouting_body) =
            match (self.settings.lan, self.settings.docker) {
                (true, true) => (
                    format!(
                        r#"    chain dns_prerouting_redirect {{
        meta l4proto {{ tcp, udp }} th dport 53 redirect to :{dns_port}
    }}

"#
                    ),
                    format!(
                        r#"        meta nfproto ipv6 return
        meta mark {routing_mark} return
        meta mark {fwmark} return
        iifname "{docker_iif}" goto dns_prerouting_redirect
        ip saddr @lan_ipv4 goto dns_prerouting_redirect
        return"#
                    ),
                ),
                (true, false) => (
                    String::new(),
                    format!(
                        r#"        meta nfproto ipv6 return
        meta mark {routing_mark} return
        meta mark {fwmark} return
        ip saddr != @lan_ipv4 return
        meta l4proto {{ tcp, udp }} th dport 53 redirect to :{dns_port}"#
                    ),
                ),
                (false, true) => (
                    String::new(),
                    format!(
                        r#"        meta nfproto ipv6 return
        meta mark {routing_mark} return
        meta mark {fwmark} return
        iifname != "{docker_iif}" return
        meta l4proto {{ tcp, udp }} th dport 53 redirect to :{dns_port}"#
                    ),
                ),
                (false, false) => (String::new(), "        return".to_string()),
            };

        let (tproxy_prerouting_extra, tproxy_prerouting_body) =
            match (self.settings.lan, self.settings.docker) {
                (true, true) => (
                    format!(
                        r#"    chain tproxy_prerouting_capture {{
        ip daddr @reserved_ipv4 return
        meta l4proto {{ tcp, udp }} th dport 53 return
        {capture_prerouting_rule}
    }}

"#
                    ),
                    format!(
                        r#"        meta nfproto ipv6 return
        meta mark {routing_mark} return
        meta mark {fwmark} return
        iifname "{docker_iif}" goto tproxy_prerouting_capture
        ip saddr @lan_ipv4 goto tproxy_prerouting_capture
        return"#
                    ),
                ),
                (true, false) => (
                    String::new(),
                    format!(
                        r#"        meta nfproto ipv6 return
        meta mark {routing_mark} return
        meta mark {fwmark} return
        ip saddr != @lan_ipv4 return
        ip daddr @reserved_ipv4 return
        meta l4proto {{ tcp, udp }} th dport 53 return
        {capture_prerouting_rule}"#
                    ),
                ),
                (false, true) => (
                    String::new(),
                    format!(
                        r#"        meta nfproto ipv6 return
        meta mark {routing_mark} return
        meta mark {fwmark} return
        iifname != "{docker_iif}" return
        ip daddr @reserved_ipv4 return
        meta l4proto {{ tcp, udp }} th dport 53 return
        {capture_prerouting_rule}"#
                    ),
                ),
                (false, false) => (String::new(), "        return".to_string()),
            };

        let dns_output_body = if self.settings.local {
            format!(
                r#"        meta nfproto ipv6 return
        meta mark {routing_mark} return
        meta mark {fwmark} return
        meta l4proto {{ tcp, udp }} th dport 53 redirect to :{dns_port}"#
            )
        } else {
            "        return".to_string()
        };

        let tproxy_output_body = if self.settings.local {
            format!(
                r#"        meta nfproto ipv6 return
        meta mark {routing_mark} return
        meta mark {fwmark} return
        ip daddr @reserved_ipv4 return
        meta l4proto {{ tcp, udp }} th dport 53 return
        {capture_output_rule}"#
            )
        } else {
            "        return".to_string()
        };

        let lan_protect_rule = if self.settings.lan {
            "        ip saddr @lan_ipv4 accept\n"
        } else {
            ""
        };

        let docker_protect_rule = if self.settings.docker {
            format!("        iifname \"{docker_iif}\" accept\n")
        } else {
            String::new()
        };

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

{proxy_ports_set}{dns_prerouting_extra}    chain dns_prerouting {{
        type nat hook prerouting priority -110; policy accept;
{dns_prerouting_body}
    }}

    chain dns_output {{
        type nat hook output priority -110; policy accept;
{dns_output_body}
    }}

{tproxy_prerouting_extra}    chain tproxy_prerouting {{
        type filter hook prerouting priority mangle - 10; policy accept;
{tproxy_prerouting_body}
    }}

    chain tproxy_output {{
        type route hook output priority mangle - 10; policy accept;
{tproxy_output_body}
    }}

    chain tproxy_mark_out {{
        type filter hook prerouting priority mangle; policy accept;
        meta nfproto ipv6 return
        {mark_out_rule}
    }}

    chain input_protect {{
        type filter hook input priority filter; policy accept;
        iif "lo" accept
{lan_protect_rule}{docker_protect_rule}        tcp dport {{ {mixed_port}, {tproxy_port}, {dns_port} }} reject
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
    use crate::config::{NetworkSettings, PortsFilter};

    fn ruleset() -> String {
        NftablesManager::new(&NetworkSettings::default()).generate_ruleset()
    }

    fn assert_no_bridge_glob(rules: &str) {
        assert!(!rules.contains("br-*"));
    }

    #[test]
    fn test_generates_default_intent_rules() {
        let rules = ruleset();
        let defaults = NetworkSettings::default();
        let fwmark = defaults.fwmark;
        let routing_mark = defaults.routing_mark;

        assert_ne!(fwmark, routing_mark);
        assert!(rules.contains("set reserved_ipv4"));
        assert!(rules.contains("set lan_ipv4"));
        assert!(rules.contains("set proxy_ports"));
        assert!(!rules.contains("chain dns_prerouting_redirect"));
        assert!(!rules.contains("chain tproxy_prerouting_capture"));
        assert!(rules.contains("chain dns_prerouting"));
        assert!(rules.contains("chain dns_output"));
        assert!(rules.contains("chain tproxy_prerouting"));
        assert!(rules.contains("chain tproxy_output"));
        assert!(rules.contains("chain tproxy_mark_out"));
        assert!(rules.contains("chain input_protect"));

        // Default scope is LAN only; docker0 is not hijacked.
        assert!(rules.contains("ip saddr != @lan_ipv4 return"));
        assert!(!rules.contains(r#"iifname "docker0""#));
        assert_no_bridge_glob(&rules);

        // TCP only by default (udp = false)
        assert!(rules.contains(&format!(
            "meta l4proto tcp th dport @proxy_ports tproxy to :7893 meta mark set {fwmark} accept"
        )));
        assert!(rules.contains(&format!(
            "meta l4proto tcp th dport @proxy_ports meta mark set {fwmark}"
        )));
        assert!(rules.contains(&format!(
            "meta mark {fwmark} meta l4proto tcp tproxy to :7893 accept"
        )));

        // DNS redirect preserved
        assert!(rules.contains("meta l4proto { tcp, udp } th dport 53 redirect to :1053"));
        assert!(rules.contains("meta l4proto { tcp, udp } th dport 53 return"));

        // Protection allows LAN, not docker
        assert!(rules.contains("ip saddr @lan_ipv4 accept"));
        assert!(!rules.contains(r#"iifname "docker0" accept"#));
    }

    #[test]
    fn test_generates_all_ports_and_udp_when_configured() {
        let settings = NetworkSettings {
            tcp: true,
            udp: true,
            ports: PortsFilter::All,
            ..NetworkSettings::default()
        };
        let rules = NftablesManager::new(&settings).generate_ruleset();

        assert!(!rules.contains("set proxy_ports"));
        assert!(!rules.contains("th dport @proxy_ports"));
        assert!(rules.contains(&format!(
            "meta l4proto {{ tcp, udp }} tproxy to :7893 meta mark set {} accept",
            settings.fwmark
        )));
        assert!(rules.contains(&format!(
            "meta l4proto {{ tcp, udp }} meta mark set {}",
            settings.fwmark
        )));
        assert!(rules.contains(&format!(
            "meta mark {} meta l4proto {{ tcp, udp }} tproxy to :7893 accept",
            settings.fwmark
        )));
    }

    #[test]
    fn test_generates_custom_ports() {
        let settings = NetworkSettings {
            ports: PortsFilter::Custom(vec![80, 443, 8080]),
            ..NetworkSettings::default()
        };
        let rules = NftablesManager::new(&settings).generate_ruleset();

        assert!(rules.contains("set proxy_ports"));
        assert!(rules.contains("elements = { 80, 443, 8080 }"));
        assert!(rules.contains("meta l4proto tcp th dport @proxy_ports tproxy to :7893"));
    }

    #[test]
    fn test_disables_docker_when_configured_false() {
        let settings = NetworkSettings {
            docker: false,
            lan: true,
            ..NetworkSettings::default()
        };
        let rules = NftablesManager::new(&settings).generate_ruleset();

        assert!(!rules.contains("chain tproxy_prerouting_capture"));
        assert!(!rules.contains(r#"iifname "docker0""#));
        assert!(rules.contains("ip saddr != @lan_ipv4 return"));
        assert!(!rules.contains(r#"iifname "docker0" accept"#));
        assert_no_bridge_glob(&rules);
    }

    #[test]
    fn test_enables_docker0_when_configured_true() {
        let settings = NetworkSettings {
            docker: true,
            lan: true,
            ..NetworkSettings::default()
        };
        let rules = NftablesManager::new(&settings).generate_ruleset();

        assert!(rules.contains("chain tproxy_prerouting_capture"));
        assert!(rules.contains(r#"iifname "docker0" goto tproxy_prerouting_capture"#));
        assert!(rules.contains(r#"ip saddr @lan_ipv4 goto tproxy_prerouting_capture"#));
        assert!(rules.contains(r#"iifname "docker0" goto dns_prerouting_redirect"#));
        assert!(rules.contains(r#"iifname "docker0" accept"#));
        assert_no_bridge_glob(&rules);
    }

    #[test]
    fn test_disables_lan_when_configured_false() {
        let settings = NetworkSettings {
            lan: false,
            docker: true,
            ..NetworkSettings::default()
        };
        let rules = NftablesManager::new(&settings).generate_ruleset();

        assert!(!rules.contains("chain tproxy_prerouting_capture"));
        assert!(rules.contains(r#"iifname != "docker0" return"#));
        assert!(!rules.contains("ip saddr @lan_ipv4 accept"));
        assert_no_bridge_glob(&rules);
    }

    #[test]
    fn test_disables_local_output_when_configured_false() {
        let settings = NetworkSettings {
            local: false,
            ..NetworkSettings::default()
        };
        let rules = NftablesManager::new(&settings).generate_ruleset();

        // Output chains should simply return
        assert!(rules.contains(
            r#"    chain dns_output {
        type nat hook output priority -110; policy accept;
        return
    }"#
        ));
        assert!(rules.contains(
            r#"    chain tproxy_output {
        type route hook output priority mangle - 10; policy accept;
        return
    }"#
        ));
        assert!(rules.contains(
            r#"    chain tproxy_mark_out {
        type filter hook prerouting priority mangle; policy accept;
        meta nfproto ipv6 return
        return
    }"#
        ));
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
    }

    #[test]
    fn test_skips_ipv6_and_does_not_bypass_loopback() {
        let rules = ruleset();
        assert!(rules.contains("meta nfproto ipv6 return"));
        assert!(!rules.contains("iif \"lo\" return"));
        assert!(!rules.contains("iif 'lo' return"));
        assert!(rules.contains("iif \"lo\" accept"));
    }

    #[test]
    fn test_tun_dump_has_no_tproxy_rules() {
        let dump = NftablesManager::new(&NetworkSettings::default()).dump();
        assert!(dump.contains("no nftables ruleset"));
        assert!(!dump.contains("tproxy to"));
        assert!(!dump.contains("masquerade"));
        assert!(!dump.contains("input_protect"));
        assert!(!dump.contains("proxy_ports"));
    }
}
