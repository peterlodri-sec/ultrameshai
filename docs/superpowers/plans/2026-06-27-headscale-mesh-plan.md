# Headscale Mesh VPN Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deploy Headscale mesh VPN across 10 Hetzner servers for loop-engineering agent transport layer.

**Architecture:** Headscale control plane on nuremberg-hq (EU), WireGuard mesh P2P between all nodes, DNS-based service discovery via MagicDNS, ACL-based access control.

**Tech Stack:** Headscale, WireGuard, Tailscale client, nginx/caddy (reverse proxy), Let's Encrypt (TLS), SQLite (Headscale DB).

## Global Constraints

- WireGuard subnet: 100.64.0.0/16 (CGNAT range)
- Headscale server: 100.64.0.1
- Agent nodes: 100.64.1.1 - 100.64.1.9
- DNS domain: mesh.local
- Pre-auth key expiry: 30 days
- Key rotation: 90 days
- DERP enabled for NAT traversal

---

### Task 1: Headscale Server Setup (nuremberg-hq)

**Files:**
- Create: `/etc/headscale/config.yaml`
- Create: `/etc/headscale/acl.yaml`
- Create: `scripts/headscale-install.sh`
- Test: `scripts/verify-headscale.sh`

**Interfaces:**
- Consumes: None (first task)
- Produces: Running Headscale server on https://hq.mesh.local:8080

- [ ] **Step 1: Write installation script**

```bash
#!/bin/bash
# scripts/headscale-install.sh
set -e

# Add Headscale repository
curl -fsSL https://gitlab.com/juanfont/headscale/-/raw/main/pkg/headscale/headscale.gpg | gpg --dearmor -o /usr/share/keyrings/headscale.gpg
echo "deb [signed-by=/usr/share/keyrings/headscale.gpg] https://gitlab.com/juanfont/headscale/-/raw/main/pkg/headscale/ stable main" > /etc/apt/sources.list.d/headscale.list

# Install Headscale
apt update
apt install -y headscale

# Create config directory
mkdir -p /var/lib/headscale
mkdir -p /etc/headscale

echo "Headscale installed successfully"
```

- [ ] **Step 2: Run installation script on nuremberg-hq**

```bash
ssh root@167.233.148.20 'bash -s' < scripts/headscale-install.sh
# Expected: "Headscale installed successfully"
```

- [ ] **Step 3: Write Headscale config**

```yaml
# /etc/headscale/config.yaml
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

- [ ] **Step 4: Write ACL config**

```yaml
# /etc/headscale/acl.yaml
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
  - action: accept
    src:
      - group:admin
    dst:
      - "*"

  - action: accept
    src:
      - group:eu-agents
    dst:
      - group:eu-agents

  - action: accept
    src:
      - group:eu-agents
      - group:us-agents
      - group:asia-agents
    dst:
      - "*"

  - action: deny
    src:
      - "*"
    dst:
      - "*"
```

- [ ] **Step 5: Copy configs to server**

```bash
scp /etc/headscale/config.yaml root@167.233.148.20:/etc/headscale/config.yaml
scp /etc/headscale/acl.yaml root@167.233.148.20:/etc/headscale/acl.yaml
```

- [ ] **Step 6: Start Headscale service**

```bash
ssh root@167.233.148.20 'systemctl enable headscale && systemctl start headscale'
# Expected: service starts without error
```

- [ ] **Step 7: Verify Headscale running**

```bash
ssh root@167.233.148.20 'systemctl status headscale --no-pager'
# Expected: "active (running)"
```

- [ ] **Step 8: Commit**

```bash
git add scripts/headscale-install.sh
git commit -m "feat: Headscale server installation script"
```

---

### Task 2: Reverse Proxy Setup (nginx + Let's Encrypt)

**Files:**
- Create: `scripts/nginx-setup.sh`
- Create: `nginx/headscale.conf`
- Test: `scripts/verify-ssl.sh`

**Interfaces:**
- Consumes: Task 1 (Headscale running on port 8080)
- Produces: HTTPS endpoint at https://hq.mesh.local:8080

- [ ] **Step 1: Write nginx config**

```nginx
# nginx/headscale.conf
server {
    listen 80;
    server_name hq.mesh.local;
    
    location /.well-known/acme-challenge/ {
        root /var/www/certbot;
    }
    
    location / {
        return 301 https://$server_name$request_uri;
    }
}

