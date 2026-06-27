use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::types::{HeartbeatRequest, HealthResponse, NodeEntry, NodeMetadata};
use crate::registry::NodeRegistry;
use crate::crypto::{verify_signature, extract_signature};

/// Application state - Arc<Mutex> for thread-safe access
pub type AppState = Arc<Mutex<NodeRegistry>>;

/// POST /heartbeat - Register/update node
pub async fn heartbeat_handler(
    State(registry): State<AppState>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> StatusCode {
    // Get secret from env — fail fast if unset in production
    let secret = std::env::var("HEARTBEAT_SECRET")
        .expect("HEARTBEAT_SECRET must be set — no default fallback");

    // Verify signature
    let Some(signature) = extract_signature(&headers) else {
        return StatusCode::UNAUTHORIZED;
    };

    if !verify_signature(&body, &signature, secret.as_bytes()).unwrap_or(false) {
        return StatusCode::UNAUTHORIZED;
    }
    
    // Parse payload
    let Ok(req) = serde_json::from_slice::<HeartbeatRequest>(&body) else {
        return StatusCode::BAD_REQUEST;
    };

    // Validate bounds
    if req.validate().is_some() {
        return StatusCode::BAD_REQUEST;
    }
    
    // Create entry
    let entry = NodeEntry::new(NodeMetadata {
        node_id: req.node_id,
        capabilities: req.capabilities,
        memory_mb: req.memory_mb,
        load_avg: req.load_avg,
        region: req.region,
    });
    
    // Register
    registry.lock().await.register_node(entry);
    
    StatusCode::OK
}

/// GET /nodes - List all nodes
pub async fn list_nodes_handler(
    State(registry): State<AppState>,
) -> Json<Vec<NodeEntry>> {
    let nodes = registry.lock().await.get_all_nodes();
    Json(nodes)
}

/// GET /health - Health check
pub async fn health_handler(
    State(registry): State<AppState>,
) -> Json<HealthResponse> {
    let (total, online, offline) = registry.lock().await.get_node_counts();
    Json(HealthResponse {
        status: "healthy".into(),
        total_nodes: total,
        online_nodes: online,
        offline_nodes: offline,
        uptime_secs: registry.lock().await.uptime_secs(),
    })
}

/// Create the axum router
pub fn create_router(registry: AppState) -> Router {
    Router::new()
        .route("/heartbeat", post(heartbeat_handler))
        .route("/nodes", get(list_nodes_handler))
        .route("/health", get(health_handler))
        .with_state(registry)
}
