# E2E Integration Demo — Active Change

**Start:** 2026-06-29
**Status:** Active
**Goal:** Wire existing components into a live demo: node-registry HTTP API serving real data, mempalace recording lifecycle events, transport UDS carrying messages between processes.

## Motivation

All 11 Rust crates compile and test separately. No integration test exercises them as a system. This change builds the first E2E harness that spawns the node-registry daemon, processes heartbeats, and reports status — closing the gap from "compiling crates" to "running system."

## Verification

```bash
cargo test --workspace          # all tests pass
cargo build                     # all crates compile
node-registry --help            # binary runs
```

## Success Criteria

1. `cargo test --workspace` passes with no regressions
2. E2E integration test script passes:
   - Starts node-registry daemon (background)
   - Sends heartbeat via curl
   - Queries `/health` and `/nodes` endpoints
   - Shuts down cleanly
