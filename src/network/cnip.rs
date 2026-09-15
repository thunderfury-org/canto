use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::config::SourceFetcher;
use crate::error::{CantoError, Result};
use crate::network::lan::is_ipv4_cidr;

pub const CN_IP_FILE: &str = "cn_ip.txt";
pub const DEFAULT_CN_IP_URL: &str =
    "https://testingcf.jsdelivr.net/gh/juewuy/ShellCrash@master/bin/geodata/china_ip_list.txt";
const MAX_CN_IP_BYTES: usize = 2 * 1024 * 1024;

pub fn cn_ip_path(work_dir: &Path) -> PathBuf {
    work_dir.join(CN_IP_FILE)
}

pub fn parse_cn_ip_text(text: &str) -> Result<Vec<String>> {
    let mut cidrs = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let token = trimmed.split_whitespace().next().unwrap_or(trimmed);
        match normalize_ipv4_cidr(token) {
            Some(cidr) => {
                if !cidrs.iter().any(|existing| existing == &cidr) {
                    cidrs.push(cidr);
                }
            }
            None => {
                warn!("Skipping invalid CN CIDR on line {}: {token}", idx + 1);
            }
        }
    }
    if cidrs.is_empty() {
        return Err(CantoError::Config(
            "cn_ip.txt contains no valid IPv4 CIDRs".to_string(),
        ));
    }
    Ok(cidrs)
}

pub fn read_cn_ip_file(work_dir: &Path) -> Result<Option<Vec<String>>> {
    let path = cn_ip_path(work_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| CantoError::Config(format!("Failed to read '{}': {e}", path.display())))?;
    if content.len() > MAX_CN_IP_BYTES {
        return Err(CantoError::Config(format!(
            "'{}' exceeds {MAX_CN_IP_BYTES} bytes",
            path.display()
        )));
    }
    match parse_cn_ip_text(&content) {
        Ok(cidrs) => Ok(Some(cidrs)),
        Err(err) => {
            warn!(
                "{} is unusable ({err}); treating it as missing",
                path.display()
            );
            Ok(None)
        }
    }
}

pub async fn load_cn_ip(work_dir: &Path, fetcher: &impl SourceFetcher) -> Result<Vec<String>> {
    let path = cn_ip_path(work_dir);
    if let Some(cidrs) = read_cn_ip_file(work_dir)? {
        info!("Using {} CN CIDRs from {}", cidrs.len(), path.display());
        return Ok(cidrs);
    }

    info!("Downloading CN IP list from {DEFAULT_CN_IP_URL}");
    let body = fetcher.fetch(DEFAULT_CN_IP_URL).await.map_err(|e| {
        CantoError::Config(format!(
            "Failed to download cn_ip.txt from '{DEFAULT_CN_IP_URL}': {e}"
        ))
    })?;
    if body.len() > MAX_CN_IP_BYTES {
        return Err(CantoError::Config(format!(
            "CN IP list from '{DEFAULT_CN_IP_URL}' exceeds {MAX_CN_IP_BYTES} bytes"
        )));
    }
    let cidrs = parse_cn_ip_text(&body)?;
    write_cn_ip_file(&path, &body)?;
    info!("Wrote {} CN CIDRs to {}", cidrs.len(), path.display());
    Ok(cidrs)
}

fn write_cn_ip_file(path: &Path, body: &str) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, body)
        .map_err(|e| CantoError::Config(format!("Failed to write '{}': {e}", path.display())))
}

fn normalize_ipv4_cidr(token: &str) -> Option<String> {
    if token.contains(':') {
        return None;
    }
    if is_ipv4_cidr(token) {
        return Some(token.to_string());
    }
    if is_ipv4_addr(token) {
        return Some(format!("{token}/32"));
    }
    None
}

