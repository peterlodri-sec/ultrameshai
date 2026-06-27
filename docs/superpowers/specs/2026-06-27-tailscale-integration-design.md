# Tailscale Mesh VPN Integration Design

**Date:** 2026-06-27
**Status:** Approved
**Author:** Loop Engineering Team

---

## 1. System Identity & Goal

Integrate loop-engineering agent transport layer with existing Tailscale mesh VPN across 10 Hetzner servers. Leverages existing Tailscale deployment (no Headscale) for agent-to-agent communication, DNS-based service discovery, and ACL-based access control.

### Success Criteria

1. All 10 servers reachable via existing Tailscale IPs
2. DNS-based service discovery via Tailscale MagicDNS
3. ACL enforcement via Tailscale ACLs
4. Node discovery service integrated with Tailscale
5. Agent transport layer uses Tailscale IPs for communication

### Scope Boundary

This design covers Tailscale integration only. Does not modify existing Tailscale deployment or affect other Tailscale users.

---

## 2. Server Inventory & Tailscale Status

| Hostname | Region | Public IP | Tailscale IP (expected) | Arch | Storage | Status |
|----------|--------|-----------|------------------------|------|---------|--------|
| `nuremberg-hq` | EU (Nuremberg) | 167.233.148.20 | 100.x.x.x | x86 | 40GB | Existing Tailscale |
| `falkenstein-1` | EU (Falkenstein) | 178.105.245.135 | 100.x.x.x | x86 | 320GB | Existing Tailscale |
| `falkenstein-2` | EU (Falkenstein) | 167.233.105.32 | 100.x.x.x | x86 | 80GB | Existing Tailscale |
| `nuremberg-1` | EU (Nuremberg) | 167.233.35.194 | 100.x.x.x | x86 | 40GB+102GB | Existing Tailscale |
| `nuremberg-2` | EU (Nuremberg) | 178.105.184.32 | 100.x.x.x | x86 | 80GB | Existing Tailscale |
| `helsinki-1` | EU (Helsinki) | 89.167.80.207 | 100.x.x.x | x86 | 320GB | Existing Tailscale |
| `hillsboro-1` | US (Hillsboro) | 5.78.122.125 | 100.x.x.x | x86 | 80GB | Existing Tailscale |
| `singapore-1` | Asia (Singapore) | 5.223.79.65 | 100.x.x.x | x86 | 40GB | Existing Tailscale |
| `runner-arm` | EU (Nuremberg) | 178.104.47.201 | 100.x.x.x | ARM | 40GB | Existing Tailscale |
| `ccx33-nbg1` | EU (Nuremberg) | 46.225.127.20 | 100.x.x.x | x86 | 240GB | Existing Tailscale |

**Note:** Actual Tailscale IPs to be discovered via `tailscale ip` commands.

---

## 3. Network Design

### 3.1 Tailscale Subnet

```
Tailscale CGNAT range: 100.64.0.0/16 (or 100.100.0.0/16)

Each node receives:
- One IPv4 address (100.x.x.x/32)
- Optional IPv6 (fd7a:115c:a1e0::/48)

Tailscale handles:
- IP assignment (DHCP-like from control plane)
- DNS resolution (MagicDNS)
- NAT traversal (DERP relay fallback)
- Peer discovery (via control plane)
```

### 3.2 DNS Configuration (Tailscale MagicDNS)

```
MagicDNS domain: <tailnet>.ts.net (auto-assigned)

Per-node DNS names:
- nuremberg-hq.<tailnet>.ts.net
- falkenstein-1.<tailnet>.ts.net
- falkenstein-2.<tailnet>.ts.net
- nuremberg-1.<tailnet>.ts.net
- nuremberg-2.<tailnet>.ts.net
- helsinki-1.<tailnet>.ts.net
- hillsboro-1.<tailnet>.ts.net
- singapore-1.<tailnet>.ts.net
- runner-arm.<tailnet>.ts.net
- ccx33-nbg1.<tailnet>.ts.net

Also accessible by hostname only (if MagicDNS enabled):
- nuremberg-hq
- falkenstein-1
- etc.
```

### 3.3 Custom DNS (Optional)

```
If custom domain needed:
- Add DNS search domain via Tailscale admin console
- Example: mesh.local → <tailnet>.ts.net

Or run split-horizon DNS on one node:
- Deploy CoreDNS on nuremberg-hq
- Configure Tailscale to use it as DNS server
- Add mesh.local records pointing to Tailscale IPs
```

---

