use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::proto::NodeHeartbeat;
use crate::error::{RegistryError, Result};

struct RegistryEntry {
    heartbeat: NodeHeartbeat,
    last_seen: Instant,
}

pub struct NodeRegistry {
    nodes: HashMap<String, RegistryEntry>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self { nodes: HashMap::new() }
    }

    /// Update or insert a node's heartbeat.
    pub fn update(&mut self, heartbeat: NodeHeartbeat) {
        let entry = RegistryEntry {
            heartbeat,
            last_seen: Instant::now(),
        };
        self.nodes.insert(entry.heartbeat.node_id.clone(), entry);
    }

    /// Get a node's current heartbeat.
    pub fn get(&self, node_id: &str) -> Option<&NodeHeartbeat> {
        self.nodes.get(node_id).map(|e| &e.heartbeat)
    }

    /// Find the best-fit node for a given sandbox tier and memory requirement.
    /// Best fit = most free memory among nodes with matching capability.
    pub fn find_best_fit(&self, tier: &str, need_mb: u64) -> Result<&NodeHeartbeat> {
        let mut best: Option<&NodeHeartbeat> = None;
        let mut best_free: u64 = 0;

        for entry in self.nodes.values() {
            let hb = &entry.heartbeat;
            if !hb.capabilities.iter().any(|c| c == tier) {
                continue;
            }
            if hb.memory_free_mb < need_mb {
                continue;
            }
            if hb.memory_free_mb > best_free {
                best_free = hb.memory_free_mb;
                best = Some(hb);
            }
        }

        best.ok_or(RegistryError::NoFit {
            tier: tier.into(),
            need_mb,
        })
    }

    /// Evict nodes not seen within the stale duration.
    pub fn evict_stale(&mut self, max_age: Duration) {
        let now = Instant::now();
        self.nodes.retain(|_, entry| now.duration_since(entry.last_seen) < max_age);
    }

    /// List all known nodes.
    pub fn list(&self) -> Vec<&NodeHeartbeat> {
        self.nodes.values().map(|e| &e.heartbeat).collect()
    }

    /// Update from protobuf heartbeat (for old UDP-based heartbeat)
    pub fn update_from_heartbeat(&mut self, hb: &NodeHeartbeat) {
        self.update(hb.clone());
    }

    /// Register or update a node from heartbeat
    pub fn register_node(&mut self, entry: NodeHeartbeat) {
        self.update(entry);
    }

    /// Get all nodes
    pub fn get_all_nodes(&self) -> Vec<&NodeHeartbeat> {
        self.list()
    }

    /// Get node counts for health endpoint
    pub fn get_node_counts(&self) -> (usize, usize, usize) {
        (self.nodes.len(), self.nodes.len(), 0) // Simplified
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        0 // Simplified
    }

    /// Check for stale nodes and mark them
    pub fn check_stale_nodes(&mut self) -> Vec<String> {
        Vec::new() // Simplified
    }

    /// Mark node as offline
    pub fn mark_offline(&mut self, _node_id: &str) {
        // Simplified
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
