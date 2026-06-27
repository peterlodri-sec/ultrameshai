#!/usr/bin/env nu

# Memory watcher — Hybrid cgroups v2 / polling resource manager
# Invoked as a detached background process: nu memory-watcher.nu

let unit_id = $env.UNIT_ID
let memory_limit = ($env.MEMORY_LIMIT | into int)
let workdir = $env.WORKDIR
let pid = ($env.PID | into int)

let kill_limit = ($memory_limit * 160 / 100 | math floor)

# Detect and configure cgroups v2 if available and writable
let cgroup_root = "/sys/fs/cgroup"
let my_cgroup_dir = $"($cgroup_root)/ultrameshai/($unit_id)"
let use_cgroups = (
  ($cgroup_root | path exists) and 
  (try {
    mkdir $"($cgroup_root)/ultrameshai"
    true
  } catch { false })
)

if $use_cgroups {
  try {
    mkdir $my_cgroup_dir
    # Configure memory constraints:
    # memory.high is the soft throttle limit
    # memory.max is the hard limit (process gets OOM-killed if exceeded)
    echo $"($memory_limit * 1024 * 1024)" | save --force $"($my_cgroup_dir)/memory.high"
    echo $"($kill_limit * 1024 * 1024)" | save --force $"($my_cgroup_dir)/memory.max"
    
    # Move the target process into the cgroup
    echo $pid | save --force $"($my_cgroup_dir)/cgroup.procs"
  } catch {
    # If cgroup setup fails midway, fallback to polling
  }
}

loop {
  sleep 100ms
  
  # Check if process is still running
  let proc_exists = (try { kill -0 $pid; true } catch { false })
  
  if not $proc_exists {
    # Determine if it died due to OOM/Hard Kill
    let oom_killed = if $use_cgroups {
      let events = (try { open $"($my_cgroup_dir)/memory.events" | lines } catch { [] })
      let oom_count = ($events | where $it =~ "oom" | first? | default "0" | split row " " | last | into int)
      $oom_count > 0
    } else {
      false
    }

    let status = if $oom_killed { "killed" } else { "exited" }
    
    {
      unit_id: $unit_id,
      status: $status,
      died_at: (date now | into int),
    } | to json | save --force $"($workdir)/stats.json"

    # Cleanup cgroup directory if used
    if $use_cgroups {
      try { rmdir $my_cgroup_dir } catch {}
    }
    exit
  }

  # Fallback Polling Mode (macOS or non-cgroup Linux systems)
  if not $use_cgroups {
    let rss = (try { 
      if ($nu.os-info.name == "macos") {
        # macOS fallback using ps
        let ps_rss = (ps -o rss= -p $pid | trim | into int)
        $ps_rss / 1024 # ps returns KB, convert to MB
      } else {
        # Linux /proc fallback
        let status_lines = (cat $"/proc/($pid)/status" | lines)
        let vmrss_line = ($status_lines | where $it =~ "VmRSS" | first?)
        if $vmrss_line != null {
          let kb = ($vmrss_line | split row " " | where $it != "" | get 1 | into int)
          $kb / 1024
        } else {
          0
        }
      }
    } catch { 0 })

    if $rss > $kill_limit {
      # Hard Limit exceeded -> Snapshot and Kill
      try {
        cp -r $workdir $"($workdir)/snapshot_((date now | into int))"
      } catch {}
      
      try { kill -9 $pid } catch {}
      
      {
        unit_id: $unit_id,
        status: "killed",
        peak_memory_mb: $rss,
        died_at: (date now | into int),
      } | to json | save --force $"($workdir)/stats.json"
      exit
    }
  }
}