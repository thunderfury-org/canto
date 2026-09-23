use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::error::{CantoError, Result};
use crate::web::studio::model::{
    NodeSource, Profile, Template, TemplateMeta, is_safe_id, validate_template_content,
};
use serde_json::Value;

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

#[derive(Debug, Serialize, Deserialize, Default)]
struct ProfilesFile {
    profiles: Vec<Profile>,
}

#[derive(Debug)]
pub struct ProfileStore {
    path: PathBuf,
    inner: RwLock<Vec<Profile>>,
}

impl ProfileStore {
    pub fn load(work_dir: impl AsRef<Path>) -> Result<Self> {
        let path = work_dir.as_ref().join("studio").join("profiles.json");
        let profiles = load_profiles(&path)?;
        Ok(Self {
            path,
            inner: RwLock::new(profiles),
        })
    }

    pub fn empty(work_dir: impl AsRef<Path>) -> Self {
        Self {
            path: work_dir.as_ref().join("studio").join("profiles.json"),
            inner: RwLock::new(Vec::new()),
        }
    }

    pub async fn list(&self) -> Vec<Profile> {
        self.inner.read().await.clone()
    }

    pub async fn get(&self, id: &str) -> Option<Profile> {
        self.inner
            .read()
            .await
            .iter()
            .find(|profile| profile.id == id)
            .cloned()
    }

    pub async fn get_by_token(&self, token: &str) -> Option<Profile> {
        self.inner
            .read()
            .await
            .iter()
            .find(|profile| profile.token == token)
            .cloned()
    }

    pub async fn insert(&self, profile: Profile) -> Result<Profile> {
        let mut guard = self.inner.write().await;
        if guard.iter().any(|item| item.id == profile.id) {
            return Err(CantoError::Web(format!(
                "profile '{}' already exists",
                profile.id
            )));
        }
        if guard.iter().any(|item| item.token == profile.token) {
            return Err(CantoError::Web("profile token already exists".to_string()));
        }
        guard.push(profile.clone());
        persist_profiles(&self.path, &guard)?;
        Ok(profile)
    }

    pub async fn replace(&self, profile: Profile) -> Result<Option<Profile>> {
        let mut guard = self.inner.write().await;
        if !guard.iter().any(|item| item.id == profile.id) {
            return Ok(None);
        }
        if guard
            .iter()
            .any(|item| item.id != profile.id && item.token == profile.token)
        {
            return Err(CantoError::Web("profile token already exists".to_string()));
        }
        if let Some(existing) = guard.iter_mut().find(|item| item.id == profile.id) {
            *existing = profile.clone();
        }
        persist_profiles(&self.path, &guard)?;
        Ok(Some(profile))
    }

    pub async fn delete(&self, id: &str) -> Result<bool> {
        let mut guard = self.inner.write().await;
        let before = guard.len();
        guard.retain(|profile| profile.id != id);
        if guard.len() == before {
            return Ok(false);
        }
        persist_profiles(&self.path, &guard)?;
        Ok(true)
    }

    pub async fn unbind_source(&self, source_id: &str) -> Result<()> {
        let mut guard = self.inner.write().await;
        let mut changed = false;
        for profile in guard.iter_mut() {
            let before = profile.source_ids.len();
            profile.source_ids.retain(|id| id != source_id);
            if profile.source_ids.len() != before {
                changed = true;
            }
        }
        if changed {
            persist_profiles(&self.path, &guard)?;
        }
        Ok(())
    }

    pub async fn find_by_template(&self, template_id: &str) -> Vec<Profile> {
        self.inner
            .read()
            .await
            .iter()
            .filter(|profile| profile.template_id == template_id)
            .cloned()
            .collect()
    }

    pub async fn check_source_removal(&self, source_id: &str) -> Vec<Profile> {
        self.inner
            .read()
            .await
            .iter()
            .filter(|profile| {
                profile.source_ids.contains(&source_id.to_string()) && profile.source_ids.len() <= 1
            })
            .cloned()
            .collect()
    }
}

