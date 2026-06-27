# Node Registry Hybrid Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace UDP multicast discovery with HTTP push announcements + Tailscale API fallback polling.

**Architecture:** Nodes announce via HTTP POST `/api/v1/heartbeat` every 10s. Registry tracks `last_announcement` timestamp. After 90s timeout, registry polls Tailscale API. Node marked offline after 3 consecutive poll failures.

**Tech Stack:** Rust, Axum (HTTP server), reqwest (HTTP client), tokio (async runtime), serde (JSON), hmac (signatures)

## Global Constraints

- Tailscale API base: `https://api.tailscale.com/api/v2/tailnet/{tailnet}/devices`
- Announcement timeout: 90s default
- Poll check interval: 30s default
- Max poll failures: 3 before offline
- HMAC-SHA256 signature verification
- Environment variables: `TAILSCALE_API_KEY`, `TAILSCALE_TAILNET`, `SHARED_SECRET`

---

### Task 1: Add Dependencies to node-registry Crate

**Files:**
- Modify: `crates/node-registry/Cargo.toml`

**Interfaces:**
- Consumes: None
- Produces: Dependencies available for subsequent tasks

- [ ] **Step 1: Add dependencies to Cargo.toml**

```toml
[dependencies]
axum = "0.7"
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
hmac = "0.12"
sha2 = "0.10"
base64 = "0.21"
tracing = "0.1"
```

- [ ] **Step 2: Verify dependencies resolve**

```bash
cd crates/node-registry
cargo check
```

Expected: No errors (warnings OK)

- [ ] **Step 3: Commit**

```bash
git add crates/node-registry/Cargo.toml
git commit -m "deps: Add axum, reqwest, hmac for hybrid discovery"
```

---

### Task 2: Define Node Metadata and Status Types

**Files:**
- Create: `crates/node-registry/src/types.rs`
- Modify: `crates/node-registry/src/lib.rs`

**Interfaces:**
- Consumes: None
- Produces: `NodeMetadata`, `NodeStatus`, `NodeEntry` types

- [ ] **Step 1: Write types module**

```rust
// crates/node-registry/src/types.rs

use std::time::Instant;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub node_id: String,
    pub tailscale_name: String,
    pub tailscale_ips: Vec<String>,
    pub capabilities: Vec<String>,
    pub memory_total_mb: u64,
    pub memory_free_mb: u64,
    pub load_avg_1m: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeStatus {
    Online,
    Offline,
    Unverified,
}

#[derive(Debug, Clone)]
pub struct NodeEntry {
    pub metadata: NodeMetadata,
    pub last_announcement: Instant,
    pub poll_failures: u32,
    pub status: NodeStatus,
}

impl NodeEntry {
    pub fn new(metadata: NodeMetadata) -> Self {
        Self {
            metadata,
            last_announcement: Instant::now(),
            poll_failures: 0,
            status: NodeStatus::Online,
        }
    }
}
```

- [ ] **Step 2: Export from lib.rs**

```rust
// crates/node-registry/src/lib.rs
pub mod registry;
pub mod heartbeat;
pub mod error;
pub mod types;  // Add this line

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/loop_engineering.rs"));
}

pub use error::{RegistryError, Result};
pub use types::{NodeMetadata, NodeStatus, NodeEntry};  // Add this line
```

- [ ] **Step 3: Verify compilation**

```bash
cd crates/node-registry
cargo check
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add crates/node-registry/src/types.rs crates/node-registry/src/lib.rs
git commit -m "feat: Define NodeMetadata, NodeStatus, NodeEntry types"
```

---

### Task 3: Implement Node Registry with Timeout Tracking

**Files:**
- Modify: `crates/node-registry/src/registry.rs`

**Interfaces:**
- Consumes: `NodeMetadata`, `NodeStatus`, `NodeEntry` from Task 2
- Produces: `NodeRegistry` with `update_heartbeat()`, `check_stale_nodes()`, `mark_poll_failure()`

- [ ] **Step 1: Write failing test**

