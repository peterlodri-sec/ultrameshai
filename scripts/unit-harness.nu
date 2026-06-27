#!/usr/bin/env nu

# Agent unit lifecycle harness
# Manages spawn/snapshot/kill for agent units

# Spawn a new agent unit
# Returns the unit's PID
def "unit spawn" [
  --unit-id: string      # Unique unit ID
  --slice-id: string     # E2E slice this unit is bound to
  --loop-type: string    # "coder", "tester", "red-team", etc.
  --sandbox-tier: string # "standard", "test", "red-team", "devops"
  --nix-shell: string    # nix shell path
  --memory-limit: int = 100  # soft memory cap in MB
] {
  # Create unit working directory
  let workdir = $"/tmp/units/$unit_id"
  mkdir $workdir

  # Write unit manifest
  {
    unit_id: $unit_id,
    slice_id: $slice_id,
    loop_type: $loop_type,
    sandbox_tier: $sandbox_tier,
    nix_shell: $nix_shell,
    memory_limit_mb: $memory_limit,
    spawned_at: (date now | into int),
    workdir: $workdir,
  } | to json | save $"($workdir)/manifest.json"

  # Spawn the unit process in a nix shell
  # The unit process reads its manifest and starts working
  let pid = (nix develop $nix-shell --command nu -c $"echo 'unit ($unit_id) spawned' | save ($workdir)/log.txt; sleep 3600" --background)

  # Track memory (background watcher)
  spawn-memory-watcher $unit_id $memory_limit $workdir $pid

  $pid
}

# Spawn a background memory watcher for a unit
def spawn-memory-watcher [
  unit_id: string
  memory_limit: int
  workdir: string
  pid: int
] {
  # Elastic: soft at memory_limit, kill at memory_limit * 1.6
  let kill_limit = ($memory_limit * 160 / 100 | math floor)

  # Background task: poll /proc/$pid/status for VmRSS
  # If > kill_limit, snapshot + kill
  # This is a simplified version — production uses cgroups
  loop {
    sleep 100ms
    let rss = (try { cat $"/proc/($pid)/status" | lines | where $it =~ "VmRSS" | first | split row " " | last | into int } catch { 0 })
    if $rss > ($kill_limit * 1024) {
      # Snapshot: copy workdir to snapshot path
      cp -r $workdir $"($workdir)/snapshot_((date now | into int))"
      # Kill
      kill $pid
      # Write death stats
      {
        unit_id: $unit_id,
        status: "killed",
        peak_memory_mb: ($rss / 1024 | math floor),
        died_at: (date now | into int),
      } | to json | save $"($workdir)/stats.json"
      break
    }
  } &
}

# Snapshot a unit's state (without killing)
def "unit snapshot" [
  unit_id: string
] {
  let workdir = $"/tmp/units/$unit_id"
  let snapshot_path = $"($workdir)/snapshot_((date now | into int))"
  cp -r $workdir $snapshot_path
  $snapshot_path
}

# Kill a unit (with snapshot)
def "unit kill" [
  unit_id: string
  pid: int
] {
  let workdir = $"/tmp/units/$unit_id"
  let snapshot_path = $"($workdir)/snapshot_((date now | into int))"
  cp -r $workdir $snapshot_path
  kill $pid
  {
    unit_id: $unit_id,
    status: "killed",
    snapshot_path: $snapshot_path,
    died_at: (date now | into int),
  } | to json | save $"($workdir)/stats.json"
}

# Get unit stats (read the stats.json written on death)
def "unit stats" [
  unit_id: string
] {
  let workdir = $"/tmp/units/$unit_id"
  open $"($workdir)/stats.json"
}