fn load_profiles(path: &Path) -> Result<Vec<Profile>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)?;
    match parse_profiles_file(&text) {
        Ok(profiles) => Ok(profiles),
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

fn parse_profiles_file(text: &str) -> std::result::Result<Vec<Profile>, serde_json::Error> {
    if let Ok(file) = serde_json::from_str::<ProfilesFile>(text) {
        return Ok(file.profiles);
    }
    serde_json::from_str::<Vec<Profile>>(text)
}

fn persist_profiles(path: &Path, profiles: &[Profile]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = ProfilesFile {
        profiles: profiles
            .iter()
            .cloned()
            .map(|mut profile| {
                profile.strip_public_url();
                profile
            })
            .collect(),
    };
    let mut formatted = serde_json::to_string_pretty(&file)?;
    if !formatted.ends_with('\n') {
        formatted.push('\n');
    }
    atomic_write(path, formatted.as_bytes())?;
    info!("Wrote profiles to {}", path.display());
    Ok(())
}

#[derive(Debug)]
pub struct TemplateStore {
    dir: PathBuf,
    inner: RwLock<Vec<Template>>,
}

impl TemplateStore {
    pub fn load(work_dir: impl AsRef<Path>) -> Result<Self> {
        let dir = work_dir.as_ref().join("studio").join("templates");
        let templates = load_templates(&dir)?;
        Ok(Self {
            dir,
            inner: RwLock::new(templates),
        })
    }

    pub fn empty(work_dir: impl AsRef<Path>) -> Self {
        Self {
            dir: work_dir.as_ref().join("studio").join("templates"),
            inner: RwLock::new(Vec::new()),
        }
    }

    pub async fn list(&self) -> Vec<Template> {
        self.inner.read().await.clone()
    }

    pub async fn get(&self, id: &str) -> Option<Template> {
        self.inner
            .read()
            .await
            .iter()
            .find(|template| template.id == id)
            .cloned()
    }

    pub async fn insert(&self, template: Template) -> Result<Template> {
        if !is_safe_id(&template.id) {
            return Err(CantoError::Web(format!(
                "invalid template id '{}'",
                template.id
            )));
        }
        let mut guard = self.inner.write().await;
        if guard.iter().any(|item| item.id == template.id) {
            return Err(CantoError::Web(format!(
                "template '{}' already exists",
                template.id
            )));
        }
        persist_template(&self.dir, &template)?;
        guard.push(template.clone());
        Ok(template)
    }

    pub async fn replace(&self, template: Template) -> Result<Option<Template>> {
        if !is_safe_id(&template.id) {
            return Err(CantoError::Web(format!(
                "invalid template id '{}'",
                template.id
            )));
        }
        let mut guard = self.inner.write().await;
        let Some(existing) = guard.iter_mut().find(|item| item.id == template.id) else {
            return Ok(None);
        };
        persist_template(&self.dir, &template)?;
        *existing = template.clone();
        Ok(Some(template))
    }

    pub async fn delete(&self, id: &str) -> Result<bool> {
        let mut guard = self.inner.write().await;
        if !guard.iter().any(|template| template.id == id) {
            return Ok(false);
        }
        let dir = template_dir(&self.dir, id);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        let legacy_file = self.dir.join(format!("{id}.json"));
        if legacy_file.exists() {
            let _ = fs::remove_file(&legacy_file);
        }
        guard.retain(|template| template.id != id);
        Ok(true)
    }
}

fn template_dir(dir: &Path, id: &str) -> PathBuf {
    dir.join(id)
}

fn is_safe_module_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn module_filename(key: &str) -> Option<String> {
    if !is_safe_module_key(key) {
        return None;
    }
    let filename = match key {
        "node_groups" => "canto.node_groups.json".to_string(),
        "policy_groups" => "canto.policy_groups.json".to_string(),
        "rule_sets" => "canto.rule_sets.json".to_string(),
        "outbounds" => return None,
        other => format!("{other}.json"),
    };
    Some(filename)
}

fn module_key_from_filename(filename: &str) -> Option<String> {
    if !filename.ends_with(".json") || filename.starts_with('.') || filename == "meta.json" {
        return None;
    }
    match filename {
        "canto.node_groups.json" => Some("node_groups".to_string()),
        "canto.policy_groups.json" => Some("policy_groups".to_string()),
        "canto.rule_sets.json" => Some("rule_sets".to_string()),
        other => {
            let stem = other.trim_end_matches(".json");
            if is_safe_module_key(stem) {
                Some(stem.to_string())
            } else {
                None
            }
        }
    }
}

fn load_templates(dir: &Path) -> Result<Vec<Template>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut templates = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            match load_template_dir(&path) {
                Ok(template) => templates.push(template),
                Err(err) => {
                    warn!(
                        "Failed to parse template directory {}; ({err})",
                        path.display()
                    );
                }
            }
        }
    }
    templates.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(a.id.cmp(&b.id)));
    Ok(templates)
}