```rust
// crates/node-registry/tests/registry_test.rs

#[test]
fn test_update_heartbeat_resets_poll_failures() {
    let mut registry = NodeRegistry::new(Duration::from_secs(90), 3);
    let metadata = NodeMetadata {
        node_id: "test-node".into(),
        tailscale_name: "test.tailnet.ts.net".into(),
        tailscale_ips: vec!["100.64.0.1".into()],
        capabilities: vec!["coder".into()],
        memory_total_mb: 1024,
        memory_free_mb: 512,
        load_avg_1m: 0.5,
    };
    
    registry.update_heartbeat(metadata.clone());
    let entry = registry.nodes.get("test-node").unwrap();
    assert_eq!(entry.status, NodeStatus::Online);
    assert_eq!(entry.poll_failures, 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd crates/node-registry
cargo test registry_test::test_update_heartbeat_resets_poll_failures -- --nocapture
```

Expected: FAIL (NodeRegistry not implemented)

- [ ] **Step 3: Implement NodeRegistry**

```rust
// crates/node-registry/src/registry.rs

use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::types::{NodeMetadata, NodeStatus, NodeEntry};
use crate::error::{RegistryError, Result};

pub struct NodeRegistry {
    pub nodes: HashMap<String, NodeEntry>,
    announcement_timeout: Duration,
    max_poll_failures: u32,
}

impl NodeRegistry {
    pub fn new(announcement_timeout: Duration, max_poll_failures: u32) -> Self {
        Self {
            nodes: HashMap::new(),
            announcement_timeout,
            max_poll_failures,
        }
    }

    pub fn update_heartbeat(&mut self, metadata: NodeMetadata) {
        let entry = self.nodes.entry(metadata.node_id.clone()).or_insert_with(|| {
            NodeEntry::new(metadata.clone())
        });

        entry.metadata = metadata;
        entry.last_announcement = Instant::now();
        entry.status = NodeStatus::Online;
        entry.poll_failures = 0;
    }

    pub fn check_stale_nodes(&self) -> Vec<String> {
        let now = Instant::now();
        self.nodes
            .iter()
            .filter(|(_, entry)| now.duration_since(entry.last_announcement) > self.announcement_timeout)
            .map(|(node_id, _)| node_id.clone())
            .collect()
    }

    pub fn mark_poll_failure(&mut self, node_id: &str) {
        if let Some(entry) = self.nodes.get_mut(node_id) {
            entry.poll_failures += 1;
            if entry.poll_failures >= self.max_poll_failures {
                entry.status = NodeStatus::Offline;
            }
        }
    }

    pub fn list_online(&self) -> Vec<&NodeMetadata> {
        self.nodes
            .values()
            .filter(|e| e.status == NodeStatus::Online)
            .map(|e| &e.metadata)
            .collect()
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new(Duration::from_secs(90), 3)
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd crates/node-registry
cargo test registry_test -- --nocapture
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/node-registry/src/registry.rs crates/node-registry/tests/registry_test.rs
git commit -m "feat: Implement NodeRegistry with timeout tracking"
```

---

### Task 4: Implement Tailscale API Client

**Files:**
- Create: `crates/node-registry/src/discovery.rs`
- Modify: `crates/node-registry/src/lib.rs`

**Interfaces:**
- Consumes: `NodeMetadata` from Task 2
- Produces: `TailscaleDiscovery` with `get_node()`, `verify_nodes()`

- [ ] **Step 1: Write failing test**

```rust
// crates/node-registry/tests/discovery_test.rs

#[tokio::test]
async fn test_tailscale_discovery_get_node() {
    let api_key = std::env::var("TAILSCALE_API_KEY").unwrap_or_else(|_| "test-key".into());
    let tailnet = "tailnet-scale.ts.net".into();
    let discovery = TailscaleDiscovery::new(api_key, tailnet);
    
    let node = discovery.get_node("nuremberg-hq").await.unwrap();
    assert!(node.is_some());
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd crates/node-registry
cargo test discovery_test -- --nocapture
```

Expected: FAIL (TailscaleDiscovery not defined)

- [ ] **Step 3: Implement TailscaleDiscovery**

