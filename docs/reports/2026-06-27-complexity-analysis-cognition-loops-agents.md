# Complexity Analysis Report: Cognition + Loops + Agents Crates

**Date:** 2026-06-27  
**Author:** Deepwork Review  
**Scope:** crates/cognition/, crates/loops/, crates/agents/

---

## Executive Summary

Phase 1 cognition layer implemented with **O(1)** hot path operations for routing and **O(1)** slice access for session messages. No nested loops found. Memory per unit well under 100MB soft cap.

**New:** Agents crate added (ADK pattern) with LoopAgent, SequentialAgent, ConditionalAgent, LlmConditionalAgent.

**Key fixes applied:**
- `Session::get_messages()` returns slice `&[ChatMessage]` instead of cloning `Vec<ChatMessage>`
- `LoopInput.context` changed from `HashMap<String,String>` to `Vec<String>` per spec
- `DeepworkLoop::process()` now calls actual LLM client with session history
- Loop type naming standardized to "deepwork" (not "deepwork-loop")

---

## Data Structures & Complexity

| Struct | Fields | Clone Cost | Hot Path Ops |
|--------|--------|------------|--------------|
| `LlmClient` | 4 fields | O(1) | `chat()` - API call |
| `ChatMessage` | Role + String | O(S) | - |
| `Session` | Vec<ChatMessage> + timestamps | O(N×S) | `get_messages()` = **O(1)** slice |
| `PromptDispatcher` | HashMap<String,Template> | O(T×M) | `dispatch()` = O(1) + O(M×K) |
| `ModelRouter` | 2× HashMap | O(L) | `get_model()` = **O(1)** |
| `LoopInput` | Vec<String> context | O(K) | - |
| `LoopOutput` | Vec<String> tool_calls | O(T) | - |
| `LoopStats` | 3× u32 | O(1) | - |
| `AgentContext` | HashMap<String,String> | O(V) | `get()`/`set()` = O(1) |
| `AgentResponse` | String + bool | O(S) | - |

**Legend:** N=messages, S=string length, T=templates/tools, M=template size, K=variables, L=loops (10), V=state vars

---

## Hot Paths Analysis

| Path | Complexity | Status |
|------|------------|--------|
| `Session::add_message()` | O(1) amortized | ✅ OK |
| `Session::get_messages()` | O(1) slice | ✅ FIXED (was O(N) clone) |
| `PromptDispatcher::dispatch()` | O(1) + O(M×K) | ⚠️ Acceptable (small templates) |
| `ModelRouter::get_model()` | O(1) | ✅ OK |
| `DeepworkLoop::process()` | O(N) + API latency | ✅ FIXED (was stub) |
| `LoopAgent::execute()` | O(I×A×S) | ⚠️ I=iterations, A=agents, S=agent time |
| `SequentialAgent::execute()` | O(A×S) | ✅ Linear in agents |
| `LlmConditionalAgent::execute()` | O(API) + O(R) | ✅ Classification + routing |

---

## Nested Loops

**None found.** All operations are single-pass or HashMap lookups.

---

## Memory Estimates

**Per unit session:**
- 100 messages × 500 bytes = **50KB** (well under 100MB soft cap)
- Timestamps: 2 × u64 = 16 bytes
- Metadata: ~100 bytes

**Per agent instance:**
- Context state: ~1KB typical
- Response buffer: ~4KB

**Total per loop instance:** ~100KB (comfortable for 10k units = ~1GB total)

---

## Recommendations (Applied)

1. ✅ `Session::get_messages()` - Return `&[ChatMessage]` slice
2. ✅ `LoopInput.context` - Change to `Vec<String>`
3. ✅ `DeepworkLoop::process()` - Add actual `client.chat()` call
4. ✅ Loop type naming - Standardize to spec names

---

## Future Considerations

1. **Streaming responses** - Consider `tokio::sync::mpsc` for token streaming
2. **Message pruning** - Add `Session::truncate(keep: usize)` for long conversations
3. **Tool call structure** - `LoopOutput.tool_calls` could use structured `ToolCall` type
4. **Error wrapping** - `LoopError` could wrap `CognitionError` and `TransportError` directly
5. **Agent persistence** - Consider snapshotting agent state for recovery

---

## Test Results

- Cognition crate: **16 tests pass**
- Loops crate: **30 tests pass**
- Agents crate: **0 tests** (new, tests pending)

---

## New: Agents Crate (ADK Pattern)

Following ADK-Rust patterns from https://adk-rust.com/docs:

| Agent | Purpose | Complexity |
|-------|---------|------------|
| `BaseAgent` trait | Common interface | - |
| `LoopAgent` | Iterative execution until exit condition | O(I×A×S) |
| `SequentialAgent` | Pipeline: agent1 → agent2 → agent3 | O(A×S) |
| `ConditionalAgent` | Rule-based routing | O(1) |
| `LlmConditionalAgent` | LLM-based classification + routing | O(API) |

**Integration:** Agents wrap loops, enabling complex workflows like:
- Critique-refine loops (LoopAgent with ExitLoopTool)
- Multi-perspective analysis (ParallelAgent pattern)
- Intelligent routing (LlmConditionalAgent)
