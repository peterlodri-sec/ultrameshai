use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use crate::registry::NodeRegistry;
use crate::discovery::TailscaleDiscovery;

/// Spawn background tasks for stale node detection
pub fn spawn_background_tasks(
    registry: Arc<Mutex<NodeRegistry>>,
    discovery: Arc<TailscaleDiscovery>,
    poll_interval_secs: u64,
) {
    // Spawn stale checker task
    tokio::spawn(async move {
        let mut int = interval(Duration::from_secs(poll_interval_secs));
        loop {
            int.tick().await;
            
            // Check stale nodes
            let stale_ids = {
                let reg = registry.lock().await;
                reg.check_stale_nodes()
            };
            
            if !stale_ids.is_empty() {
                tracing::info!("Found {} stale nodes: {:?}", stale_ids.len(), stale_ids);
                
                // Poll Tailscale API
                match discovery.get_online_device_ids().await {
                    Ok(online_ids) => {
                        // Mark nodes offline if not in Tailscale online set
                        for node_id in &stale_ids {
                            if !online_ids.contains(node_id) {
                                registry.lock().await.mark_offline(node_id);
                                tracing::info!("Marked node {} as offline", node_id);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to poll Tailscale API: {}", e);
                    }
                }
            }
        }
    });
}
