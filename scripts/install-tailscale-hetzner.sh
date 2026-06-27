#!/bin/bash
# Install Tailscale on all Hetzner servers using hcloud API
# Requires: HCLOUD_TOKEN env var

set -e

export HCLOUD_TOKEN="$HETZNER_API_KEY"

if ! command -v hcloud &> /dev/null; then
    echo "ERROR: hcloud CLI required. Install: brew install hcloud"
    exit 1
fi

echo "=== Install Tailscale on Hetzner Servers ==="
echo ""

# Get all running servers
SERVERS=$(hcloud server list -o noheader -o columns=name,ipv4)

TS_AUTHKEY="tskey-auth-k8yPzBmtR821CNTRL-X1aZtuWWT6gQiaYfVDmh5g7UYHcfWrqz"

while IFS=' ' read -r name ip; do
    echo "[$name] ($ip) Installing Tailscale..."
    
    # Install Tailscale
    hcloud ssh "$name" << 'EOF'
        if command -v tailscale &> /dev/null; then
            echo "  Tailscale already installed"
        else
            curl -fsSL https://tailscale.com/install.sh | sh
        fi
EOF
    
    # Authenticate
    hcloud ssh "$name" "tailscale up --authkey=$TS_AUTHKEY --hostname=$name"
    
    echo "  ✓ Added to tailnet"
    echo ""
done <<< "$SERVERS"

echo "=== All servers added ==="
echo "Verify: tailscale status"
