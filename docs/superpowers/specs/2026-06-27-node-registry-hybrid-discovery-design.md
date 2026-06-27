# Node Registry — Hybrid Discovery Design (Push + Fallback Poll)

**Date:** 2026-06-27
**Status:** Approved
**Author:** Loop Engineering Team

---

## 1. System Identity & Goal

Node registry is the central node discovery service for the loop-engineering agent stack. It maintains a real-time view of all nodes in the Tailscale mesh network and exposes this view to the transport layer for connection routing.

**Problem:** UDP multicast discovery (current implementation) assumes local network broadcast capability. While Tailscale routes multicast, it's not ideal for distributed mesh networks where nodes may be on different subnets or have multicast blocked.

**Solution:** Hybrid discovery combining push-based announcements (primary) with Tailscale API polling (fallback). Nodes announce themselves via HTTP heartbeat; registry polls Tailscale API only when announcements fail.

### Success Criteria

1. Nodes announce via HTTP POST every 10s (configurable)
2. Registry detects stale nodes after 90s timeout
3. Fallback poll triggers after timeout (Tailscale API)
4. Node marked offline after 3 consecutive poll failures
5. Enriched metadata: capabilities, memory, load (not just Tailscale API data)
6. Transport layer can query `/api/v1/nodes` for current node list

### Scope Boundary

This design covers node-registry crate only. Node-side announcement client is a separate concern (covered in transport crate integration).

---

## 2. Architecture

```
+------------------+     +------------------+     +------------------+
|  Node A          |     |  Node Registry   |     |  Node B          |
|  (announcer)     |     |  (receiver)      |     |  (announcer)     |
+--------+---------+     +--------+---------+     +--------+---------+
         |                        |                        |
         | POST /heartbeat        | POST /heartbeat        |
         | every 10s              | every 10s              |
         +----------------------->+<-----------------------+
                                  |
                         +--------v--------+
                         |  In-memory      |
                         |  NodeRegistry   |
                         |  - last_seen    |
                         |  - metadata     |
                         +--------+--------+
                                  |
                    +-------------+-------------+
                    |                           |
         +----------v----------+      +---------v----------+
         | Background Task     |      | Tailscale API      |
         | Check stale (30s)   |      | (fallback poll)    |
         +----------+----------+      +--------------------+
                    |
         +----------v----------+
         | If stale > 90s:     |
         | Poll Tailscale API  |
         | 3 attempts → offline|
         +---------------------+
```

### Discovery Strategy

| Mode | Trigger | Action |
|------|---------|--------|
| **Push (primary)** | Node heartbeat every 10s | Update `last_seen`, refresh metadata |
| **Fallback poll** | `last_seen > 90s` | Query Tailscale API for node status |
| **Offline** | 3 consecutive poll failures | Mark node offline, remove from active list |

### Why Hybrid?

- **Push is efficient:** No polling overhead when nodes are healthy
- **Fallback is safe:** Detects announcement failures (network partition, node crash)
- **Tailscale API is source of truth:** Confirms if node is actually offline or just not announcing

---

## 3. Components

### 3.1 `announcement.rs` — HTTP Heartbeat Receiver

**Purpose:** Accept node announcements via HTTP POST.

**Endpoint:** `POST /api/v1/heartbeat`

**Request:**
```json
{
  "node_id": "nuremberg-hq",
  "tailscale_name": "nuremberg-hq.tailnet-scale.ts.net",
  "capabilities": ["deepwork", "coder", "tester"],
  "memory_total_mb": 65536,
  "memory_free_mb": 32768,
  "load_avg_1m": 0.42,
  "signature": "base64-encoded-hmac"
}
```

**Response:**
```json
{ "status": "ok" }
```

**Validation:**
- `node_id` must be non-empty
- `signature` verified against shared secret (HMAC-SHA256)
- Rate limit: 1 heartbeat per node per 5s (prevent spam)

