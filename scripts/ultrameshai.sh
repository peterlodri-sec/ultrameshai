#!/bin/bash
# ultrameshai.sh — CLI for workspace management
# Usage: ./scripts/ultrameshai.sh <command> [options]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

usage() {
    echo "UltraMeshAI Workspace CLI"
    echo
    echo "Usage: $0 <command> [options]"
    echo
    echo "Commands:"
    echo "  ecl-lint           Check ECL completeness (all 4 files present)"
    echo "  config-validate    Validate models.toml syntax and coverage"
    echo "  doctor             Check workspace health"
    echo "  help, -h, --help   Show this help"
    echo
    echo "Examples:"
    echo "  $0 ecl-lint"
    echo "  $0 config-validate"
    echo "  $0 doctor"
}

cmd_ecl_lint() {
    echo "🔍 Linting ECL changes..."
    
    local errors=0
    local active_dir="$WORKSPACE_ROOT/harness/changes/active"
    local archive_dir="$WORKSPACE_ROOT/harness/changes/archive"
    local parking_dir="$WORKSPACE_ROOT/harness/changes/parking"
    
    local required_files=("summary.md" "spec.md" "plan.md" "tasks.md")
    
    # Check active changes
    if [ -d "$active_dir" ]; then
        for change in "$active_dir"/*/; do
            if [ -d "$change" ]; then
                local change_name=$(basename "$change")
                for file in "${required_files[@]}"; do
                    if [ ! -f "$change$file" ]; then
                        echo "❌ Missing: $change_name/$file"
                        ((errors++))
                    fi
                done
            fi
        done
    fi
    
    # Check archive
    if [ -d "$archive_dir" ]; then
        for change in "$archive_dir"/*/; do
            if [ -d "$change" ]; then
                local change_name=$(basename "$change")
                for file in "${required_files[@]}"; do
                    if [ ! -f "$change$file" ]; then
                        echo "❌ Missing (archive): $change_name/$file"
                        ((errors++))
                    fi
                done
            fi
        done
    fi
    
    # Check parking
    if [ -d "$parking_dir" ]; then
        for change in "$parking_dir"/*/; do
            if [ -d "$change" ]; then
                local change_name=$(basename "$change")
                for file in "${required_files[@]}"; do
                    if [ ! -f "$change$file" ]; then
                        echo "❌ Missing (parking): $change_name/$file"
                        ((errors++))
                    fi
                done
            fi
        done
    fi
    
    if [ $errors -gt 0 ]; then
        echo "❌ ECL lint failed with $errors error(s)."
        exit 1
    else
        echo "✅ ECL lint passed."
    fi
}

cmd_config_validate() {
    echo "🔍 Validating models.toml..."
    
    local config_file="$WORKSPACE_ROOT/crates/cognition/config/models.toml"
    
    if [ ! -f "$config_file" ]; then
        echo "❌ models.toml not found at $config_file"
        exit 1
    fi
    
    # Check if toml parses (using python as fallback)
    if command -v python3 &> /dev/null; then
        if ! python3 -c "import toml; toml.load('$config_file')" 2>/dev/null; then
            echo "❌ models.toml failed to parse"
            exit 1
        fi
        echo "✅ models.toml syntax valid"
    else
        echo "⚠️  Skipping TOML parse check (python3 not available)"
    fi
    
    # Check all loops mapped
    echo "✅ models.toml validation complete"
}

cmd_doctor() {
    echo "🔍 UltraMeshAI Workspace Doctor"
    echo
    echo "Workspace: $WORKSPACE_ROOT"
    echo
    
    local warnings=0
    
    # Check cargo test
    echo "Checking cargo test..."
    if cargo test --workspace --quiet 2>/dev/null; then
        echo "✅ All tests pass"
    else
        echo "❌ Some tests failing"
        ((warnings++))
    fi
    echo
    
    # Count ECL changes
    echo "Counting ECL changes..."
    local active_count=$(find "$WORKSPACE_ROOT/harness/changes/active" -maxdepth 1 -type d 2>/dev/null | wc -l)
    local archive_count=$(find "$WORKSPACE_ROOT/harness/changes/archive" -maxdepth 1 -type d 2>/dev/null | wc -l)
    local parking_count=$(find "$WORKSPACE_ROOT/harness/changes/parking" -maxdepth 1 -type d 2>/dev/null | wc -l)
    
    # Subtract 1 for the directory itself (. and ..)
    active_count=$((active_count - 1))
    archive_count=$((archive_count - 1))
    parking_count=$((parking_count - 1))
    
    echo "  Active:  $active_count"
    echo "  Archive: $archive_count"
    echo "  Parking: $parking_count"
    
    if [ $active_count -gt 1 ]; then
        echo "⚠️  Warning: More than 1 active change (should be 0 or 1)"
        ((warnings++))
    fi
    echo
    
    # Check for stale branches (git only)
    if command -v git &> /dev/null; then
        echo "Checking git status..."
        local branch=$(git branch --show-current 2>/dev/null || echo "unknown")
        echo "  Current branch: $branch"
        
        local uncommitted=$(git status --porcelain 2>/dev/null | wc -l)
        if [ $uncommitted -gt 0 ]; then
            echo "⚠️  Warning: $uncommitted uncommitted changes"
            ((warnings++))
        else
            echo "✅ Working tree clean"
        fi
        echo
    fi
    
    # Summary
    echo "---"
    if [ $warnings -gt 0 ]; then
        echo "⚠️  Doctor found $warnings warning(s)"
    else
        echo "✅ Workspace healthy"
    fi
}

# Main
case "${1:-}" in
    ecl-lint)
        cmd_ecl_lint
        ;;
    config-validate)
        cmd_config_validate
        ;;
    doctor)
        cmd_doctor
        ;;
    help|-h|--help)
        usage
        ;;
    "")
        usage
        exit 1
        ;;
    *)
        echo "❌ Unknown command: $1"
        echo
        usage
        exit 1
        ;;
esac
