use loop_engineering_node_registry::heartbeat::{HeartbeatBroadcaster, HeartbeatListener};
use loop_engineering_node_registry::registry::NodeRegistry;
use loop_engineering_node_registry::proto::NodeHeartbeat;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;

fn make_heartbeat(node_id: &str) -> NodeHeartbeat {
    NodeHeartbeat {
        node_id: node_id.into(),
        node_type: "vm".into(),
        cpu_cores: 8,
        memory_total_mb: 65536,
        memory_free_mb: 60000,
        units_running: 0,
        capabilities: vec!["standard".into()],
        timestamp_ms: 0,
    }
}

#[tokio::test]
async fn test_broadcast_and_receive() {
    let multicast_addr = "239.0.0.1:9999";

    let registry = Arc::new(Mutex::new(NodeRegistry::new()));
    let listener = HeartbeatListener::new(multicast_addr, registry.clone());
    let listen_handle = tokio::spawn(async move {
        listener.listen().await.unwrap();
    });

    // Give listener time to join
    tokio::time::sleep(Duration::from_millis(100)).await;

    let broadcaster = HeartbeatBroadcaster::new(multicast_addr).await.unwrap();
    broadcaster.broadcast(&make_heartbeat("vm-01")).await.unwrap();

    // Give time for message to arrive
    tokio::time::sleep(Duration::from_millis(100)).await;

    let reg = registry.lock().await;
    let node = reg.get("vm-01").unwrap();
    assert_eq!(node.node_id, "vm-01");

    listen_handle.abort();
}