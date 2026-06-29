#!/usr/bin/env nu

# UltrameshAI Unified Dev Harness & E2E CLI
#
# All paths are derived from the workspace root (the ultrameshai checkout).
# Sibling repos (portail, proposal.vaked.dev) live in the parent directory
# by default; override via ULTRAMESHAI_WORKSPACE_ROOT / sibling env vars.
#
# Usage:
#   nu mesh.nu status
#   nu mesh.nu serve
#   nu mesh.nu test
#   nu mesh.nu build
#   nu mesh.nu deploy [-m "message"]
#
# Env:
#   ULTRAMESHAI_WORKSPACE_ROOT    absolute path to the ultrameshai checkout
#                                 (default: $env.PWD)
#   ULTRAMESHAI_PORTAIL_DIR       absolute path to portail (default: sibling)
#   ULTRAMESHAI_PROPOSAL_DIR      absolute path to proposal.vaked.dev (default: sibling)
#   ULTRAMESHAI_SKIP_ECL=1        skip the ECL linter (faster local runs)

# --- helpers ---------------------------------------------------------------

# Derive the ultrameshai workspace root from $env.PWD (already inside a git
# checkout — that is the only correct way to run this harness).
def workspace-root [] {
    let pwd = ($env.PWD | default "." | path expand)
    if ($pwd | path type) == "dir" {
        try { git rev-parse --show-toplevel } catch { |_| $pwd }
    } else {
        $pwd
    }
}

# Best-effort: get the PIDs listening on a TCP port. Returns an empty list
# if nothing is listening or `lsof` is not available.
def pids-on-port [port: int] {
    let out = (do -i { lsof -t -i:$port } | complete)
    if $out.exit_code != 0 { return [] }
    $out.stdout
    | lines
    | where ($it | str trim | is-not-empty)
    | each { |line| $line | str trim | into int }
}

# Resolve a sibling path under the parent of the workspace root.
def sibling-dir [name: string] {
    let root = (workspace-root)
    $root | path dirname | path join $name
}

# Pretty print an error and exit non-zero.
def die [msg: string] {
    print -e $"(ansi red_bold)Error: ($msg)(ansi reset)"
    exit 1
}

# Resolve the path of a sibling repo, honoring an env override.
def resolve-repo [name: string, default: string] {
    let has = ($env | columns | any { |c| $c == $name })
    if not $has { return $default }
    let override = ($env | get $name)
    if ($override | path exists) { return $override }
    $default
}

# --- top-level help --------------------------------------------------------

def main [] {
    print $"(ansi cyan_bold)=== UltrameshAI Dev Harness ===(ansi reset)"
    print "Available commands:"
    print "  nu mesh.nu status       # Check gateway, git statuses, and environment"
    print "  nu mesh.nu serve        # Restart the local Rust portail gateway on port 8787 (background)"
    print "  nu mesh.nu test         # Run portail Rust tests and (optional) ECL linter"
    print "  nu mesh.nu build        # Compile Tailwind v4 CSS and sync assets to HF Space"
    print "  nu mesh.nu deploy -m    # Build, sync, and deploy to GitHub and Hugging Face"
}

# --- status ----------------------------------------------------------------

export def "main status" [] {
    print $"(ansi cyan_bold)=== Mesh Status ===(ansi reset)"

    let root = (workspace-root)

    # 1. Gateway on 8787
    let pids = (pids-on-port 8787)
    if ($pids | is-empty) {
        print $"Gateway Server: (ansi red)OFFLINE(ansi reset)"
    } else {
        print $"Gateway Server: (ansi green)ONLINE(ansi reset) [PIDs: ($pids | str join ', ')]"
    }

    # 2. Environment
    let hf_token_status = if ($env.HF_TOKEN? | default "" | str length) == 0 { $"(ansi red)NOT SET(ansi reset)" } else { $"(ansi green)SET(ansi reset)" }
    print $"HF_TOKEN: ($hf_token_status)"

    # 3. Git statuses (paths derived from workspace root, not hardcoded)
    let portail_dir    = (resolve-repo "ULTRAMESHAI_PORTAIL_DIR"  (sibling-dir "portail"))
    let proposal_dir   = (resolve-repo "ULTRAMESHAI_PROPOSAL_DIR" (sibling-dir "proposal.vaked.dev"))
    let repos = [
        { name: "ultrameshai",         path: $root }
        { name: "portail",             path: $portail_dir }
        { name: "proposal.vaked.dev",  path: $proposal_dir }
    ]

    print $"\n(ansi blue)Git Repositories:(ansi reset)"
    for repo in $repos {
        if ($repo.path | path exists) {
            cd $repo.path
            let branch = (do -i { git branch --show-current } | complete | get stdout | str trim)
            let status = (do -i { git status --porcelain } | complete | get stdout)
            let status_text = if ($status | str trim | is-empty) { $"(ansi green)Clean(ansi reset)" } else { $"(ansi yellow)Modified(ansi reset)" }
            print $"  * ($repo.name) [($branch)] ➔ ($status_text)"
        } else {
            print $"  * ($repo.name) ➔ (ansi red)Not Found(ansi reset) (path: ($repo.path))"
        }
    }
    cd $root
}

