use std::sync::Arc;
use tokio::time::{interval, Duration};
use crate::registry::NodeRegistry;
use crate::discovery::TailscaleDiscovery;

/// Spawn background tasks for stale node detection
pub fn spawn_background_tasks(
    registry: Arc<NodeRegistry>,
    discovery: Arc<TailscaleDiscovery>,
    poll_interval_secs: u64,
) {
    // Spawn stale checker task
    tokio::spawn(async move {
        let mut int = interval(Duration::from_secs(poll_interval_secs));
        loop {
            int.tick().await;
            
            // Check stale nodes
            let stale_ids = registry.check_stale_nodes().await;
            
            if !stale_ids.is_empty() {
                tracing::info!("Found {} stale nodes: {:?}", stale_ids.len(), stale_ids);
                
                // Poll Tailscale API
                match discovery.get_online_device_ids().await {
                    Ok(online_ids) => {
                        // Mark nodes offline if not in Tailscale online set
                        for node_id in &stale_ids {
                            if !online_ids.contains(node_id) {
                                registry.mark_offline(node_id).await;
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
