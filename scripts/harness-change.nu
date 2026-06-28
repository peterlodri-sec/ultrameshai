# scripts/harness-change.nu
# Nushell script to manage ECL changes

# Create a new change
def "main create" [name: string] {
    let date = (date now | format date "%Y-%m-%d")
    let change_dir = $"harness/changes/active/($date)-($name)"
    
    if ("harness/changes/active" | path exists) and (ls harness/changes/active | length) > 0 {
        print "Error: An active change already exists. Park it first."
        exit 1
    }
    
    mkdir $change_dir
    
    # Create templates
    $"# Summary: ($name)\n\nCreated: ($date)\n" | save $"($change_dir)/summary.md"
    $"# Spec: ($name)\n\n## Requirements\n" | save $"($change_dir)/spec.md"
    $"# Plan: ($name)\n\n## Steps\n" | save $"($change_dir)/plan.md"
    $"# Tasks: ($name)\n\n- [ ] Task 1\n" | save $"($change_dir)/tasks.md"
    
    print $"Created new active change: ($change_dir)"
}

# Park the active change
def "main park" [] {
    let active = (ls harness/changes/active)
    if ($active | is-empty) {
        print "No active change to park."
        exit 0
    }
    
    let active_dir = $active.0.name
    let target = $"harness/changes/parking/($active_dir | path basename)"
    
    mkdir "harness/changes/parking"
    mv $active_dir $target
    print $"Parked change to ($target)"
}

# Archive the active change
def "main archive" [] {
    let active = (ls harness/changes/active)
    if ($active | is-empty) {
        print "No active change to archive."
        exit 0
    }
    
    let active_dir = $active.0.name
    let target = $"harness/changes/archive/($active_dir | path basename)"
    
    mkdir "harness/changes/archive"
    mv $active_dir $target
    print $"Archived change to ($target)"
}

def main [] {
    print "Usage: nu scripts/harness-change.nu [create <name> | park | archive]"
}
