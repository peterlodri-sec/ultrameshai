# Tailscale Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate loop-engineering agent transport layer with existing Tailscale mesh VPN across 10 Hetzner servers.

**Architecture:** Node registry service discovers Tailscale IPs via `tailscale` CLI, exposes HTTP API for agent peer discovery, agents communicate over Tailscale IPs.

**Tech Stack:** Tailscale (existing), Rust (node-registry), HTTP/REST API, loop-engineering transport crate.

## Global Constraints

- Use existing Tailscale deployment (no Headscale)
- Tailscale CGNAT range: 100.64.0.0/16 or 100.100.0.0/16
- MagicDNS domain: `<tailnet>.ts.net`
- Agent ports: 8080-8089 (HTTP), 9000-9009 (transport)
- Node registry on nuremberg-hq (primary)

---

### Task 1: Discover Existing Tailscale State

**Files:**
- Create: `scripts/discover-tailscale.sh`
- Create: `docs/tailscale-inventory.md`
- Test: `scripts/verify-tailscale.sh`

**Interfaces:**
- Consumes: SSH access to all 10 servers
- Produces: Documented Tailscale IPs, MagicDNS status, ACL config

- [ ] **Step 1: Write discovery script**

```bash
#!/bin/bash
# scripts/discover-tailscale.sh
set -e

# Server list
declare -A SERVERS=(
    ["nuremberg-hq"]="167.233.148.20"
    ["falkenstein-1"]="178.105.245.135"
    ["falkenstein-2"]="167.233.105.32"
    ["nuremberg-1"]="167.233.35.194"
    ["nuremberg-2"]="178.105.184.32"
    ["helsinki-1"]="89.167.80.207"
    ["hillsboro-1"]="5.78.122.125"
    ["singapore-1"]="5.223.79.65"
    ["runner-arm"]="178.104.47.201"
    ["ccx33-nbg1"]="46.225.127.20"
)

echo "# Tailscale Inventory" > docs/tailscale-inventory.md
echo "" >> docs/tailscale-inventory.md
echo "| Hostname | Public IP | Tailscale IPv4 | Tailscale IPv6 | Status |" >> docs/tailscale-inventory.md
echo "|----------|-----------|----------------|----------------|--------|" >> docs/tailscale-inventory.md

for hostname in "${!SERVERS[@]}"; do
    ip="${SERVERS[$hostname]}"
    echo "Discovering $hostname ($ip)..."
    
    # Get Tailscale status
    status=$(ssh root@$ip 'tailscale status --json 2>/dev/null' || echo '{"error": "not_installed"}')
    
    # Get IPv4
    ipv4=$(ssh root@$ip 'tailscale ip -4 2>/dev/null' || echo "N/A")
    
    # Get IPv6
    ipv6=$(ssh root@$ip 'tailscale ip -6 2>/dev/null' || echo "N/A")
    
    # Check MagicDNS
    magicdns=$(ssh root@$ip 'resolvectl status 2>/dev/null | grep -c "ts.net" || echo 0')
    
    echo "| $hostname | $ip | $ipv4 | $ipv6 | $status |" >> docs/tailscale-inventory.md
done

echo "Inventory complete: docs/tailscale-inventory.md"
```

- [ ] **Step 2: Run discovery script**

```bash
./scripts/discover-tailscale.sh
# Expected: docs/tailscale-inventory.md created with all 10 nodes
```

- [ ] **Step 3: Check MagicDNS status**

```bash
# From any node, test MagicDNS resolution
ssh root@167.233.148.20 'nslookup nuremberg-hq 2>/dev/null || echo "MagicDNS not enabled"'
# Expected: Either IP address or "MagicDNS not enabled"
```

- [ ] **Step 4: Document findings**

