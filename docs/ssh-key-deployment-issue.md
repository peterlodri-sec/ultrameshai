# SSH Key Deployment Issue - Hetzner Servers

## Problem

Hetzner servers created without SSH keys cannot have keys added via API after creation. The `hcloud server add-ssh-key` command does not exist.

**Affected servers:**
- agent-node-01 (178.105.184.32 / 100.100.160.88 via Tailscale)
- All other Hetzner servers created without SSH keys

## Available SSH Keys

```
ID          NAME                                        FINGERPRINT
111977152   peter.lodri@instructure.com                 a5:3a:12:fa:53:0a:a5:71:b6:a0:4b:4c:5b:dd:3a:90
111977201   267054113+peterlodri-sec@users.noreply.github.com  09:e0:7a:5a:e9:7e:21:88:26:91:b4:65:cd:ed:d0:af
112362574   root@hetzner-server-new                    6e:7b:66:b9:6a:e5:c3:33:22:c3:0b:47:7e:06:ed:44
112362582   gh-runner@hetzner-server-new                1a:2d:48:da:24:02:04:f1:e4:46:73:dd:e7:5f:3a:9c
112541468   agentfield-v01-mvp-deploy                   ff:41:26:7c:4d:b6:d7:0c:4c:23:59:58:34:dc:96:a2
112952845   lego-api-deploy                             8d:65:36:5d:23:cd:ab:9d:87:79:7e:9c:9e:a2:2f:6e
```

## Solutions

### Option 1: Enable Rescue Mode (Recommended for existing servers)

```bash
# 1. Enable rescue mode
hcloud server enable-rescue agent-node-01 --type linux64

# 2. Reboot into rescue
hcloud server reboot agent-node-01

# 3. Wait for reboot, then SSH with rescue password
hcloud server request-console agent-node-01

# 4. Mount root partition and add SSH key
# (Use console to edit /etc/ssh/sshd_config and add key to ~/.ssh/authorized_keys)

# 5. Disable rescue and reboot
hcloud server disable-rescue agent-node-01
```

### Option 2: Recreate Server with SSH Key

```bash
# 1. Create image of current server
hcloud server create-image agent-node-01 --name agent-node-01-backup

# 2. Delete old server
hcloud server delete agent-node-01

# 3. Recreate with SSH key
hcloud server create agent-node-01 \
  --type cpx22 \
  --image agent-node-01-backup \
  --ssh-key agentfield-v01-mvp-deploy \
  --ssh-key peter.lodri@instructure.com \
  --location nbg1 \
  --public-net-ipv4 178.105.184.32  # Reuse existing IP if possible
```

### Option 3: Use Cloud-Init on Future Servers

When creating new servers, always include SSH keys:

```bash
hcloud server create <name> \
  --type cpx22 \
  --image ubuntu-24.04 \
  --location nbg1 \
  --ssh-key agentfield-v01-mvp-deploy \
  --ssh-key peter.lodri@instructure.com
```

## Nix Integration

For NixOS servers, SSH keys can be declaratively managed in `configuration.nix`:

```nix
{
  users.users.root.openssh.authorizedKeys.keys = [
    "ssh-ed25519 AAAA... peter.lodri@instructure.com"
    "ssh-ed25519 BBBB... agentfield-v01-mvp-deploy"
  ];
}
```

## Current Status

- ✅ Tailscale installed on all 10 Hetzner servers
- ✅ Node registry binary builds successfully
- ❌ SSH access blocked (no keys attached)
- ⏳ Deployment pending SSH key resolution

## Next Steps

1. Choose Option 1 (rescue mode) or Option 2 (recreate) for agent-node-01
2. Test SSH access: `ssh root@100.100.160.88`
3. Deploy node-registry: `./scripts/deploy.sh 100.100.160.88`
4. Verify service: `ssh root@100.100.160.88 'systemctl status node-registry'`

## Reference

- Hetzner Cloud API: https://docs.hetzner.cloud/
- SSH Key Management: https://docs.hetzner.cloud/#servers-add-an-ssh-key-to-a-server
- Rescue System: https://docs.hetzner.cloud/#rescue-system
