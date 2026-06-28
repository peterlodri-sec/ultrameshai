# Implementation Plan

## Step 1 — Identify stubs
Original 10 loops with stub implementations:
- bruteforce_coder.rs, deep_research.rs, testers.rs, devops.rs, ui.rs, red_team.rs, juniors.rs, ralph.rs
- yardmaster.rs already has LLM integration but wrong loop_type string

## Step 2 — Apply LLM pattern
For each stub, replace with:
```rust
use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats, Result, LoopError};
use loop_engineering_cognition::{LlmClient, Session, PromptDispatcher, Role};

pub struct <Name>Loop {
    client: LlmClient,
    session: Session,
    dispatcher: PromptDispatcher,
    stats: LoopStats,
}

impl <Name>Loop {
    pub fn new() -> Self {
        let client = LlmClient::mock("<loop-type>");
        let session = Session::new("<loop-type>-loop", "unit-000");
        let dispatcher = PromptDispatcher::default();
        Self { client, session, dispatcher, stats: LoopStats::default() }
    }
}
```

## Step 3 — Fix loop_type strings
All must match model_router keys (no "-loop" suffix):
- bruteforce-coder, deep-research, testers, devops, ui, red-team, juniors, ralph, yardmaster

## Step 4 — Update tests
Fix loop_trait_test.rs assertions to expect correct loop_type strings.

## Step 5 — Verify
- cargo check -p loop-engineering-loops
- cargo test (all 157+ tests)
- ECL lint and archive
