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
