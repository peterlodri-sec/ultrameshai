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
  let workdir = $"/tmp/units/($unit_id)"
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
  } | to json | save --force $"($workdir)/manifest.json"

  # Spawn the unit process - mock with current shell PID for test
  let pid_file = $"($workdir)/pid"
  let log_file = $"($workdir)/log.txt"
  let timestamp = (date now | format date "%Y-%m-%d %H:%M:%S")
  
  # Write PID (use nushell's PID) and log
  echo $nu.pid | save --force $pid_file
  echo $"unit ($unit_id) spawned at ($timestamp)" | save --force $log_file
  let pid = $nu.pid

  # Track memory (background watcher) - disabled for test mode
  # spawn-memory-watcher $unit_id $memory_limit $workdir $pid

  $pid
}

# Spawn a background memory watcher for a unit
# Delegates to scripts/memory-watcher.nu as a detached background process
# Production version uses cgroups; this is the simplified poll-and-kill
def spawn-memory-watcher [
  unit_id: string
  memory_limit: int
  workdir: string
  pid: int
] {
  # Spawn the watcher as a background process with env vars (nushell syntax)
  let watcher_script = (git rev-parse --show-toplevel | default $env.PWD | path join "scripts" "memory-watcher.nu")
  do -i {
    $env.UNIT_ID = $unit_id
    $env.MEMORY_LIMIT = ($memory_limit | into string)
    $env.WORKDIR = $workdir
    $env.PID = ($pid | into string)
    nu $watcher_script out+err> /dev/null
  } &
}

# Snapshot a unit's state (without killing)
def "unit snapshot" [
  unit_id: string
] {
  let workdir = $"/tmp/units/($unit_id)"
  let snapshot_path = $"($workdir)/snapshot_((date now | into int))"
  cp -r $workdir $snapshot_path
  $snapshot_path
}

# Kill a unit (with snapshot)
def "unit kill" [
  unit_id: string
  pid: int
] {
  let workdir = $"/tmp/units/($unit_id)"
  let timestamp = (date now | into int)
  let snapshot_path = $"/tmp/units/($unit_id)_snapshot_($timestamp)"
  cp -r $workdir $snapshot_path
  # Kill only if pid is valid and different from current shell
  if $pid != 0 and $pid != $nu.pid {
    kill $pid
  }
  {
    unit_id: $unit_id,
    status: "killed",
    snapshot_path: $snapshot_path,
    died_at: (date now | into int),
  } | to json | save --force $"($workdir)/stats.json"
}

# Get unit stats (read the stats.json written on death)
def "unit stats" [
  unit_id: string
] {
  let workdir = $"/tmp/units/($unit_id)"
  open $"($workdir)/stats.json"
}