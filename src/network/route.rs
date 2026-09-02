#[cfg(target_os = "linux")]
use std::process::Command;
use tracing::debug;
#[cfg(target_os = "linux")]
use tracing::{info, warn};

use crate::config::NetworkSettings;
#[cfg(target_os = "linux")]
use crate::config::ProxyMode;
#[cfg(target_os = "linux")]
use crate::error::CantoError;
use crate::error::Result;

pub struct RouteManager<'a> {
    settings: &'a NetworkSettings,
}

impl<'a> RouteManager<'a> {
    pub fn new(settings: &'a NetworkSettings) -> Self {
        Self { settings }
    }

    /// Sets up policy routing rules and route tables
    pub fn setup(&self) -> Result<()> {
        let _mark_hex = format!("{:#x}", self.settings.routing_mark);
        let _table = self.settings.table_id.to_string();

        #[cfg(target_os = "linux")]
        {
            info!(
                "Configuring policy routing: mark {} -> table {}",
                _mark_hex, _table
            );

            // Clean previous rules first
            let _ = self.teardown();

            // 1. Add ip rule for fwmark
            let rule_status = Command::new("ip")
                .args(["rule", "add", "fwmark", &_mark_hex, "table", &_table])
                .status()
                .map_err(|e| CantoError::Network(format!("Failed to execute 'ip rule': {e}")))?;

            if !rule_status.success() {
                warn!("'ip rule add' returned non-zero exit status");
            }

            // 2. Add default route in the dedicated table
            let route_status = match self.settings.mode {
                ProxyMode::Tproxy => Command::new("ip")
                    .args([
                        "route", "add", "local", "default", "dev", "lo", "table", &_table,
                    ])
                    .status(),
                ProxyMode::Tun => Command::new("ip")
                    .args([
                        "route",
                        "add",
                        "default",
                        "dev",
                        &self.settings.tun_interface,
                        "table",
                        &_table,
                    ])
                    .status(),
                ProxyMode::None => return Ok(()),
            }
            .map_err(|e| CantoError::Network(format!("Failed to execute 'ip route': {e}")))?;

            if !route_status.success() {
                warn!("'ip route add' returned non-zero exit status");
            }

            info!("Policy routing configured successfully");
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            debug!(
                "Skipping policy routing configuration (mark {_mark_hex}, table {_table}) on non-Linux OS"
            );
            Ok(())
        }
    }

    /// Removes policy routing rules and route entries
    pub fn teardown(&self) -> Result<()> {
        let _mark_hex = format!("{:#x}", self.settings.routing_mark);
        let _table = self.settings.table_id.to_string();

        #[cfg(target_os = "linux")]
        {
            debug!("Removing policy routing rules for table {_table}");

            // Flush route in custom table
            let _ = Command::new("ip")
                .args(["route", "flush", "table", &_table])
                .output();

            // Delete ip rule
            let _ = Command::new("ip")
                .args(["rule", "del", "fwmark", &_mark_hex, "table", &_table])
                .output();

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            debug!("Skipping policy routing teardown on non-Linux OS");
            Ok(())
        }
    }
}
