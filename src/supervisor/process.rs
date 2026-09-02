use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::select;
use tokio::signal;
use tracing::{error, info, warn};

use crate::error::{CantoError, Result};

pub struct ProcessSupervisor {
    binary: PathBuf,
    config_path: PathBuf,
    work_dir: PathBuf,
}

impl ProcessSupervisor {
    pub fn new(binary: PathBuf, config_path: PathBuf, work_dir: PathBuf) -> Self {
        Self {
            binary,
            config_path,
            work_dir,
        }
    }

    /// Checks if the sing-box binary is available and executable
    pub fn verify_binary(&self) -> Result<()> {
        let which_check = std::process::Command::new(&self.binary)
            .arg("version")
            .output();

        match which_check {
            Ok(output) if output.status.success() => {
                let version_str = String::from_utf8_lossy(&output.stdout);
                let first_line = version_str.lines().next().unwrap_or("sing-box");
                info!("Found sing-box: {first_line}");
                Ok(())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(CantoError::Process(format!(
                    "sing-box binary returned error status: {stderr}"
                )))
            }
            Err(_) => Err(CantoError::SingBoxNotFound(self.binary.clone())),
        }
    }

    /// Runs `sing-box check -c <config_path>` to validate configuration syntax
    pub fn check_config(&self, config: Option<&Path>) -> Result<()> {
        let target_config = config.unwrap_or(&self.config_path);
        info!(
            "Validating sing-box configuration syntax: {}",
            target_config.display()
        );

        let output = std::process::Command::new(&self.binary)
            .args(["check", "-c", target_config.to_str().unwrap_or("")])
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

    /// Spawns sing-box and supervises its lifecycle until interrupt or exit
    pub async fn run_supervised(&self) -> Result<()> {
        self.verify_binary()?;

        if !self.config_path.exists() {
            return Err(CantoError::Config(format!(
                "Config file '{}' does not exist. Did you generate it first?",
                self.config_path.display()
            )));
        }

        self.check_config(None)?;

        tokio::fs::create_dir_all(&self.work_dir).await?;

        info!(
            "Starting sing-box child process: {} run -c {}",
            self.binary.display(),
            self.config_path.display()
        );

        let mut child = Command::new(&self.binary)
            .args(["run", "-c", self.config_path.to_str().unwrap_or("")])
            .current_dir(&self.work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| CantoError::Process(format!("Failed to spawn sing-box: {e}")))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Stream stdout asynchronously
        if let Some(stdout) = stdout {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    info!(target: "sing_box", "{line}");
                }
            });
        }

        // Stream stderr asynchronously
        if let Some(stderr) = stderr {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    warn!(target: "sing_box", "{line}");
                }
            });
        }

        // Wait for process exit or shutdown signal
        select! {
            status = child.wait() => {
                match status {
                    Ok(exit_status) => {
                        if exit_status.success() {
                            info!("sing-box process exited normally");
                            Ok(())
                        } else {
                            let code = exit_status.code().unwrap_or(-1);
                            error!("sing-box process crashed or exited with code {code}");
                            Err(CantoError::Process(format!("sing-box exited with code {code}")))
                        }
                    }
                    Err(e) => {
                        error!("Error waiting for sing-box: {e}");
                        Err(CantoError::Process(e.to_string()))
                    }
                }
            }
            _ = signal::ctrl_c() => {
                info!("Received SIGINT (Ctrl+C). Terminating sing-box gracefully...");
                Self::terminate_child(&mut child).await;
                Ok(())
            }
        }
    }

    async fn terminate_child(child: &mut Child) {
        // Attempt SIGTERM or kill
        if let Err(e) = child.kill().await {
            warn!("Failed to kill sing-box child process: {e}");
        }
        let _ = child.wait().await;
        info!("sing-box process terminated");
    }
}
