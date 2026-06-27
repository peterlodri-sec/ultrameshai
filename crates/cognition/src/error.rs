use thiserror::Error;

#[derive(Error, Debug)]
pub enum CognitionError {
    #[error("LLM API error: {0}")]
    LlmApi(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    #[error("Rate limited: {0}")]
    RateLimited(String),
    
    #[error("Tool call error: {0}")]
    ToolCall(String),
    
    #[error("Session error: {0}")]
    Session(String),
    
    #[error("Provider error: {0}")]
    Provider(String),
}

pub type Result<T> = std::result::Result<T, CognitionError>;
