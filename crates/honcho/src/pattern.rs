use serde_json::Value;

// Use proto if available, otherwise use stub
#[cfg(feature = "protobuf-generated")]
use loop_engineering_transport::proto::LearningPattern as ProtoLearningPattern;

#[cfg(not(feature = "protobuf-generated"))]
use crate::pattern::proto_stub::LearningPattern as ProtoLearningPattern;

/// LearningPattern - represents a detected pattern from honcho daemon
#[derive(Debug, Clone)]
pub struct LearningPattern {
    pub pattern_id: String,
    pub pattern_type: String,  // "performance", "failure", "success", "cross-loop"
    pub confidence: f32,       // 0.0-1.0
    pub affected_loops: Vec<String>,
    pub evidence_count: i64,
    pub summary: String,
    pub embedding: Vec<f32>,
    pub metadata: Value,
    pub created_at_ms: u64,
}

impl LearningPattern {
    pub fn new(
        pattern_type: &str,
        confidence: f32,
        summary: &str,
        affected_loops: Vec<String>,
    ) -> Self {
        Self {
            pattern_id: uuid::Uuid::new_v4().to_string(),
            pattern_type: pattern_type.to_string(),
            confidence,
            affected_loops,
            evidence_count: 0,
            summary: summary.to_string(),
            embedding: vec![],
            metadata: Value::Null,
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub fn with_evidence_count(mut self, count: i64) -> Self {
        self.evidence_count = count;
        self
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = embedding;
        self
    }
}

impl From<ProtoLearningPattern> for LearningPattern {
    fn from(proto: ProtoLearningPattern) -> Self {
        let metadata: Value = serde_json::from_slice(&proto.metadata).unwrap_or(Value::Null);
        Self {
            pattern_id: proto.pattern_id,
            pattern_type: proto.pattern_type,
            confidence: proto.confidence,
            affected_loops: proto.affected_loops,
            evidence_count: proto.evidence_count,
            summary: proto.summary,
            embedding: {
                let bytes = proto.embedding;
                let mut floats = Vec::with_capacity(bytes.len() / 4);
                for chunk in bytes.chunks_exact(4) {
                    floats.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                }
                floats
            },
            metadata,
            created_at_ms: proto.created_at_ms,
        }
    }
}

impl From<LearningPattern> for ProtoLearningPattern {
    fn from(pattern: LearningPattern) -> Self {
        let mut embedding_bytes = Vec::with_capacity(pattern.embedding.len() * 4);
        for &float in &pattern.embedding {
            let bytes: [u8; 4] = float.to_le_bytes();
            embedding_bytes.extend_from_slice(&bytes);
        }

        let metadata_bytes = serde_json::to_vec(&pattern.metadata).unwrap_or_default();

        Self {
            pattern_id: pattern.pattern_id,
            pattern_type: pattern.pattern_type,
            confidence: pattern.confidence,
            affected_loops: pattern.affected_loops,
            evidence_count: pattern.evidence_count,
            summary: pattern.summary,
            embedding: embedding_bytes,
            metadata: metadata_bytes,
            created_at_ms: pattern.created_at_ms,
        }
    }
}

// Temporary stub until protobuf is regenerated
#[cfg(not(feature = "protobuf-generated"))]
pub mod proto_stub {
    #[derive(Debug, Clone)]
    pub struct LearningPattern {
        pub pattern_id: String,
        pub pattern_type: String,
        pub confidence: f32,
        pub affected_loops: Vec<String>,
        pub evidence_count: i64,
        pub summary: String,
        pub embedding: Vec<u8>,
        pub metadata: Vec<u8>,
        pub created_at_ms: u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_learning_pattern_builder() {
        let pattern = LearningPattern::new(
            "performance",
            0.85,
            "Coder loops slow down after 100 units",
            vec!["coder".into(), "tester".into()],
        )
        .with_evidence_count(50)
        .with_metadata(serde_json::json!({
            "correlation": 0.72,
            "p_value": 0.001
        }));

        assert!(pattern.pattern_id.starts_with(""));
        assert_eq!(pattern.pattern_type, "performance");
        assert_eq!(pattern.confidence, 0.85);
        assert_eq!(pattern.affected_loops.len(), 2);
        assert_eq!(pattern.evidence_count, 50);
        assert!(!pattern.summary.is_empty());
    }

    #[test]
    fn test_protobuf_conversion_from_proto() {
        let embedding = vec![0.1f32, 0.2, 0.3];
        let mut embedding_bytes = Vec::new();
        for &f in &embedding {
            let bytes: [u8; 4] = f.to_le_bytes();
            embedding_bytes.extend_from_slice(&bytes);
        }

        let proto = ProtoLearningPattern {
            pattern_id: "p1".into(),
            pattern_type: "failure".into(),
            confidence: 0.9,
            affected_loops: vec!["coder".into()],
            evidence_count: 10,
            summary: "Test pattern".into(),
            embedding: embedding_bytes,
            metadata: vec![],
            created_at_ms: 1719500000000,
        };

        let pattern: LearningPattern = proto.clone().into();

        assert_eq!(pattern.pattern_id, "p1");
        assert_eq!(pattern.pattern_type, "failure");
        assert_eq!(pattern.confidence, 0.9);
        assert_eq!(pattern.evidence_count, 10);
        assert_eq!(pattern.embedding, embedding);
    }

    #[test]
    fn test_protobuf_conversion_to_proto() {
        let pattern = LearningPattern::new(
            "success",
            0.75,
            "Test summary",
            vec!["tester".into()],
        )
        .with_evidence_count(25)
        .with_metadata(serde_json::json!({"key": "value"}));

        let proto: ProtoLearningPattern = pattern.clone().into();

        assert_eq!(proto.pattern_type, "success");
        assert_eq!(proto.confidence, 0.75);
        assert_eq!(proto.evidence_count, 25);
        assert_eq!(proto.affected_loops, vec!["tester"]);
    }

    #[test]
    fn test_protobuf_roundtrip() {
        let original = LearningPattern::new(
            "cross-loop",
            0.95,
            "Cross-loop correlation",
            vec!["coder".into(), "tester".into(), "red-team".into()],
        )
        .with_evidence_count(100)
        .with_metadata(serde_json::json!({
            "loops": ["coder", "tester", "red-team"],
            "correlation": 0.95
        }));

        let proto: ProtoLearningPattern = original.clone().into();
        let back: LearningPattern = proto.into();

        assert_eq!(back.pattern_type, original.pattern_type);
        assert_eq!(back.confidence, original.confidence);
        assert_eq!(back.evidence_count, original.evidence_count);
        assert_eq!(back.affected_loops, original.affected_loops);
        assert_eq!(back.summary, original.summary);
    }
}
