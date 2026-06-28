# Loop Logic Implementation

## What
Upgrade 38 stub loop files from echo-back stubs to real LLM-integrated loop implementations.

## Why
All 48 loop types are declared at the architecture level with prompt templates and model router entries, but only 3 (deepwork, yardmaster, juniors) have actual LLM client integration. This change fills in the remaining 38 stubs.

## How
Each stub follows the established `deepwork.rs` pattern: LlmClient + Session + PromptDispatcher dispatch -> chat -> response. Six parallel batches grouped by domain (SWE-bench, testing, devops/quality, memory/ralph, infra/utility/math, research).

## Verification
- `cargo check` — must compile without errors
- `cargo test` — all existing tests must pass, no regressions
- `nu scripts/lint-ecl.nu` — ECL compliance
