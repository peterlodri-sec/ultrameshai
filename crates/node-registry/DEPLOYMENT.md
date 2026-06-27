# Node Registry Deployment Guide

Three deployment methods available. Choose based on your infrastructure and preferences.

---

## Prerequisites (All Methods)

1. **Build the binary:**
   ```bash
   cargo build --manifest-path crates/node-registry/Cargo.toml --release
   ```

2. **SSH access:** Ensure you have SSH key access to all target servers as root

3. **Environment variable:** Set `HEARTBEAT_SECRET` on each server (in `/etc/environment` or systemd service)

---

## Option A: Ansible (Recommended for Production)

**Best for:** Idempotent deployments, rollback support, existing Ansible infrastructure

**Requirements:**
```bash
pip install ansible
```

**Setup:**
1. Copy `inventory.ini.example` to `inventory.ini`
2. Fill in server IPs
3. Update SSH key path if needed

**Deploy:**
```bash
cd crates/node-registry/scripts
ansible-playbook -i inventory.ini deploy-ansible.yml
```

**Deploy to specific servers:**
```bash
ansible-playbook -i inventory.ini deploy-ansible.yml --limit fsn1-de-01,fsn1-de-02
```

**Verify:**
```bash
ansible all -i inventory.ini -m systemd -a "name=node-registry state=started"
```

---

## Option B: Parallel SSH (Fastest, No Dependencies)

**Best for:** Quick deployments, no extra tooling, up to 50 servers

**Requirements:** None (uses system `ssh` and `scp`)

**Setup:**
1. Edit `deploy-parallel.sh` and update `SERVERS` array with your hostnames/IPs
2. Adjust `MAX_PARALLEL` if needed (default: 5)

**Deploy:**
```bash
cd crates/node-registry/scripts
./deploy-parallel.sh
```

**Output:**
```
[INFO] Building node-registry...
[INFO] Starting parallel deployment to 10 servers (max 5 concurrent)...
[INFO] [1/10] Deploying to nuremberg-hq...
[INFO] [1/10] ✓ nuremberg-hq deployment successful
...
[INFO] All deployments successful!
```

---

## Option C: Fabric (Python-Based)

**Best for:** Python shops, custom deployment logic, programmatic control

**Requirements:**
```bash
pip install fabric invoke
```

**Setup:**
1. Edit `deploy-fabric.py` if you need to customize paths
2. Set `HOSTS` environment variable or use `-H` flag

**Deploy:**
```bash
cd crates/node-registry/scripts
fab -H nuremberg-hq,fsn1-de-01,fsn1-de-02 deploy
```

**Or with environment variable:**
```bash
HOSTS=nuremberg-hq,fsn1-de-01,fsn1-de-02 fab deploy
```

---

## Post-Deployment Verification

**Check service status:**
```bash
ssh root@<server> "systemctl status node-registry"
```

**Test health endpoint:**
```bash
curl http://<server>:3030/health
```

**Expected response:**
```json
{
  "status": "healthy",
  "total_nodes": 0,
  "online_nodes": 0,
  "offline_nodes": 0,
  "uptime_secs": 123
}
```

**Test heartbeat:**
```bash
# Generate signature (example with secret "mysecret")
SIGNATURE=$(echo -n '{"node_id":"test-node","capabilities":["gpu"],"memory_mb":16384,"load_avg":0.5,"region":"eu"}' | \
  openssl dgst -sha256 -hmac "mysecret" | awk '{print $2}')

# Send heartbeat
curl -X POST http://<server>:3030/heartbeat \
  -H "Content-Type: application/json" \
  -H "X-Signature: hmac-sha256=$SIGNATURE" \
  -d '{"node_id":"test-node","capabilities":["gpu"],"memory_mb":16384,"load_avg":0.5,"region":"eu"}'
```

**List nodes:**
```bash
curl http://<server>:3030/nodes | jq
```

---

## Troubleshooting

**Service won't start:**
```bash
ssh root@<server> "journalctl -u node-registry -n 50 --no-pager"
```

**Port already in use:**
```bash
ssh root@<server> "lsof -i :3030"
```

**Binary permission issues:**
```bash
ssh root@<server> "chmod +x /opt/node-registry/node-registry"
```

**Missing HEARTBEAT_SECRET:**
```bash
ssh root@<server> "echo 'HEARTBEAT_SECRET=your_secret_here' >> /etc/environment"
ssh root@<server> "systemctl restart node-registry"
```

---

## Rollback

**Stop service:**
```bash
# Ansible
ansible all -i inventory.ini -m systemd -a "name=node-registry state=stopped"

# SSH
ssh root@<server> "systemctl stop node-registry"

# Fabric
fab -H server1,server2 run:"systemctl stop node-registry"
```

**Restore previous binary:**
```bash
ssh root@<server> "cp /opt/node-registry/node-registry.bak /opt/node-registry/node-registry"
ssh root@<server> "systemctl restart node-registry"
```

---

## Security Notes

1. **HEARTBEAT_SECRET:** Use a strong, unique secret (32+ bytes). Store in secrets manager or SOPS-encrypted file.

2. **SSH Keys:** Use ed25519 keys with passphrase. Rotate keys periodically.

3. **Firewall:** Restrict port 3030 to internal network only:
   ```bash
   ufw allow from 10.0.0.0/8 to any port 3030 proto tcp
   ```

4. **TLS:** For production, add TLS termination via nginx or traefik in front of node-registry.
