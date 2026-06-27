#!/usr/bin/env python3
"""
Fabric deployment script for node-registry
Usage: fab -H server1,server2,server3 deploy
"""

from fabric import Connection, task
from invoke import run
import os

# Configuration
REMOTE_DIR = "/opt/node-registry"
REMOTE_USER = "root"
BINARY_PATH = "target/release/node-registry"
SYSTEMD_SERVICE = "systemd/node-registry.service"

@task
def deploy(c):
    """Deploy node-registry to remote servers"""
    
    # Build binary
    print("Building node-registry...")
    run("cargo build --manifest-path crates/node-registry/Cargo.toml --release", warn=True)
    
    if not os.path.exists(BINARY_PATH):
        print(f"Binary not found at {BINARY_PATH}")
        return
    
    # Get host list
    hosts = os.environ.get('HOSTS', '').split(',')
    if not hosts or hosts == ['']:
        print("No hosts specified. Use: fab -H server1,server2 deploy")
        return
    
    success_count = 0
    fail_count = 0
    
    for host in hosts:
        host = host.strip()
        if not host:
            continue
            
        try:
            print(f"\n[{host}] Deploying...")
            
            # Connect
            conn = Connection(f"{REMOTE_USER}@{host}")
            
            # Create directory
            conn.run(f"mkdir -p {REMOTE_DIR}")
            
            # Copy binary
            conn.put(BINARY_PATH, f"{REMOTE_DIR}/node-registry")
            conn.run(f"chmod +x {REMOTE_DIR}/node-registry")
            
            # Copy systemd service
            conn.put(SYSTEMD_SERVICE, "/etc/systemd/system/node-registry.service")
            
            # Reload and restart
            conn.run("systemctl daemon-reload")
            conn.run("systemctl enable node-registry")
            conn.run("systemctl restart node-registry")
            
            # Verify
            result = conn.run("systemctl is-active node-registry", hide=True)
            if result.ok:
                print(f"[{host}] ✓ Deployment successful")
                success_count += 1
            else:
                print(f"[{host}] ✗ Service not active")
                fail_count += 1
                
        except Exception as e:
            print(f"[{host}] ✗ Error: {e}")
            fail_count += 1
    
    # Summary
    print(f"\n{'='*50}")
    print(f"Deployment Summary")
    print(f"{'='*50}")
    print(f"Total: {len(hosts)}")
    print(f"Success: {success_count}")
    print(f"Failed: {fail_count}")
    
    if fail_count > 0:
        exit(1)
