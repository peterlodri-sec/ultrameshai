use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Input to a loop processing cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopInput {
    pub slice_id: String,
    pub task_desc: String,
    pub context: Vec<String>,
}

/// Output from a loop processing cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopOutput {
    pub slice_id: String,
    pub result: String,
    pub tool_calls: Vec<String>,
    pub stats: LoopStats,
}

/// Statistics for a loop instance
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoopStats {
    pub slices_processed: u32,
    pub total_tokens: u32,
    pub peak_memory_mb: u32,
}

/// Error types for loop processing
#[derive(Debug, thiserror::Error)]
pub enum LoopError {
    #[error("Processing error: {0}")]
    Processing(String),
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Transport error: {0}")]
    Transport(String),
}

pub type Result<T> = std::result::Result<T, LoopError>;

/// Main Loop trait - all loop types implement this
#[async_trait::async_trait]
pub trait Loop {
    /// Returns the type identifier for this loop
    fn loop_type(&self) -> &str;
    
    /// Process a single input slice
    async fn process(&mut self, input: LoopInput) -> Result<LoopOutput>;
    
    /// Get current statistics
    fn stats(&self) -> LoopStats;
}
