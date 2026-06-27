use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats, Result, LoopError};
use loop_engineering_cognition::{LlmClient, Session, PromptDispatcher, ModelRouter, Role};

pub struct DeepworkLoop {
    client: LlmClient,
    session: Session,
    dispatcher: PromptDispatcher,
    stats: LoopStats,
}

impl DeepworkLoop {
    pub fn new() -> Self {
        let router = ModelRouter::default();
        let client = LlmClient::mock("deepwork");
        let session = Session::new("deepwork-loop", "unit-000");
        let dispatcher = PromptDispatcher::default();
        Self {
            client,
            session,
            dispatcher,
            stats: LoopStats::default(),
        }
    }
}

impl Default for DeepworkLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Loop for DeepworkLoop {
    fn loop_type(&self) -> &str {
        "deepwork"
    }

    async fn process(&mut self, input: LoopInput) -> Result<LoopOutput> {
        let mut variables = std::collections::HashMap::new();
        variables.insert("task".to_string(), input.task_desc.clone());
        
        let prompt = self.dispatcher
            .dispatch("deepwork", &variables)
            .unwrap_or_else(|| input.task_desc.clone());
        
        self.session.add_message(Role::User, prompt.clone());
        
        let messages = self.session.get_messages().to_vec();
        let response = self.client.chat(messages).await
            .map_err(|e| LoopError::Llm(e.to_string()))?;
        
        self.session.add_message(Role::Assistant, response.content.clone());
        self.stats.slices_processed += 1;
        
        Ok(LoopOutput {
            slice_id: input.slice_id,
            result: response.content,
            tool_calls: vec![],
            stats: self.stats.clone(),
        })
    }

    fn stats(&self) -> LoopStats {
        self.stats.clone()
    }
}
