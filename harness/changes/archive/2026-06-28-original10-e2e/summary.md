# Original 10 E2E Readiness

## What
Upgrade 8 Original 10 loop stubs to full LLM-integrated implementations matching the `deepwork.rs` pattern.

## Why
E2E testing requires all Original 10 loops to have working LLM client integration. Currently only 2/10 (deepwork, yardmaster) are complete. The remaining 8 are stubs that echo back input without calling the LLM.

## How
Apply the same LLM integration pattern to all 8 stubs:
- Add `LlmClient`, `Session`, `PromptDispatcher` fields
- Use `LlmClient::mock("<loop-type>")` for mock client
- Dispatch prompt template from `PromptDispatcher`
- Call `client.chat()` and return response

## Verification
- `cargo check -p loop-engineering-loops` — no errors
- `cargo test` — all tests pass
- E2E test harness can instantiate and run all 10 loops
