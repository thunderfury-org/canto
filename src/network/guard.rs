use std::path::Path;
#[cfg(target_os = "linux")]
use tracing::warn;
use tracing::{error, info};

use crate::config::NetworkSettings;
use crate::config::source::SourceFetcher;
use crate::error::{CantoError, Result};
use crate::network::cnip::{cn_ip_path, load_cn_ip, read_cn_ip_file};
use crate::network::lan::resolve_lan_cidrs;
use crate::network::nftables::NftablesManager;

pub const SING_BOX_TABLE: &str = "sing-box";
const ROUTING_TABLE_ID: u32 = 167;

pub struct NetworkGuard {
    settings: NetworkSettings,
}

impl NetworkGuard {
    /// Starts transparent proxy network capture asynchronously:
    /// loads assets (CN IP list & LAN CIDRs), prepares kernel tproxy,
    /// configures policy routing table 167, and applies nftables rules.
    pub async fn start(
        mut settings: NetworkSettings,
        work_dir: &Path,
        fetcher: &impl SourceFetcher,
    ) -> Result<Self> {
        settings.validate()?;
        let cnip = if settings.bypass_cn {
            load_cn_ip(work_dir, fetcher).await?
        } else {
            Vec::new()
        };

        if settings.bypass_cn && cnip.is_empty() {
            return Err(CantoError::Config(
                "network.bypass_cn is true but cn_ip.txt has no IPv4 CIDRs".to_string(),
            ));
        }

        settings.lan_cidrs = resolve_lan_cidrs(&settings.lan_cidrs)?;
        prepare_kernel_tproxy()?;

        info!("Initializing transparent proxy network rules");
        setup_policy_routing(settings.fwmark)?;
        let nft = NftablesManager::new(&settings).with_cnip(&cnip);
        nft.apply()?;

        Ok(Self { settings })
    }

    /// Generates the complete nftables ruleset string for inspection or dumping.
    pub fn dump_ruleset(settings: &NetworkSettings, work_dir: &Path) -> Result<String> {
        let mut network = settings.clone();
        network.lan_cidrs = resolve_lan_cidrs(&network.lan_cidrs)?;
        info!("LAN CIDRs: {}", network.lan_cidrs.join(", "));

        let cnip = match read_cn_ip_file(work_dir) {
            Ok(Some(cidrs)) => {
                info!(
                    "CN CIDRs: {} prefixes from {}",
                    cidrs.len(),
                    cn_ip_path(work_dir).display()
                );
                cidrs
            }
            Ok(None) => {
                if network.bypass_cn {
                    tracing::warn!(
                        "bypass_cn is true but {} is missing; dump omits set cnip",
                        cn_ip_path(work_dir).display()
                    );
                }
                Vec::new()
            }
            Err(e) => return Err(e),
        };

        Ok(NftablesManager::new(&network).with_cnip(&cnip).dump())
    }

    /// Manually triggers teardown of all network rules and policy routing
    pub fn teardown_manual(settings: &NetworkSettings) -> Result<()> {
        info!("Flushing transparent proxy network rules and route policies");
        leftover_cleanup(settings);
        Ok(())
    }
}

fn setup_policy_routing(fwmark: u32) -> Result<()> {
    let mark_hex = format!("{:#x}", fwmark);
    let table = ROUTING_TABLE_ID.to_string();

    #[cfg(target_os = "linux")]
    {
        info!("Configuring policy routing: fwmark {mark_hex} -> table {table}");
        let _ = teardown_policy_routing(fwmark);

        let rule_status = std::process::Command::new("ip")
            .args(["rule", "add", "fwmark", &mark_hex, "table", &table])
            .status()
            .map_err(|e| CantoError::Network(format!("Failed to execute 'ip rule': {e}")))?;

        if !rule_status.success() {
            tracing::warn!("'ip rule add' returned non-zero exit status");
        }

        let route_status = std::process::Command::new("ip")
            .args([
                "route", "add", "local", "default", "dev", "lo", "table", &table,
            ])
            .status()
            .map_err(|e| CantoError::Network(format!("Failed to execute 'ip route': {e}")))?;

        if !route_status.success() {
            tracing::warn!("'ip route add' returned non-zero exit status");
        }

        info!("Policy routing configured successfully");
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        tracing::debug!(
            "Skipping policy routing configuration (mark {mark_hex}, table {table}) on non-Linux OS"
        );
        Ok(())
    }
}

fn teardown_policy_routing(fwmark: u32) -> Result<()> {
    let mark_hex = format!("{:#x}", fwmark);
    let table = ROUTING_TABLE_ID.to_string();

    #[cfg(target_os = "linux")]
    {
        tracing::debug!("Removing policy routing rules for table {table}");

        let _ = std::process::Command::new("ip")
            .args(["route", "flush", "table", &table])
            .output();

        let _ = std::process::Command::new("ip")
            .args(["rule", "del", "fwmark", &mark_hex, "table", &table])
            .output();

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        tracing::debug!(
            "Skipping policy routing teardown (mark {mark_hex}, table {table}) on non-Linux OS"
        );
        Ok(())
    }
}

fn leftover_cleanup(settings: &NetworkSettings) {
    let nft = NftablesManager::new(settings);

    if let Err(e) = nft.flush() {
        error!("Error flushing nftables: {e}");
    }
    if let Err(e) = teardown_policy_routing(settings.fwmark) {
        error!("Error tearing down routes: {e}");
    }
    delete_sing_box_table();
}

fn prepare_kernel_tproxy() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        enable_ip_forward()?;
        // Local tproxy delivers marked packets via lo; strict rp_filter drops them.
        set_sysctl("/proc/sys/net/ipv4/conf/all/route_localnet", "1")?;
        set_sysctl("/proc/sys/net/ipv4/conf/lo/route_localnet", "1")?;
        set_sysctl("/proc/sys/net/ipv4/conf/all/rp_filter", "2")?;
        set_sysctl("/proc/sys/net/ipv4/conf/lo/rp_filter", "2")?;
        load_tproxy_module();
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn set_sysctl(path: &str, value: &str) -> Result<()> {
    if let Ok(current) = std::fs::read_to_string(path)
        && current.trim() == value
    {
        return Ok(());
    }
    std::fs::write(path, format!("{value}\n"))
        .map_err(|e| CantoError::Network(format!("Failed to set {path}={value}: {e}")))?;
    info!("Set {path} = {value}");
    Ok(())
}

#[cfg(target_os = "linux")]
fn enable_ip_forward() -> Result<()> {
    set_sysctl("/proc/sys/net/ipv4/ip_forward", "1")
}

#[cfg(target_os = "linux")]
fn load_tproxy_module() {
    let status = std::process::Command::new("modprobe")
        .arg("nft_tproxy")
        .status();
    match status {
        Ok(code) if code.success() => {
            info!("Loaded nft_tproxy module");
        }
        Ok(_) => {
            warn!("modprobe nft_tproxy failed; kernel may already have tproxy built in");
        }
        Err(e) => {
            warn!("Could not invoke modprobe nft_tproxy: {e}");
        }
    }
}

fn delete_sing_box_table() {
    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("nft")
            .args(["delete", "table", "inet", SING_BOX_TABLE])
            .output();
        match output {
            Ok(out) if out.status.success() => {
                info!("Removed leftover nftables table inet {SING_BOX_TABLE}");
            }
            _ => {}
        }
    }
}

impl Drop for NetworkGuard {
    fn drop(&mut self) {
        info!("NetworkGuard dropped: Restoring default system routing and firewall");
        leftover_cleanup(&self.settings);
    }
}
