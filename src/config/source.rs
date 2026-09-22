use serde_json::Value;
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::info;

use crate::error::{CantoError, Result};

const MAX_SOURCE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceLocator {
    File(PathBuf),
    Url(String),
}

impl SourceLocator {
    pub fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(CantoError::Config(
                "sing-box source is required ([singbox].source in canto.toml)".to_string(),
            ));
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("https://") || lower.starts_with("http://") {
            return Ok(Self::Url(trimmed.to_string()));
        }
        Ok(Self::File(PathBuf::from(trimmed)))
    }

    pub fn is_url(&self) -> bool {
        matches!(self, Self::Url(_))
    }
}

impl fmt::Display for SourceLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(path) => write!(f, "{}", path.display()),
            Self::Url(url) => write!(f, "{url}"),
        }
    }
}

pub trait SourceFetcher: Send + Sync {
    fn fetch(&self, url: &str) -> impl Future<Output = Result<String>> + Send;
}

#[derive(Debug, Clone)]
pub struct HttpFetcher {
    client: reqwest::Client,
    timeout: Duration,
}

impl Default for HttpFetcher {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
            timeout: Duration::from_secs(30),
        }
    }
}

impl SourceFetcher for HttpFetcher {
    async fn fetch(&self, url: &str) -> Result<String> {
        let response = self
            .client
            .get(url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| CantoError::Config(format!("Failed to fetch source from '{url}': {e}")))?;

        let status = response.status();
        if !status.is_success() {
            return Err(CantoError::Config(format!(
                "Failed to fetch source from '{url}': HTTP {status}"
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| CantoError::Config(format!("Failed to read source from '{url}': {e}")))?;
        if bytes.len() as u64 > MAX_SOURCE_BYTES {
            return Err(CantoError::Config(format!(
                "Source from '{url}' exceeds {MAX_SOURCE_BYTES} bytes"
            )));
        }

        String::from_utf8(bytes.to_vec())
            .map_err(|e| CantoError::Config(format!("Source from '{url}' is not valid UTF-8: {e}")))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefreshOutcome {
    KeepCurrent,
    Apply { raw: Value, overlayed: Value },
}

impl RefreshOutcome {
    pub fn is_applied(&self) -> bool {
        matches!(self, Self::Apply { .. })
    }
}

pub fn source_cache_path(work_dir: &Path) -> PathBuf {
    work_dir.join("source-cache.json")
}

pub fn write_source_cache(cache_path: &Path, source: &Value) -> Result<()> {
    if let Some(parent) = cache_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let mut formatted = serde_json::to_string_pretty(source)?;
    if !formatted.ends_with('\n') {
        formatted.push('\n');
    }
    std::fs::write(cache_path, formatted)?;
    info!("Wrote last-good source cache to {}", cache_path.display());
    Ok(())
}

pub(crate) fn read_source_cache(cache_path: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(cache_path).ok()?;
    parse_source_object(&content, &cache_path.display().to_string()).ok()
}

pub(crate) fn parse_source_object(content: &str, origin: &str) -> Result<Value> {
    let value: Value = serde_json::from_str(content).map_err(|e| {
        CantoError::Config(format!("Source config '{origin}' is not valid JSON: {e}"))
    })?;
    if !value.is_object() {
        return Err(CantoError::Config(format!(
            "Source config '{origin}' must be a JSON object"
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    fn temp_cache(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canto_source_{name}_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn test_parse_source_locator() {
        assert!(SourceLocator::parse("").is_err());
        assert!(matches!(
            SourceLocator::parse("https://example.com/sub").unwrap(),
            SourceLocator::Url(_)
        ));
        assert!(matches!(
            SourceLocator::parse("/etc/sing-box/config.json").unwrap(),
            SourceLocator::File(_)
        ));
    }

    #[test]
    fn test_parse_source_object_valid_and_invalid() {
        assert!(parse_source_object(r#"{"inbounds":[]}"#, "test").is_ok());
        assert!(parse_source_object(r#"[1, 2, 3]"#, "test").is_err());
        assert!(parse_source_object(r#"not json"#, "test").is_err());
    }

    #[test]
    fn test_write_source_cache_and_read_roundtrip() {
        let cache = temp_cache("roundtrip");
        let sample = json!({"outbounds":[{"tag":"direct","type":"direct"}]});
        write_source_cache(&cache, &sample).unwrap();

        let read = read_source_cache(&cache).unwrap();
        assert_eq!(read, sample);
        fs::remove_file(&cache).ok();
    }
}
