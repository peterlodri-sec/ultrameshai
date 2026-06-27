# Headscale Mesh VPN Design

**Date:** 2026-06-27
**Status:** Approved
**Author:** Loop Engineering Team

---

## 1. System Identity & Goal

Headscale Mesh VPN connects 10 Hetzner servers into a WireGuard-based mesh network for the loop-engineering agent transport layer. Provides secure, low-latency P2P communication across EU, US, and Asia regions with centralized authentication and ACL-based access control.

### Success Criteria

1. All 10 servers connected via WireGuard mesh (100.64.0.0/16)
2. Headscale control plane running on `nuremberg-hq` (167.233.148.20)
3. DNS-based service discovery (`*.mesh.local`)
4. ACL enforcement (region-based + admin access)
5. DERP fallback for NAT traversal
6. Monitoring + backup strategy operational

### Scope Boundary

This design covers Headscale mesh setup only. Agent deployment and loop-engineering transport integration are separate phases.

---

## 2. Server Inventory & Roles

| Hostname | Role | Region | Public IP | Arch | Storage |
|----------|------|--------|-----------|------|---------|
| `nuremberg-hq` | **Headscale server** | EU (Nuremberg) | 167.233.148.20 | x86 | 40GB |
| `falkenstein-1` | Agent node | EU (Falkenstein) | 178.105.245.135 | x86 | 320GB |
| `falkenstein-2` | Agent node | EU (Falkenstein) | 167.233.105.32 | x86 | 80GB |
| `nuremberg-1` | Agent node | EU (Nuremberg) | 167.233.35.194 | x86 | 40GB+102GB |
| `nuremberg-2` | Agent node | EU (Nuremberg) | 178.105.184.32 | x86 | 80GB |
| `helsinki-1` | Agent node | EU (Helsinki) | 89.167.80.207 | x86 | 320GB |
| `hillsboro-1` | Agent node | US (Hillsboro) | 5.78.122.125 | x86 | 80GB |
| `singapore-1` | Agent node | Asia (Singapore) | 5.223.79.65 | x86 | 40GB |
| `runner-arm` | ARM test node | EU (Nuremberg) | 178.104.47.201 | ARM | 40GB |
| `ccx33-nbg1` | High-perf node | EU (Nuremberg) | 46.225.127.20 | x86 | 240GB |

**Naming convention:** `<region>-<role>` format

---

## 3. Network Design

### 3.1 WireGuard Subnet Allocation

```
Private network: 100.64.0.0/16 (CGNAT range)

Headscale server: 100.64.0.1/24
Agent nodes:      100.64.1.0/24 (DHCP from Headscale)

Per-node allocation:
- nuremberg-hq:     100.64.0.1
- falkenstein-1:    100.64.1.1
- falkenstein-2:    100.64.1.2
- nuremberg-1:      100.64.1.3
- nuremberg-2:      100.64.1.4
- helsinki-1:       100.64.1.5
- hillsboro-1:      100.64.1.6
- singapore-1:      100.64.1.7
- runner-arm:       100.64.1.8
- ccx33-nbg1:       100.64.1.9
```

### 3.2 DNS Configuration

```
Internal DNS (via Headscale MagicDNS):
hq.mesh.local      → 100.64.0.1
fk1.mesh.local     → 100.64.1.1
fk2.mesh.local     → 100.64.1.2
nb1.mesh.local     → 100.64.1.3
nb2.mesh.local     → 100.64.1.4
hki.mesh.local     → 100.64.1.5
pdx.mesh.local     → 100.64.1.6
sin.mesh.local     → 100.64.1.7
arm.mesh.local     → 100.64.1.8
ccx.mesh.local     → 100.64.1.9
```

---

## 4. Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        HEADSCALE MESH VPN                                │
│                         (WireGuard + Headscale)                          │
└─────────────────────────────────────────────────────────────────────────┘

                    ┌──────────────────┐
                    │  Headscale Server │
                    │  (Control Plane)  │
                    │  nuremberg-hq     │
                    │  100.64.0.1       │
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
```

**Topology:** Star-mesh hybrid
- **Headscale server:** Single control plane (nuremberg region - most nodes)
- **WireGuard peers:** Full mesh P2P between all 10 nodes
- **DERP relay:** Fallback for NAT traversal (Hetzner has good peering)

---

## 5. Headscale Configuration

### 5.1 Server Config (`/etc/headscale/config.yaml`)

```yaml
server_url: https://hq.mesh.local:8080
listen_addr: 0.0.0.0:8080
metrics_listen_addr: 0.0.0.0:9090

grpc_listen_addr: 0.0.0.0:50443
grpc_allow_insecure: false

private_key_path: /var/lib/headscale/private.key
noise:
  private_key_path: /var/lib/headscale/noise_private.key

db_type: sqlite3
db_path: /var/lib/headscale/db.sqlite

ephemeral_node_inactivity_timeout: 30m
node_update_check_interval: 10s

ip_prefixes:
  - 100.64.0.0/16

dns_config:
  override_local_dns: true
  nameservers:
    - 1.1.1.1
    - 8.8.8.8
  domains:
    - mesh.local
  magic_dns: true

log:
  level: info
  format: text

derp:
  server:
    enabled: true
    region_id: 1
    region_code: "eu-central"
    region_name: "Europe Central"
    stun_listen_addr: "0.0.0.0:3478"
    private_key_path: /var/lib/headscale/derp.key
  urls:
    - https://controlplane.tailscale.com/derpmap/default
