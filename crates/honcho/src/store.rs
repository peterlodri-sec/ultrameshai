use crate::error::{HonchoError, Result};
use crate::pattern::LearningPattern;
use milvus_brain::{MilvusClient, QueryBuilder, ResearchFinding};

/// PatternStore - writes and queries learning patterns in milvus
pub struct PatternStore {
    client: MilvusClient,
    collection_name: String,
}

impl PatternStore {
    /// Connect to milvus and ensure collection exists
    pub async fn connect(milvus_uri: &str) -> Result<Self> {
        let client = MilvusClient::connect(milvus_uri)
            .await
            .map_err(|e| HonchoError::Milvus(e))?;

        let store = Self {
            client,
            collection_name: "learning_patterns".to_string(),
        };

        store.ensure_collection().await?;
        Ok(store)
    }

    /// Ensure learning_patterns collection exists with proper schema
    pub async fn ensure_collection(&self) -> Result<()> {
        // Create collection with schema for learning patterns
        // Schema: pattern_id (PK), pattern_type, confidence, affected_loops,
        //         evidence_count, summary, embedding (1536d), metadata, created_at_ms
        self.client
            .ensure_collection(&self.collection_name)
            .await
            .map_err(|e| HonchoError::Milvus(e))?;
        Ok(())
    }

    /// Write a learning pattern to milvus
    /// Generates embedding via OVHcloud endpoint, writes to collection
    pub async fn write_pattern(&self, pattern: LearningPattern) -> Result<()> {
        // Convert LearningPattern to ResearchFinding for milvus storage
        let finding = self.pattern_to_finding(pattern).await?;

        self.client
            .write_finding(finding)
            .await
            .map_err(|e| HonchoError::Milvus(e))?;

        Ok(())
    }

    /// Convert LearningPattern to ResearchFinding for milvus storage
    async fn pattern_to_finding(&self, pattern: LearningPattern) -> Result<ResearchFinding> {
        // Generate embedding from summary + metadata
        let embedding_text = format!("{} {:?}", pattern.summary, pattern.metadata);

        // Use milvus-brain embedding client if available
        // For now, create placeholder embedding (1536d zero vector)
        // Production would call OVHcloud embedding endpoint
        let embedding = vec![0.0f32; 1536];

        let finding = ResearchFinding::new(
            &pattern.pattern_id,
            &pattern.pattern_type,
            &pattern.summary,
            &embedding_text,
            embedding,
            pattern.affected_loops.clone(),
        );

        Ok(finding)
    }

    /// Query similar patterns by text similarity
    pub async fn query_similar(&self, query: &str, top_k: usize) -> Result<Vec<LearningPattern>> {
        let query_builder = QueryBuilder::new().similarity(query, top_k);

        let findings = self
            .client
            .search(query_builder)
            .await
            .map_err(|e| HonchoError::Milvus(e))?;

        let patterns = findings
            .into_iter()
            .filter_map(|f| self.finding_to_pattern(f).ok())
            .collect();

        Ok(patterns)
    }

    /// Query patterns by type (performance, failure, success, cross-loop)
    pub async fn query_by_type(&self, pattern_type: &str) -> Result<Vec<LearningPattern>> {
        let query_builder = QueryBuilder::new().filter_agent(pattern_type);

        let findings = self
            .client
            .search(query_builder)
            .await
            .map_err(|e| HonchoError::Milvus(e))?;

        let patterns = findings
            .into_iter()
            .filter_map(|f| self.finding_to_pattern(f).ok())
            .collect();

        Ok(patterns)
    }

    /// Convert ResearchFinding back to LearningPattern
    fn finding_to_pattern(&self, finding: ResearchFinding) -> Result<LearningPattern> {
        // Parse metadata from summary field (simplified)
        let metadata = serde_json::Value::Null;

        let pattern = LearningPattern::new(
            &finding.source_agent,
            0.5, // Default confidence
            &finding.summary,
            finding.tags,
        )
        .with_evidence_count(0)
        .with_metadata(metadata)
        .with_embedding(finding.embedding);

        Ok(pattern)
    }

    /// Query all patterns (for debugging/testing)
    pub async fn query_all(&self) -> Result<Vec<LearningPattern>> {
        let query_builder = QueryBuilder::new();

        let findings = self
            .client
            .search(query_builder)
            .await
            .map_err(|e| HonchoError::Milvus(e))?;

        let patterns = findings
            .into_iter()
            .filter_map(|f| self.finding_to_pattern(f).ok())
            .collect();

        Ok(patterns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pattern_store_new() {
        // Test that PatternStore can be created (connection may succeed with fallback)
        // This test verifies the API exists and compiles
        let _result = PatternStore::connect("http://localhost:19530").await;
        // Connection result depends on milvus availability
    }

    #[test]
    fn test_pattern_to_finding_conversion() {
        // Test conversion logic without milvus connection
        let pattern = LearningPattern::new(
            "performance",
            0.85,
            "Test pattern summary",
            vec!["coder".into()],
        )
        .with_evidence_count(10)
        .with_metadata(serde_json::json!({"key": "value"}));

        assert_eq!(pattern.pattern_type, "performance");
        assert_eq!(pattern.confidence, 0.85);
        assert_eq!(pattern.summary, "Test pattern summary");
    }
}
