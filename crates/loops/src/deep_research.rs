use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats, Result, LoopError};
use loop_engineering_cognition::{LlmClient, Session, PromptDispatcher, ModelRouter};
use std::collections::HashMap;

pub struct DeepResearchLoop {
    client: LlmClient,
    session: Session,
    dispatcher: PromptDispatcher,
    stats: LoopStats,
}

impl DeepResearchLoop {
    pub fn new() -> Self {
        let router = ModelRouter::default();
        let client = router.create_client("research", "mock-key", "http://localhost")
            .unwrap_or_else(|| LlmClient::mock("openai/gpt-4-turbo"));
        let session = Session::new("deep-research-loop", "unit-000");
        let dispatcher = PromptDispatcher::default();
        Self {
            client,
            session,
            dispatcher,
            stats: LoopStats::default(),
        }
    }
}

impl Default for DeepResearchLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Loop for DeepResearchLoop {
    fn loop_type(&self) -> &str {
        "deep-research-loop"
    }

    async fn process(&mut self, input: LoopInput) -> Result<LoopOutput> {
        let mut variables = HashMap::new();
        variables.insert("task".to_string(), input.task_desc.clone());
        
        let prompt = self.dispatcher
            .dispatch("research", &variables)
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
