use crate::error::{MilvusError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const OVHCLOUD_ENDPOINT: &str = "https://oai.endpoints.kepler.ai.cloud.ovh.net/v1";

/// OVHcloud embedding client
pub struct EmbeddingClient {
    client: Client,
    api_key: Option<String>,
    model: String,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

impl EmbeddingClient {
    pub fn new(api_key: Option<String>, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "bge-m3".to_string()),
        }
    }

    /// Generate embedding for single text
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let mut embeddings = self.batch_generate(vec![text.to_string()]).await?;
        embeddings
            .pop()
            .ok_or_else(|| MilvusError::Embedding("No embedding returned".to_string()))
    }

    /// Generate embeddings for multiple texts
    pub async fn batch_generate(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let request = EmbeddingRequest {
            model: self.model.clone(),
            input: texts,
        };

        let mut req_builder = self
            .client
            .post(format!("{}/embeddings", OVHCLOUD_ENDPOINT))
            .header("Content-Type", "application/json")
            .json(&request);

        if let Some(api_key) = &self.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| MilvusError::Embedding(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read body>".to_string());
            return Err(MilvusError::Embedding(format!(
                "API error {}: {}",
                status, body
            )));
        }

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| MilvusError::Embedding(format!("Parse error: {}", e)))?;

        // Sort by index to maintain order
        let mut embeddings: Vec<_> = embedding_response.data;
        embeddings.sort_by_key(|e| e.index);

        Ok(embeddings.into_iter().map(|e| e.embedding).collect())
    }

    /// Set the model to use
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_client_new() {
        let client = EmbeddingClient::new(None, None);
        assert_eq!(client.model, "bge-m3");
    }

    #[test]
    fn test_embedding_client_with_model() {
        let client = EmbeddingClient::new(None, Some("qwen-embedding".to_string()));
        assert_eq!(client.model, "qwen-embedding");
    }

    #[test]
    fn test_embedding_client_with_api_key() {
        let client = EmbeddingClient::new(Some("test-key".to_string()), None);
        assert_eq!(client.api_key, Some("test-key".to_string()));
    }
}
