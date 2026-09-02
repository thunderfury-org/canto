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

    /// Generates the complete nftables configuration for transparent proxying
    pub fn generate_ruleset(&self) -> String {
        let tproxy_port = self.settings.tproxy_port;
        let dns_port = self.settings.dns_port;
        let mark = self.settings.routing_mark;

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
            240.0.0.0/4,
            255.255.255.255/32
        }}
    }}

    set reserved_ipv6 {{
        type ipv6_addr
        flags interval
        elements = {{
            ::/128,
            ::1/128,
            ::ffff:0:0/96,
            64:ff9b::/96,
            100::/64,
            2001::/32,
            2001:20::/28,
            2001:db8::/32,
            2002::/16,
            fc00::/7,
            fe80::/10,
            ff00::/8
        }}
    }}

    chain dns_prerouting {{
        type nat hook prerouting priority dstnat - 10; policy accept;
        # Exclude loop prevention mark
        meta mark {mark} return

        # Redirect DNS requests (UDP/TCP 53) to local sing-box DNS listener
        meta l4proto {{ tcp, udp }} th dport 53 redirect to :{dns_port}
    }}

    chain tproxy_prerouting {{
        type filter hook prerouting priority mangle - 10; policy accept;
        # Exclude loopback interface
        iif "lo" return

        # Exclude loop prevention mark from sing-box
        meta mark {mark} return

        # Bypass reserved private addresses
        ip daddr @reserved_ipv4 return
        ip6 daddr @reserved_ipv6 return

        # Exclude DNS (already handled in nat dstnat)
        meta l4proto {{ tcp, udp }} th dport 53 return

        # Tproxy matching TCP and UDP traffic to sing-box
        meta l4proto {{ tcp, udp }} tproxy to :{tproxy_port} meta mark set {mark} accept
    }}

    chain tproxy_output {{
        type route hook output priority mangle - 10; policy accept;
        # Exclude loop prevention mark
        meta mark {mark} return

        # Bypass reserved addresses
        ip daddr @reserved_ipv4 return
        ip6 daddr @reserved_ipv6 return

        # Exclude DNS queries
        meta l4proto {{ tcp, udp }} th dport 53 return

        # Mark locally generated outbound traffic for policy routing
        meta l4proto {{ tcp, udp }} meta mark set {mark}
    }}
}}
"#
        )
    }

    /// Applies the transparent proxy ruleset
    pub fn apply(&self) -> Result<()> {
        let rules = self.generate_ruleset();

        #[cfg(target_os = "linux")]
        {
            info!("Applying nftables ruleset for table 'inet {TABLE_NAME}'");
            self.flush()?; // Clean up any pre-existing table first

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
