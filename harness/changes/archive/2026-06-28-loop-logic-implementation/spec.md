# Requirements

## Functional
1. Each loop type must use `LlmClient` for LLM inference (via mock client for now)
2. Each loop type must dispatch its prompt template from `PromptDispatcher`
3. Each loop type must maintain a `Session` for message history
4. Each loop type must return a proper `LoopOutput` with the LLM response as `result`
5. The `loop_type()` method must return the correct string matching model_router keys

## Non-functional
1. Zero behavioural changes to existing 10 loops
2. All existing tests must continue to pass
3. The generation script `/tmp/gen_stubs.sh` must be removed

## Out of scope
- Real LLM API key wiring (stubs use `LlmClient::mock`)
- Tool dispatch logic (tool_calls remains empty vec)
- Transport layer integration
- Node registry integration
