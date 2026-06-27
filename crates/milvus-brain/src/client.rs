use crate::error::{MilvusError, Result};
use crate::write::ResearchFinding;
use crate::query::QueryBuilder;

/// MilvusClient - async connection to Milvus server
pub struct MilvusClient {
    uri: String,
}

impl MilvusClient {
    /// Connect to Milvus server
    pub async fn connect(uri: &str) -> Result<Self> {
        // Validate URI format
        if !uri.starts_with("http://") && !uri.starts_with("https://") {
            return Err(MilvusError::Connection(
                "URI must start with http:// or https://".to_string(),
            ));
        }

        // In production, this would initialize the milvus-sdk client
        // For now, we return a stub that will be implemented with milvus-sdk
        Ok(Self {
            uri: uri.to_string(),
        })
    }

    /// Ensure collection exists (idempotent create)
    pub async fn ensure_collection(&self, collection_name: &str) -> Result<()> {
        // TODO: Implement with milvus-sdk
        // Check if collection exists, create if not
        // Create indexes (IVF_FLAT for embeddings, inverted for scalars)
        tracing::info!("Ensuring collection: {}", collection_name);
        Ok(())
    }

    /// Write a single finding
    pub async fn write_finding(&self, finding: ResearchFinding) -> Result<()> {
        // TODO: Implement with milvus-sdk
        tracing::debug!("Writing finding: {}", finding.finding_id);
        Ok(())
    }

    /// Batch write findings (up to 100)
    pub async fn batch_write(&self, findings: Vec<ResearchFinding>) -> Result<()> {
        if findings.len() > 100 {
            return Err(MilvusError::Write(
                "Batch size exceeds maximum of 100".to_string(),
            ));
        }
        // TODO: Implement with milvus-sdk
        tracing::debug!("Batch writing {} findings", findings.len());
        Ok(())
    }

    /// Search by similarity + metadata filters
    pub async fn search(&self, _query: QueryBuilder) -> Result<Vec<ResearchFinding>> {
        // TODO: Implement with milvus-sdk
        tracing::debug!("Executing search query");
        Ok(vec![])
    }

    /// Delete finding by ID (soft delete)
    pub async fn delete_finding(&self, finding_id: &str) -> Result<()> {
        // TODO: Implement with milvus-sdk
        tracing::debug!("Deleting finding: {}", finding_id);
        Ok(())
    }

    /// Get server URI
    pub fn uri(&self) -> &str {
        &self.uri
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect_invalid_uri() {
        let result = MilvusClient::connect("invalid-uri").await;
        assert!(result.is_err());
        match result {
            Err(MilvusError::Connection(msg)) => {
                assert!(msg.contains("http://") || msg.contains("https://"));
            }
            _ => panic!("Expected Connection error"),
        }
    }

    #[tokio::test]
    async fn test_connect_valid_uri() {
        let result = MilvusClient::connect("http://localhost:19530").await;
        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.uri(), "http://localhost:19530");
    }

    #[tokio::test]
    async fn test_batch_write_exceeds_limit() {
        let client = MilvusClient::connect("http://localhost:19530")
            .await
            .unwrap();
        let findings: Vec<ResearchFinding> = (0..101)
            .map(|i| ResearchFinding {
                finding_id: format!("finding-{}", i),
                source_agent: "test".into(),
                topic: "test".into(),
                summary: "test".into(),
                embedding: vec![0.0; 1536],
                tags: vec![],
                timestamp_ms: 0,
            })
            .collect();

        let result = client.batch_write(findings).await;
        assert!(result.is_err());
        match result {
            Err(MilvusError::Write(msg)) => {
                assert!(msg.contains("100"));
            }
            _ => panic!("Expected Write error"),
        }
    }
}
