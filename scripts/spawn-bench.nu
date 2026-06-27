#!/usr/bin/env nu

# Spawn benchmark harness
# Measures parallel unit spawn throughput

source unit-harness.nu

def main [
  --count: int = 100       # Number of units to spawn
  --cleanup                # Clean up units after test
] {
  # Spawn units in parallel
  print $"Spawning ($count) units..."
  
  let unit_ids = (1..$count | each { |i| $"bench-unit-($i)" })
  
  # Use nanosecond timestamps
  let start_ns = (date now | into int)
  
  # Parallel spawn using nushell's par-each
  let spawn_results = ($unit_ids | par-each { |unit_id|
    do {
      let pid = (unit spawn 
        --unit-id $unit_id
        --slice-id "bench-slice"
        --loop-type "coder"
        --sandbox-tier "test"
        --nix-shell "devShells.x86_64-linux.agent-unit"
        --memory-limit 100
      )
      { unit_id: $unit_id, pid: $pid, success: true }
    } catch { |err|
      { unit_id: $unit_id, pid: 0, success: false, error: ($err | into string) }
    }
  })

  let end_ns = (date now | into int)
  let elapsed_ms = (($end_ns - $start_ns) / 1000000)
  let elapsed_sec = (if $elapsed_ms > 0 { $elapsed_ms / 1000.0 } else { 0.001 })
  
  # Count successes
  let spawned_count = ($spawn_results | where success | length)
  let failed_count = ($spawn_results | where {|r| not $r.success } | length)
  let units_per_sec = ($spawned_count / $elapsed_sec)

  # Summary
  print ""
  print "=== Spawn Benchmark Summary ==="
  print $"Total time:      ($elapsed_ms) ms"
  print $"Units spawned:   ($spawned_count) / ($count)"
  print $"Failed:          ($failed_count)"
  print $"Throughput:      ($units_per_sec | math round --precision 2) units/sec"
  
  # Show failures if any
  if $failed_count > 0 {
    print ""
    print "Failed units:"
    ($spawn_results | where {|r| not $r.success } | each { |r| 
      print $"  - ($r.unit_id): ($r.error)"
    })
  }

  # Cleanup if requested
  if $cleanup {
    print ""
    print "Cleaning up units..."
    ($unit_ids | each { |unit_id|
      let workdir = $"/tmp/units/($unit_id)"
      if ($workdir | path exists) {
        rm -r $workdir
      }
    })
    print "Cleanup complete."
  }

  # Return summary as JSON
  {
    total_time_ms: $elapsed_ms,
    units_spawned: $spawned_count,
    units_failed: $failed_count,
    units_per_second: $units_per_sec,
    cleanup_performed: $cleanup
  }
}
