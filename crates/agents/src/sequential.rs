use async_trait::async_trait;
use std::sync::Arc;
use crate::base::{BaseAgent, AgentContext, AgentResponse};

/// SequentialAgent runs sub-agents in sequence, passing output to next
pub struct SequentialAgent {
    name: String,
    description: String,
    sub_agents: Vec<Arc<dyn BaseAgent>>,
}

impl SequentialAgent {
    pub fn new(name: &str, sub_agents: Vec<Arc<dyn BaseAgent>>) -> Self {
        Self {
            name: name.to_string(),
            description: format!("Sequential agent with {} stages", sub_agents.len()),
            sub_agents,
        }
    }
}

#[async_trait]
impl BaseAgent for SequentialAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn execute(&self, input: &str, context: &mut AgentContext) -> std::result::Result<AgentResponse, crate::base::AgentError> {
        let mut current_input = input.to_string();

        for agent in &self.sub_agents {
            tracing::debug!("SequentialAgent '{}' running stage '{}'", self.name, agent.name());
            let response = agent.execute(&current_input, context).await?;
            current_input = response.content;
        }

        Ok(AgentResponse::new(&current_input))
    }
}
