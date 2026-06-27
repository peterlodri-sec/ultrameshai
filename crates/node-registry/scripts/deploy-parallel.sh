#!/bin/bash
# Deploy node-registry to multiple servers via parallel SSH

set -e

# Server list - update with your Hetzner servers
SERVERS=(
    "nuremberg-hq"
    "fsn1-de-01"
    "fsn1-de-02"
    "fsn1-de-03"
    "hel1-fi-01"
    "hel1-fi-02"
    "bgp-so-01"
    "bgp-so-02"
    "bgp-so-03"
    "bgp-so-04"
)

# Configuration
REMOTE_DIR="/opt/node-registry"
REMOTE_USER="root"
BINARY_PATH="target/debug/node-registry"
SYSTEMD_SERVICE="systemd/node-registry.service"
MAX_PARALLEL=5  # Max concurrent deployments

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Build binary
log_info "Building node-registry..."
cargo build --manifest-path crates/node-registry/Cargo.toml --release

# Update binary path for release build
BINARY_PATH="target/release/node-registry"

if [ ! -f "$BINARY_PATH" ]; then
    log_error "Binary not found at $BINARY_PATH"
    exit 1
fi

# Deploy to a single server
deploy_to_server() {
    local server=$1
    local idx=$2
    local total=$3
    
    log_info "[$idx/$total] Deploying to $server..."
    
    # Create remote directory
    if ! ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no "$REMOTE_USER@$server" \
        "mkdir -p $REMOTE_DIR" 2>&1; then
        log_error "[$idx/$total] Failed to create directory on $server"
        return 1
    fi
    
    # Copy binary
    if ! scp -o ConnectTimeout=10 -o StrictHostKeyChecking=no \
        "$BINARY_PATH" "$REMOTE_USER@$server:$REMOTE_DIR/node-registry" 2>&1; then
        log_error "[$idx/$total] Failed to copy binary to $server"
        return 1
    fi
    
    # Copy systemd service
    if ! scp -o ConnectTimeout=10 -o StrictHostKeyChecking=no \
        "$SYSTEMD_SERVICE" "$REMOTE_USER@$server:/etc/systemd/system/node-registry.service" 2>&1; then
        log_error "[$idx/$total] Failed to copy systemd service to $server"
        return 1
    fi
    
    # Reload and restart service
    if ! ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no "$REMOTE_USER@$server" \
        "systemctl daemon-reload && systemctl enable node-registry && systemctl restart node-registry" 2>&1; then
        log_error "[$idx/$total] Failed to restart service on $server"
        return 1
    fi
    
    # Verify service status
    if ! ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no "$REMOTE_USER@$server" \
        "systemctl is-active node-registry" 2>&1; then
        log_error "[$idx/$total] Service not active on $server"
        return 1
    fi
    
    log_info "[$idx/$total] ✓ $server deployment successful"
    return 0
}

# Main deployment loop with parallelism
log_info "Starting parallel deployment to ${#SERVERS[@]} servers (max $MAX_PARALLEL concurrent)..."

SUCCESS_COUNT=0
FAIL_COUNT=0
declare -a FAILED_SERVERS

# Track running jobs
declare -a PIDS
declare -a SERVER_PIDS

for i in "${!SERVERS[@]}"; do
    server="${SERVERS[$i]}"
    idx=$((i + 1))
    
    # Wait if we've hit max parallel jobs
    while [ ${#PIDS[@]} -ge $MAX_PARALLEL ]; do
        # Wait for any job to finish
        for j in "${!PIDS[@]}"; do
            pid="${PIDS[$j]}"
            if ! kill -0 "$pid" 2>/dev/null; then
                wait "$pid" || ((FAIL_COUNT++)) || true
                unset 'PIDS[$j]'
                unset 'SERVER_PIDS[$j]'
                PIDS=("${PIDS[@]}")
                SERVER_PIDS=("${SERVER_PIDS[@]}")
                break
            fi
        done
        sleep 1
    done
    
    # Start deployment in background
    deploy_to_server "$server" "$idx" "${#SERVERS[@]}" &
    PIDS+=($!)
    SERVER_PIDS+=("$server")
done

# Wait for all remaining jobs
for pid in "${PIDS[@]}"; do
    wait "$pid" || ((FAIL_COUNT++)) || true
done

# Summary
echo ""
log_info "========================================="
log_info "Deployment Summary"
log_info "========================================="
log_info "Total servers: ${#SERVERS[@]}"
log_info "Successful: $(( ${#SERVERS[@]} - FAIL_COUNT ))"
log_info "Failed: $FAIL_COUNT"

if [ $FAIL_COUNT -gt 0 ]; then
    log_error "Failed servers: ${FAILED_SERVERS[*]}"
    exit 1
else
    log_info "All deployments successful!"
fi
