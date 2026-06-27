//! Core abstractions and traits for all agent implementations.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Context and state shared during the execution of an agent or agent pipeline.
#[derive(Debug, Clone, Default)]
pub struct AgentContext {
    /// Unique identifier of the current execution session.
    pub session_id: String,
    /// Key-value store containing session variables and memory.
    pub state: std::collections::HashMap<String, String>,
}

impl AgentContext {
    /// Create a new context initialized with a session ID.
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            state: std::collections::HashMap::new(),
        }
    }

    /// Retrieve a value from the state by key.
    pub fn get(&self, key: &str) -> Option<&String> {
        self.state.get(key)
    }

    /// Insert or update a value in the state by key.
    pub fn set(&mut self, key: &str, value: String) {
        self.state.insert(key.to_string(), value);
    }
}

/// Response returned by an agent containing the result and execution control flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// Text content returned by the agent.
    pub content: String,
    /// Indicates whether the task or loop execution is completed.
    pub done: bool,
}

impl AgentResponse {
    /// Create a response that signals execution is completed.
    pub fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
            done: true,
        }
    }

    /// Create a response that signals execution should continue.
    pub fn continue_(content: &str) -> Self {
        Self {
            content: content.to_string(),
            done: false,
        }
    }
}

/// Base trait that every agent in the fleet must implement.
#[async_trait]
pub trait BaseAgent: Send + Sync {
    /// The unique name of this agent.
    fn name(&self) -> &str;
    
    /// A description of the agent's purpose or capability.
    fn description(&self) -> &str;
    
    /// Execute the agent's logic with the given text input and mutable context.
    async fn execute(&self, input: &str, context: &mut AgentContext) -> std::result::Result<AgentResponse, AgentError>;
}

/// Error types occurring during agent execution.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    /// Error during execution logic (e.g. invalid status or empty states).
    #[error("Execution error: {0}")]
    Execution(String),
    
    /// Error propagated from the cognition/LLM layer.
    #[error("LLM error: {0}")]
    Llm(#[from] loop_engineering_cognition::error::CognitionError),
    
    /// Error propagated from the underlying loop runtime.
    #[error("Loop error: {0}")]
    Loop(#[from] loop_engineering_loops::traits::LoopError),
}


