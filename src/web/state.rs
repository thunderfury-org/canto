use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::config::WebSettings;
use crate::web::studio::model::now_rfc3339;
use crate::web::studio::{ProfileCompiler, ProfileStore, SourceStore, TemplateStore};

#[derive(Clone, Debug)]
pub struct WebState {
    pub settings: Arc<WebSettings>,
    pub sources: Arc<SourceStore>,
    pub templates: Arc<TemplateStore>,
    pub profiles: Arc<ProfileStore>,
    pub compiler: Arc<ProfileCompiler>,
    pub http: reqwest::Client,
    pub started_at: Instant,
    pub started_at_rfc3339: String,
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
        let sources = Arc::new(sources);
        let templates = Arc::new(templates);
        let profiles = Arc::new(profiles);
        let compiler = Arc::new(ProfileCompiler::new(
            Arc::clone(&templates),
            Arc::clone(&sources),
            Arc::clone(&profiles),
        ));
        Self {
            settings: Arc::new(settings),
            sources,
            templates,
            profiles,
            compiler,
            http: reqwest::Client::new(),
            started_at: Instant::now(),
            started_at_rfc3339: now_rfc3339(),
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
}
