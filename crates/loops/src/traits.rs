use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::domain::{MotivationSummary, RewardSummary};
use crate::error::{LoopError, Result};

/// Input to a loop processing cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopInput {
    pub slice_id: String,
    pub task_desc: String,
    pub context: Vec<String>,
    pub motivation: Option<MotivationSummary>,
}

/// Output from a loop processing cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopOutput {
    pub slice_id: String,
    pub result: String,
    pub tool_calls: Vec<String>,
    pub stats: LoopStats,
    pub reward_earned: Option<RewardSummary>,
    pub a2a_completed: bool,
}

/// Statistics for a loop instance
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoopStats {
    pub slices_processed: u32,
    pub total_tokens: u32,
    pub peak_memory_mb: u32,
}



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
