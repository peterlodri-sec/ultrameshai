use crate::error::{MilvusError, Result};
use crate::query::QueryBuilder;
use crate::write::ResearchFinding;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// MockMilvusClient - in-memory stub for unit tests
pub struct MockMilvusClient {
    findings: RwLock<HashMap<String, ResearchFinding>>,
}

impl MockMilvusClient {
    pub fn new() -> Self {
        Self {
            findings: RwLock::new(HashMap::new()),
        }
    }

    /// Write a single finding
    pub async fn write_finding(&self, finding: ResearchFinding) -> Result<()> {
        let mut findings = self.findings.write().await;
        findings.insert(finding.finding_id.clone(), finding);
        Ok(())
    }

    /// Batch write findings
    pub async fn batch_write(&self, findings: Vec<ResearchFinding>) -> Result<()> {
        if findings.len() > 100 {
            return Err(MilvusError::Write(
                "Batch size exceeds maximum of 100".to_string(),
            ));
        }
        let mut store = self.findings.write().await;
        for finding in findings {
            store.insert(finding.finding_id.clone(), finding);
        }
        Ok(())
    }

    /// Search findings with query filters
    pub async fn search(&self, query: QueryBuilder) -> Result<Vec<ResearchFinding>> {
        let findings = self.findings.read().await;
        let mut results: Vec<ResearchFinding> = findings.values().cloned().collect();

        // Apply agent filter
        if let Some(agent) = query.agent_filter() {
            results.retain(|f| f.source_agent == agent);
        }

        // Apply topic filter
        if let Some(topic) = query.topic_filter() {
            results.retain(|f| f.topic == topic);
        }

        // Apply tag filters (AND logic)
        for tag in query.tag_filters() {
            results.retain(|f| f.tags.contains(&tag.to_string()));
        }

        // Apply time range filter
        if let Some((start, end)) = query.time_range() {
            results.retain(|f| f.timestamp_ms >= start && f.timestamp_ms <= end);
        }

        // Limit to top_k
        let top_k = query.top_k();
        results.truncate(top_k);

        Ok(results)
    }

    /// Delete finding by ID
    pub async fn delete_finding(&self, finding_id: &str) -> Result<()> {
        let mut findings = self.findings.write().await;
        findings.remove(finding_id);
        Ok(())
    }

    /// Get count of stored findings
    pub async fn count(&self) -> usize {
        let findings = self.findings.read().await;
        findings.len()
    }

    /// Clear all findings
    pub async fn clear(&self) {
        let mut findings = self.findings.write().await;
        findings.clear();
    }
}

impl Default for MockMilvusClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_finding(id: &str, agent: &str, topic: &str, tags: Vec<String>) -> ResearchFinding {
        ResearchFinding::new(id, agent, topic, "test summary", vec![0.1; 1536], tags)
    }

    #[tokio::test]
    async fn test_mock_write_and_search() {
        let mock = MockMilvusClient::new();
        let finding = create_test_finding("f1", "deep-research", "tokio", vec!["rust".into()]);
        
        mock.write_finding(finding.clone()).await.unwrap();
        let results = mock.search(QueryBuilder::new()).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].finding_id, "f1");
    }

    #[tokio::test]
    async fn test_mock_batch_write() {
        let mock = MockMilvusClient::new();
        let findings: Vec<ResearchFinding> = (0..10)
            .map(|i| create_test_finding(&format!("f{}", i), "agent", "topic", vec![]))
            .collect();

        mock.batch_write(findings).await.unwrap();
        assert_eq!(mock.count().await, 10);
    }

    #[tokio::test]
    async fn test_mock_batch_write_exceeds_limit() {
        let mock = MockMilvusClient::new();
        let findings: Vec<ResearchFinding> = (0..101)
            .map(|i| create_test_finding(&format!("f{}", i), "agent", "topic", vec![]))
            .collect();

        let result = mock.batch_write(findings).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_search_with_agent_filter() {
        let mock = MockMilvusClient::new();
        mock.write_finding(create_test_finding("f1", "deep-research", "tokio", vec![]))
            .await
            .unwrap();
        mock.write_finding(create_test_finding("f2", "junior-burst", "tokio", vec![]))
            .await
            .unwrap();

        let query = QueryBuilder::new().filter_agent("deep-research");
        let results = mock.search(query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_agent, "deep-research");
    }

    #[tokio::test]
    async fn test_mock_search_with_topic_filter() {
        let mock = MockMilvusClient::new();
        mock.write_finding(create_test_finding("f1", "agent", "tokio", vec![]))
            .await
            .unwrap();
        mock.write_finding(create_test_finding("f2", "agent", "async", vec![]))
            .await
            .unwrap();

        let query = QueryBuilder::new().filter_topic("tokio");
        let results = mock.search(query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].topic, "tokio");
    }

    #[tokio::test]
    async fn test_mock_search_with_tag_filter() {
        let mock = MockMilvusClient::new();
        mock.write_finding(create_test_finding("f1", "agent", "tokio", vec!["rust".into(), "uds".into()]))
            .await
            .unwrap();
        mock.write_finding(create_test_finding("f2", "agent", "async", vec!["rust".into()]))
            .await
            .unwrap();

        let query = QueryBuilder::new().filter_tags(vec!["rust".into(), "uds".into()]);
        let results = mock.search(query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].finding_id, "f1");
    }

    #[tokio::test]
    async fn test_mock_delete_finding() {
        let mock = MockMilvusClient::new();
        mock.write_finding(create_test_finding("f1", "agent", "topic", vec![]))
            .await
            .unwrap();
        
        assert_eq!(mock.count().await, 1);
        
        mock.delete_finding("f1").await.unwrap();
        assert_eq!(mock.count().await, 0);
    }

    #[tokio::test]
    async fn test_mock_clear() {
        let mock = MockMilvusClient::new();
        for i in 0..5 {
            mock.write_finding(create_test_finding(&format!("f{}", i), "agent", "topic", vec![]))
                .await
                .unwrap();
        }

        assert_eq!(mock.count().await, 5);
        mock.clear().await;
        assert_eq!(mock.count().await, 0);
    }

    #[tokio::test]
    async fn test_mock_search_top_k() {
        let mock = MockMilvusClient::new();
        for i in 0..10 {
            mock.write_finding(create_test_finding(&format!("f{}", i), "agent", "topic", vec![]))
                .await
                .unwrap();
        }

        let query = QueryBuilder::new().similarity("test", 3);
        let results = mock.search(query).await.unwrap();

        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_mock_search_time_range() {
        let mock = MockMilvusClient::new();
        
        let mut f1 = create_test_finding("f1", "agent", "topic", vec![]);
        f1.timestamp_ms = 1000;
        
        let mut f2 = create_test_finding("f2", "agent", "topic", vec![]);
        f2.timestamp_ms = 2000;
        
        let mut f3 = create_test_finding("f3", "agent", "topic", vec![]);
        f3.timestamp_ms = 3000;

        mock.write_finding(f1).await.unwrap();
        mock.write_finding(f2).await.unwrap();
        mock.write_finding(f3).await.unwrap();

        let query = QueryBuilder::new().filter_time_range(1500, 2500);
        let results = mock.search(query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].finding_id, "f2");
    }
}
