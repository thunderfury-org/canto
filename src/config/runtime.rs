use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::config::overlay::{apply_runtime_overlay, load_source, write_runtime_config};
use crate::config::settings::{NetworkSettings, Settings};
use crate::config::source::{
    HttpFetcher, RefreshOutcome, SourceFetcher, SourceLocator, parse_source_object,
    read_source_cache, source_cache_path, write_source_cache,
};
use crate::config::validator::{ConfigValidator, SingBoxValidator};
use crate::error::{CantoError, Result};

/// Deep module managing gateway source ingestion, overlay injection,
/// syntax validation, last-good caching, and atomic runtime configuration updates.
pub struct RuntimeConfigEngine<F = HttpFetcher, V = SingBoxValidator> {
    locator: SourceLocator,
    network: NetworkSettings,
    work_dir: PathBuf,
    runtime_path: PathBuf,
    cache_path: PathBuf,
    fetcher: F,
    validator: V,
}

impl RuntimeConfigEngine<HttpFetcher, SingBoxValidator> {
    /// Constructs a production engine directly from Settings using default HttpFetcher
    /// and SingBoxValidator.
    pub fn from_settings(settings: &Settings) -> Result<Self> {
        let locator = SourceLocator::parse(&settings.singbox.source)?;
        let work_dir = settings.canto.work_dir.clone();
        let runtime_path = settings.singbox.config_path.clone();
        let cache_path = source_cache_path(&work_dir);
        let fetcher = HttpFetcher::default();
        let validator = SingBoxValidator::new(settings.singbox.binary.clone());
        Ok(Self {
            locator,
            network: settings.network.clone(),
            work_dir,
            runtime_path,
            cache_path,
            fetcher,
            validator,
        })
    }
}

