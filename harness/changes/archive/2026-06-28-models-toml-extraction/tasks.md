# Tasks

## Setup
- [ ] Create `crates/cognition/config/models.toml` with tiers + loop mappings
- [ ] Add `serde` + `toml` deps to `crates/cognition/Cargo.toml`

## Refactor
- [ ] Add TOML loading + resolution logic to `model_router.rs`
- [ ] Remove all hardcoded model string literals from `ModelRouter::new()`

## Verify
- [ ] `cargo check -p loop-engineering-cognition` -- no errors
- [ ] `cargo test` -- all pass, no regressions
- [ ] `nu scripts/lint-ecl.nu` -- ECL compliance
- [ ] Archive change