## 4. Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    EXISTING TAILSCALE MESH VPN                           │
│                    (No changes to control plane)                         │
└─────────────────────────────────────────────────────────────────────────┘

                    ┌──────────────────┐
                    │  Tailscale.com   │
                    │  (Control Plane) │
                    │  (existing)      │
                    └────────┬─────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
   ┌────▼────┐         ┌────▼────┐         ┌────▼────┐
   │  EU     │         │  US     │         │  Asia   │
   │ Region │         │ Region  │         │ Region  │
   │        │         │         │         │         │
   │ 7 nodes │         │ 1 node  │         │ 1 node  │
   └─────────┘         └─────────┘         └─────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│              LOOP ENGINEERING TRANSPORT LAYER                            │
│              (Built on top of Tailscale)                                 │
└─────────────────────────────────────────────────────────────────────────┘
```

**Key decisions:**
- **No Headscale:** Use existing Tailscale control plane
- **No client changes:** Nodes already running `tailscaled`
- **Add node discovery:** Loop-engineering service discovers Tailscale IPs
- **Add transport layer:** Agent communication over Tailscale IPs

---

## 5. Tailscale Configuration

### 5.1 Required Tailscale Features

| Feature | Status | Notes |
|---------|--------|-------|
| MagicDNS | Enable if not already | Provides `*.ts.net` DNS |
| Subnet routers | Not needed | Using host IPs only |
| Exit nodes | Not needed | Internal mesh only |
| ACLs | Review existing | Ensure agent ports allowed |
| Tags | Optional | For node grouping |

### 5.2 ACL Configuration (if custom ACLs needed)

```json
{
  "groups": {
    "group:loop-agents": [
      "tag:loop-agent"
    ]
  },
  "hosts": {
    "nuremberg-hq": "100.x.x.x",
    "falkenstein-1": "100.x.x.x"
  },
  "acls": [
    {
      "action": "accept",
      "src": ["group:loop-agents"],
      "dst": ["group:loop-agents:8080-8090,9000-9100"]
    }
  ]
}
```

### 5.3 Tags for Node Grouping

```bash
# Tag nodes for loop-engineering
tailscale set-tags --nodes <node-ids> --tags tag:loop-agent

# Tags provide:
- ACL grouping without hostname management
- Automatic policy application
- Easier node rotation
```

---

## 6. Security Design

### 6.1 Authentication

- **Existing Tailscale auth:** SSO/OIDC already configured
- **Node authorization:** Via Tailscale admin console
- **No changes needed:** Leverage existing identity management

### 6.2 Network Security

```bash
# Tailscale provides:
- Encrypted WireGuard tunnels (default)
- Mutual authentication (default)
- Automatic key rotation (default)
- Firewall per node (optional)

# Additional hardening:
- Enable Tailscale SSH (optional)
- Configure node-level firewalls (ufw/nftables)
- Use tags for ACL enforcement
```

### 6.3 Port Allocation for Loop Engineering

| Port Range | Service | Notes |
|------------|---------|-------|
| 8080-8089 | Agent HTTP API | REST/gRPC endpoints |
| 8090-8099 | Agent WebSocket | Real-time communication |
| 9000-9009 | Transport Layer | Protobuf framing |
| 9100-9109 | Monitoring | Metrics, health checks |

---

## 7. Integration Plan

### Phase 1: Discovery
1. Inventory existing Tailscale deployment
2. Document Tailscale IPs per node
3. Verify MagicDNS status
4. Review existing ACLs

### Phase 2: Node Discovery Service
1. Deploy node-registry service on nuremberg-hq
2. Service reads Tailscale IPs via `tailscale` CLI
3. Expose node registry via HTTP API
4. Agents query registry for peer IPs

### Phase 3: Transport Integration
1. Configure transport layer to use Tailscale IPs
2. Test agent-to-agent communication
3. Verify DERP fallback behavior
4. Monitor latency across regions

### Phase 4: Monitoring
1. Add Tailscale status to monitoring dashboard
2. Alert on node disconnections
3. Track cross-region latency
4. Log transport layer metrics

---

## 8. Node Discovery Service Design

### 8.1 Service Architecture

```
┌─────────────────┐     ┌─────────────────┐
│  Node Registry  │────▶│  Tailscale CLI  │
│    Service      │     │  (tailscale ip) │
└────────┬────────┘     └─────────────────┘
         │
         ▼
┌─────────────────┐
│  HTTP API       │
│  GET /nodes     │
│  GET /nodes/:id │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Agent Nodes    │
│  (query peers)  │
└─────────────────┘
```

### 8.2 API Endpoints

```rust
// GET /api/v1/nodes
// Returns: List of all registered nodes with Tailscale IPs

