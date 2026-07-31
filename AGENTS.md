# AGENTS.md

Loop-engineering agent stack. Multi-loop coding agent system targeting SWE-bench Verified ≥98%, then Rust/Zig mesh app on cloud VMs + Raspberry Pis.

## Architecture (4 Layers)

```
+-------------------------------------------------------------+
| 1. COGNITION LAYER   (LLM Client, State, Prompting)         |
+-------------------------------------------------------------+
                              │
                              ▼
+-------------------------------------------------------------+
| 2. ORCHESTRATION LAYER (Topology Mapping & Routing)         |
+-------------------------------------------------------------+
                              │
                              ▼
+-------------------------------------------------------------+
| 3. TRANSPORT LAYER     (Headscale / WireGuard Mesh VPN)     |
+-------------------------------------------------------------+
                              │
                              ▼
+-------------------------------------------------------------+
| 4. EXECUTION LAYER    (Target Node Daemon, Bun Runtimes)    |
+-------------------------------------------------------------+
```

Full design: `docs/superpowers/specs/2026-06-27-loop-engineering-agent-stack-design.md`
Phase 0 plan: `docs/superpowers/plans/2026-06-27-phase0-substrate.md`

## Tech stack

- Nix flakes — reproducible environments, no imperative installs
- Nushell — glue/scripting for unit lifecycle
- Rust (tokio, prost) — performance-critical paths (transport, registry)
- Protobuf — single IPC message format (`proto/loop_engineering.proto`)
- Headscale (or Tailscale) — mesh transport

Free LLM options: OpenRouter, OpenCode Go/Zen, Alibaba Qwen 3.6 / 3.7.

## Commands

Nix shells must be cached before any spawn benchmark — `<500ms spawn` means runtime scaffolding ready (nix shells cached + worker procs forked), NOT LLM token latency.

```bash
# First-time clone — init the kompress-ultra submodule
git submodule update --init --recursive

# Nix
nix flake check                                          # verify flake evaluates
nix develop .#agent-unit --command nu -c "version"        # enter standard shell
nix build .#agent-unit --no-link                         # warm shell cache (required before bench)
cargo test --manifest-path crates/transport/Cargo.toml   # transport crate tests

# OpenCode plugin tests (Bun)
cd .opencode && bun install && bun test
```

## Layout

```
flake.nix              # devShells: agent-unit (standard), -test, -red-team, -devops; protobuf-gen
nix/                   # agent-unit.nix (shell tiers), protobuf.nix (codegen derivation)
proto/                 # loop_engineering.proto — single typed contract across components
crates/
  transport/           # framed protobuf over UDS (working)
  node-registry/       # mesh node capacity store (NOT YET CREATED)
  agent-core/          # Layer 1: LLM client, session, prompts, tool dispatch (NOT YET CREATED)
scripts/               # nushell harness (NOT YET CREATED)
docs/superpowers/      # specs, plans
```

## Constraints

- Target hardware: cloud VMs (64GB) + Raspberry Pi (4GB). Test on both.
- Memory cap per agent unit: 100MB soft, 150MB elastic, snapshot+kill at >160MB.
- Protobuf messages max 4MB (framed transport).
- macOS hosts cannot run `nix flake check` or `nix develop` — verify on target Linux nodes.

## Conventions

- TDD for all Rust crates (failing test first, then implementation).
- Caveman mode active in this project (see `~/.config/opencode/AGENTS.caveman.md`). Code, commits, PRs, and security warnings stay normal English.
- SDD workspace at `.superpowers/sdd/` tracks task progress in `progress.md` ledger — check it before re-dispatching tasks.

## Agent Harness & ECL

This project uses an Evolutionary Change Log (ECL) to track changes.
- Rules: [docs/ECL.md](file:///Users/lodripeter/workspace/peterlodri-sec/ultrameshai/docs/ECL.md)
- Status: [docs/STATUS.md](file:///Users/lodripeter/workspace/peterlodri-sec/ultrameshai/docs/STATUS.md)
- Architecture: [docs/ARCHITECTURE.md](file:///Users/lodripeter/workspace/peterlodri-sec/ultrameshai/docs/ARCHITECTURE.md)

### Verification
Run ECL linting and tests before any merge:
```bash
nu scripts/lint-ecl.nu
cargo test
```