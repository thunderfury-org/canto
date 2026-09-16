use std::future::Future;
use tokio::net::TcpListener;
use tracing::info;

use crate::config::WebSettings;
use crate::error::{CantoError, Result};
use crate::web::router::create_app;
use crate::web::state::WebState;

pub struct WebServer {
    settings: WebSettings,
}

impl WebServer {
    pub fn new(settings: WebSettings) -> Self {
        Self { settings }
    }

    pub async fn run_with_signal<F>(&self, shutdown_signal: F) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let state = WebState::new(self.settings.clone());
        let app = create_app(state);

        let listener = TcpListener::bind(&self.settings.listen)
            .await
            .map_err(|e| {
                CantoError::Network(format!(
                    "Failed to bind web listen address {}: {e}",
                    self.settings.listen
                ))
            })?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| CantoError::Network(format!("Failed to get local address: {e}")))?;

        info!("Web Studio listening on http://{}", local_addr);

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .map_err(|e| CantoError::Network(format!("Web Studio server error: {e}")))?;

        info!("Web Studio server stopped gracefully");
        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        self.run_with_signal(wait_for_shutdown_signal()).await
    }
}

pub async fn wait_for_shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