**Implementation:**
```rust
use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};

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

pub fn announcement_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/heartbeat", post(handle_heartbeat))
}

async fn handle_heartbeat(
    State(state): State<AppState>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>> {
    // Verify signature
    if !state.verify_signature(&req.node_id, &req.signature) {
        return Err(RegistryError::InvalidSignature);
    }

    // Update registry
    let mut registry = state.registry.lock().await;
    registry.update_heartbeat(NodeMetadata {
        node_id: req.node_id,
        tailscale_name: req.tailscale_name,
        // ... populate from request
    });

    Ok(Json(HeartbeatResponse { status: "ok".into() }))
}
```

### 3.2 `discovery.rs` — Tailscale API Client (Fallback)

**Purpose:** Query Tailscale API when node announcements fail.

**API Endpoint:** `https://api.tailscale.com/api/v2/tailnet/{tailnet}/devices`

**Authentication:** Bearer token (API key from `TAILSCALE_API_KEY` env)

**Implementation:**
```rust
use reqwest::{Client, header::AUTHORIZATION};
use serde::Deserialize;

pub struct TailscaleDiscovery {
    client: Client,
    tailnet: String,
    api_key: String,
}

#[derive(Deserialize)]
pub struct TailscaleDevice {
    pub id: String,
    pub name: String,
    pub addresses: Vec<String>,
    pub isOnline: bool,
    pub lastSeen: Option<String>,
}

impl TailscaleDiscovery {
    pub fn new(api_key: String, tailnet: String) -> Self {
        Self {
            client: Client::new(),
            tailnet,
            api_key,
        }
    }

    pub async fn get_node(&self, node_name: &str) -> Result<Option<TailscaleDevice>> {
        let url = format!(
            "https://api.tailscale.com/api/v2/tailnet/{}/devices",
            self.tailnet
        );

        let response = self.client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(RegistryError::TailscaleApiError(response.status().to_string()));
        }

        let devices: Vec<TailscaleDevice> = response.json().await?;
        Ok(devices.into_iter()
            .find(|d| d.name == node_name || d.name.starts_with(node_name)))
    }

    pub async fn verify_nodes(&self, node_names: &[String]) -> Result<Vec<NodeStatus>> {
        // Batch verify multiple nodes
        // Return status for each: Online, Offline, Unknown
    }
}
```

### 3.3 `registry.rs` — Node Store with Timeout Tracking

**Purpose:** Maintain in-memory node registry with announcement tracking.

**Data Structure:**
```rust
use std::collections::HashMap;
use std::time::Instant;

pub struct NodeEntry {
    pub metadata: NodeMetadata,
    pub last_announcement: Instant,
    pub poll_failures: u32,
    pub status: NodeStatus,
}

pub struct NodeMetadata {
    pub node_id: String,
    pub tailscale_name: String,
    pub tailscale_ips: Vec<String>,
    pub capabilities: Vec<String>,
    pub memory_total_mb: u64,
    pub memory_free_mb: u64,
    pub load_avg_1m: f32,
}

#[derive(Clone, PartialEq)]
pub enum NodeStatus {
    Online,
    Offline,
    Unverified,  // Announcement stale, waiting for poll result
}

pub struct NodeRegistry {
    nodes: HashMap<String, NodeEntry>,
    announcement_timeout: Duration,  // 90s default
    max_poll_failures: u32,  // 3 default
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
            NodeEntry {
                metadata: metadata.clone(),
                last_announcement: Instant::now(),
                poll_failures: 0,
                status: NodeStatus::Online,
            }
        });

        entry.metadata = metadata;
        entry.last_announcement = Instant::now();
        entry.status = NodeStatus::Online;
        entry.poll_failures = 0;  // Reset on successful announcement
    }

    pub fn check_stale_nodes(&mut self) -> Vec<String> {
        let now = Instant::now();
        let mut stale_nodes = Vec::new();

        for (node_id, entry) in &self.nodes {
            if now.duration_since(entry.last_announcement) > self.announcement_timeout {
                stale_nodes.push(node_id.clone());
            }
        }

        stale_nodes
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
```

### 3.4 Background Task — Stale Node Checker

**Purpose:** Periodically check for stale nodes and trigger fallback poll.

