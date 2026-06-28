use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryPolicy {
    Immediate,
    ExponentialBackoff { base_ms: u64, max_retries: u32 },
    NoRetry,
}

#[derive(Debug, Error)]
pub enum LoopError {
    #[error("Transient LLM error: {0}")]
    LlmTransient(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("Permanent LLM error: {0}")]
    LlmPermanent(String),
    #[error("Transport error: {0}")]
    Transport(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("State machine violation: {0}")]
    StateViolation(String),
    #[error("A2A call failed: {0}")]
    A2AFailed(String),
    #[error("Motivation error: {0}")]
    Motivation(String),
}

impl LoopError {
    pub fn retry_policy(&self) -> RetryPolicy {
        match self {
            Self::LlmTransient(_) => RetryPolicy::ExponentialBackoff { base_ms: 100, max_retries: 3 },
            Self::Transport(_) => RetryPolicy::Immediate,
            _ => RetryPolicy::NoRetry,
        }
    }
    pub fn is_retryable(&self) -> bool { !matches!(self.retry_policy(), RetryPolicy::NoRetry) }
}

pub type Result<T> = std::result::Result<T, LoopError>;
