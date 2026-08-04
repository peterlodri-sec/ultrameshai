#!/usr/bin/env nu

# E2E Integration Test — Node Registry + Transport + Mempalace
#
# Usage: nu scripts/test-e2e-integration.nu

const PORT = 3000
const HOST = $"http://localhost:(3000)"
const SECRET = "test-secret-change-me-in-production"

def main [] {
    print "=== Node Registry E2E Integration Test ==="
    print ""

    # Check port availability
    let port_check = (try { curl -s -o /dev/null -w "%{http_code}" ($HOST + "/health") --connect-timeout 1 } catch { "000" })
    if $port_check != "000" {
        print $"❌ Port (3000) is already in use (response: ($port_check))"
        exit 1
    }
    print "✅ Port 3000 is free"

    # Build
    print ""
    print "Building node-registry..."
    cargo build -p loop-engineering-node-registry out+err> /dev/null
    if $env.LAST_EXIT_CODE != 0 {
        print "❌ Build failed"
        exit 1
    }
    print "✅ Build OK"

    # Start daemon
    print ""
    print "Starting node-registry daemon..."
    let daemon_log = "/tmp/node-registry-e2e.log"
    rm -f $daemon_log

    let pid = (do -i {
        $env.HEARTBEAT_SECRET = $SECRET
        $env.NODE_REGISTRY_ADDR = "0.0.0.0:3000"
        $env.NODE_REGISTRY_TAILNET = "test.ts.net"
        $env.STALE_THRESHOLD_SECS = "300"
        $env.POLL_INTERVAL_SECS = "3600"
        cargo run -p loop-engineering-node-registry out+err> $daemon_log &
    })
    print $"Daemon PID: ($pid)"

    # Wait for readiness
    print "Waiting for daemon to be ready..."
    mut ready = false
    for i in 1..20 {
        let status = (try { curl -s -o /dev/null -w "%{http_code}" ($HOST + "/health") --connect-timeout 1 } catch { "000" })
        if $status == "200" {
            $ready = true
            break
        }
        sleep 250ms
    }
    if not $ready {
        print "❌ Daemon failed to start within 5 seconds"
        let log = (try { open $daemon_log } catch { "(no log)" })
        print $"(char nl)Last log lines:"
        print ($log | str substring [..2000])
        exit 1
    }
    print "✅ Daemon is ready"

    # Test 1: Health check
    print ""
    print "--- Test 1: GET /health ---"
    let health_str = (curl -s ($HOST + "/health"))
    print $health_str
    let health = ($health_str | from json)
    if $health.status != "healthy" {
        print $"❌ Health status is '($health.status)', expected 'healthy'"
        exit 1
    }
    print "✅ Health check OK"

    # Test 2: Signed heartbeat
    print ""
    print "--- Test 2: POST /heartbeat (signed) ---"
    let payload = '{"node_id":"test-node-01","capabilities":["standard","test"],"memory_mb":65536,"load_avg":0.42,"region":"test-lab"}'
    let sig = (echo $payload | openssl dgst -sha256 -hmac $SECRET -hex | str trim | split row " " | last)
    let hdr = $"x-signature: hmac-sha256=($sig)"
    let hb_code = (curl -s -X POST ($HOST + "/heartbeat") -H "Content-Type: application/json" -H $hdr -d $payload -o /dev/null -w "%{http_code}")
    if $hb_code != "200" {
        print $"❌ Heartbeat returned ($hb_code), expected 200"
        exit 1
    }
    print "✅ Heartbeat accepted (200)"

    # Test 3: Health after heartbeat
    print ""
    print "--- Test 3: GET /health (after heartbeat) ---"
    let health2_str = (curl -s ($HOST + "/health"))
    print $health2_str
    let health2 = ($health2_str | from json)
    if $health2.total_nodes < 1 {
        print $"❌ Expected >=1 node, got ($health2.total_nodes)"
        exit 1
    }
    print "✅ Node count OK"

    # Test 4: List nodes
    print ""
    print "--- Test 4: GET /nodes ---"
    let nodes_str = (curl -s ($HOST + "/nodes"))
    let nodes = ($nodes_str | from json)
    print $"Nodes: ($nodes | length)"
    let found = ($nodes | where $it.metadata.node_id == "test-node-01" | length)
    if $found == 0 {
        print "❌ Expected node 'test-node-01' in /nodes"
        exit 1
    }
    print "✅ Node list OK"

    # Test 5: Update heartbeat
    print ""
    print "--- Test 5: POST /heartbeat (update) ---"
    let payload2 = '{"node_id":"test-node-01","capabilities":["standard","test","red-team"],"memory_mb":131072,"load_avg":0.15,"region":"test-lab"}'
    let sig2 = (echo $payload2 | openssl dgst -sha256 -hmac $SECRET -hex | str trim | split row " " | last)
    let hdr2 = $"x-signature: hmac-sha256=($sig2)"
    let hb2_code = (curl -s -X POST ($HOST + "/heartbeat") -H "Content-Type: application/json" -H $hdr2 -d $payload2 -o /dev/null -w "%{http_code}")
    if $hb2_code != "200" {
        print $"❌ Heartbeat update returned ($hb2_code), expected 200"
        exit 1
    }
    # Verify updated memory
    let nodes2_str = (curl -s ($HOST + "/nodes"))
    let nodes2 = ($nodes2_str | from json)
    let updated = ($nodes2 | where $it.metadata.node_id == "test-node-01" | first)
    if $updated.metadata.memory_mb != 131072 {
        print $"❌ Expected memory_mb=131072, got ($updated.metadata.memory_mb)"
        exit 1
    }
    print "✅ Heartbeat update OK (memory_mb=131072)"

    # Test 6: Unsigned heartbeat rejected
    print ""
    print "--- Test 6: POST /heartbeat (unsigned, expect 401) ---"
    let unsigned_code = (curl -s -X POST ($HOST + "/heartbeat") -H "Content-Type: application/json" -d $payload -o /dev/null -w "%{http_code}")
    if $unsigned_code != "401" {
        print $"❌ Unsigned heartbeat returned ($unsigned_code), expected 401"
        exit 1
    }
    print "✅ Unsigned heartbeat rejected (401)"

    # Cleanup
    print ""
    print "--- Cleanup ---"
    let daemon_pid = (try { lsof -ti tcp:3000 } catch { "" })
    if ($daemon_pid | str trim | str length) > 0 {
        kill --signal 15 ($daemon_pid | str trim | into int)
        sleep 500ms
        print "✅ Daemon shut down gracefully"
    } else {
        print "⚠️  Could not find daemon process"
    }

    print ""
    print "=== All E2E integration tests passed! ==="
}
