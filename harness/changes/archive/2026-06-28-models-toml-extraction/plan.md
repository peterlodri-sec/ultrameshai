# Implementation Plan

## Step 1 -- Create models.toml
File: `crates/cognition/config/models.toml`

```toml
[tiers]
FRONTIER = "meta-llama@latest?params>=70"
REASONING = "deepseek@latest"
CODE_BEST = "code_chat@latest"
CODE_MID = "code_chat@latest?params>=14"
CODE_SMALL = "mistral@latest"
CODE_SPEC = "codestral@latest"
GEN_LARGE = "qwen@latest"
GEN_MID = "qwen@latest?params>=32"
GEN_SMALL = "mistral@latest"
GEN_TINY = "mistral@latest"
MATH = "mistral@latest"

[loops]
# Original 10
deepwork = "FRONTIER"
bruteforce-coder = "CODE_BEST"
deep-research = "FRONTIER"
testers = "CODE_MID"
yardmaster = "FRONTIER"
devops = "CODE_MID"
ui = "FRONTIER"
red-team = "CODE_BEST"
juniors = "CODE_SMALL"
ralph = "GEN_TINY"

# Coder sub-loops
coder-planner = "FRONTIER"
coder-editor = "CODE_BEST"
coder-reviewer = "CODE_BEST"

# Deepwork sub-loops
deepwork-decomposer = "FRONTIER"
deepwork-verifier = "REASONING"

# Red-team sub-loops
redteam-fuzzer = "CODE_BEST"
redteam-analyzer = "REASONING"

# Research / Knowledge
librarian = "FRONTIER"
research-web = "FRONTIER"
research-docs = "GEN_LARGE"
research-patterns = "REASONING"

# SWE-bench pipeline
issue-analyzer = "FRONTIER"
codebase-explorer = "CODE_BEST"
fix-planner = "REASONING"
fix-implementer = "CODE_BEST"
edge-case-analyzer = "FRONTIER"
regression-checker = "CODE_MID"
diff-builder = "CODE_SPEC"

# Testing expanded
tester-unit = "CODE_SMALL"
tester-integration = "CODE_MID"
tester-benchmark = "CODE_SMALL"
tester-property = "CODE_MID"
tester-mutation = "CODE_BEST"

# DevOps expanded
devops-build = "CODE_SMALL"
devops-package = "CODE_SMALL"
devops-deploy = "CODE_SMALL"
devops-cache = "GEN_SMALL"

# Quality
quality-lint = "CODE_SMALL"
quality-audit = "CODE_MID"
quality-coverage = "CODE_SMALL"
quality-typecheck = "CODE_SMALL"
quality-style = "CODE_SMALL"

# Memory / Learning
memory-indexer = "GEN_SMALL"
memory-pattern-miner = "REASONING"
memory-summarizer = "GEN_SMALL"

# Ralph expanded
ralph-coder = "GEN_TINY"
ralph-research = "GEN_TINY"
ralph-meta = "FRONTIER"

# Infrastructure
infra-provisioner = "GEN_SMALL"
infra-monitor = "GEN_SMALL"
infra-balancer = "GEN_MID"

# Utility / Cross-cutting
reporter = "FRONTIER"
validator = "REASONING"
notebook = "CODE_MID"

# Math / Scientific
math-solver = "MATH"
math-verify = "MATH"
```

## Step 2 -- Refactor model_router.rs
- Add `use serde::Deserialize;` and TOML loading infrastructure
- Define internal `ModelsConfig` struct with `tiers: HashMap<String, String>` and `loops: HashMap<String, String>`
- Load via `include_str!("../config/models.toml")` at compile time
- `ModelRouter::new()` resolves tiers → produces same `default_models: HashMap<String, String>` as today
- All public methods remain untouched

## Step 3 -- Add serde/toml deps to cognition crate
Add to `crates/cognition/Cargo.toml`:
```toml
serde = { workspace = true, features = ["derive"] }
toml = "0.8"
```

## Step 4 -- Verify
- `cargo check -p loop-engineering-cognition` -- no errors
- `cargo test` -- all 157+ tests pass
- Specifically: `all_loops_mapped` test still passes (same 48 loops, same models)
- `nu scripts/lint-ecl.nu`

## Step 5 -- Archive
- Move to `harness/changes/archive/`
