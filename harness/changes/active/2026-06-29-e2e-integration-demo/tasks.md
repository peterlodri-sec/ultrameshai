# Tasks — E2E Integration Demo

## Task 1: E2E Integration Test Script
- [ ] Write `scripts/test-e2e-integration.nu`
- [ ] Verify: `nu scripts/test-e2e-integration.nu` passes

## Task 2: Graceful Shutdown
- [ ] Check `main.rs` for SIGTERM handler
- [ ] Add if missing
- [ ] Verify clean shutdown

## Task 3: Progress Ledger + Archive
- [ ] Update `.superpowers/sdd/progress.md`
- [ ] Run `nu scripts/lint-ecl.nu`
- [ ] Archive to `harness/changes/archive/`