impl<F, V> RuntimeConfigEngine<F, V>
where
    F: SourceFetcher,
    V: ConfigValidator,
{
    /// Constructs an engine with explicit dependencies (useful for tests with mock fetcher/validator).
    pub fn new(
        locator: SourceLocator,
        network: NetworkSettings,
        work_dir: PathBuf,
        runtime_path: PathBuf,
        fetcher: F,
        validator: V,
    ) -> Self {
        let cache_path = source_cache_path(&work_dir);
        Self {
            locator,
            network,
            work_dir,
            runtime_path,
            cache_path,
            fetcher,
            validator,
        }
    }

    pub fn locator(&self) -> &SourceLocator {
        &self.locator
    }

    pub fn is_url_source(&self) -> bool {
        self.locator.is_url()
    }

    pub fn runtime_path(&self) -> &Path {
        &self.runtime_path
    }

    pub fn cache_path(&self) -> &Path {
        &self.cache_path
    }

    /// Obtains raw source config from file or URL (falling back to cache on failure).
    pub async fn obtain_raw_source(&self) -> Result<Value> {
        match &self.locator {
            SourceLocator::Url(url) => {
                let live = match self.fetcher.fetch(url).await {
                    Ok(body) => parse_source_object(&body, url),
                    Err(err) => Err(err),
                };
                match live {
                    Ok(value) => Ok(value),
                    Err(err) => read_source_cache(&self.cache_path).ok_or(err),
                }
            }
            SourceLocator::File(path) => load_source(path),
        }
    }

    /// Prepares initial runtime config: fetches source, applies overlay, validates syntax,
    /// atomically writes to runtime path, and writes source cache if URL.
    pub async fn prepare_initial(&self) -> Result<()> {
        if let Some(parent) = self.runtime_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(&self.work_dir)?;
        info!("Loading source config from {}", self.locator);
        let raw = self.obtain_raw_source().await?;
        let overlayed = apply_runtime_overlay(raw.clone(), &self.network)?;

        let next = self.runtime_path.with_extension("json.next");
        write_runtime_config(&overlayed, &next)?;

        if let Err(e) = self.validator.validate_config(&next) {
            let _ = fs::remove_file(&next);
            return Err(e);
        }

        if let Err(e) = fs::rename(&next, &self.runtime_path) {
            let _ = fs::remove_file(&next);
            return Err(CantoError::Io(e));
        }

        if self.locator.is_url()
            && let Err(e) = write_source_cache(&self.cache_path, &raw)
        {
            warn!("Failed to write source cache: {e}");
        }

        info!(
            "Runtime configuration successfully initialized at {}",
            self.runtime_path.display()
        );
        Ok(())
    }

    /// Refreshes source at runtime.
    ///
    /// Fetches live source (if URL). If fetch fails or config check fails,
    /// upholds ADR-0004 by keeping current runtime configuration intact.
    /// Returns `Ok(RefreshOutcome::Apply)` if configuration changed and was atomically applied,
    /// or `Ok(RefreshOutcome::KeepCurrent)` if unchanged or failed.
    pub async fn refresh(&self) -> Result<RefreshOutcome> {
        let SourceLocator::Url(url) = &self.locator else {
            return Ok(RefreshOutcome::KeepCurrent);
        };

        let body = match self.fetcher.fetch(url).await {
            Ok(body) => body,
            Err(e) => {
                warn!("Source refresh fetch failed: {e}; keeping current config");
                info!("Source refresh kept the current configuration");
                return Ok(RefreshOutcome::KeepCurrent);
            }
        };

        let raw = match parse_source_object(&body, url) {
            Ok(raw) => raw,
            Err(e) => {
                warn!("Source refresh JSON parse failed: {e}; keeping current config");
                info!("Source refresh kept the current configuration");
                return Ok(RefreshOutcome::KeepCurrent);
            }
        };

        let overlayed = match apply_runtime_overlay(raw.clone(), &self.network) {
            Ok(val) => val,
            Err(e) => {
                warn!("Source refresh overlay failed: {e}; keeping current config");
                info!("Source refresh kept the current configuration");
                return Ok(RefreshOutcome::KeepCurrent);
            }
        };

        if let Some(current) = self.read_current_runtime_json()
            && current == overlayed
        {
            info!("Source refresh kept the current configuration");
            return Ok(RefreshOutcome::KeepCurrent);
        }

        let next = self.runtime_path.with_extension("json.next");
        if let Err(e) = write_runtime_config(&overlayed, &next) {
            warn!("Failed to write staging config: {e}");
            let _ = fs::remove_file(&next);
            info!("Source refresh kept the current configuration");
            return Ok(RefreshOutcome::KeepCurrent);
        }

        if let Err(e) = self.validator.validate_config(&next) {
            warn!("Refreshed config failed validation: {e}; keeping current config");
            let _ = fs::remove_file(&next);
            info!("Source refresh kept the current configuration");
            return Ok(RefreshOutcome::KeepCurrent);
        }

        if let Err(e) = fs::rename(&next, &self.runtime_path) {
            warn!("Failed to install refreshed runtime config: {e}");
            let _ = fs::remove_file(&next);
            info!("Source refresh kept the current configuration");
            return Ok(RefreshOutcome::KeepCurrent);
        }

        if let Err(e) = write_source_cache(&self.cache_path, &raw) {
            warn!("Failed to write source cache: {e}");
        }

        info!("Refreshed runtime configuration successfully applied");
        Ok(RefreshOutcome::Apply { raw, overlayed })
    }

    /// Exports overlay config to a custom output path (used by `canto config generate`).
    pub async fn export_overlay(&self, output: Option<&Path>) -> Result<PathBuf> {
        let out_path = output.unwrap_or(&self.runtime_path);
        let raw = self.obtain_raw_source().await?;
        let overlayed = apply_runtime_overlay(raw, &self.network)?;
        write_runtime_config(&overlayed, out_path)?;
        Ok(out_path.to_path_buf())
    }

    pub fn read_current_runtime_json(&self) -> Option<Value> {
        let text = fs::read_to_string(&self.runtime_path).ok()?;
        serde_json::from_str(&text).ok().filter(Value::is_object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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

    fn temp_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "canto_runtime_engine_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn test_prepare_initial_with_valid_url_source() {
        let dir = temp_test_dir("init_ok");
        let url = "https://config.example/source.json";
        let fetcher = FakeFetcher {
            responses: HashMap::from([(
                url.to_string(),
                Ok(r#"{"inbounds":[{"type":"tun","tag":"tun-in"}],"outbounds":[{"tag":"direct","type":"direct"}]}"#.to_string()),
            )]),
        };
        let runtime_path = dir.join("config.json");
        let locator = SourceLocator::parse(url).unwrap();
        let engine = RuntimeConfigEngine::new(
            locator,
            NetworkSettings::default(),
            dir.clone(),
            runtime_path.clone(),
            fetcher,
            crate::config::validator::NoopValidator,
        );

        engine.prepare_initial().await.unwrap();

        assert!(runtime_path.exists());
        assert!(engine.cache_path().exists());
        let content: Value =
            serde_json::from_str(&fs::read_to_string(&runtime_path).unwrap()).unwrap();
        assert_eq!(content["inbounds"][0]["tag"], "mixed-in");
        assert_eq!(content["inbounds"][1]["tag"], "tproxy-in");

        fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_prepare_initial_fails_and_cleans_up_on_validator_error() {
        let dir = temp_test_dir("init_fail");
        let url = "https://config.example/source.json";
        let fetcher = FakeFetcher {
            responses: HashMap::from([(
                url.to_string(),
                Ok(r#"{"inbounds":[{"type":"tun","tag":"tun-in"}]}"#.to_string()),
            )]),
        };
        let runtime_path = dir.join("config.json");
        let locator = SourceLocator::parse(url).unwrap();
        let engine = RuntimeConfigEngine::new(
            locator,
            NetworkSettings::default(),
            dir.clone(),
            runtime_path.clone(),
            fetcher,
            |_: &Path| {
                Err(CantoError::Config(
                    "syntax error in sing-box check".to_string(),
                ))
            },
        );

        let err = engine.prepare_initial().await.unwrap_err();
        assert!(err.to_string().contains("syntax error"));
        assert!(!runtime_path.exists());
        assert!(!dir.join("config.json.next").exists());
        assert!(!engine.cache_path().exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_refresh_keeps_current_on_check_failure() {
        let dir = temp_test_dir("refresh_fail");
        let url = "https://config.example/source.json";
        let initial_payload = r#"{"inbounds":[{"type":"tun","tag":"tun-in"}],"outbounds":[{"tag":"old","type":"direct"}]}"#;
        let updated_payload = r#"{"inbounds":[{"type":"tun","tag":"tun-in"}],"outbounds":[{"tag":"new","type":"direct"}]}"#;

        let fetcher = FakeFetcher {
            responses: HashMap::from([(url.to_string(), Ok(initial_payload.to_string()))]),
        };
        let runtime_path = dir.join("config.json");
        let locator = SourceLocator::parse(url).unwrap();

        // 1. Initial success
        let engine = RuntimeConfigEngine::new(
            locator.clone(),
            NetworkSettings::default(),
            dir.clone(),
            runtime_path.clone(),
            fetcher,
            crate::config::validator::NoopValidator,
        );
        engine.prepare_initial().await.unwrap();

        // 2. Refresh with failing validator
        let fetcher_updated = FakeFetcher {
            responses: HashMap::from([(url.to_string(), Ok(updated_payload.to_string()))]),
        };
        let engine_refresh = RuntimeConfigEngine::new(
            locator,
            NetworkSettings::default(),
            dir.clone(),
            runtime_path.clone(),
            fetcher_updated,
            |_: &Path| Err(CantoError::Config("invalid new config".to_string())),
        );

        let outcome = engine_refresh.refresh().await.unwrap();
        assert!(matches!(outcome, RefreshOutcome::KeepCurrent));

        // Ensure old config remains active
        let current: Value =
            serde_json::from_str(&fs::read_to_string(&runtime_path).unwrap()).unwrap();
        assert_eq!(current["outbounds"][0]["tag"], "old");

        fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_refresh_applies_and_updates_cache_when_valid() {
        let dir = temp_test_dir("refresh_ok");
        let url = "https://config.example/source.json";
        let initial_payload = r#"{"inbounds":[{"type":"tun","tag":"tun-in"}],"outbounds":[{"tag":"v1","type":"direct"}]}"#;
        let updated_payload = r#"{"inbounds":[{"type":"tun","tag":"tun-in"}],"outbounds":[{"tag":"v2","type":"direct"}]}"#;

        let fetcher = FakeFetcher {
            responses: HashMap::from([(url.to_string(), Ok(initial_payload.to_string()))]),
        };
        let runtime_path = dir.join("config.json");
        let locator = SourceLocator::parse(url).unwrap();

        let engine = RuntimeConfigEngine::new(
            locator.clone(),
            NetworkSettings::default(),
            dir.clone(),
            runtime_path.clone(),
            fetcher,
            crate::config::validator::NoopValidator,
        );
        engine.prepare_initial().await.unwrap();

        let fetcher_updated = FakeFetcher {
            responses: HashMap::from([(url.to_string(), Ok(updated_payload.to_string()))]),
        };
        let engine_refresh = RuntimeConfigEngine::new(
            locator,
            NetworkSettings::default(),
            dir.clone(),
            runtime_path.clone(),
            fetcher_updated,
            crate::config::validator::NoopValidator,
        );

        let outcome = engine_refresh.refresh().await.unwrap();
        let RefreshOutcome::Apply { overlayed, .. } = outcome else {
            panic!("expected Apply");
        };
        assert_eq!(overlayed["outbounds"][0]["tag"], "v2");

        let current: Value =
            serde_json::from_str(&fs::read_to_string(&runtime_path).unwrap()).unwrap();
        assert_eq!(current["outbounds"][0]["tag"], "v2");

        fs::remove_dir_all(&dir).ok();
    }
}
