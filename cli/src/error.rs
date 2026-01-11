use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Home directory not found")]
    HomeDirNotFound,

    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("Command failed: {command} - {reason}")]
    CommandFailed { command: String, reason: String },

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Process spawn error: {0}")]
    ProcessError(#[from] std::io::Error),

    #[error("Seed must be at least 32 bytes")]
    InvalidSeed,
}
