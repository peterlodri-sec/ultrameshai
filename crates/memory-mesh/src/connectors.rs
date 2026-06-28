use crate::{Memory, Connector, ConnectorType};
use uuid::Uuid;
use chrono::Utc;

/// Create a similarity connector between two memories
pub fn create_similarity_connector(from: &Memory, to: &Memory, weight: f32) -> Connector {
    Connector {
        id: Uuid::new_v4(),
        from_id: from.id,
        to_id: to.id,
        connector_type: ConnectorType::Similarity,
        weight,
        created_at: Utc::now(),
        metadata: serde_json::json!({"embedding_distance": 1.0 - weight}),
    }
}

/// Create a temporal connector (A happened before B)
pub fn create_temporal_connector(from: &Memory, to: &Memory) -> Connector {
    Connector {
        id: Uuid::new_v4(),
        from_id: from.id,
        to_id: to.id,
        connector_type: ConnectorType::Temporal,
        weight: 1.0,
        created_at: Utc::now(),
        metadata: serde_json::json!({
            "time_delta_ms": (to.encoded_at - from.encoded_at).num_milliseconds()
        }),
    }
}

/// Create a causal connector (A triggered B)
pub fn create_causal_connector(from: &Memory, to: &Memory, confidence: f32) -> Connector {
    Connector {
        id: Uuid::new_v4(),
        from_id: from.id,
        to_id: to.id,
        connector_type: ConnectorType::Causal,
        weight: confidence,
        created_at: Utc::now(),
        metadata: serde_json::json!({"causal_confidence": confidence}),
    }
}

/// Create a reinforcement connector (strengthened by access)
pub fn create_reinforcement_connector(from: &Memory, to: &Memory) -> Connector {
    let weight = (from.strength + to.strength) / 2.0;
    Connector {
        id: Uuid::new_v4(),
        from_id: from.id,
        to_id: to.id,
        connector_type: ConnectorType::Reinforcement,
        weight,
        created_at: Utc::now(),
        metadata: serde_json::json!({
            "from_strength": from.strength,
            "to_strength": to.strength
        }),
    }
}
