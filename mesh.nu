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
#   ULTRAMESHAI_THREADS           parallelism for status/test (default: 4)

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

# The parallelism level for par-each, honoring ULTRAMESHAI_THREADS.
def mesh-threads [] {
    let raw = ($env.ULTRAMESHAI_THREADS? | default "4")
    let n = (try { $raw | into int } catch { 4 })
    if $n < 1 { 4 } else { $n }
}

# Pretty print an error and exit non-zero. Carries a hint when provided.
def die [
    msg: string
    --hint: string = "" # a suggestion for what to check / how to fix
    --code: int = 1     # the exit code to use
] {
    print -e $"(ansi red_bold)Error: ($msg)(ansi reset)"
    if ($hint | str length) > 0 {
        print -e $"(ansi yellow)Hint: ($hint)(ansi reset)"
    }
    exit $code
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

    # 3. Git statuses (paths derived from workspace root, not hardcoded) —
    #    checked in parallel via par-each. Each thread runs in its own scope,
    #    so the `cd` is safe.
    let portail_dir    = (resolve-repo "ULTRAMESHAI_PORTAIL_DIR"  (sibling-dir "portail"))
    let proposal_dir   = (resolve-repo "ULTRAMESHAI_PROPOSAL_DIR" (sibling-dir "proposal.vaked.dev"))
    let repos = [
        { name: "ultrameshai",         path: $root }
        { name: "portail",             path: $portail_dir }
        { name: "proposal.vaked.dev",  path: $proposal_dir }
    ]

    print $"\n(ansi blue)Git Repositories:(ansi reset)"
    let results = ($repos | par-each --threads (mesh-threads) { |repo|
        if not ($repo.path | path exists) {
            { name: $repo.name, line: $"(ansi red)Not Found(ansi reset) (path: ($repo.path))" }
        } else {
            cd $repo.path
            let branch_res = (do -i { git branch --show-current } | complete)
            let branch = ($branch_res.stdout | str trim)
            if $branch_res.exit_code != 0 {
                { name: $repo.name, line: $"(ansi red)Git error(ansi reset) (branch unknown)" }
            } else {
                let status_res = (do -i { git status --porcelain } | complete)
                if $status_res.exit_code != 0 {
                    { name: $repo.name, line: $"(ansi red)Git error(ansi reset) (status failed)" }
                } else {
                    let status_text = if ($status_res.stdout | str trim | is-empty) { $"(ansi green)Clean(ansi reset)" } else { $"(ansi yellow)Modified(ansi reset)" }
                    { name: $repo.name, line: $"($branch) ➔ ($status_text)" }
                }
            }
        }
    })
    # keep the display order stable (par-each may reorder)
    for repo in $repos {
        let hit = ($results | where name == $repo.name | first)
        print $"  * ($repo.name) [($hit.line)]"
    }
    cd $root
}

# --- serve -----------------------------------------------------------------