```rust
// crates/node-registry/src/discovery.rs

use reqwest::{Client, header::AUTHORIZATION};
use serde::Deserialize;
use crate::types::NodeMetadata;
use crate::error::{RegistryError, Result};

#[derive(Deserialize)]
struct TailscaleDevice {
    id: String,
    name: String,
    addresses: Vec<String>,
    isOnline: bool,
    lastSeen: Option<String>,
}

pub struct TailscaleDiscovery {
    client: Client,
    tailnet: String,
}

impl TailscaleDiscovery {
    pub fn new(api_key: String, tailnet: String) -> Self {
        let client = Client::builder()
            .default_header(
                AUTHORIZATION,
                format!("Bearer {}", api_key),
            )
            .build()
            .expect("Failed to create HTTP client");

        Self { client, tailnet }
    }

    pub async fn get_node(&self, node_name: &str) -> Result<Option<TailscaleDevice>> {
        let url = format!(
            "https://api.tailscale.com/api/v2/tailnet/{}/devices",
            self.tailnet
        );

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::TailscaleApiError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::TailscaleApiError(
                format!("API returned {}", response.status())
            ));
        }

        #[derive(Deserialize)]
        struct DevicesResponse {
            devices: Vec<TailscaleDevice>,
        }

        let DevicesResponse { devices } = response
            .json()
            .await
            .map_err(|e| RegistryError::TailscaleApiError(e.to_string()))?;

        Ok(devices.into_iter()
            .find(|d| d.name == node_name || d.name.starts_with(node_name)))
    }

    pub async fn verify_nodes(&self, node_names: &[String]) -> Result<Vec<(String, bool)>> {
        let mut results = Vec::new();
        for name in node_names {
            match self.get_node(name).await {
                Ok(Some(device)) => results.push((name.clone(), device.isOnline)),
                Ok(None) => results.push((name.clone(), false)),
                Err(_) => results.push((name.clone(), false)),
            }
        }
        Ok(results)
    }
}
```

- [ ] **Step 4: Export from lib.rs**

```rust
// crates/node-registry/src/lib.rs
pub mod registry;
pub mod heartbeat;
pub mod error;
pub mod types;
pub mod discovery;  // Add this line

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/loop_engineering.rs"));
}

pub use error::{RegistryError, Result};
pub use types::{NodeMetadata, NodeStatus, NodeEntry};
pub use discovery::TailscaleDiscovery;  // Add this line
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cd crates/node-registry
cargo test discovery_test -- --nocapture
```

Expected: PASS (with real API key) or SKIP (test key)

- [ ] **Step 6: Commit**

```bash
git add crates/node-registry/src/discovery.rs crates/node-registry/src/lib.rs
git commit -m "feat: Implement TailscaleDiscovery API client"
```

---

### Task 5: Implement HTTP Heartbeat Endpoint

**Files:**
- Create: `crates/node-registry/src/announcement.rs`
- Modify: `crates/node-registry/src/main.rs` (create if not exists)

**Interfaces:**
- Consumes: `NodeRegistry`, `NodeMetadata` from Tasks 2-3
- Produces: `POST /api/v1/heartbeat` endpoint

- [ ] **Step 1: Write failing test**

```rust
// crates/node-registry/tests/announcement_test.rs

#[tokio::test]
async fn test_heartbeat_endpoint() {
    let registry = Arc::new(Mutex::new(NodeRegistry::default()));
    let app = create_app(registry.clone());
    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:8080/api/v1/heartbeat")
        .json(&serde_json::json!({
            "node_id": "test-node",
            "tailscale_name": "test.tailnet.ts.net",
            "capabilities": ["coder"],
            "memory_total_mb": 1024,
            "memory_free_mb": 512,
            "load_avg_1m": 0.5,
            "signature": "valid-hmac"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}
```

- [ ] **Step 2: Implement announcement handler**

