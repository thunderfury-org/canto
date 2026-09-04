#[cfg(target_os = "linux")]
use std::process::Command;
use tracing::debug;
#[cfg(target_os = "linux")]
use tracing::{info, warn};

use crate::config::NetworkSettings;
#[cfg(target_os = "linux")]
use crate::error::CantoError;
use crate::error::Result;

/// Policy routing table. Avoid 100 (clash/ShellCrash) and 253-255 (kernel).
const ROUTING_TABLE_ID: u32 = 167;

pub struct RouteManager<'a> {
    settings: &'a NetworkSettings,
}

impl<'a> RouteManager<'a> {
    pub fn new(settings: &'a NetworkSettings) -> Self {
        Self { settings }
    }

    /// Sets up policy routing rules and route tables
    pub fn setup(&self) -> Result<()> {
        let mark_hex = format!("{:#x}", self.settings.fwmark);
        let table = ROUTING_TABLE_ID.to_string();

        #[cfg(target_os = "linux")]
        {
            info!("Configuring policy routing: fwmark {mark_hex} -> table {table}");

            let _ = self.teardown();

            let rule_status = Command::new("ip")
                .args(["rule", "add", "fwmark", &mark_hex, "table", &table])
                .status()
                .map_err(|e| CantoError::Network(format!("Failed to execute 'ip rule': {e}")))?;

            if !rule_status.success() {
                warn!("'ip rule add' returned non-zero exit status");
            }

            let route_status = Command::new("ip")
                .args([
                    "route", "add", "local", "default", "dev", "lo", "table", &table,
                ])
                .status()
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
                "Skipping policy routing configuration (mark {mark_hex}, table {table}) on non-Linux OS"
            );
            Ok(())
        }
    }

    /// Removes policy routing rules and route entries
    pub fn teardown(&self) -> Result<()> {
        let mark_hex = format!("{:#x}", self.settings.fwmark);
        let table = ROUTING_TABLE_ID.to_string();

        #[cfg(target_os = "linux")]
        {
            debug!("Removing policy routing rules for table {table}");

            let _ = Command::new("ip")
                .args(["route", "flush", "table", &table])
                .output();

            let _ = Command::new("ip")
                .args(["rule", "del", "fwmark", &mark_hex, "table", &table])
                .output();

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            debug!(
                "Skipping policy routing teardown (mark {mark_hex}, table {table}) on non-Linux OS"
            );
            Ok(())
        }
    }
}
