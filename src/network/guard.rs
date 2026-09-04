#[cfg(target_os = "linux")]
use tracing::warn;
use tracing::{error, info};

use crate::config::NetworkSettings;
#[cfg(target_os = "linux")]
use crate::error::CantoError;
use crate::error::Result;
use crate::network::lan::resolve_lan_cidrs;
use crate::network::nftables::NftablesManager;
use crate::network::route::RouteManager;

pub struct NetworkGuard {
    settings: NetworkSettings,
}

impl NetworkGuard {
    /// Applies network rules and returns a guard that will clean them up when dropped
    pub fn setup(mut settings: NetworkSettings) -> Result<Self> {
        settings.lan_cidrs = resolve_lan_cidrs(&settings.lan_cidrs)?;
        prepare_kernel()?;

        let route = RouteManager::new(&settings);
        let nft = NftablesManager::new(&settings);

        info!("Initializing transparent proxy network rules");
        route.setup()?;
        nft.apply()?;

        Ok(Self { settings })
    }

    /// Manually triggers teardown of all network rules
    pub fn teardown_manual(settings: &NetworkSettings) -> Result<()> {
        info!("Flushing transparent proxy network rules and route policies");
        let nft = NftablesManager::new(settings);
        let route = RouteManager::new(settings);

        if let Err(e) = nft.flush() {
            error!("Error flushing nftables: {e}");
        }
        if let Err(e) = route.teardown() {
            error!("Error tearing down routes: {e}");
        }

        Ok(())
    }
}

fn prepare_kernel() -> Result<()> {
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

impl Drop for NetworkGuard {
    fn drop(&mut self) {
        info!("NetworkGuard dropped: Restoring default system routing and firewall");
        let nft = NftablesManager::new(&self.settings);
        let route = RouteManager::new(&self.settings);

        let _ = nft.flush();
        let _ = route.teardown();
    }
}
