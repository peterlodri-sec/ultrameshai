#!/bin/bash
# Deploy node-registry to multiple servers via parallel SSH

set -e

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Server lists (parallel arrays for compatibility)
SERVER_NAMES=(
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

SERVER_DESC=(
    "Nuremberg HQ (Primary)"
    "Falkenstein DE-01"
    "Falkenstein DE-02"
    "Falkenstein DE-03"
    "Helsinki FI-01"
    "Helsinki FI-02"
    "BGP SO-01"
    "BGP SO-02"
    "BGP SO-03"
    "BGP SO-04"
)

# Configuration
REMOTE_DIR="/opt/node-registry"
REMOTE_USER="root"
BINARY_PATH="$PROJECT_ROOT/target/release/node-registry"
SYSTEMD_SERVICE="$PROJECT_ROOT/crates/node-registry/systemd/node-registry.service"
MAX_PARALLEL=5  # Max concurrent deployments

# Interactive server selection
echo "========================================="
echo "  Node Registry Deployment"
echo "========================================="
echo ""
echo "Select servers to deploy to:"
echo ""

# Display numbered list
for i in "${!SERVER_NAMES[@]}"; do
    echo "  [$((i+1))] ${SERVER_DESC[$i]} (${SERVER_NAMES[$i]})"
done
echo "  [a] All servers"
echo "  [q] Quit"
echo ""

# Get selection
read -p "Enter selection (e.g., 1,3,5 or a): " selection

if [[ "$selection" == "q" ]]; then
    echo "Deployment cancelled."
    exit 0
fi

if [[ "$selection" == "a" ]]; then
    SERVERS=("${SERVER_NAMES[@]}")
else
    # Parse comma-separated numbers
    IFS=',' read -ra nums <<< "$selection"
    SERVERS=()
    for num in "${nums[@]}"; do
        num=$(echo "$num" | tr -d ' ')
        if [[ "$num" =~ ^[0-9]+$ ]] && [ "$num" -ge 1 ] && [ "$num" -le "${#SERVER_NAMES[@]}" ]; then
            SERVERS+=("${SERVER_NAMES[$((num-1))]}")
        else
            echo "Invalid selection: $num"
            exit 1
        fi
    done
fi

if [ ${#SERVERS[@]} -eq 0 ]; then
    echo "No servers selected."
    exit 1
fi

echo ""
echo "Selected servers (${#SERVERS[@]}):"
for server in "${SERVERS[@]}"; do
    echo "  - $server"
done
echo ""
read -p "Continue? (y/n): " confirm
if [[ "$confirm" != "y" ]]; then
    echo "Deployment cancelled."
    exit 0
fi

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
