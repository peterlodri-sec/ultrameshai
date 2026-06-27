#!/bin/bash
# Add Hetzner servers to Tailscale
# Requires: TS_AUTHKEY env var set from https://login.tailscale.com/admin/settings/keys

set -e

if [ -z "$TS_AUTHKEY" ]; then
    echo "ERROR: TS_AUTHKEY env var required"
    echo "Generate at: https://login.tailscale.com/admin/settings/keys"
    exit 1
fi

# Server list - Hetzner Cloud
# Format: "hostname:root@ip"
SERVERS=(
    "nuremberg-hq:root@167.233.148.20"
    "fsn1-de-01:root@178.105.245.135"
    "fsn1-de-02:root@167.233.105.32"
    "fsn1-de-03:root@178.105.184.32"
    "hel1-fi-01:root@89.167.80.207"
    "hel1-fi-02:root@5.223.79.65"
    "bgp-so-01:root@167.233.35.194"
    "bgp-so-02:root@46.225.127.20"
    "bgp-so-03:root@178.104.47.201"
    "bgp-so-04:root@5.78.122.125"
)

echo "=== Add Hetzner Servers to Tailscale ==="
echo ""

for entry in "${SERVERS[@]}"; do
    name="${entry%%:*}"
    ssh_target="${entry#*:}"
    
    echo "[$name] Installing Tailscale..."
    
    # Install Tailscale
    ssh "$ssh_target" << 'EOF'
        # Check if already installed
        if command -v tailscale &> /dev/null; then
            echo "Tailscale already installed"
            tailscale --version
        else
            # Install via tailscale script with checksum verification
            curl -fsSL https://tailscale.com/install.sh -o /tmp/tailscale-install.sh
            curl -fsSL https://tailscale.com/install.sh.sha256 -o /tmp/tailscale-install.sh.sha256
            (cd /tmp && sha256sum -c tailscale-install.sh.sha256) && sh /tmp/tailscale-install.sh
            rm -f /tmp/tailscale-install.sh /tmp/tailscale-install.sh.sha256
        fi
EOF
    
    echo "[$name] Authenticating..."
    
    # Authenticate with auth key
    ssh "$ssh_target" "tailscale up --authkey=${TS_AUTHKEY} --hostname=${name}"
    
    echo "[$name] ✓ Added to tailnet"
    echo ""
done

echo "=== All servers added ==="
echo "Verify: tailscale status"
