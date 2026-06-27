use serde::{Deserialize, Serialize};

/// ResearchFinding - represents a single research finding stored in milvus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchFinding {
    pub finding_id: String,
    pub source_agent: String,
    pub topic: String,
    pub summary: String,
    pub embedding: Vec<f32>,
    pub tags: Vec<String>,
    pub timestamp_ms: u64,
}

impl ResearchFinding {
    pub fn new(
        finding_id: &str,
        source_agent: &str,
        topic: &str,
        summary: &str,
        embedding: Vec<f32>,
        tags: Vec<String>,
    ) -> Self {
        Self {
            finding_id: finding_id.to_string(),
            source_agent: source_agent.to_string(),
            topic: topic.to_string(),
            summary: summary.to_string(),
            embedding,
            tags,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// Create with auto-generated UUID
    pub fn with_uuid(
        source_agent: &str,
        topic: &str,
        summary: &str,
        embedding: Vec<f32>,
        tags: Vec<String>,
    ) -> Self {
        let finding_id = format!("finding-{}", uuid::Uuid::new_v4());
        Self::new(&finding_id, source_agent, topic, summary, embedding, tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_research_finding_new() {
        let finding = ResearchFinding::new(
            "test-1",
            "deep-research",
            "tokio UDS",
            "Pipelined protobuf over UDS",
            vec![0.1; 1536],
            vec!["rust".into(), "uds".into()],
        );

        assert_eq!(finding.finding_id, "test-1");
        assert_eq!(finding.source_agent, "deep-research");
        assert_eq!(finding.topic, "tokio UDS");
        assert_eq!(finding.tags, vec!["rust", "uds"]);
        assert_eq!(finding.embedding.len(), 1536);
    }

    #[test]
    fn test_research_finding_with_uuid() {
        let finding = ResearchFinding::with_uuid(
            "junior-burst",
            "async io",
            "Test summary",
            vec![0.5; 1536],
            vec![],
        );

        assert!(finding.finding_id.starts_with("finding-"));
        assert_eq!(finding.source_agent, "junior-burst");
        assert_eq!(finding.topic, "async io");
        assert!(finding.timestamp_ms > 0);
    }

    #[test]
    fn test_research_finding_timestamp() {
        let finding1 = ResearchFinding::new("f1", "a", "t", "s", vec![], vec![]);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let finding2 = ResearchFinding::new("f2", "a", "t", "s", vec![], vec![]);

        assert!(finding2.timestamp_ms >= finding1.timestamp_ms);
    }
}
