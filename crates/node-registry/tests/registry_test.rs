use loop_engineering_node_registry::registry::NodeRegistry;
use loop_engineering_node_registry::proto::NodeHeartbeat;
use std::time::Duration;

fn make_heartbeat(node_id: &str, memory_free_mb: u64, units: u32) -> NodeHeartbeat {
    NodeHeartbeat {
        node_id: node_id.into(),
        node_type: "vm".into(),
        cpu_cores: 8,
        memory_total_mb: 65536,
        memory_free_mb,
        units_running: units,
        capabilities: vec!["standard".into(), "test".into()],
        timestamp_ms: 0,
    }
}

#[test]
fn test_register_and_query_node() {
    let mut registry = NodeRegistry::new();
    let hb = make_heartbeat("vm-01", 60000, 10);
    registry.update(hb);

    let node = registry.get("vm-01").unwrap();
    assert_eq!(node.memory_free_mb, 60000);
    assert_eq!(node.units_running, 10);
}

#[test]
fn test_find_best_fit_node() {
    let mut registry = NodeRegistry::new();
    registry.update(make_heartbeat("vm-01", 60000, 10));
    registry.update(make_heartbeat("vm-02", 30000, 50));
    registry.update(make_heartbeat("rpi-01", 3000, 2));

    // Want standard tier, need 100MB
    let best = registry.find_best_fit("standard", 100).unwrap();
    assert_eq!(best.node_id, "vm-01"); // most free memory
}

#[test]
fn test_find_best_fit_filters_by_capability() {
    let mut registry = NodeRegistry::new();
    registry.update(make_heartbeat("vm-01", 60000, 10)); // standard+test
    let mut hb = make_heartbeat("vm-02", 60000, 10);
    hb.capabilities = vec!["red-team".into()];
    registry.update(hb);

    // Want red-team — only vm-02 has it
    let best = registry.find_best_fit("red-team", 100).unwrap();
    assert_eq!(best.node_id, "vm-02");
}

#[test]
fn test_stale_nodes_evicted() {
    let mut registry = NodeRegistry::new();
    registry.update(make_heartbeat("vm-01", 60000, 10));

    // Simulate time passing
    std::thread::sleep(Duration::from_millis(10));
    registry.evict_stale(Duration::from_millis(5));

    assert!(registry.get("vm-01").is_none());
}