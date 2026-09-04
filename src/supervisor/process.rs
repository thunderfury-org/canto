use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::select;
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
        let config_arg = target_config.to_str().ok_or_else(|| {
            CantoError::Config(format!(
                "Config path is not valid UTF-8: {}",
                target_config.display()
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

    /// Spawns sing-box and supervises its lifecycle until interrupt or exit.
    ///
    /// Callers must verify the binary and check the config before applying
    /// network rules and invoking this method.
    pub async fn run_supervised(&self) -> Result<()> {
        if !self.config_path.exists() {
            return Err(CantoError::Config(format!(
                "Config file '{}' does not exist. Did you generate it first?",
                self.config_path.display()
            )));
        }

        tokio::fs::create_dir_all(&self.work_dir).await?;

        let config_arg = self.config_path.to_str().ok_or_else(|| {
            CantoError::Config(format!(
                "Config path is not valid UTF-8: {}",
                self.config_path.display()
            ))
        })?;

        info!(
            "Starting sing-box child process: {} run -c {}",
            self.binary.display(),
            config_arg
        );

        let mut child = Command::new(&self.binary)
            .args(["run", "-c", config_arg])
            .current_dir(&self.work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| CantoError::Process(format!("Failed to spawn sing-box: {e}")))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        if let Some(stdout) = stdout {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    info!(target: "sing_box", "{line}");
                }
            });
        }

        if let Some(stderr) = stderr {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    warn!(target: "sing_box", "{line}");
                }
            });
        }

        Self::wait_for_exit_or_signal(&mut child).await
    }

    async fn wait_for_exit_or_signal(child: &mut Child) -> Result<()> {
        #[cfg(unix)]
        {
            let mut sigterm = tokio::signal::unix::signal(
                tokio::signal::unix::SignalKind::terminate(),
            )
            .map_err(|e| CantoError::Process(format!("Failed to listen for SIGTERM: {e}")))?;

            select! {
                status = child.wait() => Self::map_exit_status(status),
                _ = tokio::signal::ctrl_c() => {
                    info!("Received SIGINT (Ctrl+C). Terminating sing-box gracefully...");
                    Self::terminate_child(child).await;
                    Ok(())
                }
                _ = sigterm.recv() => {
                    info!("Received SIGTERM. Terminating sing-box gracefully...");
                    Self::terminate_child(child).await;
                    Ok(())
                }
            }
        }

        #[cfg(not(unix))]
        {
            select! {
                status = child.wait() => Self::map_exit_status(status),
                _ = tokio::signal::ctrl_c() => {
                    info!("Received SIGINT (Ctrl+C). Terminating sing-box gracefully...");
                    Self::terminate_child(child).await;
                    Ok(())
                }
            }
        }
    }

    fn map_exit_status(status: std::io::Result<std::process::ExitStatus>) -> Result<()> {
        match status {
            Ok(exit_status) => {
                if exit_status.success() {
                    info!("sing-box process exited normally");
                    Ok(())
                } else {
                    let code = exit_status.code().unwrap_or(-1);
                    error!("sing-box process crashed or exited with code {code}");
                    Err(CantoError::Process(format!(
                        "sing-box exited with code {code}"
                    )))
                }
            }
            Err(e) => {
                error!("Error waiting for sing-box: {e}");
                Err(CantoError::Process(e.to_string()))
            }
        }
    }

    async fn terminate_child(child: &mut Child) {
        #[cfg(unix)]
        {
            if let Some(pid) = child.id() {
                let _ = Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .status()
                    .await;
                if tokio::time::timeout(std::time::Duration::from_secs(5), child.wait())
                    .await
                    .is_ok()
                {
                    info!("sing-box process terminated");
                    return;
                }
            }
        }

        if let Err(e) = child.kill().await {
            warn!("Failed to kill sing-box child process: {e}");
        }
        let _ = child.wait().await;
        info!("sing-box process terminated");
    }
}
