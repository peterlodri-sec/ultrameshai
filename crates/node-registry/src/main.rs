use std::sync::Arc;
use loop_engineering_node_registry::{NodeRegistry, TailscaleDiscovery, create_router, spawn_background_tasks};

#[tokio::main]
async fn main() {
    // Init logging
    tracing_subscriber::fmt::init();
    
    // Get config from env
    let tailnet = std::env::var("NODE_REGISTRY_TAILNET")
        .unwrap_or_else(|_| "todolistsec.ts.net".into());
    let poll_interval = std::env::var("POLL_INTERVAL_SECS")
        .unwrap_or_else(|_| "60".into())
        .parse::<u64>()
        .unwrap_or(60);
    let stale_threshold = std::env::var("STALE_THRESHOLD_SECS")
        .unwrap_or_else(|_| "90".into())
        .parse::<u64>()
        .unwrap_or(90);
    
    // Create registry and discovery
    let registry = Arc::new(NodeRegistry::new(stale_threshold, 3));
    let discovery = Arc::new(TailscaleDiscovery::new(tailnet));
    
    // Spawn background tasks
    spawn_background_tasks(
        Arc::clone(&registry),
        Arc::clone(&discovery),
        poll_interval,
    );
    
    // Create router
    let app = create_router(Arc::clone(&registry));
    
    // Get bind address
    let addr = std::env::var("NODE_REGISTRY_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".into());
    
    tracing::info!("Starting node-registry on {}", addr);
    
    // Run server
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
