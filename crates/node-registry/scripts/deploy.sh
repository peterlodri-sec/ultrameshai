#!/bin/bash
# Deploy node-registry to target server
# Usage: ./deploy.sh <server-ip-or-hostname>
# Example: ./deploy.sh 100.100.160.88

set -e

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

cd "$PROJECT_ROOT"

# Check for server argument
if [ -z "$1" ]; then
    echo "Usage: $0 <server-ip-or-hostname>"
    echo "Example: $0 100.100.160.88"
    echo ""
    echo "Available servers in tailnet:"
    tailscale status | grep -v "offline" | grep -v "lodris-macbook"
    exit 1
fi

SERVER="$1"

echo "Building node-registry (release)..."
cargo build --manifest-path crates/node-registry/Cargo.toml --release

echo "Copying binary to $SERVER..."
scp target/release/loop-engineering-node-registry root@$SERVER:/opt/node-registry/node-registry

echo "Copying systemd service..."
scp crates/node-registry/systemd/node-registry.service root@$SERVER:/etc/systemd/system/

echo "Restarting service..."
ssh root@$SERVER "systemctl daemon-reload && systemctl enable node-registry && systemctl restart node-registry"

echo "Deployment complete!"
echo "Verify: ssh root@$SERVER 'systemctl status node-registry'"