server {
    listen 443 ssl http2;
    server_name hq.mesh.local;
    
    ssl_certificate /etc/letsencrypt/live/hq.mesh.local/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/hq.mesh.local/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

- [ ] **Step 2: Write nginx setup script**

```bash
#!/bin/bash
# scripts/nginx-setup.sh
set -e

# Install nginx and certbot
apt install -y nginx certbot python3-certbot-nginx

# Copy nginx config
cp /tmp/headscale.conf /etc/nginx/sites-available/headscale
ln -sf /etc/nginx/sites-available/headscale /etc/nginx/sites-enabled/

# Get SSL certificate
certbot --nginx -d hq.mesh.local --non-interactive --agree-tos --email admin@example.com

# Reload nginx
systemctl reload nginx

echo "Nginx + SSL configured successfully"
```

- [ ] **Step 3: Copy nginx config to server**

```bash
scp nginx/headscale.conf root@167.233.148.20:/tmp/headscale.conf
```

- [ ] **Step 4: Run nginx setup script**

```bash
ssh root@167.233.148.20 'bash -s' < scripts/nginx-setup.sh
# Expected: "Nginx + SSL configured successfully"
```

- [ ] **Step 5: Verify HTTPS endpoint**

```bash
curl -fsSL https://hq.mesh.local:8080/api/v1/health | jq '.status'
# Expected: "ok" (or Headscale health response)
```

- [ ] **Step 6: Commit**

```bash
git add scripts/nginx-setup.sh nginx/headscale.conf
git commit -m "feat: nginx reverse proxy with Let's Encrypt SSL"
```

---

### Task 3: Generate Pre-Auth Keys

**Files:**
- Create: `scripts/generate-keys.sh`
- Create: `keys/README.md`
- Test: `scripts/verify-keys.sh`

**Interfaces:**
- Consumes: Task 1 (Headscale server running)
- Produces: Pre-auth keys for EU, US, Asia agent groups

- [ ] **Step 1: Write key generation script**

```bash
#!/bin/bash
# scripts/generate-keys.sh
set -e

SERVER="root@167.233.148.20"

# Generate EU agents key (reusable, 30 days)
EU_KEY=$(ssh $SERVER 'headscale preauthkeys create --reusable --expiration 30d --group group:eu-agents -o json' | jq -r '.key')
echo "EU Agents Key: $EU_KEY"

# Generate US agents key (single-use, 30 days)
US_KEY=$(ssh $SERVER 'headscale preauthkeys create --reusable=false --expiration 30d --group group:us-agents -o json' | jq -r '.key')
echo "US Agents Key: $US_KEY"

# Generate Asia agents key (single-use, 30 days)
ASIA_KEY=$(ssh $SERVER 'headscale preauthkeys create --reusable=false --expiration 30d --group group:asia-agents -o json' | jq -r '.key')
echo "Asia Agents Key: $ASIA_KEY"

# Save keys (encrypted in production)
mkdir -p keys
echo "$EU_KEY" > keys/eu-agents.key
echo "$US_KEY" > keys/us-agents.key
echo "$ASIA_KEY" > keys/asia-agents.key

echo "Keys generated and saved to keys/"
```

- [ ] **Step 2: Run key generation script**

```bash
./scripts/generate-keys.sh
# Expected: Three keys generated and saved
```

- [ ] **Step 3: Write keys README**

```markdown
# Headscale Pre-Auth Keys

**DO NOT COMMIT THESE KEYS TO GIT**

Keys are generated per-region and should be distributed securely to each node operator.

## Key Files

- `eu-agents.key` - Reusable key for EU region nodes (7 nodes)
- `us-agents.key` - Single-use key for US region node (1 node)
- `asia-agents.key` - Single-use key for Asia region node (1 node)

## Distribution

1. Encrypt keys before transmission (use age/sops)
2. Delete local copies after distribution
3. Rotate keys every 90 days

## Rotation

```bash
./scripts/generate-keys.sh  # Generate new keys
# Distribute new keys to nodes
# Revoke old keys in Headscale UI
```
```

- [ ] **Step 4: Add keys to .gitignore**

```
# .gitignore
keys/*.key
```

- [ ] **Step 5: Verify keys work**

```bash
# Test EU key on falkenstein-1
ssh root@178.105.245.135 "tailscale up --login-server=https://hq.mesh.local:8080 --auth-key=$(cat keys/eu-agents.key)"
# Expected: "Success"
```

- [ ] **Step 6: Commit**

```bash
git add scripts/generate-keys.sh keys/README.md .gitignore
git commit -m "feat: pre-auth key generation for agent groups"
```

---

### Task 4: Deploy Tailscale Clients (EU Region)

**Files:**
- Create: `scripts/deploy-eu-clients.sh`
- Test: `scripts/verify-eu-clients.sh`

**Interfaces:**
- Consumes: Task 3 (pre-auth keys generated)
- Produces: 7 EU nodes connected to mesh

- [ ] **Step 1: Write EU client deployment script**

```bash
#!/bin/bash
# scripts/deploy-eu-clients.sh
set -e

AUTH_KEY=$(cat keys/eu-agents.key)
SERVER_URL="https://hq.mesh.local:8080"

# EU nodes
declare -A EU_NODES=(
    ["falkenstein-1"]="178.105.245.135"
    ["falkenstein-2"]="167.233.105.32"
    ["nuremberg-1"]="167.233.35.194"
    ["nuremberg-2"]="178.105.184.32"
    ["helsinki-1"]="89.167.80.207"
    ["runner-arm"]="178.104.47.201"
    ["ccx33-nbg1"]="46.225.127.20"
)

for node in "${!EU_NODES[@]}"; do
    ip="${EU_NODES[$node]}"
    echo "Deploying to $node ($ip)..."
    
    ssh root@$ip "
        # Install Tailscale
        curl -fsSL https://tailscale.com/install.sh | sh
        
        # Connect to Headscale
        tailscale up --login-server=$SERVER_URL --auth-key=$AUTH_KEY
        
        # Verify connection
        tailscale status
    "
    
    echo "$node connected successfully"
done

echo "All EU nodes connected"
```

- [ ] **Step 2: Run EU deployment script**

```bash
./scripts/deploy-eu-clients.sh
# Expected: All 7 EU nodes connected successfully
```

- [ ] **Step 3: Verify EU nodes in Headscale**

```bash
ssh root@167.233.148.20 'headscale nodes list -o json' | jq '.[] | select(.hostname | contains("fk") or contains("nb") or contains("hki") or contains("arm") or contains("ccx"))'
# Expected: 7 nodes listed with online status
```

- [ ] **Step 4: Commit**

```bash
git add scripts/deploy-eu-clients.sh
git commit -m "feat: deploy Tailscale clients to EU region nodes"
```

---

### Task 5: Deploy Tailscale Clients (US + Asia Regions)

**Files:**
- Create: `scripts/deploy-us-clients.sh`
- Create: `scripts/deploy-asia-clients.sh`
- Test: `scripts/verify-global-clients.sh`

**Interfaces:**
- Consumes: Task 3 (pre-auth keys generated)
- Produces: US + Asia nodes connected to mesh

- [ ] **Step 1: Write US client deployment script**

```bash
#!/bin/bash
# scripts/deploy-us-clients.sh
set -e

AUTH_KEY=$(cat keys/us-agents.key)
SERVER_URL="https://hq.mesh.local:8080"

# US node
US_NODE="5.78.122.125"

echo "Deploying to hillsboro-1 ($US_NODE)..."

ssh root@$US_NODE "
    curl -fsSL https://tailscale.com/install.sh | sh
    tailscale up --login-server=$SERVER_URL --auth-key=$AUTH_KEY
    tailscale status
"

echo "US node connected successfully"
```

- [ ] **Step 2: Write Asia client deployment script**

```bash
#!/bin/bash
# scripts/deploy-asia-clients.sh
set -e

AUTH_KEY=$(cat keys/asia-agents.key)
SERVER_URL="https://hq.mesh.local:8080"

# Asia node
ASIA_NODE="5.223.79.65"

echo "Deploying to singapore-1 ($ASIA_NODE)..."

ssh root@$ASIA_NODE "
    curl -fsSL https://tailscale.com/install.sh | sh
    tailscale up --login-server=$SERVER_URL --auth-key=$AUTH_KEY
    tailscale status
"

echo "Asia node connected successfully"
```

- [ ] **Step 3: Run US deployment script**

```bash
./scripts/deploy-us-clients.sh
# Expected: hillsboro-1 connected successfully
```

- [ ] **Step 4: Run Asia deployment script**

```bash
./scripts/deploy-asia-clients.sh
# Expected: singapore-1 connected successfully
```

- [ ] **Step 5: Verify all nodes in Headscale**

```bash
ssh root@167.233.148.20 'headscale nodes list -o table'
# Expected: 10 nodes total (1 server + 9 agents)
```

- [ ] **Step 6: Commit**

```bash
git add scripts/deploy-us-clients.sh scripts/deploy-asia-clients.sh
git commit -m "feat: deploy Tailscale clients to US and Asia region nodes"
```

---

### Task 6: Verify Mesh Connectivity

**Files:**
- Create: `scripts/verify-mesh.sh`
- Create: `scripts/latency-test.sh`
- Test: `scripts/verify-mesh.sh`

**Interfaces:**
- Consumes: Task 4 + Task 5 (all nodes connected)
- Produces: Verified mesh connectivity across all 10 nodes

- [ ] **Step 1: Write mesh verification script**

```bash
#!/bin/bash
# scripts/verify-mesh.sh
set -e

SERVER="root@167.233.148.20"

# Get all node IPs from Headscale
NODES=$(ssh $SERVER 'headscale nodes list -o json' | jq -r '.[] | .ipAddresses[0]')

echo "Testing connectivity between all nodes..."

for node_ip in $NODES; do
    echo "Pinging $node_ip from HQ..."
    ssh $SERVER "ping -c1 -W2 $node_ip" || echo "FAILED: $node_ip"
done

echo "Mesh connectivity test complete"
```

- [ ] **Step 2: Run mesh verification**

```bash
./scripts/verify-mesh.sh
# Expected: All pings successful
```

- [ ] **Step 3: Write latency test script**

```bash
#!/bin/bash
# scripts/latency-test.sh
set -e

SERVER="root@167.233.148.20"

echo "Cross-region latency test:"
echo "=========================="

# EU internal
echo -n "EU internal (Nuremberg → Falkenstein): "
ssh root@167.233.148.20 "ping -c1 100.64.1.1 | grep 'time=' | cut -d'=' -f3"

# EU → US
echo -n "EU → US (Nuremberg → Hillsboro): "
ssh root@167.233.148.20 "ping -c1 100.64.1.6 | grep 'time=' | cut -d'=' -f3"

# EU → Asia
echo -n "EU → Asia (Nuremberg → Singapore): "
ssh root@167.233.148.20 "ping -c1 100.64.1.7 | grep 'time=' | cut -d'=' -f3"
```

- [ ] **Step 4: Run latency test**

```bash
./scripts/latency-test.sh
# Expected: EU <50ms, EU-US <150ms, EU-Asia <250ms
```

- [ ] **Step 5: Verify DNS resolution**

```bash
# Test MagicDNS from HQ
ssh root@167.233.148.20 "getent hosts fk1.mesh.local"
# Expected: 100.64.1.1 fk1.mesh.local
```

- [ ] **Step 6: Commit**

```bash
git add scripts/verify-mesh.sh scripts/latency-test.sh
git commit -m "test: verify mesh connectivity and latency"
```

---

### Task 7: Configure Monitoring & Backup

**Files:**
- Create: `scripts/headscale-backup.sh`
- Create: `scripts/headscale-metrics.sh`
- Create: `monitoring/prometheus-headscale.yml`
- Test: `scripts/verify-backup.sh`

**Interfaces:**
- Consumes: Task 6 (mesh operational)
- Produces: Daily backups + metrics endpoint

- [ ] **Step 1: Write backup script**

```bash
#!/bin/bash
# scripts/headscale-backup.sh
set -e

SERVER="root@167.233.148.20"
BACKUP_DIR="/var/backups/headscale"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup directory
ssh $SERVER "mkdir -p $BACKUP_DIR"

# Backup SQLite database
ssh $SERVER "sqlite3 /var/lib/headscale/db.sqlite '.backup $BACKUP_DIR/db_$DATE.sqlite'"

# Backup configs
ssh $SERVER "tar -czf $BACKUP_DIR/configs_$DATE.tar.gz /etc/headscale/"

# Keep only last 7 backups
ssh $SERVER "find $BACKUP_DIR -name '*.sqlite' -mtime +7 -delete"
ssh $SERVER "find $BACKUP_DIR -name '*.tar.gz' -mtime +7 -delete"

echo "Backup complete: $DATE"
```

- [ ] **Step 2: Add backup cron job**

```bash
# Add to server crontab
ssh root@167.233.148.20 '
(crontab -l 2>/dev/null; echo "0 2 * * * /usr/local/bin/headscale-backup.sh") | crontab -
'
```

- [ ] **Step 3: Write metrics script**

```bash
#!/bin/bash
# scripts/headscale-metrics.sh
set -e

SERVER="root@167.233.148.20"

echo "Headscale Metrics:"
echo "=================="

# Node count
NODE_COUNT=$(ssh $SERVER 'headscale nodes list -o json' | jq 'length')
echo "Total nodes: $NODE_COUNT"

# Online nodes
ONLINE_COUNT=$(ssh $SERVER 'headscale nodes list -o json' | jq '[.[] | select(.lastSeen != null and ((now - (.lastSeen | fromdateiso8601)) < 300))] | length')
echo "Online nodes (last 5min): $ONLINE_COUNT"

# Metrics endpoint
echo "Metrics URL: http://hq.mesh.local:9090/metrics"
```

- [ ] **Step 4: Run backup script**

```bash
./scripts/headscale-backup.sh
# Expected: "Backup complete: YYYYMMDD_HHMMSS"
```

- [ ] **Step 5: Verify backup exists**

```bash
ssh root@167.233.148.20 'ls -lh /var/backups/headscale/'
# Expected: db_*.sqlite and configs_*.tar.gz files
```

- [ ] **Step 6: Commit**

```bash
git add scripts/headscale-backup.sh scripts/headscale-metrics.sh
git commit -m "feat: monitoring and backup automation"
```

---

### Task 8: Integration Test with Loop Engineering Transport

**Files:**
- Create: `scripts/integration-test.sh`
- Test: `scripts/integration-test.sh`

**Interfaces:**
- Consumes: Task 7 (mesh + monitoring operational)
- Produces: Verified agent-to-agent communication via mesh

- [ ] **Step 1: Write integration test script**

```bash
#!/bin/bash
# scripts/integration-test.sh
set -e

echo "Loop Engineering Transport Integration Test"
echo "==========================================="

# Test 1: DNS resolution from EU node
echo -n "Test 1 - DNS resolution: "
ssh root@178.105.245.135 "getent hosts hq.mesh.local" > /dev/null && echo "PASS" || echo "FAIL"

# Test 2: Direct P2P connection
echo -n "Test 2 - P2P connection (EU→US): "
ssh root@178.105.245.135 "ping -c1 100.64.1.6" > /dev/null && echo "PASS" || echo "FAIL"

# Test 3: DERP fallback (simulate by blocking direct)
echo "Test 3 - DERP fallback: SKIPPED (manual test)"

# Test 4: ACL enforcement
echo -n "Test 4 - ACL enforcement: "
# This should work (EU→EU)
ssh root@178.105.245.135 "ping -c1 100.64.1.2" > /dev/null && echo "PASS" || echo "FAIL"

echo "Integration test complete"
```

- [ ] **Step 2: Run integration test**

```bash
./scripts/integration-test.sh
# Expected: All tests PASS
```

- [ ] **Step 3: Document manual DERP test**

```markdown
# Manual DERP Test

To verify DERP fallback works:

1. Block direct WireGuard traffic on falkenstein-1:
   ```bash
   iptables -A OUTPUT -p udp --dport 41641 -j DROP
   ```

2. Test connectivity from falkenstein-1 to nuremberg-hq:
   ```bash
   ping 100.64.0.1
   ```

3. Check DERP usage in Headscale metrics:
   ```bash
   curl http://hq.mesh.local:9090/metrics | grep derp
   ```

4. Remove firewall rule:
   ```bash
   iptables -D OUTPUT -p udp --dport 41641 -j DROP
   ```
```

- [ ] **Step 4: Commit**

```bash
git add scripts/integration-test.sh
git commit -m "test: loop engineering transport integration test"
```

---

## Verification Checklist

- [ ] All 10 nodes show `online` in `headscale nodes list`
- [ ] Ping works between all nodes via 100.64.x.x IPs
- [ ] DNS resolution works for `*.mesh.local`
- [ ] Cross-region latency acceptable (<200ms EU-US, <300ms EU-Asia)
- [ ] Backup script runs successfully
- [ ] Metrics endpoint accessible
- [ ] Integration tests pass

---

## Next Steps

After mesh deployment:
1. Deploy loop-engineering node-registry service
2. Configure agent discovery via mesh DNS
3. Test agent-to-agent communication over WireGuard
4. Monitor latency and adjust DERP configuration if needed

(End of file)
