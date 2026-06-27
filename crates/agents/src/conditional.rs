use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use crate::base::{BaseAgent, AgentContext, AgentResponse};
use loop_engineering_cognition::{LlmClient, ChatMessage, Role};

/// ConditionalAgent routes to sub-agent based on rule-based condition
pub struct ConditionalAgent {
    name: String,
    description: String,
    condition: Arc<dyn Fn(&AgentContext) -> bool + Send + Sync>,
    true_agent: Arc<dyn BaseAgent>,
    false_agent: Arc<dyn BaseAgent>,
}

impl ConditionalAgent {
    pub fn new<F>(
        name: &str,
        condition: F,
        true_agent: Arc<dyn BaseAgent>,
        false_agent: Arc<dyn BaseAgent>,
    ) -> Self
    where
        F: Fn(&AgentContext) -> bool + Send + Sync + 'static,
    {
        Self {
            name: name.to_string(),
            description: format!("Conditional agent '{}'", name),
            condition: Arc::new(condition),
            true_agent,
            false_agent,
        }
    }
}

#[async_trait]
impl BaseAgent for ConditionalAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn execute(&self, input: &str, context: &mut AgentContext) -> std::result::Result<AgentResponse, crate::base::AgentError> {
        let agent = if (self.condition)(context) {
            &self.true_agent
        } else {
            &self.false_agent
        };

        tracing::debug!("ConditionalAgent '{}' routing to '{}'", self.name, agent.name());
        agent.execute(input, context).await
    }
}

/// LlmConditionalAgent uses LLM to classify and route to appropriate sub-agent
pub struct LlmConditionalAgent {
    name: String,
    description: String,
    classifier_client: LlmClient,
    instruction: String,
    routes: HashMap<String, Arc<dyn BaseAgent>>,
    default_agent: Option<Arc<dyn BaseAgent>>,
}

impl LlmConditionalAgent {
    pub fn new(name: &str, classifier_client: LlmClient, instruction: &str) -> Self {
        Self {
            name: name.to_string(),
            description: format!("LLM conditional agent '{}'", name),
            classifier_client,
            instruction: instruction.to_string(),
            routes: HashMap::new(),
            default_agent: None,
        }
    }

    pub fn route(mut self, category: &str, agent: Arc<dyn BaseAgent>) -> Self {
        self.routes.insert(category.to_string(), agent);
        self
    }

    pub fn default_route(mut self, agent: Arc<dyn BaseAgent>) -> Self {
        self.default_agent = Some(agent);
        self
    }
}

#[async_trait]
impl BaseAgent for LlmConditionalAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn execute(&self, input: &str, context: &mut AgentContext) -> std::result::Result<AgentResponse, crate::base::AgentError> {
        // Classify input using LLM
        let messages = vec![
            ChatMessage {
                role: Role::System,
                content: self.instruction.clone(),
            },
            ChatMessage {
                role: Role::User,
                content: input.to_string(),
            },
        ];

        let classification = self.classifier_client.chat(messages).await?;
        let category = classification.content.trim().to_lowercase();

        tracing::debug!("LlmConditionalAgent '{}' classified as '{}'", self.name, category);

        // Route to appropriate agent
        let agent = self.routes.get(&category)
            .or_else(|| self.routes.values().next())
            .or_else(|| self.default_agent.as_ref());

        match agent {
            Some(a) => {
                tracing::info!("Routing to agent: {}", a.name());
                a.execute(input, context).await
            }
            None => Err(crate::base::AgentError::Execution(format!("No route for category: {}", category))),
        }
    }
}
