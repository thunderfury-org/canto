use std::path::PathBuf;
use std::sync::Arc;

use crate::config::WebSettings;
use crate::web::studio::SourceStore;

#[derive(Clone, Debug)]
pub struct WebState {
    pub settings: Arc<WebSettings>,
    pub sources: Arc<SourceStore>,
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
        Self {
            settings: Arc::new(settings),
            sources: Arc::new(sources),
            http: reqwest::Client::new(),
        }
    }
}
