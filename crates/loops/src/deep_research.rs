use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats, Result, LoopError};
use loop_engineering_cognition::{LlmClient, ResearchSession, PromptDispatcher, ModelRouter};
use std::collections::HashMap;

pub struct DeepResearchLoop {
    client: LlmClient,
    session: Option<ResearchSession>,
    dispatcher: PromptDispatcher,
    stats: LoopStats,
}

impl DeepResearchLoop {
    pub fn new() -> Self {
        let router = ModelRouter::default();
        let client = router.create_client("research", "mock-key", "http://localhost")
            .unwrap_or_else(|| LlmClient::mock("openai/gpt-4-turbo"));
        let dispatcher = PromptDispatcher::default();
        Self {
            client,
            session: None,
            dispatcher,
            stats: LoopStats::default(),
        }
    }

    pub async fn with_session(milvus_uri: &str, source_agent: &str) -> Self {
        let router = ModelRouter::default();
        let client = router.create_client("research", "mock-key", "http://localhost")
            .unwrap_or_else(|| LlmClient::mock("openai/gpt-4-turbo"));
        let session = ResearchSession::new("deep-research-loop", "unit-000", milvus_uri, source_agent)
            .await
            .unwrap();
        let dispatcher = PromptDispatcher::default();
        Self {
            client,
            session: Some(session),
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