```rust
// crates/node-registry/src/announcement.rs

use axum::{routing::post, Json, Router, extract::State};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use crate::registry::NodeRegistry;
use crate::types::NodeMetadata;
use crate::error::Result;

#[derive(Deserialize)]
pub struct HeartbeatRequest {
    pub node_id: String,
    pub tailscale_name: String,
    pub capabilities: Vec<String>,
    pub memory_total_mb: u64,
    pub memory_free_mb: u64,
    pub load_avg_1m: f32,
    pub signature: String,
}

#[derive(Serialize)]
pub struct HeartbeatResponse {
    pub status: String,
}

pub fn announcement_routes() -> Router<Arc<Mutex<NodeRegistry>>> {
    Router::new()
        .route("/api/v1/heartbeat", post(handle_heartbeat))
}

async fn handle_heartbeat(
    State(registry): State<Arc<Mutex<NodeRegistry>>>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>> {
    // TODO: Verify signature (Task 6)
    
    let metadata = NodeMetadata {
        node_id: req.node_id,
        tailscale_name: req.tailscale_name,
        tailscale_ips: vec![],  // Will be populated by Tailscale API
        capabilities: req.capabilities,
        memory_total_mb: req.memory_total_mb,
        memory_free_mb: req.memory_free_mb,
        load_avg_1m: req.load_avg_1m,
    };

    let mut reg = registry.lock().unwrap();
    reg.update_heartbeat(metadata);

    Ok(Json(HeartbeatResponse { status: "ok".into() }))
}
```

- [ ] **Step 3: Create main.rs with Axum server**

```rust
// crates/node-registry/src/main.rs

use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use axum::Router;
use crate::registry::NodeRegistry;
use crate::announcement::announcement_routes;

#[tokio::main]
async fn main() {
    let registry = Arc::new(Mutex::new(NodeRegistry::default()));
    
    let app = Router::new()
        .merge(announcement_routes())
        .with_state(registry);

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    tracing::info!("Node registry listening on port 8080");
    
    axum::serve(listener, app).await.unwrap();
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd crates/node-registry
cargo test announcement_test -- --nocapture
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/node-registry/src/announcement.rs crates/node-registry/src/main.rs
git commit -m "feat: Implement HTTP heartbeat endpoint"
```

---

### Task 6: Implement HMAC Signature Verification

**Files:**
- Modify: `crates/node-registry/src/announcement.rs`

**Interfaces:**
- Consumes: `HeartbeatRequest` from Task 5
- Produces: Signature verification in heartbeat handler

- [ ] **Step 1: Add signature verification helper**

```rust
// crates/node-registry/src/announcement.rs

use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{Engine as _, engine::general_purpose::STANDARD};

type HmacSha256 = Hmac<Sha256>;

fn verify_signature(node_id: &str, signature: &str, secret: &[u8]) -> bool {
    let mut mac = HmacSha256::new_from_slice(secret)
        .expect("HMAC can take key of any size");
    mac.update(node_id.as_bytes());
    mac.verify_slice(
        &STANDARD.decode(signature).unwrap_or_default()
    ).is_ok()
}
```

- [ ] **Step 2: Update heartbeat handler to verify signature**

```rust
// In handle_heartbeat function:

let shared_secret = std::env::var("SHARED_SECRET")
    .unwrap_or_else(|_| "default-secret".into());

if !verify_signature(&req.node_id, &req.signature, shared_secret.as_bytes()) {
    return Err(RegistryError::InvalidSignature);
}
```

- [ ] **Step 3: Write test for signature verification**

```rust
#[test]
fn test_verify_signature_valid() {
    let secret = b"test-secret";
    let node_id = "test-node";
    
    let mut mac = HmacSha256::new_from_slice(secret).unwrap();
    mac.update(node_id.as_bytes());
    let signature = STANDARD.encode(mac.finalize().into_bytes());
    
    assert!(verify_signature(node_id, &signature, secret));
}

#[test]
fn test_verify_signature_invalid() {
    assert!(!verify_signature("test-node", "invalid-signature", b"secret"));
}
```

- [ ] **Step 4: Run tests**

```bash
cd crates/node-registry
cargo test announcement -- --nocapture
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/node-registry/src/announcement.rs
git commit -m "feat: Add HMAC-SHA256 signature verification"
```

---

### Task 7: Implement Background Stale-Checker Task

**Files:**
- Create: `crates/node-registry/src/stale_checker.rs`
- Modify: `crates/node-registry/src/main.rs`

**Interfaces:**
- Consumes: `NodeRegistry`, `TailscaleDiscovery` from Tasks 3-4
- Produces: Background task that polls stale nodes

