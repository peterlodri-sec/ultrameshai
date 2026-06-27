#!/usr/bin/env nu

# UltrameshAI Telemetry Dashboard
# Displays live stats, loop performance, and resource utilization from mempalace.db

def main [
  --db-path: string = "mempalace.db"
] {
  if not ($db-path | path exists) {
    print $"(ansi red_bold)Error: Database file '($db-path)' not found.(ansi reset)"
    print "Please run some agent units first to populate telemetry."
    return
  }

  print $"(ansi cyan_bold)=================================================================(ansi reset)"
  print $"(ansi white_bold)🛸 UltrameshAI Fleet Telemetry Dashboard (mempalace)(ansi reset)"
  print $"(ansi cyan_bold)=================================================================(ansi reset)"
  print ""

  # 1. Overall Status Summary
  print $"(ansi green_bold)📊 Overall Execution Summary:(ansi reset)"
  let status_raw = (sqlite3 -json $db-path "
    SELECT status, COUNT(*) as count 
    FROM unit_stats 
    GROUP BY status
  " | from json)

  if ($status_raw | is-empty) {
    print "No telemetry records found."
    return
  }

  let total_units = ($status_raw | math sum | get count)
  let status_table = ($status_raw | append { status: "total", count: $total_units })
  print ($status_table | table --expand)
  print ""

  # 2. Loop Type Performance & Memory Metrics
  print $"(ansi green_bold)⚙️ Loop Type Performance & Resource Metrics:(ansi reset)"
  let loop_metrics = (sqlite3 -json $db-path "
    SELECT 
      loop_type, 
      COUNT(*) as executions,
      ROUND(AVG(died_at_ms - spawned_at_ms)) as avg_runtime_ms,
      ROUND(AVG(peak_memory_mb), 1) as avg_memory_mb,
      MAX(peak_memory_mb) as max_memory_mb
    FROM unit_stats 
    GROUP BY loop_type
  " | from json)

  if not ($loop_metrics | is-empty) {
    print ($loop_metrics | table --expand)
  } else {
    print "No loop metrics available."
  }
  print ""

  # 3. Recent Executions
  print $"(ansi green_bold)⏱️ Last 10 Unit Executions:(ansi reset)"
  let recent_units = (sqlite3 -json $db-path "
    SELECT 
      unit_id, 
      slice_id, 
      loop_type, 
      status, 
      (died_at_ms - spawned_at_ms) as runtime_ms, 
      peak_memory_mb,
      snapshot_path
    FROM unit_stats 
    ORDER BY died_at_ms DESC 
    LIMIT 10
  " | from json)

  if not ($recent_units | is-empty) {
    print ($recent_units | table --expand)
  } else {
    print "No recent executions found."
  }
}