export def "main serve" [] {
    print $"(ansi cyan_bold)=== Starting Portail Gateway ===(ansi reset)"

    let root = (workspace-root)
    let portail_dir = (resolve-repo "ULTRAMESHAI_PORTAIL_DIR" (sibling-dir "portail"))
    if not ($portail_dir | path exists) {
        die $"portail directory not found at ($portail_dir)." --hint "Set ULTRAMESHAI_PORTAIL_DIR to the portail checkout path."
    }

    # Kill existing process(es) on 8787
    let pids = (pids-on-port 8787)
    if not ($pids | is-empty) {
        print $"  Clearing port 8787 (killing PIDs: ($pids | str join ', '))..."
        for pid in $pids {
            let killed = (do -i { kill -9 $pid } | complete)
            if $killed.exit_code != 0 {
                print -e $"(ansi yellow)  Warning: could not kill PID ($pid) (exit ($killed.exit_code))(ansi reset)"
            }
        }
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
    # a short wait lets a fast compile failure surface into the log before we report
    sleep 2sec
    let pids_after = (pids-on-port 8787)
    if ($pids_after | is-empty) {
        print -e $"(ansi yellow)  Warning: nothing is listening on 8787 yet — check ($log_file).(ansi reset)"
    } else {
        print $"(ansi green)  Portail gateway up on 8787 [PIDs: ($pids_after | str join ', ')](ansi reset)"
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
    let portail_dir = (resolve-repo "ULTRAMESHAI_PORTAIL_DIR" (sibling-dir "portail"))
    let lint_script = ($root | path join "scripts/lint-ecl.nu")

    # Collect the steps to run; portail tests and the ECL linter are
    # independent, so they run in parallel via par-each.
    mut steps = []
    if not $skip_portail {
        $steps = ($steps | append { key: "portail", label: "portail Rust tests", dir: $portail_dir })
    }
    if not $skip_ecl {
        $steps = ($steps | append { key: "ecl", label: "ECL linter", dir: $root })
    }

    if ($steps | is-empty) {
        print $"(ansi yellow)  Nothing to run — both steps skipped.(ansi reset)"
        print $"\n(ansi green_bold)=== All Verification Checks Passed! ===(ansi reset)"
        return
    }

    # Run the steps in parallel. Each closure captures its own env; the
    # `cd` inside each closure is scoped to that thread.
    let results = ($steps | par-each --threads (mesh-threads) { |s|
        if $s.key == "portail" {
            if not ($s.dir | path exists) {
                { key: "portail", ok: false, skip: true, out: $"portail directory not found at ($s.dir) — skipping." }
            } else {
                cd $s.dir
                let res = (do -i { cargo test } | complete)
                if $res.exit_code != 0 {
                    { key: "portail", ok: false, out: $res.stdout, err: $res.stderr, code: $res.exit_code }
                } else {
                    { key: "portail", ok: true, out: $res.stdout }
                }
            }
        } else {
            let lint_script = ($root | path join "scripts/lint-ecl.nu")
            if not ($lint_script | path exists) {
                { key: "ecl", ok: false, skip: true, out: $"($lint_script) not found. Skipping ECL." }
            } else {
                cd $root
                let res = (do -i { nu $lint_script } | complete)
                if $res.exit_code != 0 {
                    { key: "ecl", ok: false, out: $res.stdout, err: $res.stderr, code: $res.exit_code }
                } else {
                    { key: "ecl", ok: true, out: $res.stdout }
                }
            }
        }
    })

    # Report in a stable order (portail first, ecl second).
    let order = ($steps | each { |s| $s.key })
    for key in $order {
        let r = ($results | where key == $key | first)
        if ($r | get -o skip | default false) {
            print $"(ansi yellow)  ($r.out)(ansi reset)"
        } else if ($r | get -o ok | default false) {
            print $"(ansi blue)[step] ($key)(ansi reset)"
            print $r.out
            print $"(ansi green)✓ ($key) passed.(ansi reset)"
        } else {
            print -e $"(ansi red_bold)Error: ($key) failed!(ansi reset)"
            if (($r | get -o out | default "") | str trim | is-not-empty) { print -e $r.out }
            if (($r | get -o err | default "") | str trim | is-not-empty) { print -e $r.err }
            print -e $"(ansi yellow)Hint: check the failing step's logs or run it directly.(ansi reset)"
            exit 1
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
        die $"build.nu not found at ($build_engine)." --hint "Set ULTRAMESHAI_PROPOSAL_DIR to the proposal.vaked.dev checkout path."
    }
    let res = (do -i { nu $build_engine } | complete)
    if $res.exit_code != 0 {
        if ($res.stdout | str trim | is-not-empty) { print -e $res.stdout }
        if ($res.stderr | str trim | is-not-empty) { print -e $res.stderr }
        let msg = $"build failed with exit code ($res.exit_code)"
        die $msg --hint "Run the build engine directly: nu ($build_engine)"
    }
    print $res.stdout
    print $"(ansi green)✓ Build succeeded.(ansi reset)"
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
        die $"build.nu not found at ($build_engine)." --hint "Set ULTRAMESHAI_PROPOSAL_DIR to the proposal.vaked.dev checkout path."
    }
    if (($env.HF_TOKEN? | default "") | str length) == 0 {
        die "HF_TOKEN environment variable is not set. Deployment aborted." --hint "Export HF_TOKEN (a Hugging Face write token) and retry."
    }

    let push_flag = (if $dry_run { [] } else { ["--push"] })
    let res = (do -i { nu $build_engine ...$push_flag --message $message } | complete)
    if $res.exit_code != 0 {
        if ($res.stdout | str trim | is-not-empty) { print -e $res.stdout }
        if ($res.stderr | str trim | is-not-empty) { print -e $res.stderr }
        let msg = $"deploy failed with exit code ($res.exit_code)"
        die $msg --hint "Run the build engine directly: nu ($build_engine) ...$push_flag --message \"$message\""
    }
    print $res.stdout
    print $"(ansi green)✓ Deploy succeeded.(ansi reset)"
}