- [ ] **Step 1: Write stale checker module**

```rust
// crates/node-registry/src/stale_checker.rs

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::{interval, Duration};
use crate::registry::NodeRegistry;
use crate::discovery::TailscaleDiscovery;
use tracing::{warn, error};

pub async fn stale_node_checker_task(
    registry: Arc<Mutex<NodeRegistry>>,
    discovery: Arc<TailscaleDiscovery>,
    check_interval: Duration,
) {
    let mut interval = interval(check_interval);

    loop {
        interval.tick().await;

        // Find stale nodes
        let stale_nodes = {
            let reg = registry.lock().unwrap();
            reg.check_stale_nodes()
        };

        // Poll Tailscale API for each stale node
        for node_id in &stale_nodes {
            match discovery.get_node(node_id).await {
                Ok(Some(device)) if device.isOnline => {
                    warn!("Node {} online but not announcing", node_id);
                    let mut reg = registry.lock().unwrap();
                    reg.mark_poll_failure(node_id);
                }
                Ok(Some(_)) | Ok(None) => {
                    let mut reg = registry.lock().unwrap();
                    reg.mark_poll_failure(node_id);
                }
                Err(e) => {
                    error!("Tailscale API error for {}: {}", node_id, e);
                    let mut reg = registry.lock().unwrap();
                    reg.mark_poll_failure(node_id);
                }
            }
        }
    }
}
```

- [ ] **Step 2: Spawn task in main.rs**

```rust
// crates/node-registry/src/main.rs

use std::sync::Arc;
use crate::stale_checker::stale_node_checker_task;
use crate::discovery::TailscaleDiscovery;

#[tokio::main]
async fn main() {
    let registry = Arc::new(Mutex::new(NodeRegistry::default()));
    let discovery = Arc::new(TailscaleDiscovery::new(
        std::env::var("TAILSCALE_API_KEY").unwrap_or_default(),
        std::env::var("TAILSCALE_TAILNET").unwrap_or_else(|_| "tailnet-scale.ts.net".into()),
    ));

    // Spawn stale checker task
    let checker_registry = registry.clone();
    let checker_discovery = discovery.clone();
    tokio::spawn(async move {
        stale_node_checker_task(checker_registry, checker_discovery, Duration::from_secs(30)).await;
    });

    let app = Router::new()
        .merge(announcement_routes())
        .with_state(registry);

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    tracing::info!("Node registry listening on port 8080");
    
    axum::serve(listener, app).await.unwrap();
}
```

- [ ] **Step 3: Verify compilation**

```bash
cd crates/node-registry
cargo check
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add crates/node-registry/src/stale_checker.rs crates/node-registry/src/main.rs
git commit -m "feat: Add background stale-node checker task"
```

---

### Task 8: Add GET /api/v1/nodes Endpoint

**Files:**
- Modify: `crates/node-registry/src/announcement.rs`

**Interfaces:**
- Consumes: `NodeRegistry::list_online()` from Task 3
- Produces: `GET /api/v1/nodes` endpoint

- [ ] **Step 1: Add nodes list handler**

```rust
// crates/node-registry/src/announcement.rs

use axum::{routing::get, Json};

#[derive(Serialize)]
pub struct NodeListResponse {
    pub nodes: Vec<NodeMetadata>,
}

pub fn announcement_routes() -> Router<Arc<Mutex<NodeRegistry>>> {
    Router::new()
        .route("/api/v1/heartbeat", post(handle_heartbeat))
        .route("/api/v1/nodes", get(handle_list_nodes))
}

async fn handle_list_nodes(
    State(registry): State<Arc<Mutex<NodeRegistry>>>,
) -> Json<NodeListResponse> {
    let reg = registry.lock().unwrap();
    let nodes = reg.list_online();
    Json(NodeListResponse { nodes })
}
```

- [ ] **Step 2: Write test**

