use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats, Result, LoopError};
use loop_engineering_cognition::{LlmClient, Session, PromptDispatcher, ModelRouter};
use std::collections::HashMap;

pub struct JuniorsLoop {
    client: LlmClient,
    session: Session,
    dispatcher: PromptDispatcher,
    stats: LoopStats,
}

impl JuniorsLoop {
    pub fn new() -> Self {
        let router = ModelRouter::default();
        let client = router.create_client_for_juniors("mock-key", "http://localhost");
        let session = Session::new("juniors-loop", "unit-000");
        let dispatcher = PromptDispatcher::default();
        Self {
            client,
            session,
            dispatcher,
            stats: LoopStats::default(),
        }
    }
}

impl Default for JuniorsLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Loop for JuniorsLoop {
    fn loop_type(&self) -> &str {
        "juniors-loop"
    }

    async fn process(&mut self, input: LoopInput) -> Result<LoopOutput> {
        let mut variables = HashMap::new();
        variables.insert("task".to_string(), input.task_desc.clone());
        
        let prompt = self.dispatcher
            .dispatch("coder", &variables)
            .unwrap_or_else(|| input.task_desc.clone());
        
        self.stats.slices_processed += 1;
        
        Ok(LoopOutput {
            slice_id: input.slice_id,
            result: prompt,
            tool_calls: vec![],
            stats: self.stats.clone(),
        })
    }

    fn stats(&self) -> LoopStats {
        self.stats.clone()
    }
}