fn load_template_dir(dir: &Path) -> Result<Template> {
    let stem = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let meta_path = dir.join("meta.json");
    let mut meta: TemplateMeta = if meta_path.exists() {
        let text = fs::read_to_string(&meta_path)?;
        serde_json::from_str(&text)?
    } else {
        TemplateMeta {
            id: stem.to_string(),
            name: stem.to_string(),
            description: String::new(),
            updated_at: None,
        }
    };
    if meta.id.is_empty() {
        meta.id = stem.to_string();
    }
    if !is_safe_id(&meta.id) {
        return Err(CantoError::Web(format!(
            "invalid template id '{}'",
            meta.id
        )));
    }

    let mut content_map = serde_json::Map::new();
    let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(key) = module_key_from_filename(file_name) else {
            continue;
        };
        let text = fs::read_to_string(&path)?;
        let val: Value = serde_json::from_str(&text)
            .map_err(|err| CantoError::Web(format!("failed to parse {}: {err}", path.display())))?;
        content_map.insert(key, val);
    }

    let content = Value::Object(content_map);
    if let Err(err) = validate_template_content(&content) {
        return Err(CantoError::Web(err));
    }

    Ok(Template {
        id: meta.id,
        name: meta.name,
        description: meta.description,
        updated_at: meta.updated_at,
        content,
    })
}

fn persist_template(dir: &Path, template: &Template) -> Result<()> {
    fs::create_dir_all(dir)?;
    let target_dir = template_dir(dir, &template.id);
    let tmp_dir = dir.join(format!(".{}.tmp", template.id));
    let old_dir = dir.join(format!(".{}.old", template.id));

    if tmp_dir.exists() {
        let _ = fs::remove_dir_all(&tmp_dir);
    }
    if old_dir.exists() {
        let _ = fs::remove_dir_all(&old_dir);
    }

    fs::create_dir_all(&tmp_dir)?;

    let meta = TemplateMeta {
        id: template.id.clone(),
        name: template.name.clone(),
        description: template.description.clone(),
        updated_at: template.updated_at.clone(),
    };
    write_pretty_json(&tmp_dir.join("meta.json"), &meta)?;

    if let Some(obj) = template.content.as_object() {
        for (key, val) in obj {
            if val.is_null() {
                continue;
            }
            if let Some(filename) = module_filename(key) {
                write_pretty_json(&tmp_dir.join(filename), val)?;
            }
        }
    }

    if target_dir.exists() {
        fs::rename(&target_dir, &old_dir)?;
    }
    if let Err(err) = fs::rename(&tmp_dir, &target_dir) {
        if old_dir.exists() {
            let _ = fs::rename(&old_dir, &target_dir);
        }
        return Err(CantoError::Io(err));
    }
    if old_dir.exists() {
        let _ = fs::remove_dir_all(&old_dir);
    }

    let legacy_file = dir.join(format!("{}.json", template.id));
    if legacy_file.exists() {
        let _ = fs::remove_file(&legacy_file);
    }

    info!("Wrote template {} to {}", template.id, target_dir.display());
    Ok(())
}