**Implementation:**
```rust
use tokio::time::{interval, Duration};

pub async fn stale_node_checker_task(
    registry: Arc<Mutex<NodeRegistry>>,
    discovery: Arc<TailscaleDiscovery>,
    check_interval: Duration,  // 30s default
) {
    let mut interval = interval(check_interval);

    loop {
        interval.tick().await;

        // Find stale nodes
        let stale_nodes = {
            let reg = registry.lock().await;
            reg.check_stale_nodes()
        };

        // Poll Tailscale API for each stale node
        for node_id in stale_nodes {
            let status = discovery.get_node(&node_id).await;

            let mut reg = registry.lock().await;
            match status {
                Ok(Some(device)) if device.isOnline => {
                    // Node is online but not announcing
                    tracing::warn!("Node {} online but not announcing", node_id);
                    reg.mark_poll_failure(&node_id);  // Increment failure count
                }
                Ok(Some(_)) | Ok(None) => {
                    // Node is offline or not in tailnet
                    reg.mark_poll_failure(&node_id);
                }
                Err(_) => {
                    // API error
                    reg.mark_poll_failure(&node_id);
                }
            }
        }
    }
}
```

---

## 4. API Endpoints

### `POST /api/v1/heartbeat`

**Purpose:** Node announces presence and metadata.

**Request:**
```json
{
  "node_id": "nuremberg-hq",
  "tailscale_name": "nuremberg-hq.tailnet-scale.ts.net",
  "capabilities": ["deepwork", "coder"],
  "memory_total_mb": 65536,
  "memory_free_mb": 32768,
  "load_avg_1m": 0.42,
  "signature": "hmac-sha256-base64"
}
```

**Response:** `200 OK`
```json
{ "status": "ok" }
```

**Errors:**
- `401 Unauthorized` — Invalid signature
- `400 Bad Request` — Missing required fields

### `GET /api/v1/nodes`

**Purpose:** Transport layer queries available nodes.

**Response:** `200 OK`
```json
[
  {
    "node_id": "nuremberg-hq",
    "tailscale_name": "nuremberg-hq.tailnet-scale.ts.net",
    "tailscale_ips": ["100.64.0.1"],
    "capabilities": ["deepwork", "coder"],
    "memory_total_mb": 65536,
    "memory_free_mb": 32768,
    "load_avg_1m": 0.42,
    "status": "online"
  }
]
```

### `GET /api/v1/nodes/{node_id}`

**Purpose:** Get specific node details.

**Response:** `200 OK`
```json
{
  "node_id": "nuremberg-hq",
  "tailscale_name": "nuremberg-hq.tailnet-scale.ts.net",
  "status": "online",
  "last_announcement": "2026-06-27T10:30:00Z"
}
```

**Errors:**
- `404 Not Found` — Node not in registry

### `GET /api/v1/health`

**Purpose:** Health check for load balancer / systemd.

**Response:** `200 OK`
```json
{ "status": "ok", "nodes_online": 10 }
```

---

## 5. Configuration

### Environment Variables

```bash
# Required
TAILSCALE_API_KEY=tskey_...          # Tailscale API key
TAILSCALE_TAILNET=tailnet-scale.ts.net  # Tailnet name

# Optional (defaults)
ANNOUNCEMENT_TIMEOUT_SECS=90         # Timeout before fallback poll
POLL_CHECK_INTERVAL_SECS=30          # How often to check stale nodes
MAX_POLL_FAILURES=3                  # Failures before marking offline
HTTP_PORT=8080                       # Registry HTTP port
SHARED_SECRET=...                    # HMAC signature secret
```

### Config File (Optional)