# --- serve -----------------------------------------------------------------

export def "main serve" [] {
    print $"(ansi cyan_bold)=== Starting Portail Gateway ===(ansi reset)"

    let root = (workspace-root)
    let portail_dir = (resolve-repo "ULTRAMESHAI_PORTAIL_DIR" (sibling-dir "portail"))
    if not ($portail_dir | path exists) {
        die $"portail directory not found at ($portail_dir). Set ULTRAMESHAI_PORTAIL_DIR to override."
    }

    # Kill existing process(es) on 8787
    let pids = (pids-on-port 8787)
    if not ($pids | is-empty) {
        print $"  Clearing port 8787 (killing PIDs: ($pids | str join ', '))..."
        for pid in $pids { kill -9 $pid }
        sleep 1sec
    }

    # Launch in the background, redirect to logs/server.log
    let log_dir = ($portail_dir | path join "logs")
    mkdir $log_dir
    let log_file = ($log_dir | path join "server.log")

    print $"(ansi blue)  Spawning portail server in background. Logs: ($log_file)(ansi reset)"
    cd $portail_dir
    with-env { PORTAIL_LOG_DIR: $log_dir } {
        ^cargo run --manifest-path ($portail_dir | path join "Cargo.toml") --bin portail serve
        | out+err | save --append $log_file
    }
}

# --- test ------------------------------------------------------------------

export def "main test" [
    --skip-portail   # skip the portail cargo test step
    --skip-ecl       # skip the ECL linter
] {
    print $"(ansi cyan_bold)=== Running Multi-Repo Tests ===(ansi reset)"

    let root = (workspace-root)
    let skip_ecl = ($skip_ecl or ($env.ULTRAMESHAI_SKIP_ECL? == "1"))

    # 1. Portail tests
    if not $skip_portail {
        print $"(ansi blue)[1/2] Running portail Rust tests...(ansi reset)"
        let portail_dir = (resolve-repo "ULTRAMESHAI_PORTAIL_DIR" (sibling-dir "portail"))
        if not ($portail_dir | path exists) {
            print $"(ansi yellow)  portail directory not found at ($portail_dir) — skipping.(ansi reset)"
        } else {
            cd $portail_dir
            let test_res = (do -i { cargo test } | complete)
            if $test_res.exit_code != 0 {
                print -e $"(ansi red_bold)Error: Portail tests failed!(ansi reset)"
                print -e $test_res.stdout
                print -e $test_res.stderr
                cd $root
                exit 1
            }
            print $test_res.stdout
            print $"(ansi green)✓ Portail tests passed.(ansi reset)"
        }
    }

    # 2. ECL linter — use $root we captured BEFORE the cd, not a fresh rev-parse
    if not $skip_ecl {
        print $"\n(ansi blue)[2/2] Running ECL Linter...(ansi reset)"
        cd $root
        let lint_script = ($root | path join "scripts/lint-ecl.nu")
        if ($lint_script | path exists) {
            let lint_res = (do -i { nu $lint_script } | complete)
            if $lint_res.exit_code != 0 {
                print -e $"(ansi red_bold)Error: ECL linting failed!(ansi reset)"
                print -e $lint_res.stdout
                print -e $lint_res.stderr
                exit 1
            }
            print $lint_res.stdout
            print $"(ansi green)✓ ECL linting passed.(ansi reset)"
        } else {
            print $"(ansi yellow)  ($lint_script) not found. Skipping ECL.(ansi reset)"
        }
    }

    print $"\n(ansi green_bold)=== All Verification Checks Passed! ===(ansi reset)"
}

# --- build -----------------------------------------------------------------

export def "main build" [] {
    let root = (workspace-root)
    let proposal_dir = (resolve-repo "ULTRAMESHAI_PROPOSAL_DIR" (sibling-dir "proposal.vaked.dev"))
    let build_engine = ($proposal_dir | path join "build.nu")

    if not ($build_engine | path exists) {
        die $"build.nu not found at ($build_engine). Set ULTRAMESHAI_PROPOSAL_DIR to override."
    }
    nu $build_engine
}

# --- deploy ----------------------------------------------------------------

export def "main deploy" [
    --message (-m): string = "chore: production build and deploy" # Commit message
    --dry-run  # Build but do not push
] {
    let root = (workspace-root)
    let proposal_dir = (resolve-repo "ULTRAMESHAI_PROPOSAL_DIR" (sibling-dir "proposal.vaked.dev"))
    let build_engine = ($proposal_dir | path join "build.nu")

    if not ($build_engine | path exists) {
        die $"build.nu not found at ($build_engine). Set ULTRAMESHAI_PROPOSAL_DIR to override."
    }
    if (($env.HF_TOKEN? | default "") | str length) == 0 {
        die "HF_TOKEN environment variable is not set. Deployment aborted."
    }

    let push_flag = (if $dry_run { [] } else { ["--push"] })
    nu $build_engine ...$push_flag --message $message
}
