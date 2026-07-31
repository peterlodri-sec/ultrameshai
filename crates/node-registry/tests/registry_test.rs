use loop_engineering_node_registry::registry::NodeRegistry;
use loop_engineering_node_registry::types::{NodeEntry, NodeMetadata, NodeStatus};

fn make_node_entry(node_id: &str, memory_mb: u64) -> NodeEntry {
    NodeEntry::new(NodeMetadata {
        node_id: node_id.to_string(),
        capabilities: vec!["standard".to_string(), "test".to_string()],
        memory_mb,
        load_avg: Some(0.5),
        region: Some("eu".to_string()),
    })
}

#[test]
fn test_register_and_query_node() {
    let mut registry = NodeRegistry::new(90);
    let entry = make_node_entry("vm-01", 60000);
    registry.register_node(entry);

    let nodes = registry.get_all_nodes();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].metadata.memory_mb, 60000);
}

#[test]
fn test_register_multiple_nodes() {
    let mut registry = NodeRegistry::new(90);
    registry.register_node(make_node_entry("vm-01", 60000));
    registry.register_node(make_node_entry("vm-02", 30000));
    registry.register_node(make_node_entry("rpi-01", 3000));

    let nodes = registry.get_all_nodes();
    assert_eq!(nodes.len(), 3);
}

#[test]
fn test_node_counts() {
    let mut registry = NodeRegistry::new(90);
    registry.register_node(make_node_entry("vm-01", 60000));
    registry.register_node(make_node_entry("vm-02", 30000));

    let (total, online, offline) = registry.get_node_counts();
    assert_eq!(total, 2);
    assert_eq!(online, 2);
    assert_eq!(offline, 0);
}

#[test]
fn test_uptime() {
    let registry = NodeRegistry::new(90);
    let _uptime = registry.uptime_secs();
    // uptime >= 0 always holds for u64
}

#[test]
fn test_heartbeat_validation_valid() {
    use loop_engineering_node_registry::types::HeartbeatRequest;
    let req = HeartbeatRequest {
        node_id: "vm-01".to_string(),
        capabilities: vec!["gpu".to_string()],
        memory_mb: 60000,
        load_avg: Some(0.3),
        region: Some("eu-west".to_string()),
    };
    assert_eq!(req.validate(), None);
}

#[test]
fn test_heartbeat_validation_empty_node_id() {
    use loop_engineering_node_registry::types::HeartbeatRequest;
    let req = HeartbeatRequest {
        node_id: "".to_string(),
        capabilities: vec![],
        memory_mb: 0,
        load_avg: None,
        region: None,
    };
    assert_eq!(req.validate(), Some("node_id"));
}

#[test]
fn test_heartbeat_validation_long_node_id() {
    use loop_engineering_node_registry::types::HeartbeatRequest;
    let req = HeartbeatRequest {
        node_id: "a".repeat(65),
        capabilities: vec![],
        memory_mb: 0,
        load_avg: None,
        region: None,
    };
    assert_eq!(req.validate(), Some("node_id"));
}

#[test]
fn test_heartbeat_validation_invalid_chars() {
    use loop_engineering_node_registry::types::HeartbeatRequest;
    let req = HeartbeatRequest {
        node_id: "bad\0char".to_string(),
        capabilities: vec![],
        memory_mb: 0,
        load_avg: None,
        region: None,
    };
    assert_eq!(req.validate(), Some("node_id"));
}

#[test]
fn test_heartbeat_validation_too_many_caps() {
    use loop_engineering_node_registry::types::HeartbeatRequest;
    let req = HeartbeatRequest {
        node_id: "vm-01".to_string(),
        capabilities: (0..21).map(|i| format!("cap_{}", i)).collect(),
        memory_mb: 0,
        load_avg: None,
        region: None,
    };
    assert_eq!(req.validate(), Some("capabilities"));
}

#[test]
fn test_heartbeat_validation_cap_too_long() {
    use loop_engineering_node_registry::types::HeartbeatRequest;
    let req = HeartbeatRequest {
        node_id: "vm-01".to_string(),
        capabilities: vec!["a".repeat(65)],
        memory_mb: 0,
        load_avg: None,
        region: None,
    };
    assert_eq!(req.validate(), Some("capabilities"));
}

#[test]
fn test_heartbeat_validation_memory_oob() {
    use loop_engineering_node_registry::types::HeartbeatRequest;
    let req = HeartbeatRequest {
        node_id: "vm-01".to_string(),
        capabilities: vec![],
        memory_mb: 1_000_001,
        load_avg: None,
        region: None,
    };
    assert_eq!(req.validate(), Some("memory_mb"));
}

#[test]
fn test_node_entry_failures() {
    use loop_engineering_node_registry::types::{NodeEntry, NodeMetadata, NodeStatus};
    let mut entry = NodeEntry::new(NodeMetadata {
        node_id: "rpi-01".to_string(),
        capabilities: vec![],
        memory_mb: 3000,
        load_avg: None,
        region: None,
    });
    assert_eq!(entry.status, NodeStatus::Online);

    // 2 failures: still online
    entry.mark_failure();
    entry.mark_failure();
    assert_eq!(entry.status, NodeStatus::Online);

    // 3rd failure: offline
    entry.mark_failure();
    assert_eq!(entry.status, NodeStatus::Offline);
    assert_eq!(entry.consecutive_failures, 3);

    // Heartbeat resets
    entry.update_heartbeat();
    assert_eq!(entry.status, NodeStatus::Online);
    assert_eq!(entry.consecutive_failures, 0);
}
