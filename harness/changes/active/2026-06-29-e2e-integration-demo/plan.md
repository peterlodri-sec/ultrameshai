# E2E Integration Demo — Plan

## Files

- Create: `scripts/test-e2e-integration.nu` — main integration test script
- Modify: `crates/node-registry/src/main.rs` — add graceful shutdown (optional, if missing)
- Create: `.superpowers/sdd/progress.md` update (ledger)

## Tasks

### Task 1: E2E Integration Test Script

**Files:** Create `scripts/test-e2e-integration.nu`

**Steps:**
- [ ] Write nushell script that:
  1. Checks port 3000 is free
  2. Sets `HEARTBEAT_SECRET` and starts node-registry in background
  3. Waits for `/health` to respond 200
  4. Sends POST `/heartbeat` with signed payload
  5. Sends GET `/health` and verifies total_nodes=1
  6. Sends GET `/nodes` and verifies node_id matches
  7. Kills daemon, checks exit code
- [ ] Verify: `nu scripts/test-e2e-integration.nu`

### Task 2: Graceful Shutdown

**Files:** Modify `crates/node-registry/src/main.rs`

**Steps:**
- [ ] Check if graceful shutdown via SIGTERM handler exists
- [ ] If not, add axum `shutdown_signal` or tokio signal handler
- [ ] Verify: daemon stops cleanly on SIGTERM

### Task 3: Progress Ledger Update

**Files:** Modify `.superpowers/sdd/progress.md`

**Steps:**
- [ ] Add task completion entries
- [ ] Verify: `nu scripts/lint-ecl.nu` passes
