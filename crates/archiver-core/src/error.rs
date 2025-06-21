use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Settings error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Home directory not found")]
    HomeDirNotFound,

    #[error("{0}")]
    Custom(String),
    
    #[error("Failed to walk directory: {0}")]
    WalkDir(#[from] walkdir::Error),
}

pub type Result<T> = std::result::Result<T, Error>;