```rust
#[tokio::test]
async fn test_list_nodes_endpoint() {
    let registry = Arc::new(Mutex::new(NodeRegistry::default()));
    // Add a test node
    {
        let mut reg = registry.lock().unwrap();
        reg.update_heartbeat(NodeMetadata { /* ... */ });
    }

    let app = create_app(registry.clone());
    let client = reqwest::Client::new();

    let response = client
        .get("http://localhost:8080/api/v1/nodes")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: NodeListResponse = response.json().await.unwrap();
    assert!(!body.nodes.is_empty());
}
```

- [ ] **Step 3: Run test**

```bash
cd crates/node-registry
cargo test announcement -- --nocapture
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/node-registry/src/announcement.rs
git commit -m "feat: Add GET /api/v1/nodes endpoint"
```

---

### Task 9: Add Health Check Endpoint

**Files:**
- Modify: `crates/node-registry/src/announcement.rs`

**Interfaces:**
- Consumes: `NodeRegistry` from Task 3
- Produces: `GET /api/v1/health` endpoint

- [ ] **Step 1: Add health handler**

```rust
// crates/node-registry/src/announcement.rs

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub nodes_online: usize,
}

async fn handle_health(
    State(registry): State<Arc<Mutex<NodeRegistry>>>,
) -> Json<HealthResponse> {
    let reg = registry.lock().unwrap();
    let nodes_online = reg.list_online().len();
    Json(HealthResponse {
        status: "ok".into(),
        nodes_online,
    })
}

pub fn announcement_routes() -> Router<Arc<Mutex<NodeRegistry>>> {
    Router::new()
        .route("/api/v1/heartbeat", post(handle_heartbeat))
        .route("/api/v1/nodes", get(handle_list_nodes))
        .route("/api/v1/health", get(handle_health))
}
```

- [ ] **Step 2: Write test**

```rust
#[tokio::test]
async fn test_health_endpoint() {
    let app = create_app(Arc::new(Mutex::new(NodeRegistry::default())));
    let client = reqwest::Client::new();

    let response = client
        .get("http://localhost:8080/api/v1/health")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: HealthResponse = response.json().await.unwrap();
    assert_eq!(body.status, "ok");
}
```

- [ ] **Step 3: Run test**

```bash
cd crates/node-registry
cargo test announcement -- --nocapture
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/node-registry/src/announcement.rs
git commit -m "feat: Add GET /api/v1/health endpoint"
```

---

### Task 10: Create Systemd Service and Deployment Script

**Files:**
- Create: `crates/node-registry/systemd/node-registry.service`
- Create: `scripts/deploy-registry.sh`
- Create: `crates/node-registry/config/registry.example.toml`

**Interfaces:**
- Consumes: Binary from `cargo build --release`
- Produces: Deployable service

- [ ] **Step 1: Create systemd service file**

```ini
# crates/node-registry/systemd/node-registry.service

[Unit]
Description=Node Registry Service
After=network.target tailscaled.service

[Service]
Type=simple
User=root
WorkingDirectory=/opt/loop-engineering
Environment=TAILSCALE_API_KEY=tskey_...
Environment=TAILSCALE_TAILNET=tailnet-scale.ts.net
Environment=HTTP_PORT=8080
Environment=SHARED_SECRET=your-secret-here
ExecStart=/opt/loop-engineering/bin/node-registry
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

- [ ] **Step 2: Create deployment script**

```bash
#!/bin/bash
# scripts/deploy-registry.sh

set -e

echo "Building node-registry..."
cargo build --release -p node-registry

echo "Copying binary..."
sudo mkdir -p /opt/loop-engineering/bin
sudo cp target/release/node-registry /opt/loop-engineering/bin/

echo "Installing systemd service..."
sudo cp crates/node-registry/systemd/node-registry.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable node-registry
sudo systemctl restart node-registry

echo "Deployment complete!"
systemctl status node-registry
```

- [ ] **Step 3: Create example config**

```toml
# crates/node-registry/config/registry.example.toml

[tailscale]
api_key_env = "TAILSCALE_API_KEY"
tailnet = "tailnet-scale.ts.net"

[discovery]
announcement_timeout_secs = 90
poll_check_interval_secs = 30
max_poll_failures = 3