fn is_ipv4_addr(value: &str) -> bool {
    is_ipv4_cidr(&format!("{value}/32"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CantoError;
    use std::collections::HashMap;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct FakeFetcher {
        responses: HashMap<String, std::result::Result<String, String>>,
    }

    impl SourceFetcher for FakeFetcher {
        async fn fetch(&self, url: &str) -> Result<String> {
            match self.responses.get(url) {
                Some(Ok(body)) => Ok(body.clone()),
                Some(Err(message)) => Err(CantoError::Config(message.clone())),
                None => Err(CantoError::Config(format!("unexpected fetch of {url}"))),
            }
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("canto_cnip_{name}_{}_{nanos}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_parses_comments_ipv6_bare_ip_and_dedupes() {
        let cidrs = parse_cn_ip_text(
            r#"
# comment
1.1.8.0/24
1.1.8.0/24
9.9.9.10
2400:3200::/32
not-a-cidr
1.2.4.0/24 extra
"#,
        )
        .unwrap();
        assert_eq!(
            cidrs,
            vec![
                "1.1.8.0/24".to_string(),
                "9.9.9.10/32".to_string(),
                "1.2.4.0/24".to_string(),
            ]
        );
    }

    #[test]
    fn test_parse_rejects_empty_list() {
        let err = parse_cn_ip_text("# none\n2400::/32\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("no valid IPv4 CIDRs"), "{err}");
    }

    #[tokio::test]
    async fn test_load_uses_existing_file_without_fetch() {
        let dir = temp_dir("existing");
        fs::write(cn_ip_path(&dir), "1.1.8.0/24\n9.9.9.10/32\n").unwrap();
        let fetcher = FakeFetcher {
            responses: HashMap::new(),
        };
        let cidrs = load_cn_ip(&dir, &fetcher).await.unwrap();
        fs::remove_dir_all(&dir).ok();
        assert_eq!(
            cidrs,
            vec!["1.1.8.0/24".to_string(), "9.9.9.10/32".to_string()]
        );
    }

    #[tokio::test]
    async fn test_load_downloads_when_missing() {
        let dir = temp_dir("download");
        let fetcher = FakeFetcher {
            responses: HashMap::from([(
                DEFAULT_CN_IP_URL.to_string(),
                Ok("1.1.8.0/24\n# skip\n1.2.4.0/24\n".to_string()),
            )]),
        };
        let cidrs = load_cn_ip(&dir, &fetcher).await.unwrap();
        let written = fs::read_to_string(cn_ip_path(&dir)).unwrap();
        fs::remove_dir_all(&dir).ok();
        assert_eq!(
            cidrs,
            vec!["1.1.8.0/24".to_string(), "1.2.4.0/24".to_string()]
        );
        assert!(written.contains("1.1.8.0/24"));
    }

    #[tokio::test]
    async fn test_load_errors_when_fetch_fails_and_file_missing() {
        let dir = temp_dir("fail");
        let fetcher = FakeFetcher {
            responses: HashMap::from([(
                DEFAULT_CN_IP_URL.to_string(),
                Err("connection refused".to_string()),
            )]),
        };
        let err = load_cn_ip(&dir, &fetcher).await.unwrap_err().to_string();
        fs::remove_dir_all(&dir).ok();
        assert!(err.contains("Failed to download cn_ip.txt"), "{err}");
        assert!(err.contains("connection refused"), "{err}");
    }

    #[tokio::test]
    async fn test_unusable_file_is_redownloaded() {
        let dir = temp_dir("unusable");
        fs::write(cn_ip_path(&dir), "# empty\n2400::/32\n").unwrap();
        let fetcher = FakeFetcher {
            responses: HashMap::from([(
                DEFAULT_CN_IP_URL.to_string(),
                Ok("8.8.8.0/24\n".to_string()),
            )]),
        };
        let cidrs = load_cn_ip(&dir, &fetcher).await.unwrap();
        fs::remove_dir_all(&dir).ok();
        assert_eq!(cidrs, vec!["8.8.8.0/24".to_string()]);
    }
}
