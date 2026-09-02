use tracing::{error, info};

use crate::config::NetworkSettings;
use crate::error::Result;
use crate::network::nftables::NftablesManager;
use crate::network::route::RouteManager;

pub struct NetworkGuard {
    settings: NetworkSettings,
    active: bool,
}

impl NetworkGuard {
    /// Applies network rules and returns a guard that will clean them up when dropped
    pub fn setup(settings: NetworkSettings) -> Result<Self> {
        let route = RouteManager::new(&settings);
        let nft = NftablesManager::new(&settings);

        info!("Initializing transparent proxy network rules");
        route.setup()?;
        nft.apply()?;

        Ok(Self {
            settings,
            active: true,
        })
    }

    /// Disarms the guard so it does not tear down network rules upon drop
    pub fn disarm(&mut self) {
        self.active = false;
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

impl Drop for NetworkGuard {
    fn drop(&mut self) {
        if self.active {
            info!("NetworkGuard dropped: Restoring default system routing and firewall");
            let nft = NftablesManager::new(&self.settings);
            let route = RouteManager::new(&self.settings);

            let _ = nft.flush();
            let _ = route.teardown();
        }
    }
}
