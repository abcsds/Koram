use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Immich API error: {0}")]
    ImmichApi(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Operation cancelled")]
    Cancelled,
}

pub const PERMISSION_HINT: &str =
    "Ensure the volume mounts (/app/config, /app/cache) are writable by the container user (uid 1000 by default).";