// GET /api/v1/nodes/:hostname
// Returns: Single node info (IP, region, capabilities)

// POST /api/v1/nodes/register
// Body: { hostname, region, capabilities }
// Returns: { node_id, tailscale_ip }

// DELETE /api/v1/nodes/:id
// Returns: 204 No Content
```

### 8.3 Data Model

```rust
struct Node {
    id: String,           // Unique node ID
    hostname: String,     // e.g., "nuremberg-hq"
    tailscale_ipv4: String, // e.g., "100.64.0.1"
    tailscale_ipv6: Option<String>,
    region: String,       // e.g., "eu-central"
    capabilities: Vec<String>, // e.g., ["agent", "transport"]
    last_seen: DateTime,  // Last heartbeat
    status: NodeStatus,   // Online/Offline
}
```

---

## 9. Deployment Steps

### 9.1 Pre-Deployment Checklist

- [ ] Verify Tailscale running on all 10 nodes
- [ ] Document Tailscale IPs for each node
- [ ] Enable MagicDNS (if not already)
- [ ] Review ACLs for agent port access
- [ ] Tag nodes with `tag:loop-agent`

### 9.2 Node Registry Deployment

1. Deploy node-registry service on nuremberg-hq
2. Service auto-discovers Tailscale peers
3. Expose API on Tailscale IP (port 8080)
4. Register all 10 nodes in registry

### 9.3 Agent Deployment

1. Deploy agent binary to all nodes
2. Configure agent to query node registry
3. Agent discovers peer Tailscale IPs
4. Test agent-to-agent communication

---

## 10. Monitoring & Operations

### Health Checks

```bash
# Tailscale status on each node
tailscale status

# Get node's Tailscale IP
tailscale ip -4

# Test connectivity to peer
ping <peer-tailscale-ip>

# Check MagicDNS
nslookup nuremberg-hq.<tailnet>.ts.net
```

### Metrics to Track

- Tailscale connection status (per node)
- DERP relay usage (fallback indicator)
- Cross-region latency (EU-US, EU-Asia)
- Node registry API response time
- Agent-to-agent message latency

### Alerting

- Node disconnected from Tailscale (>5 min)
- DERP usage >10% (indicates NAT issues)
- Cross-region latency spike (>500ms)
- Node registry API errors

---

## 11. Failure Scenarios

| Failure | Impact | Mitigation |
|---------|--------|------------|
| Tailscale control plane down | Existing connections persist, no new auth | Tailscale.com has high uptime; peers maintain mesh |
| Single node Tailscale disconnect | That node unreachable | Agent reschedules to other nodes |
| DERP relay unavailable | NAT traversal fails for some nodes | Direct P2P works for most Hetzner nodes |
| Region network outage | Regional nodes unreachable | Cross-region redundancy (EU has 7 nodes) |
| Node registry down | Agents can't discover peers | Cache last-known peer IPs; retry registry |

---

## 12. Open Questions

- What is the existing Tailscale tailnet name?
- Is MagicDNS already enabled?
- Are there existing ACLs that need updating?
- Do we need custom DNS (mesh.local) or is `*.ts.net` sufficient?
- Should node registry be highly available (multi-node) or single-instance?

---

## Appendix A: Tailscale CLI Commands

```bash
# Check Tailscale status
tailscale status

# Get this node's IPs
tailscale ip -4
tailscale ip -6

# List all nodes in tailnet
tailscale status --json

# Get specific node IP
tailscale ip --peer=nuremberg-hq

# Check DERP usage
tailscale netcheck

# Test connectivity
tailscale ping nuremberg-hq
```

---

## Appendix B: Node Registry Bootstrap Config

```yaml
# config/node-registry.yaml
server:
  bind_addr: 0.0.0.0:8080
  tailscale_enabled: true

discovery:
  method: tailscale_cli  # Use `tailscale` CLI
  poll_interval: 30s     # Check for new nodes every 30s

nodes:
  # Pre-seed known nodes (optional)
  - hostname: nuremberg-hq
    region: eu-central
    capabilities: ["registry", "agent"]
  - hostname: falkenstein-1
    region: eu-central
    capabilities: ["agent"]
  # ... etc
```

---

## Appendix C: Verification Checklist

- [ ] All 10 nodes show `online` in `tailscale status`
- [ ] MagicDNS resolves all hostnames
- [ ] Ping works between all nodes via Tailscale IPs
- [ ] Cross-region latency acceptable (<200ms EU-US, <300ms EU-Asia)
- [ ] Node registry API returns all 10 nodes
- [ ] Agents can communicate via Tailscale IPs
- [ ] DERP fallback tested (optional)

(End of file)
