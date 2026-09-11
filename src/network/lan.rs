use tracing::{info, warn};

use crate::config::NetworkSettings;
use crate::error::{CantoError, Result};

const FALLBACK_LAN_CIDRS: &[&str] = &["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"];
pub const DOCKER_IIFNAME: &str = "docker0";
pub const LOCAL_MIXED_LISTEN: &str = "127.0.0.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanLink {
    pub cidr: String,
    pub iface: String,
    pub src: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunCapture {
    pub mixed_listen: String,
    pub include_interface: Vec<String>,
    pub exclude_interface: Vec<String>,
    pub auto_redirect: bool,
}

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

    match detect_lan_links(true) {
        Ok(links) if !links.is_empty() => {
            let cidrs = unique_cidrs(&links);
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

/// Resolves TUN include/exclude interfaces and the mixed listen address.
///
/// `strict` is true for `canto run` on Linux: missing LAN interfaces fail.
/// `config generate` / `check` and non-Linux use `strict = false`.
pub fn resolve_tun_capture(network: &NetworkSettings, strict: bool) -> Result<TunCapture> {
    let mut include_interface = Vec::new();
    if network.lan {
        include_interface = resolve_lan_interfaces(&network.lan_cidrs, strict)?;
    }
    if network.docker
        && !include_interface
            .iter()
            .any(|iface| iface == DOCKER_IIFNAME)
    {
        include_interface.push(DOCKER_IIFNAME.to_string());
    }

    let mut exclude_interface = Vec::new();
    if !network.docker {
        exclude_interface.push(DOCKER_IIFNAME.to_string());
    }

    let auto_redirect = network.lan || network.docker;
    let mixed_listen = if !network.lan && !network.docker {
        LOCAL_MIXED_LISTEN.to_string()
    } else {
        first_listen_ip(&network.lan_cidrs, &include_interface)
            .unwrap_or_else(|| LOCAL_MIXED_LISTEN.to_string())
    };

    Ok(TunCapture {
        mixed_listen,
        include_interface,
        exclude_interface,
        auto_redirect,
    })
}

fn resolve_lan_interfaces(configured: &[String], strict: bool) -> Result<Vec<String>> {
    if !configured.is_empty() {
        let cidrs = validate_cidrs(configured)?;
        let links = match detect_lan_links(false) {
            Ok(links) => links,
            Err(e) if strict => return Err(e),
            Err(e) => {
                warn!("LAN interface detection failed ({e})");
                Vec::new()
            }
        };
        let mut ifaces = Vec::new();
        for cidr in &cidrs {
            for link in links.iter().filter(|link| &link.cidr == cidr) {
                if !ifaces.iter().any(|existing| existing == &link.iface) {
                    ifaces.push(link.iface.clone());
                }
            }
        }
        if ifaces.is_empty() && strict {
            return Err(CantoError::Config(format!(
                "no interface matched network.lan_cidrs ({})",
                cidrs.join(", ")
            )));
        }
        if !ifaces.is_empty() {
            info!(
                "Using LAN interfaces for CIDRs {}: {}",
                cidrs.join(", "),
                ifaces.join(", ")
            );
        }
        return Ok(ifaces);
    }

    match detect_lan_links(true) {
        Ok(links) if !links.is_empty() => {
            let ifaces = unique_ifaces(&links);
            info!("Detected LAN interfaces: {}", ifaces.join(", "));
            Ok(ifaces)
        }
        Ok(_) if strict => Err(CantoError::Config(
            "no LAN interfaces detected for TUN include_interface".to_string(),
        )),
        Err(e) if strict => Err(e),
        Ok(_) | Err(_) => Ok(Vec::new()),
    }
}

fn first_listen_ip(configured: &[String], include_interface: &[String]) -> Option<String> {
    let skip_veth = configured.is_empty();
    let links = detect_lan_links(skip_veth).unwrap_or_default();
    for iface in include_interface {
        if let Some(src) = links
            .iter()
            .find(|link| &link.iface == iface)
            .and_then(|link| link.src.clone())
        {
            return Some(src);
        }
    }
    None
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

fn detect_lan_links(skip_veth: bool) -> Result<Vec<LanLink>> {
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
        Ok(parse_scope_link_links(&stdout, skip_veth))
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = skip_veth;
        Err(CantoError::Network(
            "LAN CIDR auto-detect is only available on Linux".to_string(),
        ))
    }
}

pub fn parse_scope_link_cidrs(route_table: &str) -> Vec<String> {
    unique_cidrs(&parse_scope_link_links(route_table, true))
}

pub fn parse_scope_link_links(route_table: &str, skip_veth: bool) -> Vec<LanLink> {
    let mut links = Vec::new();
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
        if is_skipped_iface(iface, skip_veth) {
            continue;
        }
        let src = parts
            .windows(2)
            .find(|pair| pair[0] == "src")
            .map(|pair| pair[1].to_string());
        if !links
            .iter()
            .any(|existing: &LanLink| existing.iface == iface && existing.cidr == cidr)
        {
            links.push(LanLink {
                cidr: cidr.to_string(),
                iface: iface.to_string(),
                src,
            });
        }
    }
    links
}

fn unique_cidrs(links: &[LanLink]) -> Vec<String> {
    let mut cidrs = Vec::new();
    for link in links {
        if !cidrs.iter().any(|existing| existing == &link.cidr) {
            cidrs.push(link.cidr.clone());
        }
    }
    cidrs
}

fn unique_ifaces(links: &[LanLink]) -> Vec<String> {
    let mut ifaces = Vec::new();
    for link in links {
        if !ifaces.iter().any(|existing| existing == &link.iface) {
            ifaces.push(link.iface.clone());
        }
    }
    ifaces
}

fn is_ignored_cidr(cidr: &str) -> bool {
    cidr.starts_with("127.") || cidr.starts_with("169.254.") || cidr.starts_with("224.")
}

fn is_skipped_iface(iface: &str, skip_veth: bool) -> bool {
    let name = iface.to_ascii_lowercase();
    if name == "lo" || name == "wan" || name.ends_with("-wan") || name.contains("pppoe") {
        return true;
    }
    if name.starts_with("wan") {
        return name.chars().nth(3).is_none_or(|c| !c.is_ascii_alphabetic());
    }

    let mut prefixes: Vec<&str> = vec![
        "docker",
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
    if skip_veth {
        prefixes.push("veth");
    }
    if prefixes
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

    const TABLE: &str = "\
192.168.1.0/24 dev br-lan proto kernel scope link src 192.168.1.1
10.10.0.0/24 dev eth0 proto kernel scope link src 10.10.0.1
172.17.0.0/16 dev docker0 proto kernel scope link src 172.17.0.1
169.254.0.0/16 dev eth0 proto kernel scope link src 169.254.12.34
192.168.8.0/24 dev wan proto kernel scope link src 192.168.8.1
100.64.0.0/10 dev tailscale0 proto kernel scope link src 100.64.0.2
192.168.50.0/24 dev wlan0 proto kernel scope link src 192.168.50.1
192.168.100.0/24 dev veth-lan-gw proto kernel scope link src 192.168.100.1
";

    #[test]
    fn test_parse_scope_link_keeps_lan_and_skips_virtual() {
        let cidrs = parse_scope_link_cidrs(TABLE);
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
    fn test_configured_cidrs_keep_veth_and_src() {
        let links = parse_scope_link_links(TABLE, false);
        let veth = links
            .iter()
            .find(|link| link.iface == "veth-lan-gw")
            .unwrap();
        assert_eq!(veth.cidr, "192.168.100.0/24");
        assert_eq!(veth.src.as_deref(), Some("192.168.100.1"));
        assert!(links.iter().all(|link| link.iface != "docker0"));
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

    #[test]
    fn test_tun_capture_local_only_disables_auto_redirect() {
        let network = NetworkSettings {
            lan: false,
            docker: false,
            bypass_cn: false,
            ..NetworkSettings::default()
        };
        let capture = resolve_tun_capture(&network, false).unwrap();
        assert!(!capture.auto_redirect);
        assert_eq!(capture.mixed_listen, LOCAL_MIXED_LISTEN);
        assert!(capture.include_interface.is_empty());
        assert_eq!(capture.exclude_interface, vec![DOCKER_IIFNAME.to_string()]);
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn test_tun_capture_non_strict_allows_empty_ifaces() {
        let network = NetworkSettings::default();
        let capture = resolve_tun_capture(&network, false).unwrap();
        assert!(capture.auto_redirect);
        assert_eq!(capture.mixed_listen, LOCAL_MIXED_LISTEN);
        assert!(capture.include_interface.is_empty());
    }
}
