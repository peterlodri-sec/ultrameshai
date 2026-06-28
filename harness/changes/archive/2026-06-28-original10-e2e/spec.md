# Requirements

## Functional
1. All 8 stub loops must follow the `deepwork.rs` pattern (LlmClient + Session + PromptDispatcher)
2. Each loop must use correct `loop_type()` string matching model_router keys:
   - bruteforce-coder, deep-research, testers, devops, ui, red-team, juniors, ralph
3. Each loop must dispatch its prompt template and call `client.chat()`
4. Each loop must return `LoopOutput.result` with the LLM response

## Non-functional
1. Zero behavior change to existing tests
2. Yardmaster and deepwork remain unchanged (already correct)
3. Honcho pattern logic in `juniors.rs` and `deep_research.rs` preserved (additional methods stay)

## Out of scope
- Tool dispatch implementation (tool_calls remains empty vec for now)
- Real API key wiring (mock clients sufficient for E2E)
