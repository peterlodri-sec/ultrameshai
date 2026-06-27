use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use crate::types::{NodeMetadata, NodeStatus, NodeEntry};
use crate::proto::NodeHeartbeat;

/// Node registry with timeout tracking
pub struct NodeRegistry {
    nodes: Arc<RwLock<std::collections::HashMap<String, NodeEntry>>>,
    stale_threshold_secs: u64,
    offline_threshold: u32,
    start_time: DateTime<Utc>,
}

impl NodeRegistry {
    pub fn new(stale_threshold_secs: u64, offline_threshold: u32) -> Self {
        Self {
            nodes: Arc::new(RwLock::new(std::collections::HashMap::new())),
            stale_threshold_secs,
            offline_threshold,
            start_time: Utc::now(),
        }
    }

    /// Register or update a node from heartbeat
    pub async fn register_node(&self, entry: NodeEntry) {
        let mut nodes = self.nodes.write().await;
        nodes.insert(entry.metadata.node_id.clone(), entry);
    }

    /// Mark node as offline after consecutive failures
    pub async fn mark_offline(&self, node_id: &str) {
        let mut nodes = self.nodes.write().await;
        if let Some(entry) = nodes.get_mut(node_id) {
            entry.mark_failure();
        }
    }

    /// Get all nodes
    pub async fn get_all_nodes(&self) -> Vec<NodeEntry> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }

    /// Check for stale nodes and mark them
    pub async fn check_stale_nodes(&self) -> Vec<String> {
        let now = Utc::now();
        let threshold = Duration::seconds(self.stale_threshold_secs as i64);
        let mut stale_ids = Vec::new();

        let mut nodes = self.nodes.write().await;
        for (id, entry) in nodes.iter_mut() {
            if now.signed_duration_since(entry.last_heartbeat) > threshold {
                entry.mark_failure();
                if entry.status == NodeStatus::Offline {
                    stale_ids.push(id.clone());
                }
            }
        }

        stale_ids
    }

    /// Get node counts for health endpoint
    pub async fn get_node_counts(&self) -> (usize, usize, usize) {
        let nodes = self.nodes.read().await;
        let total = nodes.len();
        let online = nodes.values().filter(|n| n.status == NodeStatus::Online).count();
        let offline = total - online;
        (total, online, offline)
    }

    pub fn uptime_secs(&self) -> u64 {
        Utc::now().signed_duration_since(self.start_time).num_seconds() as u64
    }

    /// Update from protobuf heartbeat (for old UDP-based heartbeat)
    pub fn update_from_heartbeat(&mut self, hb: &NodeHeartbeat) {
        let entry = NodeEntry::new(NodeMetadata {
            node_id: hb.node_id.clone(),
            capabilities: hb.capabilities.clone(),
            memory_mb: hb.memory_total_mb,
            load_avg: None,
            region: None,
        });
        // Simplified - just for compilation
    }
}
