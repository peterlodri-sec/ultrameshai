use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats};
use crate::error::{Result};

pub struct CoderEditorLoop {
    stats: LoopStats,
}

impl CoderEditorLoop {
    pub fn new() -> Self {
        Self {
            stats: LoopStats::default(),
        }
    }
}

impl Default for CoderEditorLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Loop for CoderEditorLoop {
    fn loop_type(&self) -> &str {
        "coder-editor"
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
