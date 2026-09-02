use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CantoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML decoding error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network orchestration error: {0}")]
    Network(String),

    #[error("Process supervisor error: {0}")]
    Process(String),

    #[error("sing-box binary not found at: {0}")]
    SingBoxNotFound(PathBuf),

    #[error("Command '{command}' failed with exit code {code}: {stderr}")]
    CommandFailed {
        command: String,
        code: i32,
        stderr: String,
    },
}

pub type Result<T> = std::result::Result<T, CantoError>;