```

### 5.2 ACL Policy (`/etc/headscale/acl.yaml`)

```yaml
groups:
  group:admin:
    - nuremberg-hq
  group:eu-agents:
    - falkenstein-1
    - falkenstein-2
    - nuremberg-1
    - nuremberg-2
    - helsinki-1
    - runner-arm
    - ccx33-nbg1
  group:us-agents:
    - hillsboro-1
  group:asia-agents:
    - singapore-1

hosts: {}

acls:
  # Admin can access all nodes
  - action: accept
    src:
      - group:admin
    dst:
      - "*"

  # EU agents can talk to each other
  - action: accept
    src:
      - group:eu-agents
    dst:
      - group:eu-agents

  # Cross-region: EU ↔ US ↔ Asia (for agent transport)
  - action: accept
    src:
      - group:eu-agents
      - group:us-agents
      - group:asia-agents
    dst:
      - "*"

  # Default deny
  - action: deny
    src:
      - "*"
    dst:
      - "*"
```

---

## 6. Security Design

### 6.1 Authentication

- **Pre-auth keys:** Per-node keys with expiry (30 days)
- **Key rotation:** 90-day rotation policy
- **ACLs:** Role-based access control (region groups + admin)

### 6.2 Firewall Rules

```bash
# Headscale server (nuremberg-hq)
ufw allow 8080/tcp    # HTTPS (Headscale API)
ufw allow 50443/tcp   # gRPC
ufw allow 3478/udp    # STUN
ufw allow 41641/udp   # WireGuard
ufw allow 22/tcp      # SSH (restricted to admin IPs)
```

### 6.3 TLS/HTTPS

- **Certificate:** Let's Encrypt via reverse proxy (nginx/caddy)
- **Domain:** `hq.mesh.local` (must resolve to 167.233.148.20)
- **Auto-renewal:** Certbot cron job

---

## 7. Deployment Plan

### Phase 1: Headscale Server Setup (nuremberg-hq)
1. Install Headscale (`apt install headscale` or Docker)
2. Configure `/etc/headscale/config.yaml`
3. Generate pre-auth keys
4. Set up reverse proxy (nginx/caddy) for HTTPS
5. Configure DNS (`hq.mesh.local` → 167.233.148.20)

### Phase 2: Client Deployment (all nodes)
1. Install Tailscale/Headscale client
2. Register with pre-auth key
3. Verify mesh connectivity
4. Configure DNS resolution

### Phase 3: Validation
1. Ping all nodes via mesh IPs
2. Test cross-region latency
3. Verify DERP fallback
4. Test ACL enforcement

### Phase 4: Integration
1. Deploy loop-engineering transport layer
2. Configure node discovery via mesh DNS
3. Test agent-to-agent communication

---

## 8. Monitoring & Operations

### Health Checks

```bash
# Headscale status
headscale nodes list

# WireGuard status
wg show

# Connectivity test
for ip in 100.64.1.{1..9}; do ping -c1 $ip; done
```

### Metrics

- Headscale metrics: `http://hq.mesh.local:9090/metrics`
- WireGuard traffic: `wg show all transfer`
- DERP usage: Headscale dashboard

### Backup Strategy

- **Headscale DB:** Daily SQLite backup (`/var/lib/headscale/db.sqlite`)
- **Configs:** Git-managed in `/etc/headscale/`
- **Keys:** Encrypted backup (age/sops)

---

## 9. Failure Scenarios

| Failure | Impact | Mitigation |
|---------|--------|------------|
| Headscale server down | No new auth, existing peers continue | Server is stateless after auth; peers maintain mesh |
| DERP relay down | NAT traversal fails | Direct P2P works for most Hetzner nodes |
| Single node down | That node unreachable | Mesh continues; agent reschedules |
| Region outage | Regional nodes unreachable | Cross-region redundancy (EU has 7 nodes) |

---

## 10. Open Questions

- DNS setup: Use Hetzner DNS, Cloudflare, or self-hosted?
- Certificate management: Let's Encrypt vs self-signed for internal?
- Monitoring: Prometheus + Grafana integration?
- Alerting: Slack/PagerDuty integration for node failures?

---

## Appendix A: Pre-Auth Key Generation

```bash
# Generate reusable key for EU agents (expires 30 days)
headscale preauthkeys create --reusable --expiration 30d --group group:eu-agents

# Generate single-use key for US agent
headscale preauthkeys create --reusable=false --group group:us-agents

# Generate single-use key for Asia agent
headscale preauthkeys create --reusable=false --group group:asia-agents
```

---

## Appendix B: Client Connection Command

```bash
# On each agent node (replace KEY with actual pre-auth key)
tailscale up --login-server=https://hq.mesh.local:8080 --auth-key=KEY

# Verify connection
tailscale status
```

---

## Appendix C: Verification Checklist

- [ ] All 10 nodes show `online` in `headscale nodes list`
- [ ] Ping works between all nodes via 100.64.x.x IPs
- [ ] DNS resolution works for `*.mesh.local`
- [ ] Cross-region latency acceptable (<200ms EU-US, <300ms EU-Asia)
- [ ] ACL enforcement verified (region isolation test)
- [ ] DERP fallback tested (block direct P2P, verify relay works)
- [ ] Backup script tested and verified

(End of file)
