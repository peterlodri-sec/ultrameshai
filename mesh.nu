#!/usr/bin/env nu

# UltrameshAI Unified Dev Harness & E2E CLI
# Simplify and automate multi-repo workflows for both humans and AI agents.

def main [] {
    print $"(ansi cyan_bold)=== UltrameshAI Dev Harness ===(ansi reset)"
    print "Available commands:"
    print "  nu mesh.nu status       # Check gateway, git statuses, and environment"
    print "  nu mesh.nu serve        # Safely restart the local Rust portail gateway on port 8787"
    print "  nu mesh.nu test         # Run portail Rust tests and ECL linter"
    print "  nu mesh.nu build        # Compile Tailwind v4 CSS and sync assets to HF Space"
    print "  nu mesh.nu deploy -p    # Build, sync, and deploy to GitHub and Hugging Face"
}

# Check the status of the local gateway, Git repositories, and environment variables
export def "main status" [] {
    print $"(ansi cyan_bold)=== Mesh Status ===(ansi reset)"
    
    # 1. Check if port 8787 is active
    let gateway_pid = (do { lsof -t -i:8787 } | complete | get stdout | str trim)
    if ($gateway_pid | is-empty) {
        print $"Gateway Server: (ansi red)OFFLINE(ansi reset)"
    } else {
        print $"Gateway Server: (ansi green)ONLINE(ansi reset) [PID: ($gateway_pid)]"
    }

    # 2. Check Environment
    let hf_token_status = if ($env.HF_TOKEN? | is-empty) { $"(ansi red)NOT SET(ansi reset)" } else { $"(ansi green)SET(ansi reset)" }
    print $"HF_TOKEN: ($hf_token_status)"

    # Derive workspace root from this script's location
    let workspace_dir = (git rev-parse --show-toplevel)
    
    # 3. Check Git Statuses
    let repos = [
        { name: "ultrameshai", path: $workspace_dir },
        { name: "portail", path: $"($workspace_dir | path dirname)/portail" },
        { name: "proposal.vaked.dev", path: $"($workspace_dir | path dirname)/proposal.vaked.dev" }
    ]

    print $"\n(ansi blue)Git Repositories:(ansi reset)"
    for repo in $repos {
        if ($repo.path | path exists) {
            cd $repo.path
            let branch = (git branch --show-current | str trim)
            let status = (git status --porcelain)
            let status_text = if ($status | is-empty) { $"(ansi green)Clean(ansi reset)" } else { $"(ansi yellow)Modified(ansi reset)" }
            print $"  * ($repo.name) [($branch)] ➔ ($status_text)"
        } else {
            print $"  * ($repo.name) ➔ (ansi red)Not Found(ansi reset)"
        }
    }
}

# Safely restart the local Rust portail gateway on port 8787
export def "main serve" [] {
    print $"(ansi cyan_bold)=== Starting Portail Gateway ===(ansi reset)"
    let portail_dir = "/Users/lodripeter/workspace/peterlodri-sec/portail"
    
    # Kill existing process on 8787
    print $"(ansi -e '90m')  Clearing port 8787...(ansi reset)"
    let pid = (do { lsof -t -i:8787 } | complete | get stdout | str trim)
    if not ($pid | is-empty) {
        print $"(ansi -e '90m')  Killing process ($pid)...(ansi reset)"
        kill -9 ($pid | into int)
        sleep 1sec
    }

    # Run cargo serve
    print $"(ansi blue)  Spawning portail server... (ansi reset)"
    cd $portail_dir
    # Run in background via Nu's spawn/exec behavior or notify
    print $"(ansi yellow)  Note: Server launched in background. Check logs at ($portail_dir)/logs/.(ansi reset)"
    
    # We use a background task via shell execution
    # In Nushell, we can run it as an asynchronous job or let the agent runner manage it
    let log_dir = $"($portail_dir)/logs"
    mkdir $log_dir
    
    # Run the server
    with-env { PORTAIL_LOG_DIR: $log_dir } {
        run-external "cargo" "run" "--manifest-path" $"($portail_dir)/Cargo.toml" "--bin" "portail" "serve"
    }
}

# Run portail Rust tests and ECL linter
export def "main test" [] {
    print $"(ansi cyan_bold)=== Running Multi-Repo Tests ===(ansi reset)"
    
    # 1. Run Portail Tests
    print $"(ansi blue)[1/2] Running portail Rust tests...(ansi reset)"
    let portail_dir = (git rev-parse --show-toplevel | path dirname | path join "portail")
    cd $portail_dir
    let test_res = (do { cargo test } | complete)
    if $test_res.exit_code != 0 {
        print -e $"(ansi red_bold)Error: Portail tests failed!(ansi reset)"
        print -e $test_res.stdout
        exit 1
    }
    print $"(ansi green)✓ Portail tests passed.(ansi reset)"

    # 2. Run ECL Linter
    print $"\n(ansi blue)[2/2] Running ECL Linter...(ansi reset)"
    let ultramesh_dir = (git rev-parse --show-toplevel)
    cd $ultramesh_dir
    if ("scripts/lint-ecl.nu" | path exists) {
        let lint_res = (do { nu scripts/lint-ecl.nu } | complete)
        if $lint_res.exit_code != 0 {
            print -e $"(ansi red_bold)Error: ECL linting failed!(ansi reset)"
            print -e $lint_res.stderr
            exit 1
        }
        print $"(ansi green)✓ ECL linting passed.(ansi reset)"
    } else {
        print $"(ansi yellow)Warning: scripts/lint-ecl.nu not found. Skipping.(ansi reset)"
    }
    
    print $"\n(ansi green_bold)=== All Verification Checks Passed! ===(ansi reset)"
}

# Compile Tailwind v4 CSS and sync assets to HF Space
export def "main build" [] {
    let proposal_dir = (git rev-parse --show-toplevel | path dirname | path join "proposal.vaked.dev")
    let build_engine = $"($proposal_dir)/build.nu"
    
    if ($build_engine | path exists) {
        nu $build_engine
    } else {
        print -e $"(ansi red_bold)Error: build.nu not found in ($proposal_dir)(ansi reset)"
        exit 1
    }
}

# Build, sync, and deploy to GitHub and Hugging Face
export def "main deploy" [
    --message (-m): string = "chore: production build and deploy" # Commit message
] {
    let proposal_dir = (git rev-parse --show-toplevel | path dirname | path join "proposal.vaked.dev")
    let build_engine = $"($proposal_dir)/build.nu"
    
    if ($build_engine | path exists) {
        # Check for HF_TOKEN
        if ($env.HF_TOKEN? | is-empty) {
            print -e $"(ansi red_bold)Error: HF_TOKEN environment variable is not set. Deployment aborted.(ansi reset)"
            exit 1
        }
        nu $build_engine --push --message $message
    } else {
        print -e $"(ansi red_bold)Error: build.nu not found in ($proposal_dir)(ansi reset)"
        exit 1
    }
}
