//! # Loop-Engineering Agents Crate
//!
//! Provides the base traits and orchestrators for building agent teams, pipelines, 
//! and routing fleets. Core abstractions include sequential pipelines, loop executors,
//! and rule-based or LLM-based conditional routers.

pub mod base;
pub mod loop_agent;
pub mod sequential;
pub mod conditional;

pub use base::{BaseAgent, AgentContext, AgentResponse, AgentError};
pub use loop_agent::LoopAgent;
pub use sequential::SequentialAgent;
pub use conditional::{ConditionalAgent, LlmConditionalAgent};