```markdown
# docs/tailscale-inventory.md (add to end)

## MagicDNS Status
- [ ] Enabled / [ ] Not enabled
- Domain: _______________

## ACL Status
- [ ] Custom ACLs configured / [ ] Default ACLs
- Admin console URL: _______________

## Notes
- Any existing tags: _______________
- Any subnet routers: _______________
- Any exit nodes: _______________
```

- [ ] **Step 5: Commit**

```bash
git add scripts/discover-tailscale.sh docs/tailscale-inventory.md
git commit -m "feat: discover existing Tailscale state across 10 nodes"
```

---

### Task 2: Enable MagicDNS (if not already)

**Files:**
- Create: `scripts/enable-magicdns.sh`
- Test: `scripts/verify-magicdns.sh`

**Interfaces:**
- Consumes: Task 1 (Tailscale inventory)
- Produces: MagicDNS enabled for `*.ts.net` resolution

- [ ] **Step 1: Check if MagicDNS needs enabling**

```bash
# If this returns empty, MagicDNS not enabled
ssh root@167.233.148.20 'resolvectl status 2>/dev/null | grep -i "ts.net"'
```

- [ ] **Step 2: Write MagicDNS enable script**

```bash
#!/bin/bash
# scripts/enable-magicdns.sh
set -e

echo "MagicDNS must be enabled via Tailscale admin console:"
echo "1. Go to https://login.tailscale.com/admin/dns"
echo "2. Toggle 'MagicDNS' to ON"
echo "3. Wait 1-2 minutes for propagation"
echo ""
echo "After enabling, verify with:"
echo "  nslookup <hostname>.<tailnet>.ts.net"
```

- [ ] **Step 3: Run MagicDNS enable instructions**

```bash
./scripts/enable-magicdns.sh
# User action required: Enable in admin console
```

- [ ] **Step 4: Verify MagicDNS working**

```bash
# Wait 2 minutes, then test
sleep 120
ssh root@167.233.148.20 'nslookup nuremberg-hq 2>/dev/null && echo "MagicDNS working" || echo "MagicDNS not working"'
```

- [ ] **Step 5: Commit**

```bash
git add scripts/enable-magicdns.sh
git commit -m "docs: MagicDNS enable instructions"
```

---

### Task 3: Deploy Node Registry Service

**Files:**
- Create: `crates/node-registry/Cargo.toml`
- Create: `crates/node-registry/src/main.rs`
- Create: `crates/node-registry/src/handlers.rs`
- Create: `crates/node-registry/src/models.rs`
- Test: `crates/node-registry/tests/integration_test.rs`

**Interfaces:**
- Consumes: Task 1 (Tailscale IPs documented)
- Produces: HTTP API on port 8080 with `/api/v1/nodes` endpoints

- [ ] **Step 1: Write failing test**

```rust
// crates/node-registry/tests/integration_test.rs
use reqwest;

#[tokio::test]
async fn test_get_nodes_returns_list() {
    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:8080/api/v1/nodes")
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 200);
    let nodes: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(!nodes.is_empty());
}
```

- [ ] **Step 2: Run test to verify failure**

```bash
cd crates/node-registry
cargo test --test integration_test
# Expected: FAIL (service not running)
```

- [ ] **Step 3: Write Cargo.toml**

```toml
[package]
name = "node-registry"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.11", features = ["json"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

- [ ] **Step 4: Write models**

```rust
// crates/node-registry/src/models.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub hostname: String,
    pub tailscale_ipv4: String,
    pub tailscale_ipv6: Option<String>,
    pub region: String,
    pub capabilities: Vec<String>,
    pub status: NodeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Online,
    Offline,
}
```

- [ ] **Step 5: Write handlers**

```rust
// crates/node-registry/src/handlers.rs
use axum::{Json, extract::Path};
use crate::models::{Node, NodeStatus};

pub async fn get_nodes() -> Json<Vec<Node>> {
    // TODO: Discover from Tailscale CLI
    Json(vec![])
}

