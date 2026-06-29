use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats};
use crate::error::{Result};

pub struct DevopsLoop {
    stats: LoopStats,
}

impl DevopsLoop {
    pub fn new() -> Self {
        Self {
            stats: LoopStats::default(),
        }
    }
}

impl Default for DevopsLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Loop for DevopsLoop {
    fn loop_type(&self) -> &str {
        "devops"
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
        })
    }

    fn stats(&self) -> LoopStats {
        self.stats.clone()
    }
}
