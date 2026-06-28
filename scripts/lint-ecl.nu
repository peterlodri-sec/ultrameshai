# scripts/lint-ecl.nu
# Nushell script to lint ECL directory structure and files

def main [] {
    print "🔍 Linting ECL changes..."
    mut errors = 0
    
    let active_dirs = if ("harness/changes/active" | path exists) { ls harness/changes/active } else { [] }
    let archive_dirs = if ("harness/changes/archive" | path exists) { ls harness/changes/archive } else { [] }
    let parking_dirs = if ("harness/changes/parking" | path exists) { ls harness/changes/parking } else { [] }
    
    let all_changes = ($active_dirs | append $archive_dirs | append $parking_dirs)
    
    for change in $all_changes {
        if $change.type == "dir" {
            let required_files = ["summary.md", "spec.md", "plan.md", "tasks.md"]
            for file in $required_files {
                let file_path = $"($change.name)/($file)"
                if not ($file_path | path exists) {
                    print $"❌ Missing required file: ($file_path)"
                    $errors = $errors + 1
                }
            }
        }
    }
    
    if $errors > 0 {
        print $"❌ ECL lint failed with ($errors) error(s)."
        exit 1
    } else {
        print "✅ ECL lint passed."
    }
}
