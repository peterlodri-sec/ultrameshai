#!/bin/bash
# Deploy node-registry to nuremberg-hq

set -e

echo "Building node-registry..."
cargo build --manifest-path crates/node-registry/Cargo.toml

echo "Copying binary to nuremberg-hq..."
scp crates/node-registry/target/debug/node-registry root@nuremberg-hq:/opt/node-registry/node-registry

echo "Copying systemd service..."
scp crates/node-registry/systemd/node-registry.service root@nuremberg-hq:/etc/systemd/system/

echo "Restarting service..."
ssh root@nuremberg-hq "systemctl daemon-reload && systemctl enable node-registry && systemctl restart node-registry"

echo "Deployment complete!"
echo "Verify: ssh root@nuremberg-hq 'systemctl status node-registry'"
