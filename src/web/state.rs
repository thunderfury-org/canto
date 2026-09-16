use std::sync::Arc;

use crate::config::WebSettings;

#[derive(Clone, Debug)]
pub struct WebState {
    pub settings: Arc<WebSettings>,
}

impl WebState {
    pub fn new(settings: WebSettings) -> Self {
        Self {
            settings: Arc::new(settings),
        }
    }
}
