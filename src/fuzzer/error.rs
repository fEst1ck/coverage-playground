use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FuzzerError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to run target: {0}")]
    TargetExecution(String),

    #[error("Invalid coverage data: {0}")]
    InvalidCoverage(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, FuzzerError>;