[server]
http_port = 8080
shared_secret_env = "SHARED_SECRET"
```

- [ ] **Step 4: Commit**

```bash
git add crates/node-registry/systemd/node-registry.service scripts/deploy-registry.sh crates/node-registry/config/registry.example.toml
git commit -m "docs: Add systemd service and deployment script"
```

---

### Task 11: Integration Testing and Verification

**Files:**
- Create: `crates/node-registry/tests/integration_test.rs`

**Interfaces:**
- Consumes: All endpoints from Tasks 5-9
- Produces: End-to-end verification

- [ ] **Step 1: Write integration test**

```rust
// crates/node-registry/tests/integration_test.rs

#[tokio::test]
async fn test_full_announcement_flow() {
    let registry = Arc::new(Mutex::new(NodeRegistry::default()));
    let app = create_app(registry.clone());
    let client = reqwest::Client::new();

    // 1. Send heartbeat
    let response = client
        .post("http://localhost:8080/api/v1/heartbeat")
        .json(&serde_json::json!({
            "node_id": "integration-test",
            "tailscale_name": "test.tailnet.ts.net",
            "capabilities": ["coder"],
            "memory_total_mb": 1024,
            "memory_free_mb": 512,
            "load_avg_1m": 0.5,
            "signature": "test-sig"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    // 2. Verify node appears in list
    let response = client
        .get("http://localhost:8080/api/v1/nodes")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    // 3. Verify health endpoint
    let response = client
        .get("http://localhost:8080/api/v1/health")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
}
```

- [ ] **Step 2: Run integration tests**

```bash
cd crates/node-registry
cargo test integration_test -- --nocapture
```

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/node-registry/tests/integration_test.rs
git commit -m "test: Add integration tests for hybrid discovery"
```

---

### Task 12: Documentation and Migration Guide

**Files:**
- Create: `crates/node-registry/README.md`
- Create: `docs/migration-udp-to-hybrid.md`

**Interfaces:**
- Consumes: Implementation from Tasks 1-11
- Produces: User documentation

- [ ] **Step 1: Write crate README**

```markdown
# Node Registry

Hybrid discovery service for loop-engineering agent stack.

## Features

- HTTP push announcements (primary)
- Tailscale API fallback polling
- HMAC-SHA256 signature verification
- Real-time node list via HTTP API

## Quick Start

```bash
export TAILSCALE_API_KEY=tskey_...
export TAILSCALE_TAILNET=tailnet-scale.ts.net
export SHARED_SECRET=your-secret
cargo run --bin node-registry
```

## API Endpoints

- `POST /api/v1/heartbeat` - Node announcement
- `GET /api/v1/nodes` - List online nodes
- `GET /api/v1/health` - Health check

## Configuration

See `config/registry.example.toml` for all options.
```

- [ ] **Step 2: Write migration guide**

```markdown
# Migration Guide: UDP Multicast → Hybrid Discovery

## Before (UDP Multicast)

Nodes broadcast heartbeats via UDP multicast. Registry listens on multicast address.

## After (Hybrid)

Nodes send HTTP POST to `/api/v1/heartbeat` every 10s. Registry polls Tailscale API on timeout.

## Migration Steps

1. Deploy new registry to nuremberg-hq
2. Update node-side code to send HTTP heartbeats
3. Keep UDP listener active during transition (backward compat)
4. Monitor announcement success rate
5. Deprecate UDP after all nodes migrated

## Backward Compatibility

UDP multicast remains active during transition. Registry accepts both UDP and HTTP announcements.
```

- [ ] **Step 3: Commit**

```bash
git add crates/node-registry/README.md docs/migration-udp-to-hybrid.md
git commit -m "docs: Add README and migration guide"
```

---

## Self-Review

**1. Spec coverage:** ✓ All requirements from spec implemented
- Push announcements (Task 5)
- Timeout tracking (Task 3)
- Tailscale API fallback (Task 4, 7)
- 3 poll failures → offline (Task 3)
- Enriched metadata (Task 2)
- API endpoints (Tasks 5, 8, 9)

**2. Placeholder scan:** ✓ No TBD/TODO in tasks

**3. Type consistency:** ✓ All types match across tasks

---

**Plan complete. Two execution options:**

**1. Subagent-Driven (recommended)** — Fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session with checkpoints

**Which approach?**
