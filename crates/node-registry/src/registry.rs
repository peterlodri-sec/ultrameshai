use std::collections::HashMap;
use std::time::Instant;
use chrono::Duration;
use crate::types::{NodeEntry, NodeStatus};

pub struct NodeRegistry {
    nodes: HashMap<String, NodeEntry>,
    stale_threshold_secs: u64,
    start_time: Instant,
}

impl NodeRegistry {
    pub fn new(stale_threshold_secs: u64) -> Self {
        Self {
            nodes: HashMap::new(),
            stale_threshold_secs,
            start_time: Instant::now(),
        }
    }

    /// Register or update a node from heartbeat
    pub fn register_node(&mut self, entry: NodeEntry) {
        self.nodes.insert(entry.metadata.node_id.clone(), entry);
    }

    /// Get all nodes
    pub fn get_all_nodes(&self) -> Vec<NodeEntry> {
        self.nodes.values().cloned().collect()
    }

    /// Get node counts for health endpoint
    pub fn get_node_counts(&self) -> (usize, usize, usize) {
        let total = self.nodes.len();
        let online = self.nodes.values().filter(|n| n.status == NodeStatus::Online).count();
        let offline = total - online;
        (total, online, offline)
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs() as u64
    }

    /// Check for stale nodes and return their IDs
    pub fn check_stale_nodes(&self) -> Vec<String> {
        let now = chrono::Utc::now();
        let threshold = Duration::seconds(self.stale_threshold_secs as i64);
        self.nodes.iter()
            .filter(|(_, entry)| now.signed_duration_since(entry.last_heartbeat) > threshold)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Mark node as offline
    pub fn mark_offline(&mut self, node_id: &str) {
        if let Some(entry) = self.nodes.get_mut(node_id) {
            entry.mark_failure();
        }
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new(90)
    }
}
