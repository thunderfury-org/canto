use tracing::{info, warn};

use crate::error::{CantoError, Result};

const FALLBACK_LAN_CIDRS: &[&str] = &["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"];

pub fn fallback_lan_cidrs() -> Vec<String> {
    FALLBACK_LAN_CIDRS
        .iter()
        .map(|cidr| (*cidr).to_string())
        .collect()
}

/// Resolves LAN CIDRs used as the nftables source allowlist.
///
/// Non-empty `configured` values win. Otherwise Linux auto-detects
/// `scope link` routes; other platforms and detection failures fall back to RFC1918.
pub fn resolve_lan_cidrs(configured: &[String]) -> Result<Vec<String>> {
    if !configured.is_empty() {
        let cidrs = validate_cidrs(configured)?;
        info!("Using configured LAN CIDRs: {}", cidrs.join(", "));
        return Ok(cidrs);
    }

    match detect_lan_cidrs() {
        Ok(cidrs) if !cidrs.is_empty() => {
            info!("Detected LAN CIDRs: {}", cidrs.join(", "));
            Ok(cidrs)
        }
        Ok(_) => {
            warn!("No LAN CIDRs detected; falling back to RFC1918");
            Ok(fallback_lan_cidrs())
        }
        Err(e) => {
            warn!("LAN CIDR detection failed ({e}); falling back to RFC1918");
            Ok(fallback_lan_cidrs())
        }
    }
}

fn validate_cidrs(cidrs: &[String]) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for cidr in cidrs {
        let trimmed = cidr.trim();
        if !is_ipv4_cidr(trimmed) {
            return Err(CantoError::Config(format!(
                "invalid IPv4 CIDR in network.lan_cidrs: {trimmed}"
            )));
        }
        if !out.iter().any(|existing: &String| existing == trimmed) {
            out.push(trimmed.to_string());
        }
    }
    if out.is_empty() {
        return Err(CantoError::Config(
            "network.lan_cidrs is empty after validation".to_string(),
        ));
    }
    Ok(out)
}

fn is_ipv4_cidr(value: &str) -> bool {
    let Some((ip, prefix)) = value.split_once('/') else {
        return false;
    };
    let Ok(prefix) = prefix.parse::<u8>() else {
        return false;
    };
    if prefix > 32 {
        return false;
    }
    let octets: Vec<&str> = ip.split('.').collect();
    if octets.len() != 4 {
        return false;
    }
    octets.iter().all(|octet| {
        octet
            .parse::<u8>()
            .is_ok_and(|n| *octet == "0" || !octet.starts_with('0') || n == 0 && octet.len() == 1)
    })
}

fn detect_lan_cidrs() -> Result<Vec<String>> {
    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("ip")
            .args(["-4", "route", "show", "scope", "link"])
            .output()
            .map_err(|e| CantoError::Network(format!("Failed to execute 'ip route': {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CantoError::CommandFailed {
                command: "ip -4 route show scope link".to_string(),
                code: output.status.code().unwrap_or(-1),
                stderr: stderr.to_string(),
            });
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_scope_link_cidrs(&stdout))
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(CantoError::Network(
            "LAN CIDR auto-detect is only available on Linux".to_string(),
        ))
    }
}

pub fn parse_scope_link_cidrs(route_table: &str) -> Vec<String> {
    let mut cidrs = Vec::new();
    for line in route_table.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let cidr = parts[0];
        if !is_ipv4_cidr(cidr) || is_ignored_cidr(cidr) {
            continue;
        }
        let iface = parts
            .windows(2)
            .find(|pair| pair[0] == "dev")
            .map(|pair| pair[1]);
        let Some(iface) = iface else {
            continue;
        };
        if is_skipped_iface(iface) {
            continue;
        }
        if !cidrs.iter().any(|existing: &String| existing == cidr) {
            cidrs.push(cidr.to_string());
        }
    }
    cidrs
}

fn is_ignored_cidr(cidr: &str) -> bool {
    cidr.starts_with("127.") || cidr.starts_with("169.254.") || cidr.starts_with("224.")
}

fn is_skipped_iface(iface: &str) -> bool {
    let name = iface.to_ascii_lowercase();
    if name == "lo" || name == "wan" || name.ends_with("-wan") || name.contains("pppoe") {
        return true;
    }
    if name.starts_with("wan") {
        return name.chars().nth(3).is_none_or(|c| !c.is_ascii_alphabetic());
    }

    const PREFIXES: &[&str] = &[
        "docker",
        "veth",
        "virbr",
        "vnet",
        "podman",
        "vboxnet",
        "lxcbr",
        "xenbr",
        "vmbr",
        "vmnic",
        "ovs",
        "utun",
        "tun",
        "ppp",
        "tailscale",
        "wgs",
    ];
    if PREFIXES
        .iter()
        .any(|prefix| name == *prefix || name.starts_with(prefix))
    {
        return true;
    }

    name == "wg"
        || name.starts_with("wg-")
        || (name.starts_with("wg") && name.chars().nth(2).is_some_and(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scope_link_keeps_lan_and_skips_virtual() {
        let table = "\
192.168.1.0/24 dev br-lan proto kernel scope link src 192.168.1.1
10.10.0.0/24 dev eth0 proto kernel scope link src 10.10.0.1
172.17.0.0/16 dev docker0 proto kernel scope link src 172.17.0.1
169.254.0.0/16 dev eth0 proto kernel scope link src 169.254.12.34
192.168.8.0/24 dev wan proto kernel scope link src 192.168.8.1
100.64.0.0/10 dev tailscale0 proto kernel scope link src 100.64.0.2
192.168.50.0/24 dev wlan0 proto kernel scope link src 192.168.50.1
";
        let cidrs = parse_scope_link_cidrs(table);
        assert_eq!(
            cidrs,
            vec![
                "192.168.1.0/24".to_string(),
                "10.10.0.0/24".to_string(),
                "192.168.50.0/24".to_string(),
            ]
        );
    }

    #[test]
    fn test_resolve_configured_cidrs_and_reject_invalid() {
        let cidrs =
            resolve_lan_cidrs(&[" 192.168.5.0/24 ".to_string(), "10.0.0.0/8".to_string()]).unwrap();
        assert_eq!(cidrs, vec!["192.168.5.0/24", "10.0.0.0/8"]);

        let err = resolve_lan_cidrs(&["192.168.5.0".to_string()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("invalid IPv4 CIDR"));
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn test_empty_config_falls_back_off_linux() {
        let cidrs = resolve_lan_cidrs(&[]).unwrap();
        assert_eq!(cidrs, fallback_lan_cidrs());
    }

    #[test]
    fn test_resolve_empty_config_yields_usable_cidrs() {
        let cidrs = resolve_lan_cidrs(&[]).unwrap();
        assert!(!cidrs.is_empty());
        assert!(cidrs.iter().all(|cidr| is_ipv4_cidr(cidr)));
    }
}
