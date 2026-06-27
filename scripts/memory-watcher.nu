#!/usr/bin/env nu

# Memory watcher — polls /proc/$pid/status for VmRSS
# If over kill limit, snapshots workdir + kills process + writes stats.json
# Invoked as: nu memory-watcher.nu <unit_id> <memory_limit_mb> <workdir> <pid>

let unit_id = $env.UNIT_ID
let memory_limit = ($env.MEMORY_LIMIT | into int)
let workdir = $env.WORKDIR
let pid = ($env.PID | into int)

let kill_limit = ($memory_limit * 160 / 100 | math floor)

loop {
  sleep 100ms
  let rss = (try { cat $"/proc/($pid)/status" | lines | where $it =~ "VmRSS" | first | split row " " | last | into int } catch { null })
  
  # Process no longer exists - natural death
  if $rss == null {
    {
      unit_id: $unit_id,
      status: "exited",
      died_at: (date now | into int),
    } | to json | save $"($workdir)/stats.json"
    exit
  }
  
  # Over memory limit - kill
  if $rss > ($kill_limit * 1024) {
    cp -r $workdir $"($workdir)/snapshot_((date now | into int))"
    kill $pid
    {
      unit_id: $unit_id,
      status: "killed",
      peak_memory_mb: ($rss / 1024 | math floor),
      died_at: (date now | into int),
    } | to json | save $"($workdir)/stats.json"
    exit
  }
}