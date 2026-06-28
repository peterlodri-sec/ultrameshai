use crate::A2AExchange;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;

/// A2A client for mandatory inter-agent calls
pub struct A2AClient {
    node_id: Uuid,
    base_url: String,
    api_key: Option<String>,
}

impl A2AClient {
    pub fn new(node_id: Uuid, base_url: String, api_key: Option<String>) -> Self {
        Self {
            node_id,
            base_url,
            api_key,
        }
    }

    /// Make mandatory A2A call to random mesh node
    pub async fn call(
        &self,
        to_node: Uuid,
        from_loop: &str,
        to_loop: Option<&str>,
        request: serde_json::Value,
    ) -> Result<A2AExchange, A2AError> {
        let start = std::time::Instant::now();

        // Build request
        let client = reqwest::Client::new();
        let mut req = client
            .post(&format!("{}/api/v1/a2a", self.base_url))
            .json(&json!({
                "from_node": self.node_id,
                "to_node": to_node,
                "from_loop": from_loop,
                "to_loop": to_loop,
                "payload": request
            }));

        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        // Execute call
        let response = req.send().await;
        let latency_ms = start.elapsed().as_millis() as u32;

        match response {
            Ok(resp) => {
                let response_json = resp.json::<serde_json::Value>().await.ok();
                Ok(A2AExchange {
                    id: Uuid::new_v4(),
                    from_node: self.node_id,
                    to_node,
                    from_loop: from_loop.to_string(),
                    to_loop: to_loop.map(String::from),
                    request,
                    response: response_json,
                    latency_ms: Some(latency_ms),
                    success: true,
                    error_message: None,
                    logged_at: Utc::now(),
                })
            }
            Err(e) => Ok(A2AExchange {
                id: Uuid::new_v4(),
                from_node: self.node_id,
                to_node,
                from_loop: from_loop.to_string(),
                to_loop: to_loop.map(String::from),
                request,
                response: None,
                latency_ms: Some(latency_ms),
                success: false,
                error_message: Some(e.to_string()),
                logged_at: Utc::now(),
            }),
        }
    }

    /// Log A2A exchange to Supabase
    pub async fn log_exchange(
        &self,
        exchange: &A2AExchange,
        supabase_url: &str,
        supabase_key: &str,
    ) -> Result<(), A2AError> {
        let client = reqwest::Client::new();
        client
            .post(&format!("{}/rest/v1/a2a_exchanges", supabase_url))
            .header("apikey", supabase_key)
            .header("Authorization", format!("Bearer {}", supabase_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::to_value(exchange)?)
            .send()
            .await?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum A2AError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Select random mesh node for A2A call
pub fn select_random_node(known_nodes: &[Uuid]) -> Uuid {
    use rand::Rng;
    let mut rng = rand::rng();
    *known_nodes.get(rng.random_range(0..known_nodes.len())).unwrap_or(&known_nodes[0])
}
