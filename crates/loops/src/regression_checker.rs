use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats};
use crate::error::{LoopError};
use crate::error::{Result};
use loop_engineering_cognition::{LlmClient, Session, PromptDispatcher, Role};

pub struct RegressionCheckerLoop {
    client: LlmClient,
    session: Session,
    dispatcher: PromptDispatcher,
    stats: LoopStats,
}

impl RegressionCheckerLoop {
    pub fn new() -> Self {
        let client = LlmClient::mock("regression-checker");
        let session = Session::new("regression-checker-loop", "unit-000");
        let dispatcher = PromptDispatcher::default();
        Self { client, session, dispatcher, stats: LoopStats::default() }
    }
}

impl Default for RegressionCheckerLoop {
    fn default() -> Self { Self::new() }
}

#[async_trait::async_trait]
impl Loop for RegressionCheckerLoop {
    fn loop_type(&self) -> &str { "regression-checker" }

    async fn process(&mut self, input: LoopInput) -> Result<LoopOutput> {
        let mut variables = std::collections::HashMap::new();
        variables.insert("task".to_string(), input.task_desc.clone());
        let prompt = self.dispatcher
            .dispatch("regression-checker", &variables)
            .unwrap_or_else(|| input.task_desc.clone());

        self.session.add_message(Role::User, prompt.clone());
        let messages = self.session.get_messages().to_vec();
        let response = self.client.chat(messages)
            .await
            .map_err(|e| LoopError::LlmPermanent(e.to_string()))?;

        self.session.add_message(Role::Assistant, response.content.clone());
        self.stats.slices_processed += 1;

        Ok(LoopOutput {
            slice_id: input.slice_id,
            result: response.content,
            tool_calls: vec![],
            stats: self.stats.clone(),
            reward_earned: None,
            a2a_completed: false,
            reward_earned: None,
            a2a_completed: false,
        })
    }

    fn stats(&self) -> LoopStats { self.stats.clone() }
}

