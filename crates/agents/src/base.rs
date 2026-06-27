use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Agent execution context
#[derive(Debug, Clone, Default)]
pub struct AgentContext {
    pub session_id: String,
    pub state: std::collections::HashMap<String, String>,
}

impl AgentContext {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            state: std::collections::HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.state.get(key)
    }

    pub fn set(&mut self, key: &str, value: String) {
        self.state.insert(key.to_string(), value);
    }
}

/// Agent response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub content: String,
    pub done: bool,
}

impl AgentResponse {
    pub fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
            done: true,
        }
    }

    pub fn continue_(content: &str) -> Self {
        Self {
            content: content.to_string(),
            done: false,
        }
    }
}

/// Base trait for all agents
#[async_trait]
pub trait BaseAgent: Send + Sync {
    /// Agent identifier
    fn name(&self) -> &str;
    
    /// Agent description
    fn description(&self) -> &str;
    
    /// Execute the agent with given input and context
    async fn execute(&self, input: &str, context: &mut AgentContext) -> std::result::Result<AgentResponse, AgentError>;
}

/// Agent error types
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Execution error: {0}")]
    Execution(String),
    
    #[error("LLM error: {0}")]
    Llm(#[from] loop_engineering_cognition::error::CognitionError),
    
    #[error("Loop error: {0}")]
    Loop(#[from] loop_engineering_loops::traits::LoopError),
}


