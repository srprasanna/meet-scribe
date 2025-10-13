/// Error types for Meet Scribe
///
/// Uses thiserror for ergonomic error handling with proper Display implementations.
use thiserror::Error;

/// Main error type for the application
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Keychain error: {0}")]
    Keychain(#[from] keyring::Error),

    #[error("Keychain error: {0}")]
    KeychainError(String),

    #[error("Audio capture error: {0}")]
    AudioCapture(String),

    #[error("Transcription service error: {0}")]
    Transcription(String),

    #[error("LLM service error: {0}")]
    Llm(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("{0}")]
    Other(String),
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, AppError>;

/// Convert AppError to a string for Tauri IPC
impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}
