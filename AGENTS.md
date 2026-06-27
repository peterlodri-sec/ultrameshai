# AGENTS.md

Loop-engineering agent stack. Multi-loop coding agent system targeting SWE-bench Verified ≥98%, then a Rust/Zig mesh app on cloud VMs + Raspberry Pis.

## Architecture

5 layers: Cognition, Orchestration, Transport, Execution, Memory. 10 loops (deepwork, bruteforce-coder, deep-research, testers, yardmaster, devops, UI, red-team, juniors, ralph). Decentralized scheduling over headscale/WireGuard mesh. Same-node IPC = pipelined protobuf over UDS.

Full design: `docs/superpowers/specs/2026-06-27-loop-engineering-agent-stack-design.md`
Phase 0 plan: `docs/superpowers/plans/2026-06-27-phase0-substrate.md`

## Tech stack

- Nix flakes for all reproducible environments — no imperative installs
- Nushell as glue/scripting layer for unit lifecycle
- Rust (tokio, prost) for performance-critical paths (transport, registry)
- Protobuf as the single IPC message format (`proto/loop_engineering.proto`)
- Headscale (or existing Tailscale) as mesh transport

## Commands

Nix shells must be cached before any spawn benchmark — `<500ms spawn` means runtime scaffolding ready (nix shells cached + worker procs forked), NOT LLM token latency.

```bash
nix flake check                                          # verify flake evaluates
nix develop .#agent-unit --command nu -c "version"        # enter standard shell
nix build .#agent-unit --no-link                         # warm shell cache (required before bench)
cargo test --manifest-path crates/transport/Cargo.toml   # transport crate tests
cargo test --manifest-path crates/node-registry/Cargo.toml # registry crate tests
```

## Layout

```
flake.nix              # devShells: agent-unit (standard), -test, -red-team, -devops; protobuf-gen
nix/                   # agent-unit.nix (shell tiers), protobuf.nix (codegen derivation)
proto/                 # loop_engineering.proto — single typed contract across components
crates/transport/      # framed protobuf over UDS (length-delimited, 4MB max, pipelined)
crates/node-registry/  # mesh node capacity store + UDP multicast heartbeat
scripts/               # nushell: unit-harness.nu, spawn-bench.nu, integration tests
docs/                  # headscale setup, specs, plans
```

## Constraints

- Target hardware: cloud VMs (64GB) + Raspberry Pi (4GB). Test on both.
- Memory cap per agent unit: 100MB soft, 150MB elastic, snapshot+kill at >160MB.
- Protobuf messages max 4MB (framed transport).
- macOS hosts cannot run `nix flake check` or `nix develop` against Linux shells — verify on target Linux nodes.

## Conventions

- TDD for all Rust crates (failing test first, then implementation).
- Caveman mode active in this project (see `~/.config/opencode/AGENTS.caveman.md`). Code, commits, PRs, and security warnings stay normal English.
- SDD workspace at `.superpowers/sdd/` tracks task progress in `progress.md` ledger — check it before re-dispatching tasks.