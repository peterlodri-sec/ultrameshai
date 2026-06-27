//! Error types for agent-core.

#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Missing environment variable: {0}")]
    MissingEnv(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AgentError>;
