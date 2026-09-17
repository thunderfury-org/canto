use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::error::{CantoError, Result};
use crate::web::studio::model::NodeSource;

#[derive(Debug, Serialize, Deserialize, Default)]
struct SourcesFile {
    sources: Vec<NodeSource>,
}

#[derive(Debug)]
pub struct SourceStore {
    path: PathBuf,
    inner: RwLock<Vec<NodeSource>>,
}

impl SourceStore {
    pub fn load(work_dir: impl AsRef<Path>) -> Result<Self> {
        let path = work_dir.as_ref().join("studio").join("sources.json");
        let sources = load_sources(&path)?;
        Ok(Self {
            path,
            inner: RwLock::new(sources),
        })
    }

    pub fn empty(work_dir: impl AsRef<Path>) -> Self {
        Self {
            path: work_dir.as_ref().join("studio").join("sources.json"),
            inner: RwLock::new(Vec::new()),
        }
    }

    pub async fn list(&self) -> Vec<NodeSource> {
        self.inner.read().await.clone()
    }

    pub async fn get(&self, id: &str) -> Option<NodeSource> {
        self.inner
            .read()
            .await
            .iter()
            .find(|source| source.id == id)
            .cloned()
    }

    pub async fn insert(&self, mut source: NodeSource) -> Result<NodeSource> {
        source.sync_counts();
        let mut guard = self.inner.write().await;
        if guard.iter().any(|item| item.id == source.id) {
            return Err(CantoError::Web(format!(
                "source '{}' already exists",
                source.id
            )));
        }
        guard.push(source.clone());
        persist(&self.path, &guard)?;
        Ok(source)
    }

    pub async fn replace(&self, mut source: NodeSource) -> Result<Option<NodeSource>> {
        source.sync_counts();
        let mut guard = self.inner.write().await;
        let Some(existing) = guard.iter_mut().find(|item| item.id == source.id) else {
            return Ok(None);
        };
        *existing = source.clone();
        persist(&self.path, &guard)?;
        Ok(Some(source))
    }

    pub async fn delete(&self, id: &str) -> Result<bool> {
        let mut guard = self.inner.write().await;
        let before = guard.len();
        guard.retain(|source| source.id != id);
        if guard.len() == before {
            return Ok(false);
        }
        persist(&self.path, &guard)?;
        Ok(true)
    }
}

fn load_sources(path: &Path) -> Result<Vec<NodeSource>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)?;
    match parse_sources_file(&text) {
        Ok(mut sources) => {
            for source in &mut sources {
                source.sync_counts();
            }
            Ok(sources)
        }
        Err(err) => {
            let bak = path.with_extension("json.bak");
            warn!(
                "Failed to parse {}; backing up to {} ({err})",
                path.display(),
                bak.display()
            );
            let _ = fs::rename(path, &bak);
            Ok(Vec::new())
        }
    }
}

fn parse_sources_file(text: &str) -> std::result::Result<Vec<NodeSource>, serde_json::Error> {
    if let Ok(file) = serde_json::from_str::<SourcesFile>(text) {
        return Ok(file.sources);
    }
    serde_json::from_str::<Vec<NodeSource>>(text)
}

fn persist(path: &Path, sources: &[NodeSource]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = SourcesFile {
        sources: sources.to_vec(),
    };
    let mut formatted = serde_json::to_string_pretty(&file)?;
    if !formatted.ends_with('\n') {
        formatted.push('\n');
    }
    atomic_write(path, formatted.as_bytes())?;
    info!("Wrote node sources to {}", path.display());
    Ok(())
}

fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let tmp_name = format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("sources.json")
    );
    let tmp_path = path.with_file_name(tmp_name);
    {
        let mut file = File::create(&tmp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::studio::model::{SourceKind, SourceStatus};
    use serde_json::json;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "canto_sources_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn sample_source(id: &str) -> NodeSource {
        let mut source = NodeSource {
            id: id.to_string(),
            name: "manual".to_string(),
            kind: SourceKind::Manual,
            url: None,
            content: Some("vless://uuid@example.com:443#n".to_string()),
            status: SourceStatus::Active,
            last_updated: Some("2026-01-01T00:00:00Z".to_string()),
            last_error: None,
            node_count: 0,
            nodes: vec![json!({"type":"vless","tag":"n","server":"example.com","server_port":443})],
        };
        source.sync_counts();
        source
    }

    #[tokio::test]
    async fn test_atomic_roundtrip() {
        let dir = temp_dir("roundtrip");
        let store = SourceStore::load(&dir).unwrap();
        assert!(store.list().await.is_empty());

        store.insert(sample_source("src_a")).await.unwrap();
        let path = dir.join("studio").join("sources.json");
        assert!(path.exists());
        let persisted = fs::read_to_string(&path).unwrap();
        assert!(persisted.contains("\"sources\""));
        assert!(persisted.contains("src_a"));

        let reloaded = SourceStore::load(&dir).unwrap();
        let listed = reloaded.list().await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "src_a");
        assert_eq!(listed[0].node_count, 1);
        fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_corrupt_file_is_backed_up() {
        let dir = temp_dir("corrupt");
        let path = dir.join("studio");
        fs::create_dir_all(&path).unwrap();
        let file = path.join("sources.json");
        fs::write(&file, "not-json").unwrap();
        let store = SourceStore::load(&dir).unwrap();
        assert!(store.list().await.is_empty());
        assert!(path.join("sources.json.bak").exists());
        fs::remove_dir_all(&dir).ok();
    }
}
