use serde_json::Value;
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::info;

use crate::config::NetworkSettings;
use crate::config::Settings;
use crate::config::overlay::{
    apply_runtime_overlay, apply_runtime_overlay_with_capture, load_source,
};
use crate::error::{CantoError, Result};
use crate::network::TunCapture;

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

pub trait SourceFetcher {
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

pub async fn obtain_source(
    locator: &SourceLocator,
    cache_path: &Path,
    fetcher: &impl SourceFetcher,
) -> Result<Value> {
    match locator {
        SourceLocator::Url(url) => {
            let live = match fetcher.fetch(url).await {
                Ok(body) => parse_source_object(&body, url),
                Err(err) => Err(err),
            };
            match live {
                Ok(value) => Ok(value),
                Err(err) => read_source_cache(cache_path).ok_or(err),
            }
        }
        SourceLocator::File(path) => load_source(path),
    }
}

pub async fn prepare_runtime_config(
    settings: &Settings,
    fetcher: &impl SourceFetcher,
) -> Result<Value> {
    let locator = SourceLocator::parse(&settings.singbox.source)?;
    let cache = source_cache_path(&settings.canto.work_dir);
    let source = obtain_source(&locator, &cache, fetcher).await?;
    apply_runtime_overlay(source, &settings.network)
}

pub enum RefreshOutcome {
    KeepCurrent,
    Apply { raw: Value, overlayed: Value },
}

pub async fn refresh_source(
    locator: &SourceLocator,
    fetcher: &impl SourceFetcher,
    network: &NetworkSettings,
    capture: Option<&TunCapture>,
    check: impl Fn(&Value) -> Result<()>,
) -> RefreshOutcome {
    let live = match live_source(locator, fetcher).await {
        Ok(value) => value,
        Err(_) => return RefreshOutcome::KeepCurrent,
    };
    let overlayed = match apply_runtime_overlay_with_capture(live.clone(), network, capture) {
        Ok(value) => value,
        Err(_) => return RefreshOutcome::KeepCurrent,
    };
    match check(&overlayed) {
        Ok(()) => RefreshOutcome::Apply {
            raw: live,
            overlayed,
        },
        Err(_) => RefreshOutcome::KeepCurrent,
    }
}

async fn live_source(locator: &SourceLocator, fetcher: &impl SourceFetcher) -> Result<Value> {
    match locator {
        SourceLocator::Url(url) => parse_source_object(&fetcher.fetch(url).await?, url),
        SourceLocator::File(path) => load_source(path),
    }
}

fn read_source_cache(cache_path: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(cache_path).ok()?;
    parse_source_object(&content, &cache_path.display().to_string()).ok()
}

fn parse_source_object(content: &str, origin: &str) -> Result<Value> {
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
    use crate::error::CantoError;
    use serde_json::json;
    use std::collections::HashMap;
    use std::fs;

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

    #[tokio::test]
    async fn test_obtains_json_object_from_url() {
        let url = "https://config.example/source.json";
        let fetcher = FakeFetcher {
            responses: HashMap::from([(
                url.to_string(),
                Ok(r#"{"outbounds":[{"tag":"直连","type":"direct"}]}"#.to_string()),
            )]),
        };
        let cache = temp_cache("url_ok");
        let locator = SourceLocator::parse(url).unwrap();
        let source = obtain_source(&locator, &cache, &fetcher).await.unwrap();
        fs::remove_file(&cache).ok();

        assert_eq!(
            source,
            json!({"outbounds":[{"tag":"直连","type":"direct"}]})
        );
    }

    #[tokio::test]
    async fn test_url_fetch_failure_uses_cache() {
        let url = "https://config.example/source.json";
        let fetcher = FakeFetcher {
            responses: HashMap::from([(url.to_string(), Err("connection refused".to_string()))]),
        };
        let cache = temp_cache("url_cache");
        fs::write(
            &cache,
            r#"{"outbounds":[{"tag":"缓存节点","type":"direct"}]}"#,
        )
        .unwrap();
        let locator = SourceLocator::parse(url).unwrap();
        let source = obtain_source(&locator, &cache, &fetcher).await.unwrap();
        fs::remove_file(&cache).ok();

        assert_eq!(
            source,
            json!({"outbounds":[{"tag":"缓存节点","type":"direct"}]})
        );
    }

    #[tokio::test]
    async fn test_url_fetch_failure_without_cache_errors() {
        let url = "https://config.example/source.json";
        let fetcher = FakeFetcher {
            responses: HashMap::from([(url.to_string(), Err("connection refused".to_string()))]),
        };
        let cache = temp_cache("url_nocache");
        let locator = SourceLocator::parse(url).unwrap();
        let err = obtain_source(&locator, &cache, &fetcher)
            .await
            .unwrap_err()
            .to_string();
        fs::remove_file(&cache).ok();
        assert!(err.contains("connection refused"), "{err}");
    }

    #[tokio::test]
    async fn test_file_source_loads_local_json() {
        let path = temp_cache("file_source");
        fs::write(
            &path,
            r#"{"outbounds":[{"tag":"文件节点","type":"direct"}]}"#,
        )
        .unwrap();
        let fetcher = FakeFetcher {
            responses: HashMap::new(),
        };
        let locator = SourceLocator::parse(path.to_str().unwrap()).unwrap();
        let source = obtain_source(&locator, &temp_cache("unused"), &fetcher)
            .await
            .unwrap();
        fs::remove_file(&path).ok();
        assert_eq!(
            source,
            json!({"outbounds":[{"tag":"文件节点","type":"direct"}]})
        );
    }

    #[tokio::test]
    async fn test_refresh_keeps_current_when_fetch_fails() {
        let url = "https://config.example/source.json";
        let fetcher = FakeFetcher {
            responses: HashMap::from([(url.to_string(), Err("timeout".to_string()))]),
        };
        let locator = SourceLocator::parse(url).unwrap();
        let outcome = refresh_source(
            &locator,
            &fetcher,
            &NetworkSettings::default(),
            None,
            |_| panic!("check should not run when fetch fails"),
        )
        .await;
        assert!(matches!(outcome, RefreshOutcome::KeepCurrent));
    }

    #[tokio::test]
    async fn test_refresh_keeps_current_when_check_fails() {
        let url = "https://config.example/source.json";
        let fetcher = FakeFetcher {
            responses: HashMap::from([(
                url.to_string(),
                Ok(r#"{"outbounds":[{"tag":"直连","type":"direct"}]}"#.to_string()),
            )]),
        };
        let locator = SourceLocator::parse(url).unwrap();
        let outcome = refresh_source(
            &locator,
            &fetcher,
            &NetworkSettings::default(),
            None,
            |_| Err(CantoError::Config("sing-box check failed".to_string())),
        )
        .await;
        assert!(matches!(outcome, RefreshOutcome::KeepCurrent));
    }

    #[tokio::test]
    async fn test_refresh_applies_overlayed_config_when_check_passes() {
        let url = "https://config.example/source.json";
        let fetcher = FakeFetcher {
            responses: HashMap::from([(
                url.to_string(),
                Ok(r#"{"inbounds":[{"type":"tun","tag":"tun-in"}],"outbounds":[{"tag":"直连","type":"direct"}]}"#.to_string()),
            )]),
        };
        let locator = SourceLocator::parse(url).unwrap();
        let outcome = refresh_source(
            &locator,
            &fetcher,
            &NetworkSettings {
                bypass_cn: false,
                ..NetworkSettings::default()
            },
            None,
            |_| Ok(()),
        )
        .await;
        let RefreshOutcome::Apply { raw, overlayed } = outcome else {
            panic!("expected Apply, got KeepCurrent");
        };
        assert_eq!(raw["inbounds"][0]["tag"], "tun-in");
        assert_eq!(overlayed["inbounds"][0]["tag"], "mixed-in");
        assert_eq!(overlayed["inbounds"][1]["tag"], "tun-in");
        assert_eq!(overlayed["outbounds"][0]["tag"], "直连");
        assert!(overlayed["route"].get("default_mark").is_none());
    }

    #[tokio::test]
    async fn test_write_source_cache_roundtrips_for_fetch_fallback() {
        let url = "https://config.example/source.json";
        let fetcher = FakeFetcher {
            responses: HashMap::from([(url.to_string(), Err("offline".to_string()))]),
        };
        let cache = temp_cache("written_cache");
        write_source_cache(
            &cache,
            &json!({"outbounds":[{"tag":"上次成功","type":"direct"}]}),
        )
        .unwrap();
        let locator = SourceLocator::parse(url).unwrap();
        let source = obtain_source(&locator, &cache, &fetcher).await.unwrap();
        fs::remove_file(&cache).ok();
        assert_eq!(
            source,
            json!({"outbounds":[{"tag":"上次成功","type":"direct"}]})
        );
    }

    #[tokio::test]
    async fn test_url_invalid_json_uses_cache() {
        let url = "https://config.example/source.json";
        let fetcher = FakeFetcher {
            responses: HashMap::from([(url.to_string(), Ok("not-json".to_string()))]),
        };
        let cache = temp_cache("bad_json_cache");
        fs::write(
            &cache,
            r#"{"outbounds":[{"tag":"缓存节点","type":"direct"}]}"#,
        )
        .unwrap();
        let locator = SourceLocator::parse(url).unwrap();
        let source = obtain_source(&locator, &cache, &fetcher).await.unwrap();
        fs::remove_file(&cache).ok();
        assert_eq!(
            source,
            json!({"outbounds":[{"tag":"缓存节点","type":"direct"}]})
        );
    }
}
