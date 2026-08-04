use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::signal;
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
    
    // Create registry wrapped in Arc<Mutex> for thread-safe access
    let registry = Arc::new(Mutex::new(NodeRegistry::new(stale_threshold)));
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
    
    // Run server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("node-registry listening on {}", addr);
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
    
    tracing::info!("node-registry shut down gracefully");
}

/// Wait for SIGTERM or SIGINT to trigger graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received, draining...");
}
