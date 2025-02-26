use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum FuzzerError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Failed to run target: {0}")]
    TargetExecution(String),
    
    #[error("Invalid coverage data: {0}")]
    InvalidCoverage(String),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

pub type Result<T> = std::result::Result<T, FuzzerError>; 