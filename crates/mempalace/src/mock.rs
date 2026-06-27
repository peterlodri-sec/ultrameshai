use crate::error::Result;
use crate::stats::UnitStats;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// MockMempalaceClient - in-memory stub for unit tests
pub struct MockMempalaceClient {
    stats: RwLock<HashMap<String, UnitStats>>,
}

impl MockMempalaceClient {
    pub fn new() -> Self {
        Self {
            stats: RwLock::new(HashMap::new()),
        }
    }

    pub async fn write_stats(&self, stats: UnitStats) -> Result<()> {
        let mut store = self.stats.write().await;
        store.insert(stats.unit_id.clone(), stats);
        Ok(())
    }

    pub async fn query_all(&self) -> Result<Vec<UnitStats>> {
        let store = self.stats.read().await;
        Ok(store.values().cloned().collect())
    }

    pub async fn query_stats(&self, query: StatsQueryBuilder) -> Result<Vec<UnitStats>> {
        let store = self.stats.read().await;
        let mut results: Vec<_> = store.values().cloned().collect();

        // Apply filters
        if let Some(slice_id) = query.slice_id {
            results.retain(|s| s.slice_id == slice_id);
        }
        if let Some(loop_type) = query.loop_type {
            results.retain(|s| s.loop_type == loop_type);
        }
        if let Some(status) = query.status {
            results.retain(|s| s.status == status);
        }

        Ok(results)
    }

    pub async fn get_unit(&self, unit_id: &str) -> Result<Option<UnitStats>> {
        let store = self.stats.read().await;
        Ok(store.get(unit_id).cloned())
    }

    pub async fn clear(&self) -> Result<()> {
        let mut store = self.stats.write().await;
        store.clear();
        Ok(())
    }
}

impl Default for MockMempalaceClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Query builder for filtering stats
pub struct StatsQueryBuilder {
    pub(crate) slice_id: Option<String>,
    pub(crate) loop_type: Option<String>,
    pub(crate) status: Option<String>,
}

impl StatsQueryBuilder {
    pub fn new() -> Self {
        Self {
            slice_id: None,
            loop_type: None,
            status: None,
        }
    }

    pub fn filter_slice_id(mut self, slice_id: &str) -> Self {
        self.slice_id = Some(slice_id.to_string());
        self
    }

    pub fn filter_loop_type(mut self, loop_type: &str) -> Self {
        self.loop_type = Some(loop_type.to_string());
        self
    }

    pub fn filter_status(mut self, status: &str) -> Self {
        self.status = Some(status.to_string());
        self
    }

    pub fn build(self) -> Self {
        self
    }
}

impl Default for StatsQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_write_and_query_all() {
        let mock = MockMempalaceClient::new();
        let stats = UnitStats::new(
            "u1".into(),
            "s1".into(),
            "coder".into(),
            1000,
            2000,
        );
        mock.write_stats(stats).await.unwrap();
        let results = mock.query_all().await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].unit_id, "u1");
    }

    #[tokio::test]
    async fn test_mock_query_by_status() {
        let mock = MockMempalaceClient::new();
        mock.write_stats(
            UnitStats::new("u1".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_status("completed"),
        )
        .await
        .unwrap();
        mock.write_stats(
            UnitStats::new("u2".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_status("killed"),
        )
        .await
        .unwrap();

        let query = StatsQueryBuilder::new().filter_status("completed").build();
        let results = mock.query_stats(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "completed");
    }

    #[tokio::test]
    async fn test_mock_query_by_slice() {
        let mock = MockMempalaceClient::new();
        mock.write_stats(
            UnitStats::new("u1".into(), "s1".into(), "coder".into(), 1000, 2000),
        )
        .await
        .unwrap();
        mock.write_stats(
            UnitStats::new("u2".into(), "s2".into(), "coder".into(), 1000, 2000),
        )
        .await
        .unwrap();

        let query = StatsQueryBuilder::new().filter_slice_id("s1").build();
        let results = mock.query_stats(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].slice_id, "s1");
    }

    #[tokio::test]
    async fn test_mock_get_unit() {
        let mock = MockMempalaceClient::new();
        let stats = UnitStats::new(
            "u1".into(),
            "s1".into(),
            "coder".into(),
            1000,
            2000,
        );
        mock.write_stats(stats.clone()).await.unwrap();

        let retrieved = mock.get_unit("u1").await.unwrap().unwrap();
        assert_eq!(retrieved.unit_id, "u1");
        assert_eq!(retrieved.slice_id, "s1");

        let not_found = mock.get_unit("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_mock_clear() {
        let mock = MockMempalaceClient::new();
        for i in 0..5 {
            mock.write_stats(UnitStats::new(
                format!("u{}", i),
                "s1".into(),
                "coder".into(),
                1000,
                2000,
            ))
            .await
            .unwrap();
        }

        let all = mock.query_all().await.unwrap();
        assert_eq!(all.len(), 5);

        mock.clear().await.unwrap();
        let all_after = mock.query_all().await.unwrap();
        assert_eq!(all_after.len(), 0);
    }

    #[tokio::test]
    async fn test_mock_query_by_loop_type() {
        let mock = MockMempalaceClient::new();
        mock.write_stats(
            UnitStats::new("u1".into(), "s1".into(), "coder".into(), 1000, 2000),
        )
        .await
        .unwrap();
        mock.write_stats(
            UnitStats::new("u2".into(), "s1".into(), "tester".into(), 1000, 2000),
        )
        .await
        .unwrap();

        let query = StatsQueryBuilder::new().filter_loop_type("coder").build();
        let results = mock.query_stats(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].loop_type, "coder");
    }

    #[tokio::test]
    async fn test_mock_query_combined_filters() {
        let mock = MockMempalaceClient::new();
        mock.write_stats(
            UnitStats::new("u1".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_status("completed"),
        )
        .await
        .unwrap();
        mock.write_stats(
            UnitStats::new("u2".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_status("killed"),
        )
        .await
        .unwrap();
        mock.write_stats(
            UnitStats::new("u3".into(), "s2".into(), "tester".into(), 1000, 2000)
                .with_status("completed"),
        )
        .await
        .unwrap();

        let query = StatsQueryBuilder::new()
            .filter_slice_id("s1")
            .filter_loop_type("coder")
            .filter_status("completed")
            .build();
        let results = mock.query_stats(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].unit_id, "u1");
    }
}
