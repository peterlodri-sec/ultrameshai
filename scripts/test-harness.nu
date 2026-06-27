#!/usr/bin/env nu

# Test: spawn a unit, verify it exists, kill it, verify stats
source unit-harness.nu

print "Testing unit spawn..."
let pid = (unit spawn --unit-id "test-001" --slice-id "slice-001" --loop-type "coder" --sandbox-tier "standard" --nix-shell ".#agent-unit")
print $"Spawned unit with PID: ($pid)"

# Verify manifest exists
let manifest = (open "/tmp/units/test-001/manifest.json")
if $manifest.unit_id != "test-001" { error make { msg: "manifest unit_id mismatch" } }
print "Manifest OK"

# Wait for unit to be ready
sleep 200ms
# Kill it
unit kill "test-001" $pid
print "Killed unit"

# Verify stats
let stats = (unit stats "test-001")
if $stats.status != "killed" { error make { msg: "stats status mismatch" } }
print $"Stats OK: ($stats.status)"

print "All harness tests passed!"