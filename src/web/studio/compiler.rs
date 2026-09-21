use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::web::studio::expand::expand_profile;
use crate::web::studio::model::{Profile, new_profile_token};
use crate::web::studio::store::{ProfileStore, SourceStore, TemplateStore};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledProfile {
    pub config: Value,
    pub matched_map: BTreeMap<String, Vec<String>>,
    pub total_nodes: usize,
    pub used_count: usize,
    pub etag: String,
    #[serde(skip)]
    pub raw_bytes: Vec<u8>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CompileError {
    #[error("profile '{0}' not found")]
    ProfileNotFound(String),

    #[error("subscription not found")]
    TokenNotFound(String),

    #[error("template not found")]
    TemplateNotFound(String),

    #[error("source '{0}' not found")]
    SourceNotFound(String),

    #[error("at least one source is required")]
    EmptySources,

    #[error("failed to allocate a unique profile token")]
    TokenAllocationFailed,

    #[error("failed to serialize compiled config: {0}")]
    SerializationError(String),
}

#[derive(Debug, Clone)]
pub struct ProfileCompiler {
    templates: Arc<TemplateStore>,
    sources: Arc<SourceStore>,
    profiles: Arc<ProfileStore>,
}

impl ProfileCompiler {
    pub fn new(
        templates: Arc<TemplateStore>,
        sources: Arc<SourceStore>,
        profiles: Arc<ProfileStore>,
    ) -> Self {
        Self {
            templates,
            sources,
            profiles,
        }
    }

    pub async fn compile_by_id(&self, id: &str) -> Result<CompiledProfile, CompileError> {
        let Some(profile) = self.profiles.get(id).await else {
            return Err(CompileError::ProfileNotFound(id.to_string()));
        };
        self.compile(&profile).await
    }

    pub async fn compile_by_token(&self, token: &str) -> Result<CompiledProfile, CompileError> {
        let Some(profile) = self.profiles.get_by_token(token).await else {
            return Err(CompileError::TokenNotFound(token.to_string()));
        };
        self.compile(&profile).await
    }

    pub async fn compile(&self, profile: &Profile) -> Result<CompiledProfile, CompileError> {
        let Some(template) = self.templates.get(&profile.template_id).await else {
            return Err(CompileError::TemplateNotFound(profile.template_id.clone()));
        };

        let mut nodes = Vec::new();
        for source_id in &profile.source_ids {
            let Some(source) = self.sources.get(source_id).await else {
                continue;
            };
            nodes.extend(source.nodes);
        }

        let expansion = expand_profile(&template.content, &nodes);
        let raw_bytes = serde_json::to_vec(&expansion.config)
            .map_err(|err| CompileError::SerializationError(err.to_string()))?;
        let etag = etag_for(&raw_bytes);

        Ok(CompiledProfile {
            config: expansion.config,
            matched_map: expansion.matched_map,
            total_nodes: expansion.total_nodes,
            used_count: expansion.used_count,
            etag,
            raw_bytes,
        })
    }

    pub async fn validate_refs(
        &self,
        template_id: &str,
        source_ids: &[String],
    ) -> Result<(), CompileError> {
        if source_ids.is_empty() {
            return Err(CompileError::EmptySources);
        }
        if self.templates.get(template_id).await.is_none() {
            return Err(CompileError::TemplateNotFound(template_id.to_string()));
        }
        for source_id in source_ids {
            if self.sources.get(source_id).await.is_none() {
                return Err(CompileError::SourceNotFound(source_id.clone()));
            }
        }
        Ok(())
    }

    pub async fn allocate_unique_token(&self) -> Result<String, CompileError> {
        for _ in 0..8 {
            let token = new_profile_token().map_err(|_| CompileError::TokenAllocationFailed)?;
            if self.profiles.get_by_token(&token).await.is_none() {
                return Ok(token);
            }
        }
        Err(CompileError::TokenAllocationFailed)
    }
}

pub fn etag_for(body: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    body.hash(&mut hasher);
    format!("\"{:016x}\"", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::studio::model::{NodeSource, SourceKind, Template};
    use serde_json::json;
    use std::path::PathBuf;

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "canto_compiler_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&path);
        path
    }

    #[tokio::test]
    async fn test_compile_profile_and_by_id_and_token() {
        let dir = test_dir("compile");
        let templates = Arc::new(TemplateStore::empty(&dir));
        let sources = Arc::new(SourceStore::empty(&dir));
        let profiles = Arc::new(ProfileStore::empty(&dir));

        let template = Template {
            id: "tpl_main".to_string(),
            name: "Main Template".to_string(),
            description: "Test".to_string(),
            updated_at: None,
            content: json!({
                "outbounds": [
                    {
                        "type": "selector",
                        "tag": "select",
                        "outbounds": ["{.*}"]
                    }
                ]
            }),
        };
        templates.insert(template).await.unwrap();

        let mut source = NodeSource {
            id: "src_1".to_string(),
            name: "Primary".to_string(),
            kind: SourceKind::Manual,
            url: None,
            content: None,
            status: Default::default(),
            last_updated: None,
            last_error: None,
            node_count: 0,
            nodes: Vec::new(),
        };
        source.mark_active(vec![
            json!({ "type": "vless", "tag": "HK-Node", "server": "1.1.1.1" }),
            json!({ "type": "vless", "tag": "US-Node", "server": "2.2.2.2" }),
        ]);
        sources.insert(source).await.unwrap();

        let profile = Profile {
            id: "prof_1".to_string(),
            name: "Gateway Profile".to_string(),
            description: "".to_string(),
            template_id: "tpl_main".to_string(),
            source_ids: vec!["src_1".to_string()],
            token: "tok_test123".to_string(),
            updated_at: None,
            public_url: None,
        };
        profiles.insert(profile.clone()).await.unwrap();

        let compiler = ProfileCompiler::new(templates, sources, profiles);

        // Compile by profile
        let compiled = compiler.compile(&profile).await.unwrap();
        assert_eq!(compiled.total_nodes, 2);
        assert_eq!(compiled.used_count, 2);
        assert!(!compiled.etag.is_empty());
        assert!(!compiled.raw_bytes.is_empty());
        assert_eq!(
            compiled.config["outbounds"][0]["outbounds"],
            json!(["HK-Node", "US-Node"])
        );

        // Compile by id
        let by_id = compiler.compile_by_id("prof_1").await.unwrap();
        assert_eq!(by_id.etag, compiled.etag);

        // Compile by token
        let by_token = compiler.compile_by_token("tok_test123").await.unwrap();
        assert_eq!(by_token.etag, compiled.etag);
    }

    #[tokio::test]
    async fn test_compile_errors() {
        let dir = test_dir("compile");
        let templates = Arc::new(TemplateStore::empty(&dir));
        let sources = Arc::new(SourceStore::empty(&dir));
        let profiles = Arc::new(ProfileStore::empty(&dir));

        let compiler = ProfileCompiler::new(templates, sources, profiles);

        assert_eq!(
            compiler.compile_by_id("missing").await.unwrap_err(),
            CompileError::ProfileNotFound("missing".to_string())
        );

        assert_eq!(
            compiler.compile_by_token("missing").await.unwrap_err(),
            CompileError::TokenNotFound("missing".to_string())
        );
    }

    #[tokio::test]
    async fn test_validate_refs() {
        let dir = test_dir("compile");
        let templates = Arc::new(TemplateStore::empty(&dir));
        let sources = Arc::new(SourceStore::empty(&dir));
        let profiles = Arc::new(ProfileStore::empty(&dir));

        let template = Template {
            id: "tpl_valid".to_string(),
            name: "Tpl".to_string(),
            description: "".to_string(),
            updated_at: None,
            content: json!({}),
        };
        templates.insert(template).await.unwrap();

        let source = NodeSource {
            id: "src_valid".to_string(),
            name: "Src".to_string(),
            kind: SourceKind::Manual,
            url: None,
            content: None,
            status: Default::default(),
            last_updated: None,
            last_error: None,
            node_count: 0,
            nodes: Vec::new(),
        };
        sources.insert(source).await.unwrap();

        let compiler = ProfileCompiler::new(templates, sources, profiles);

        assert_eq!(
            compiler.validate_refs("tpl_valid", &[]).await.unwrap_err(),
            CompileError::EmptySources
        );

        assert_eq!(
            compiler
                .validate_refs("tpl_missing", &["src_valid".to_string()])
                .await
                .unwrap_err(),
            CompileError::TemplateNotFound("tpl_missing".to_string())
        );

        assert_eq!(
            compiler
                .validate_refs("tpl_valid", &["src_missing".to_string()])
                .await
                .unwrap_err(),
            CompileError::SourceNotFound("src_missing".to_string())
        );

        assert!(
            compiler
                .validate_refs("tpl_valid", &["src_valid".to_string()])
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_allocate_unique_token() {
        let dir = test_dir("compile");
        let templates = Arc::new(TemplateStore::empty(&dir));
        let sources = Arc::new(SourceStore::empty(&dir));
        let profiles = Arc::new(ProfileStore::empty(&dir));

        let compiler = ProfileCompiler::new(templates, sources, profiles);
        let token = compiler.allocate_unique_token().await.unwrap();
        assert!(token.starts_with("tok_"));
    }
}
