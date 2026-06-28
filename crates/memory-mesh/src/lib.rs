pub mod connectors;
pub mod recall;
pub mod a2a;
pub mod rewards;
pub mod supabase;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Memory node in the 4D mesh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: Uuid,
    pub embedding: Vec<f32>,  // 1536-dim semantic space
    pub content: String,
    pub loop_type: String,
    pub slice_id: Option<String>,
    pub encoded_at: DateTime<Utc>,
    pub accessed_at: Vec<DateTime<Utc>>,
    pub access_count: u32,
    pub decay_rate: f32,
    pub strength: f32,  // Reinforcement score
    pub metadata: serde_json::Value,
}

impl Memory {
    pub fn new(content: String, loop_type: String, embedding: Vec<f32>) -> Self {
        Self {
            id: Uuid::new_v4(),
            embedding,
            content,
            loop_type,
            slice_id: None,
            encoded_at: Utc::now(),
            accessed_at: vec![],
            access_count: 0,
            decay_rate: 0.01,
            strength: 1.0,
            metadata: serde_json::json!({}),
        }
    }

    pub fn touch(&mut self) {
        self.accessed_at.push(Utc::now());
        self.access_count += 1;
        self.strength += 0.1;
    }
}

/// Connector type (edge in the graph)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorType {
    Similarity,   // Semantic proximity
    Temporal,     // Sequential patterns
    Causal,       // A triggered B
    Reinforcement, // Access count strengthens
}

/// Inter-connector (edge between memories)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connector {
    pub id: Uuid,
    pub from_id: Uuid,
    pub to_id: Uuid,
    pub connector_type: ConnectorType,
    pub weight: f32,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

/// A2A exchange (mandatory for all agents)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AExchange {
    pub id: Uuid,
    pub from_node: Uuid,
    pub to_node: Uuid,
    pub from_loop: String,
    pub to_loop: Option<String>,
    pub request: serde_json::Value,
    pub response: Option<serde_json::Value>,
    pub latency_ms: Option<u32>,
    pub success: bool,
    pub error_message: Option<String>,
    pub logged_at: DateTime<Utc>,
}

/// Agent motivation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMotivation {
    pub agent_id: Uuid,
    pub purpose: String,  // "Why am I doing this?"
    pub reward_type: String,  // "points", "priority", "access_token"
    pub reward_amount: f32,
    pub intrinsic_drive: String,  // "curiosity", "completion", "optimization"
    pub social_pressure: f32,  // Peer expectation score
}

/// Agent motivation aggregate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMotivationAggregate {
    pub agent_id: Uuid,
    pub total_points: f32,
    pub completion_rate: f32,
    pub a2a_count: u32,
    pub last_active: DateTime<Utc>,
    pub motivation_tier: MotivationTier,
    pub intrinsic_drive: String,
    pub social_pressure: f32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MotivationTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
}

/// Reward entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReward {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub loop_type: String,
    pub task_id: Option<Uuid>,
    pub reward_type: String,
    pub reward_amount: f32,
    pub motivation_score: f32,
    pub success: bool,
    pub earned_at: DateTime<Utc>,
}
