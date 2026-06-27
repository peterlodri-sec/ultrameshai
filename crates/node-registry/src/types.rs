use serde::{Deserialize, Serialize};

/// Node capabilities and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub node_id: String,
    pub capabilities: Vec<String>,
    pub memory_mb: u64,
    pub load_avg: Option<f32>,
    pub region: Option<String>,
}

/// Node liveness status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeStatus {
    Online,
    Offline,
    Degraded,
}

/// Full registry entry with timeout tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEntry {
    pub metadata: NodeMetadata,
    pub status: NodeStatus,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub tailscale_node_id: Option<String>,
    pub consecutive_failures: u32,
}

impl NodeEntry {
    pub fn new(metadata: NodeMetadata) -> Self {
        Self {
            metadata,
            status: NodeStatus::Online,
            last_heartbeat: chrono::Utc::now(),
            tailscale_node_id: None,
            consecutive_failures: 0,
        }
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = chrono::Utc::now();
        self.consecutive_failures = 0;
        self.status = NodeStatus::Online;
    }

    pub fn mark_failure(&mut self) {
        self.consecutive_failures += 1;
        if self.consecutive_failures >= 3 {
            self.status = NodeStatus::Offline;
        }
    }
}

/// Heartbeat request payload
#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub node_id: String,
    pub capabilities: Vec<String>,
    pub memory_mb: u64,
    pub load_avg: Option<f32>,
    pub region: Option<String>,
}

impl HeartbeatRequest {
    /// Validate bounds on all fields. Returns Some(field_name) if invalid.
    pub fn validate(&self) -> Option<&'static str> {
        if self.node_id.is_empty()
            || self.node_id.len() > 64
            || !self.node_id.chars().all(|c| c.is_ascii_graphic() || c == '-' || c == '_' || c == ':' || c == '.')
        {
            return Some("node_id");
        }
        if self.capabilities.len() > 20 {
            return Some("capabilities");
        }
        if self.capabilities.iter().any(|c| c.len() > 64) {
            return Some("capabilities");
        }
        if self.memory_mb > 1_000_000 {
            return Some("memory_mb");
        }
        None
    }
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub total_nodes: usize,
    pub online_nodes: usize,
    pub offline_nodes: usize,
    pub uptime_secs: u64,
}
