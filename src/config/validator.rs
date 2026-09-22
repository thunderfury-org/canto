use std::path::{Path, PathBuf};
use tracing::info;

use crate::error::{CantoError, Result};

/// Seam for validating sing-box configuration syntax before applying it to the gateway.
pub trait ConfigValidator: Send + Sync {
    fn validate_config(&self, path: &Path) -> Result<()>;
}

impl<F> ConfigValidator for F
where
    F: Fn(&Path) -> Result<()> + Send + Sync,
{
    fn validate_config(&self, path: &Path) -> Result<()> {
        self(path)
    }
}

/// A no-op validator that always passes (useful for tests or mock environments).
#[derive(Debug, Clone, Default)]
pub struct NoopValidator;

impl ConfigValidator for NoopValidator {
    fn validate_config(&self, _path: &Path) -> Result<()> {
        Ok(())
    }
}

/// Production validator adapter that invokes `sing-box check -c <path>`.
#[derive(Debug, Clone)]
pub struct SingBoxValidator {
    binary: PathBuf,
}

impl SingBoxValidator {
    pub fn new(binary: PathBuf) -> Self {
        Self { binary }
    }
}

impl ConfigValidator for SingBoxValidator {
    fn validate_config(&self, path: &Path) -> Result<()> {
        let config_arg = path.to_str().ok_or_else(|| {
            CantoError::Config(format!(
                "Config path is not valid UTF-8: {}",
                path.display()
            ))
        })?;

        info!("Validating sing-box configuration syntax: {config_arg}");

        let output = std::process::Command::new(&self.binary)
            .args(["check", "-c", config_arg])
            .output()
            .map_err(|e| CantoError::Process(format!("Failed to invoke sing-box check: {e}")))?;

        if output.status.success() {
            info!("Configuration check passed successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let msg = if !stderr.is_empty() { stderr } else { stdout };
            Err(CantoError::Config(format!(
                "sing-box config validation failed:\n{msg}"
            )))
        }
    }
}