```toml
# /opt/loop-engineering/config/registry.toml

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

---

## 6. Error Handling

### Signature Verification Failure

**Cause:** Node sends invalid or missing HMAC signature.

**Response:** `401 Unauthorized`

**Action:** Log warning, reject heartbeat.

### Tailscale API Rate Limit

**Cause:** Too many API requests (Tailscale rate limit: 1000 requests/hour per API key).

**Response:** Retry after `Retry-After` header duration.

**Action:** Exponential backoff (1s, 2s, 4s, 8s, 16s, max 5min).

### Tailscale API Network Error

**Cause:** Network partition, DNS failure, API downtime.

**Action:** Retry up to 3 times, then mark node offline.

### Node Announces After Being Marked Offline

**Cause:** Node recovers, resumes announcements.

**Action:** Accept heartbeat, mark online, reset poll failure count.

---

## 7. Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_update_heartbeat() {
        let mut registry = NodeRegistry::new(Duration::from_secs(90), 3);
        let metadata = NodeMetadata { /* ... */ };
        registry.update_heartbeat(metadata.clone());

        let entry = registry.nodes.get("test-node").unwrap();
        assert_eq!(entry.status, NodeStatus::Online);
        assert_eq!(entry.poll_failures, 0);
    }

    #[test]
    fn test_registry_stale_detection() {
        let mut registry = NodeRegistry::new(Duration::from_secs(1), 3);
        // Add node, wait 2s, check stale
        std::thread::sleep(Duration::from_secs(2));
        let stale = registry.check_stale_nodes();
        assert!(stale.contains(&"test-node".to_string()));
    }

    #[test]
    fn test_mark_poll_failure() {
        let mut registry = NodeRegistry::new(Duration::from_secs(90), 3);
        // Add node, mark 3 failures
        for _ in 0..3 {
            registry.mark_poll_failure("test-node");
        }
        let entry = registry.nodes.get("test-node").unwrap();
        assert_eq!(entry.status, NodeStatus::Offline);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_heartbeat_endpoint() {
    let app = create_test_app();
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

#[tokio::test]
async fn test_tailscale_api_discovery() {
    let discovery = TailscaleDiscovery::new(
        std::env::var("TAILSCALE_API_KEY").unwrap(),
        "tailnet-scale.ts.net".into(),
    );

    let node = discovery.get_node("nuremberg-hq").await.unwrap();
    assert!(node.is_some());
    assert!(node.unwrap().isOnline);
}
```

---

## 8. Deployment

### Systemd Service

```ini
# /etc/systemd/system/node-registry.service

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
ExecStart=/opt/loop-engineering/bin/node-registry
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

### Startup Sequence

1. Load environment variables
2. Initialize `NodeRegistry` with config
3. Initialize `TailscaleDiscovery` client
4. Start HTTP server (Axum)
5. Spawn background stale-checker task
6. Log "Node registry started"

---

## 9. Migration from UDP Multicast

### Current State

- `heartbeat.rs` — UDP multicast broadcaster/listener
- `registry.rs` — In-memory store (no timeout tracking)

### Migration Steps

1. **Add announcement endpoint** (`announcement.rs`)
2. **Extend registry** with timeout tracking (`last_announcement`, `poll_failures`)
3. **Add Tailscale API client** (`discovery.rs`)
4. **Add background stale-checker task**
5. **Deprecate UDP multicast** (keep for backward compat, log warning)
6. **Update node-side code** to send HTTP heartbeats

### Backward Compatibility

- UDP multicast listener remains active (optional feature flag)
- Nodes can use either UDP or HTTP announcement
- Registry accepts both, normalizes to same `NodeEntry`

---

## 10. Open Questions

1. **Shared secret distribution:** How do nodes get the HMAC signing secret?
   - **Proposal:** Distribute via Tailscale secrets management or environment variable on deployment

2. **Memory limits:** How many nodes can registry track before memory becomes an issue?
   - **Proposal:** Benchmark with 1000 nodes (expected: <10MB RAM)

3. **Persistence:** Should registry survive restarts?
   - **Proposal:** No — nodes re-announce on startup. Add optional SQLite cache for audit trail.

---

## 11. Next Steps

1. **Implementation plan** — Invoke `writing-plans` skill
2. **Node-side announcement client** — Add to transport crate
3. **Deploy to nuremberg-hq** — Replace UDP multicast with HTTP announcement
4. **Monitor** — Track announcement success rate, fallback poll frequency

---

## Appendix A: Tailscale API Reference

**Endpoint:** `GET /api/v2/tailnet/{tailnet}/devices`

**Response:**
```json
{
  "devices": [
    {
      "id": "abc123",
      "name": "nuremberg-hq",
      "addresses": ["100.64.0.1"],
      "isOnline": true,
      "lastSeen": "2026-06-27T10:30:00Z"
    }
  ]
}
```

**Rate Limits:** 1000 requests/hour per API key

**Auth:** `Authorization: Bearer tskey_...`

**Docs:** https://tailscale.com/api

(End of file)
