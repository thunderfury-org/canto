use std::path::PathBuf;
use std::sync::Arc;

use crate::config::WebSettings;
use crate::web::studio::{ProfileStore, SourceStore, TemplateStore};

#[derive(Clone, Debug)]
pub struct WebState {
    pub settings: Arc<WebSettings>,
    pub sources: Arc<SourceStore>,
    pub templates: Arc<TemplateStore>,
    pub profiles: Arc<ProfileStore>,
    pub http: reqwest::Client,
}

impl WebState {
    pub fn new(settings: WebSettings, work_dir: PathBuf) -> Self {
        let sources = match SourceStore::load(&work_dir) {
            Ok(store) => store,
            Err(err) => {
                tracing::warn!(
                    "Failed to load node sources from {}: {err}; starting empty",
                    work_dir.display()
                );
                SourceStore::empty(&work_dir)
            }
        };
        let templates = match TemplateStore::load(&work_dir) {
            Ok(store) => store,
            Err(err) => {
                tracing::warn!(
                    "Failed to load templates from {}: {err}; starting empty",
                    work_dir.display()
                );
                TemplateStore::empty(&work_dir)
            }
        };
        let profiles = match ProfileStore::load(&work_dir) {
            Ok(store) => store,
            Err(err) => {
                tracing::warn!(
                    "Failed to load profiles from {}: {err}; starting empty",
                    work_dir.display()
                );
                ProfileStore::empty(&work_dir)
            }
        };
        Self {
            settings: Arc::new(settings),
            sources: Arc::new(sources),
            templates: Arc::new(templates),
            profiles: Arc::new(profiles),
            http: reqwest::Client::new(),
        }
    }
}
