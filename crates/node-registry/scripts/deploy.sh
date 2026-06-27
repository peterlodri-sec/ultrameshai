#!/bin/bash
# Deploy node-registry to target server

set -e

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_ROOT"

echo "Building node-registry (release)..."
cargo build --manifest-path crates/node-registry/Cargo.toml --release

echo "Copying binary to fsn1-de-01..."
scp crates/node-registry/target/release/node-registry root@fsn1-de-01:/opt/node-registry/node-registry

echo "Copying systemd service..."
scp crates/node-registry/systemd/node-registry.service root@fsn1-de-01:/etc/systemd/system/

echo "Restarting service..."
ssh root@fsn1-de-01 "systemctl daemon-reload && systemctl enable node-registry && systemctl restart node-registry"

echo "Deployment complete!"
echo "Verify: ssh root@fsn1-de-01 'systemctl status node-registry'"
