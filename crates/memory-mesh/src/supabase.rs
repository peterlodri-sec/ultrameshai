//! Supabase client for 4D memory mesh
use crate::{Memory, Connector, A2AExchange, AgentReward, AgentMotivationAggregate};
use reqwest::Client;
use serde_json::json;
use uuid::Uuid;

pub struct SupabaseClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl SupabaseClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
        }
    }

    /// Insert a memory node
    pub async fn insert_memory(&self, memory: &Memory) -> Result<(), SupabaseError> {
        self.client
            .post(&format!("{}/rest/v1/memories", self.base_url))
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::to_value(memory)?)
            .send()
            .await?;
        Ok(())
    }

    /// Query memories by embedding similarity
    pub async fn query_similar(
        &self,
        embedding: &[f32],
        limit: u32,
    ) -> Result<Vec<Memory>, SupabaseError> {
        let response = self.client
            .post(&format!("{}/rest/v1/rpc/memories_similar", self.base_url))
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "query_embedding": embedding,
                "match_limit": limit
            }))
            .send()
            .await?;
        
        let memories: Vec<Memory> = response.json().await?;
        Ok(memories)
    }

    /// Insert a connector edge
    pub async fn insert_connector(&self, connector: &Connector) -> Result<(), SupabaseError> {
        self.client
            .post(&format!("{}/rest/v1/connectors", self.base_url))
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::to_value(connector)?)
            .send()
            .await?;
        Ok(())
    }

    /// Log A2A exchange
    pub async fn log_a2a(&self, exchange: &A2AExchange) -> Result<(), SupabaseError> {
        self.client
            .post(&format!("{}/rest/v1/a2a_exchanges", self.base_url))
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::to_value(exchange)?)
            .send()
            .await?;
        Ok(())
    }

    /// Insert reward entry
    pub async fn insert_reward(&self, reward: &AgentReward) -> Result<(), SupabaseError> {
        self.client
            .post(&format!("{}/rest/v1/agent_rewards", self.base_url))
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::to_value(reward)?)
            .send()
            .await?;
        Ok(())
    }

    /// Get agent motivation aggregate
    pub async fn get_motivation(&self, agent_id: Uuid) -> Result<Option<AgentMotivationAggregate>, SupabaseError> {
        let response = self.client
            .get(&format!("{}/rest/v1/agent_motivations?agent_id=eq.{}", self.base_url, agent_id))
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .send()
            .await?;
        
        let mut results: Vec<AgentMotivationAggregate> = response.json().await?;
        Ok(results.pop())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SupabaseError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
