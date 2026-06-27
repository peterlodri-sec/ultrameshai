/// QueryBuilder for similarity search with metadata filters
pub struct QueryBuilder {
    query_text: Option<String>,
    query_embedding: Option<Vec<f32>>,
    top_k: usize,
    agent_filter: Option<String>,
    topic_filter: Option<String>,
    tag_filters: Vec<String>,
    time_range: Option<(u64, u64)>,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self {
            query_text: None,
            query_embedding: None,
            top_k: 10,
            agent_filter: None,
            topic_filter: None,
            tag_filters: vec![],
            time_range: None,
        }
    }

    /// Set similarity search with text (will be embedded)
    pub fn similarity(mut self, text: &str, top_k: usize) -> Self {
        self.query_text = Some(text.to_string());
        self.top_k = top_k;
        self
    }

    /// Set similarity search with pre-computed embedding
    pub fn similarity_embedding(mut self, embedding: Vec<f32>, top_k: usize) -> Self {
        self.query_embedding = Some(embedding);
        self.top_k = top_k;
        self
    }

    /// Filter by agent_id
    pub fn filter_agent(mut self, agent_id: &str) -> Self {
        self.agent_filter = Some(agent_id.to_string());
        self
    }

    /// Filter by topic
    pub fn filter_topic(mut self, topic: &str) -> Self {
        self.topic_filter = Some(topic.to_string());
        self
    }

    /// Filter by tags (AND logic)
    pub fn filter_tags(mut self, tags: Vec<String>) -> Self {
        self.tag_filters = tags;
        self
    }

    /// Filter by time range (start_ms, end_ms)
    pub fn filter_time_range(mut self, start_ms: u64, end_ms: u64) -> Self {
        self.time_range = Some((start_ms, end_ms));
        self
    }

    /// Build the query
    pub fn build(self) -> Self {
        self
    }

    /// Get query text
    pub fn query_text(&self) -> Option<&str> {
        self.query_text.as_deref()
    }

    /// Get query embedding
    pub fn query_embedding(&self) -> Option<&Vec<f32>> {
        self.query_embedding.as_ref()
    }

    /// Get top_k
    pub fn top_k(&self) -> usize {
        self.top_k
    }

    /// Get agent filter
    pub fn agent_filter(&self) -> Option<&str> {
        self.agent_filter.as_deref()
    }

    /// Get topic filter
    pub fn topic_filter(&self) -> Option<&str> {
        self.topic_filter.as_deref()
    }

    /// Get tag filters
    pub fn tag_filters(&self) -> &[String] {
        &self.tag_filters
    }

    /// Get time range
    pub fn time_range(&self) -> Option<(u64, u64)> {
        self.time_range
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_basic() {
        let query = QueryBuilder::new()
            .similarity("test query", 5)
            .build();

        assert_eq!(query.query_text(), Some("test query"));
        assert_eq!(query.top_k(), 5);
    }

    #[test]
    fn test_query_builder_filters() {
        let query = QueryBuilder::new()
            .similarity("test", 10)
            .filter_agent("deep-research")
            .filter_topic("tokio UDS")
            .filter_tags(vec!["rust".into(), "uds".into()])
            .filter_time_range(1719400000000, 1719500000000)
            .build();

        assert_eq!(query.agent_filter(), Some("deep-research"));
        assert_eq!(query.topic_filter(), Some("tokio UDS"));
        assert_eq!(query.tag_filters(), &["rust".to_string(), "uds".to_string()]);
        assert_eq!(query.time_range(), Some((1719400000000, 1719500000000)));
    }

    #[test]
    fn test_query_builder_embedding() {
        let embedding = vec![0.1, 0.2, 0.3];
        let query = QueryBuilder::new()
            .similarity_embedding(embedding.clone(), 3)
            .build();

        assert_eq!(query.query_embedding(), Some(&embedding));
        assert_eq!(query.top_k(), 3);
        assert_eq!(query.query_text(), None);
    }

    #[test]
    fn test_query_builder_default_top_k() {
        let query = QueryBuilder::new().build();
        assert_eq!(query.top_k(), 10);
    }
}
