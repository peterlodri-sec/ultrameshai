use std::collections::HashMap;
use crate::types::{NodeEntry, NodeStatus};

pub struct NodeRegistry {
    nodes: HashMap<String, NodeEntry>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self { nodes: HashMap::new() }
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
        let online = self.nodes.values().filter(|n| matches!(n.status, NodeStatus::Online)).count();
        let offline = self.nodes.values().filter(|n| matches!(n.status, NodeStatus::Offline)).count();
        (total, online, offline)
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        0 // Simplified
    }

    /// Check for stale nodes and return their IDs
    pub fn check_stale_nodes(&self) -> Vec<String> {
        Vec::new() // Simplified
    }

    /// Mark node as offline
    pub fn mark_offline(&self, _node_id: &str) {
        // Simplified
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