fn write_pretty_json<T: Serialize>(path: &Path, val: &T) -> Result<()> {
    let mut formatted = serde_json::to_string_pretty(val)?;
    if !formatted.ends_with('\n') {
        formatted.push('\n');
    }
    let mut file = File::create(path)?;
    file.write_all(formatted.as_bytes())?;
    file.sync_all()?;
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

    fn sample_template(id: &str) -> Template {
        Template {
            id: id.to_string(),
            name: "网关模板".to_string(),
            description: "tproxy".to_string(),
            updated_at: Some("2026-01-01T00:00:00Z".to_string()),
            content: json!({
                "node_groups": [{
                    "type": "urltest",
                    "tag": "香港节点",
                    "outbounds": ["{(?i)(港|hk)}"]
                }],
                "policy_groups": [
                    { "type": "direct", "tag": "direct" },
                    {
                        "type": "selector",
                        "tag": "默认策略",
                        "outbounds": ["香港节点", "direct"]
                    }
                ]
            }),
        }
    }

    #[tokio::test]
    async fn test_template_directory_roundtrip() {
        let dir = temp_dir("templates");
        let store = TemplateStore::load(&dir).unwrap();
        assert!(store.list().await.is_empty());

        store.insert(sample_template("tpl_a")).await.unwrap();
        let target_dir = dir.join("studio").join("templates").join("tpl_a");
        assert!(target_dir.is_dir());
        assert!(target_dir.join("meta.json").is_file());
        assert!(target_dir.join("canto.node_groups.json").is_file());
        assert!(target_dir.join("canto.policy_groups.json").is_file());
        assert!(!target_dir.join("outbounds.json").exists());

        let persisted = fs::read_to_string(target_dir.join("canto.node_groups.json")).unwrap();
        assert!(persisted.contains("{(?i)(港|hk)}"));

        let reloaded = TemplateStore::load(&dir).unwrap();
        let listed = reloaded.list().await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "tpl_a");
        assert_eq!(listed[0].name, "网关模板");
        assert_eq!(
            listed[0].content["node_groups"][0]["outbounds"][0],
            "{(?i)(港|hk)}"
        );

        assert!(reloaded.delete("tpl_a").await.unwrap());
        assert!(!target_dir.exists());
        assert!(reloaded.list().await.is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    fn sample_profile(id: &str) -> Profile {
        Profile {
            id: id.to_string(),
            name: "家庭网关".to_string(),
            description: "tproxy".to_string(),
            template_id: "tpl_a".to_string(),
            source_ids: vec!["src_a".to_string()],
            token: "tok_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            updated_at: Some("2026-01-01T00:00:00Z".to_string()),
            public_url: Some("http://example/sub/tok_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
        }
    }

    #[tokio::test]
    async fn test_profile_file_roundtrip_strips_public_url() {
        let dir = temp_dir("profiles");
        let store = ProfileStore::load(&dir).unwrap();
        assert!(store.list().await.is_empty());

        store.insert(sample_profile("prof_a")).await.unwrap();
        let path = dir.join("studio").join("profiles.json");
        let persisted = fs::read_to_string(&path).unwrap();
        assert!(persisted.contains("profiles"));
        assert!(persisted.contains("prof_a"));
        assert!(persisted.contains("tok_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(!persisted.contains("publicUrl"));

        let reloaded = ProfileStore::load(&dir).unwrap();
        let listed = reloaded.list().await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "prof_a");
        assert_eq!(listed[0].template_id, "tpl_a");
        assert_eq!(listed[0].source_ids, vec!["src_a".to_string()]);
        assert_eq!(listed[0].public_url, None);
        fs::remove_dir_all(&dir).ok();
    }
}
