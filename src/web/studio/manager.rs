use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

use crate::error::Result;
use crate::web::studio::model::{NodeSource, SourceKind};
use crate::web::studio::parser::parse_subscription;
use crate::web::studio::store::SourceStore;

const MAX_SUBSCRIPTION_BYTES: usize = 10 * 1024 * 1024;
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Deep module managing node source ingestion, subscription fetching,
/// dialect decoding, and node state transitions.
#[derive(Debug, Clone)]
pub struct NodeSourceManager {
    store: Arc<SourceStore>,
    client: reqwest::Client,
}

impl NodeSourceManager {
    pub fn new(store: Arc<SourceStore>, client: reqwest::Client) -> Self {
        Self { store, client }
    }

    pub fn store(&self) -> &Arc<SourceStore> {
        &self.store
    }

    /// Ingests node definitions into a source.
    /// If Subscription kind, fetches and parses content from URL.
    /// If Manual kind with provided content string, parses it.
    pub async fn ingest(&self, source: &mut NodeSource) -> std::result::Result<(), String> {
        match source.kind {
            SourceKind::Subscription => {
                let Some(url) = source.url.clone() else {
                    return Err("subscription source requires a url".to_string());
                };
                if !is_http_url(&url) {
                    return Err("subscription url must start with http:// or https://".to_string());
                }
                self.fetch_and_apply_nodes(source, &url).await;
                Ok(())
            }
            SourceKind::Manual => {
                if let Some(content) = source.content.clone() {
                    match parse_subscription(&content) {
                        Ok(nodes) => {
                            source.mark_active(nodes);
                            Ok(())
                        }
                        Err(err) => Err(format!("failed to parse node definitions: {err}")),
                    }
                } else if !source.nodes.is_empty() {
                    source.mark_active(source.nodes.clone());
                    Ok(())
                } else {
                    Err("manual source requires content or nodes".to_string())
                }
            }
        }
    }

    /// Refreshes an existing source by ID.
    pub async fn refresh(&self, id: &str) -> Result<Option<NodeSource>> {
        let Some(mut source) = self.store.get(id).await else {
            return Ok(None);
        };

        match source.kind {
            SourceKind::Subscription => {
                let Some(url) = source.url.clone() else {
                    source.mark_error("subscription source requires a url".to_string());
                    return self.store.replace(source).await;
                };
                self.fetch_and_apply_nodes(&mut source, &url).await;
            }
            SourceKind::Manual => {
                if let Some(content) = source.content.clone() {
                    match parse_subscription(&content) {
                        Ok(nodes) => source.mark_active(nodes),
                        Err(err) => source.mark_error(err),
                    }
                } else if source.nodes.is_empty() {
                    source.mark_error("manual source has no content to refresh".to_string());
                } else {
                    source.mark_active(source.nodes.clone());
                }
            }
        }

        self.store.replace(source).await
    }

    async fn fetch_and_apply_nodes(&self, source: &mut NodeSource, url: &str) {
        match self.fetch_subscription(url).await {
            Ok(body) => match parse_subscription(&body) {
                Ok(nodes) => source.mark_active(nodes),
                Err(err) => {
                    warn!("Failed to parse subscription from '{url}': {err}");
                    source.mark_error(err);
                }
            },
            Err(err) => {
                warn!("{err}");
                source.mark_error(err);
            }
        }
    }

    pub async fn fetch_subscription(&self, url: &str) -> std::result::Result<String, String> {
        let response = self
            .client
            .get(url)
            .header(
                "User-Agent",
                format!("canto/{} (sing-box)", env!("CARGO_PKG_VERSION")),
            )
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .send()
            .await
            .map_err(|err| format!("failed to fetch '{url}': {err}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("failed to fetch '{url}': HTTP {status}"));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|err| format!("failed to read '{url}': {err}"))?;
        if bytes.len() > MAX_SUBSCRIPTION_BYTES {
            return Err(format!("subscription from '{url}' exceeds 10MB"));
        }
        String::from_utf8(bytes.to_vec())
            .map_err(|err| format!("subscription from '{url}' is not valid UTF-8: {err}"))
    }
}

pub fn is_http_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::studio::model::SourceStatus;
    use std::path::PathBuf;

    fn temp_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "canto_manager_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn test_ingest_manual_source_content() {
        let dir = temp_test_dir("manual_ok");
        let store = Arc::new(SourceStore::empty(&dir));
        let manager = NodeSourceManager::new(store, reqwest::Client::new());

        let mut source = NodeSource {
            id: "src_1".to_string(),
            name: "手动源".to_string(),
            kind: SourceKind::Manual,
            url: None,
            content: Some("vless://user@host.com:443#HK-Node".to_string()),
            status: SourceStatus::Idle,
            last_updated: None,
            last_error: None,
            node_count: 0,
            nodes: Vec::new(),
        };

        manager.ingest(&mut source).await.unwrap();
        assert_eq!(source.status, SourceStatus::Active);
        assert_eq!(source.node_count, 1);
        assert_eq!(source.nodes[0]["tag"], "HK-Node");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_ingest_manual_source_invalid_content() {
        let dir = temp_test_dir("manual_err");
        let store = Arc::new(SourceStore::empty(&dir));
        let manager = NodeSourceManager::new(store, reqwest::Client::new());

        let mut source = NodeSource {
            id: "src_2".to_string(),
            name: "错误手动源".to_string(),
            kind: SourceKind::Manual,
            url: None,
            content: Some("not-valid-node".to_string()),
            status: SourceStatus::Idle,
            last_updated: None,
            last_error: None,
            node_count: 0,
            nodes: Vec::new(),
        };

        let err = manager.ingest(&mut source).await.unwrap_err();
        assert!(err.contains("failed to parse"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_ingest_subscription_requires_http_url() {
        let dir = temp_test_dir("sub_url_err");
        let store = Arc::new(SourceStore::empty(&dir));
        let manager = NodeSourceManager::new(store, reqwest::Client::new());

        let mut source = NodeSource {
            id: "src_3".to_string(),
            name: "非HTTP订阅".to_string(),
            kind: SourceKind::Subscription,
            url: Some("ftp://example.com/nodes".to_string()),
            content: None,
            status: SourceStatus::Idle,
            last_updated: None,
            last_error: None,
            node_count: 0,
            nodes: Vec::new(),
        };

        let err = manager.ingest(&mut source).await.unwrap_err();
        assert!(err.contains("http:// or https://"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_refresh_manual_source() {
        let dir = temp_test_dir("refresh_manual");
        let store = Arc::new(SourceStore::empty(&dir));
        let manager = NodeSourceManager::new(Arc::clone(&store), reqwest::Client::new());

        let source = NodeSource {
            id: "src_4".to_string(),
            name: "待刷新源".to_string(),
            kind: SourceKind::Manual,
            url: None,
            content: Some("trojan://pass@host.com:443#Trojan-Node".to_string()),
            status: SourceStatus::Idle,
            last_updated: None,
            last_error: None,
            node_count: 0,
            nodes: Vec::new(),
        };
        store.insert(source).await.unwrap();

        let updated = manager.refresh("src_4").await.unwrap().unwrap();
        assert_eq!(updated.status, SourceStatus::Active);
        assert_eq!(updated.node_count, 1);
        assert_eq!(updated.nodes[0]["tag"], "Trojan-Node");

        std::fs::remove_dir_all(&dir).ok();
    }
}
