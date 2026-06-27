# Headscale Mesh Setup

This guide covers setting up a headscale control plane and joining cloud VMs + Raspberry Pis to the mesh. The agent stack uses this mesh for cross-node communication (WireGuard) and node discovery (heartbeat multicast).

## 1. Headscale Server

Install headscale on a control node (cloud VM or dedicated server):

```bash
# Install headscale
curl -fsSL https://headscale.net/install.sh | sh

# Configure
sudo cp config-example.yaml /etc/headscale/config.yaml
# Edit /etc/headscale/config.yaml:
#   server_url: https://headscale.yourdomain.com
#   listen_addr: 0.0.0.0:8080
#   magic_dns: true
#   base_domain: mesh.local

# Start
sudo systemctl enable --now headscale
```

## 2. Create a User

```bash
headscale users create loop-engineering
```

## 3. Join a Cloud VM

On each VM:

```bash
# Install tailscale client (works with headscale)
curl -fsSL https://tailscale.com/install.sh | sh

# Point at headscale instead of tailscale.com
tailscale up --login-server https://headscale.yourdomain.com --auth-key tskey-...

# Verify
tailscale status
```

## 4. Join a Raspberry Pi

```bash
# Install on RPi (arm64)
curl -fsSL https://tailscale.com/install.sh | sh

# Join
tailscale up --login-server https://headscale.yourdomain.com --auth-key tskey-...

# Verify
tailscale status
```

## 5. Tailscale -> Headscale Migration

If already on Tailscale (commercial), migration is one command per node:

```bash
# On each node:
sudo tailscale down
sudo tailscale up --login-server https://headscale.yourdomain.com --auth-key tskey-...
```

## 6. Verify Mesh

From any node:

```bash
# List all nodes
tailscale status

# Ping another node
tailscale ping <node-name>
```

## 7. Node Registry Integration

Each node runs the heartbeat broadcaster (from `crates/node-registry`) to advertise:
- CPU cores
- Total/free memory
- Units currently running
- Capabilities (standard, test, red-team, devops)

The heartbeat goes over UDP multicast on the mesh. Other nodes listen and maintain a `NodeRegistry` for self-scheduling.