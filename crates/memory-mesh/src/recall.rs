use crate::{Memory, Connector, ConnectorType};
use uuid::Uuid;
use rand::Rng;

/// Recall result with confidence score
#[derive(Debug, Clone)]
pub struct RecallResult {
    pub memory: Memory,
    pub confidence: f32,
    pub path: Vec<Uuid>,  // Graph walk path
}

/// Stochastic graph walk for random recall
pub fn stochastic_recall(
    start_memory: &Memory,
    connectors: &[Connector],
    max_steps: u32,
    threshold: f32,
) -> Vec<RecallResult> {
    let mut results = Vec::new();
    let mut visited = vec![start_memory.id];
    let mut current_id = start_memory.id;
    let mut path = vec![current_id];

    for _ in 0..max_steps {
        // Find all connectors from current node
        let outgoing: Vec<&Connector> = connectors
            .iter()
            .filter(|c| c.from_id == current_id && !visited.contains(&c.to_id))
            .collect();

        if outgoing.is_empty() {
            break;
        }

        // Stochastic selection (weighted random)
        let mut rng = rand::rng();
        let total_weight: f32 = outgoing.iter().map(|c| c.weight).sum();
        let mut random = rng.random::<f32>() * total_weight;

        let next = outgoing
            .iter()
            .find(|c| {
                random -= c.weight;
                random <= 0.0
            })
            .copied()
            .unwrap_or(outgoing[0]);

        // Check confidence threshold ("deja vu")
        if next.weight >= threshold {
            // This is a "deja vu" moment - high confidence connection
            results.push(RecallResult {
                memory: Memory::new(
                    format!("Recalled via {:?}", next.connector_type),
                    "recall".to_string(),
                    vec![],
                ),
                confidence: next.weight,
                path: path.clone(),
            });
        }

        visited.push(next.to_id);
        path.push(next.to_id);
        current_id = next.to_id;
    }

    results
}

/// Check if confidence threshold is crossed (deja vu trigger)
pub fn check_deja_vu(confidence: f32, threshold: f32) -> bool {
    confidence >= threshold
}

/// Get related memories by connector type
pub fn get_related_by_type(
    memory_id: Uuid,
    connectors: &[Connector],
    connector_type: ConnectorType,
) -> Vec<Uuid> {
    connectors
        .iter()
        .filter(|c| c.from_id == memory_id && c.connector_type == connector_type)
        .map(|c| c.to_id)
        .collect()
}
