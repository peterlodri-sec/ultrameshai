use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats};
use crate::error::{Result};

pub struct RedteamFuzzerLoop {
    stats: LoopStats,
}

impl RedteamFuzzerLoop {
    pub fn new() -> Self {
        Self {
            stats: LoopStats::default(),
        }
    }
}

impl Default for RedteamFuzzerLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Loop for RedteamFuzzerLoop {
    fn loop_type(&self) -> &str {
        "redteam-fuzzer"
    }

    async fn process(&mut self, input: LoopInput) -> Result<LoopOutput> {
        self.stats.slices_processed += 1;

        Ok(LoopOutput {
            slice_id: input.slice_id,
            result: input.task_desc,
            tool_calls: vec![],
            stats: self.stats.clone(),
            reward_earned: None,
            a2a_completed: false,
            reward_earned: None,
            a2a_completed: false,
        })
    }

    fn stats(&self) -> LoopStats {
        self.stats.clone()
    }
}
