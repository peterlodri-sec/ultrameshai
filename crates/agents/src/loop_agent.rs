use async_trait::async_trait;
use std::sync::Arc;
use crate::base::{BaseAgent, AgentContext, AgentResponse};

/// LoopAgent runs sub-agents repeatedly until exit condition or max iterations
pub struct LoopAgent {
    name: String,
    description: String,
    sub_agents: Vec<Arc<dyn BaseAgent>>,
    max_iterations: usize,
}

impl LoopAgent {
    pub fn new(name: &str, sub_agents: Vec<Arc<dyn BaseAgent>>) -> Self {
        Self {
            name: name.to_string(),
            description: format!("Loop agent with {} sub-agents", sub_agents.len()),
            sub_agents,
            max_iterations: 10, // default
        }
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }
}

#[async_trait]
impl BaseAgent for LoopAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn execute(&self, input: &str, context: &mut AgentContext) -> std::result::Result<AgentResponse, crate::base::AgentError> {
        let mut current_input = input.to_string();
        let mut iteration = 0;

        while iteration < self.max_iterations {
            iteration += 1;
            tracing::info!("LoopAgent '{}' iteration {}/{}", self.name, iteration, self.max_iterations);

            // Run all sub-agents sequentially
            for agent in &self.sub_agents {
                let response = agent.execute(&current_input, context).await?;
                current_input = response.content;

                // Check for exit signal (agent marks done=true)
                if response.done && iteration > 1 {
                    tracing::info!("LoopAgent '{}' exiting early at iteration {}", self.name, iteration);
                    return Ok(AgentResponse::new(&current_input));
                }
            }
        }

        Ok(AgentResponse::new(&current_input))
    }
}
