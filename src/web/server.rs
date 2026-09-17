use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tracing::info;

use crate::config::WebSettings;
use crate::error::{CantoError, Result};
use crate::web::router::create_app;
use crate::web::state::WebState;

pub struct WebServer {
    settings: WebSettings,
    work_dir: PathBuf,
}

pub struct BoundWebServer {
    settings: WebSettings,
    work_dir: PathBuf,
    listener: TcpListener,
    local_addr: SocketAddr,
}

impl WebServer {
    pub fn new(settings: WebSettings, work_dir: PathBuf) -> Self {
        Self { settings, work_dir }
    }

    pub async fn bind(&self) -> Result<BoundWebServer> {
        let listener = TcpListener::bind(&self.settings.listen)
            .await
            .map_err(|e| {
                CantoError::Web(format!(
                    "Failed to bind web listen address {}: {e}",
                    self.settings.listen
                ))
            })?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| CantoError::Web(format!("Failed to get local address: {e}")))?;

        info!("Web Studio listening on http://{}", local_addr);
        Ok(BoundWebServer {
            settings: self.settings.clone(),
            work_dir: self.work_dir.clone(),
            listener,
            local_addr,
        })
    }

    pub async fn run_with_signal<F>(&self, shutdown_signal: F) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let bound = self.bind().await?;
        bound.run_with_signal(shutdown_signal).await
    }

    pub async fn run(&self) -> Result<()> {
        self.run_with_signal(wait_for_shutdown_signal()).await
    }
}

impl BoundWebServer {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn run_with_signal<F>(self, shutdown_signal: F) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let state = WebState::new(self.settings, self.work_dir);
        let app = create_app(state);

        axum::serve(self.listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .map_err(|e| CantoError::Web(format!("Web Studio server error: {e}")))?;

        info!("Web Studio server stopped gracefully");
        Ok(())
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