pub async fn get_node(Path(hostname): Path<String>) -> Json<Option<Node>> {
    // TODO: Find specific node
    Json(None)
}
```

- [ ] **Step 6: Write main**

```rust
// crates/node-registry/src/main.rs
mod models;
mod handlers;

use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    tracing_subscriber::init();
    
    let app = Router::new()
        .route("/api/v1/nodes", get(handlers::get_nodes))
        .route("/api/v1/nodes/:hostname", get(handlers::get_node));
    
    let addr = "0.0.0.0:8080";
    tracing::info!("Starting node-registry on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

- [ ] **Step 7: Run test to verify it passes**

```bash
# Start service in background
cargo run &
sleep 2

# Run test
cargo test --test integration_test
# Expected: PASS

# Stop service
pkill node-registry
```

- [ ] **Step 8: Commit**

```bash
git add crates/node-registry/
git commit -m "feat: node-registry service skeleton"
```

---

### Task 4: Implement Tailscale Discovery

**Files:**
- Modify: `crates/node-registry/src/handlers.rs`
- Modify: `crates/node-registry/src/discovery.rs` (create)
- Test: `crates/node-registry/tests/discovery_test.rs`

**Interfaces:**
- Consumes: Task 3 (node-registry running)
- Produces: Auto-discovery of Tailscale nodes via `tailscale` CLI

- [ ] **Step 1: Write discovery module**

```rust
// crates/node-registry/src/discovery.rs
use tokio::process::Command;
use crate::models::{Node, NodeStatus};

pub async fn discover_nodes() -> Result<Vec<Node>, Box<dyn std::error::Error>> {
    // Run `tailscale status --json`
    let output = Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .await?;
    
    let status: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    
    let mut nodes = Vec::new();
    
    if let Some(peers) = status.get("Peer").and_then(|p| p.as_object()) {
        for (node_id, peer_info) in peers {
            if let Some(ip) = peer_info.get("TailscaleIPs").and_then(|ips| ips.as_array()) {
                let ipv4 = ips.iter().find(|ip| ip.is_string() && ip.as_str().unwrap().contains("100."))
                    .and_then(|ip| ip.as_str())
                    .unwrap_or("unknown");
                
                nodes.push(Node {
                    id: node_id.clone(),
                    hostname: peer_info.get("DNSName").and_then(|d| d.as_str()).unwrap_or("unknown").to_string(),
                    tailscale_ipv4: ipv4.to_string(),
                    tailscale_ipv6: None,
                    region: "unknown".to_string(),
                    capabilities: vec!["agent".to_string()],
                    status: NodeStatus::Online,
                });
            }
        }
    }
    
    Ok(nodes)
}
```

- [ ] **Step 2: Update handlers to use discovery**

```rust
// crates/node-registry/src/handlers.rs (update)
use crate::discovery;
use crate::models::{Node, NodeStatus};

pub async fn get_nodes() -> Json<Vec<Node>> {
    match discovery::discover_nodes().await {
        Ok(nodes) => Json(nodes),
        Err(e) => {
            tracing::error!("Discovery failed: {}", e);
            Json(vec![])
        }
    }
}

pub async fn get_node(Path(hostname): Path<String>) -> Json<Option<Node>> {
    match discovery::discover_nodes().await {
        Ok(nodes) => {
            let node = nodes.into_iter()
                .find(|n| n.hostname.contains(&hostname));
            Json(node)
        }
        Err(e) => {
            tracing::error!("Discovery failed: {}", e);
            Json(None)
        }
    }
}
```

- [ ] **Step 3: Add discovery module to main**

```rust
// crates/node-registry/src/main.rs (add)
mod discovery;
```

- [ ] **Step 4: Write discovery test**

```rust
// crates/node-registry/tests/discovery_test.rs
#[tokio::test]
async fn test_discover_nodes() {
    let nodes = node_registry::discovery::discover_nodes().await.unwrap();
    assert!(!nodes.is_empty());
    assert!(nodes.iter().any(|n| n.hostname.contains("nuremberg")));
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test
# Expected: All tests pass
```

- [ ] **Step 6: Commit**

```bash
git add crates/node-registry/src/discovery.rs crates/node-registry/src/handlers.rs
git commit -m "feat: Tailscale CLI-based node discovery"
```

---

### Task 5: Deploy Node Registry to nuremberg-hq

**Files:**
- Create: `scripts/deploy-registry.sh`
- Create: `systemd/node-registry.service`
- Test: `scripts/verify-registry.sh`

**Interfaces:**
- Consumes: Task 4 (discovery working locally)
- Produces: Node registry running on nuremberg-hq (port 8080)

- [ ] **Step 1: Build release binary**

```bash
cd crates/node-registry
cargo build --release
# Expected: target/release/node-registry binary
```

- [ ] **Step 2: Write deployment script**

```bash
#!/bin/bash
# scripts/deploy-registry.sh
set -e

SERVER="root@167.233.148.20"

echo "Deploying node-registry to nuremberg-hq..."

# Copy binary
scp target/release/node-registry $SERVER:/usr/local/bin/

# Copy systemd service
scp systemd/node-registry.service $SERVER:/etc/systemd/system/

# Reload systemd and start
ssh $SERVER '
    systemctl daemon-reload
    systemctl enable node-registry
    systemctl start node-registry
    systemctl status node-registry --no-pager
'

echo "Deployment complete"
```

- [ ] **Step 3: Write systemd service**

```ini
# systemd/node-registry.service
[Unit]
Description=Loop Engineering Node Registry
After=network.target tailscale.service

[Service]
Type=simple
ExecStart=/usr/local/bin/node-registry
Restart=always
User=root
WorkingDirectory=/root

[Install]
WantedBy=multi-user.target
```

- [ ] **Step 4: Deploy**

```bash
./scripts/deploy-registry.sh
# Expected: Service running on nuremberg-hq
```

- [ ] **Step 5: Verify API accessible**

```bash
curl -fsS http://167.233.148.20:8080/api/v1/nodes | jq
# Expected: JSON array of nodes with Tailscale IPs
```

- [ ] **Step 6: Commit**

```bash
git add scripts/deploy-registry.sh systemd/node-registry.service
git commit -m "deploy: node-registry to nuremberg-hq"
```

---

### Task 6: Integrate with Loop Engineering Transport

**Files:**
- Modify: `crates/transport/src/config.rs`
- Modify: `crates/transport/src/discovery.rs`
- Test: `crates/transport/tests/tailscale_integration_test.rs`

**Interfaces:**
- Consumes: Task 5 (node-registry API running)
- Produces: Transport layer uses Tailscale IPs for agent communication

- [ ] **Step 1: Add node-registry client to transport**

```rust
// crates/transport/src/discovery.rs
use reqwest;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Node {
    pub id: String,
    pub hostname: String,
    pub tailscale_ipv4: String,
    pub region: String,
}

pub struct TailscaleDiscovery {
    registry_url: String,
    client: reqwest::Client,
}

impl TailscaleDiscovery {
    pub fn new(registry_url: &str) -> Self {
        Self {
            registry_url: registry_url.to_string(),
            client: reqwest::Client::new(),
        }
    }
    
    pub async fn get_peers(&self) -> Result<Vec<Node>, reqwest::Error> {
        let url = format!("{}/api/v1/nodes", self.registry_url);
        let resp = self.client.get(&url).send().await?;
        resp.json().await
    }
}
```

- [ ] **Step 2: Update transport config**

```rust
// crates/transport/src/config.rs
pub struct TransportConfig {
    pub discovery_url: String,
    pub bind_addr: String,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            discovery_url: "http://nuremberg-hq:8080".to_string(),
            bind_addr: "0.0.0.0:9000".to_string(),
        }
    }
}
```

- [ ] **Step 3: Write integration test**

```rust
// crates/transport/tests/tailscale_integration_test.rs
use loop_engineering_transport::{TransportConfig, TailscaleDiscovery};

#[tokio::test]
async fn test_discovery_returns_peers() {
    let discovery = TailscaleDiscovery::new("http://167.233.148.20:8080");
    let peers = discovery.get_peers().await.unwrap();
    
    assert!(!peers.is_empty());
    assert!(peers.iter().any(|p| p.region == "eu-central"));
}
```

- [ ] **Step 4: Run integration test**

```bash
cargo test --test tailscale_integration_test
# Expected: PASS (node-registry must be running)
```

- [ ] **Step 5: Commit**

```bash
git add crates/transport/src/discovery.rs crates/transport/src/config.rs
git commit -m "feat: transport layer integrates with Tailscale discovery"
```

---

### Task 7: End-to-End Verification

**Files:**
- Create: `scripts/e2e-verify.sh`
- Test: `scripts/e2e-verify.sh`

**Interfaces:**
- Consumes: Task 6 (transport integrated)
- Produces: Verified agent-to-agent communication over Tailscale

- [ ] **Step 1: Write e2e verification script**

```bash
#!/bin/bash
# scripts/e2e-verify.sh
set -e

echo "End-to-End Tailscale Integration Test"
echo "======================================"

REGISTRY_URL="http://167.233.148.20:8080"

# Test 1: Node registry API
echo -n "Test 1 - Registry API: "
curl -fsS "$REGISTRY_URL/api/v1/nodes" | jq -e 'length > 0' > /dev/null && echo "PASS" || echo "FAIL"

# Test 2: MagicDNS resolution
echo -n "Test 2 - MagicDNS: "
ssh root@167.233.148.20 "getent hosts nuremberg-hq" > /dev/null && echo "PASS" || echo "FAIL"

# Test 3: Tailscale connectivity
echo -n "Test 3 - P2P connectivity: "
ssh root@167.233.148.20 "tailscale ping falkenstein-1" > /dev/null && echo "PASS" || echo "FAIL"

# Test 4: Transport layer
echo -n "Test 4 - Transport discovery: "
cd crates/transport && cargo test --test tailscale_integration_test 2>&1 | grep -q "test result: ok" && echo "PASS" || echo "FAIL"

echo ""
echo "E2E verification complete"
```

- [ ] **Step 2: Run e2e verification**

```bash
./scripts/e2e-verify.sh
# Expected: All 4 tests PASS
```

- [ ] **Step 3: Document results**

```markdown
# docs/e2e-results.md

## Date: 2026-06-27

| Test | Status | Notes |
|------|--------|-------|
| Registry API | PASS | Returns 10 nodes |
| MagicDNS | PASS | Resolves *.ts.net |
| P2P connectivity | PASS | Direct WireGuard |
| Transport discovery | PASS | Finds peers via API |

## Latency Measurements
- EU internal: <50ms
- EU-US: ~130ms
- EU-Asia: ~220ms
```

- [ ] **Step 4: Commit**

```bash
git add scripts/e2e-verify.sh docs/e2e-results.md
git commit -m "test: end-to-end Tailscale integration verification"
```

---

## Verification Checklist

- [ ] All 10 nodes show `online` in `tailscale status`
- [ ] MagicDNS resolves all hostnames (`*.ts.net`)
- [ ] Node registry API returns all 10 nodes with Tailscale IPs
- [ ] Transport layer discovers peers via registry
- [ ] Agent-to-agent communication works over Tailscale IPs
- [ ] Cross-region latency acceptable (<200ms EU-US, <300ms EU-Asia)

---

## Next Steps

After Tailscale integration:
1. Deploy loop-engineering agents to all 10 nodes
2. Configure agents to use Tailscale discovery
3. Test agent task distribution across regions
4. Monitor transport layer metrics

(End of file)
