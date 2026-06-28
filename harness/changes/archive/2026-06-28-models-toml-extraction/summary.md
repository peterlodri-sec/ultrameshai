# Model Config TOML Extraction

## What
Extract model tier aliases and loop-to-model mappings from hardcoded Rust strings in
`model_router.rs` into a standalone project-owned TOML file.

## Why
- **Git-tracked**: model config updates via `git pull`, no opencode restart required
- **CI-validatable**: `cargo test` catches broken mappings automatically
- **Single source of truth**: removes config fragmentation between project and opencode layer
- **Opencode-independent**: agents don't need a config reload to pick up model changes

## How
1. Create `crates/cognition/config/models.toml` with tier definitions + loop mappings
2. Refactor `ModelRouter::new()` to load from TOML at construction time
3. Keep all public API surface identical (get_model, register_custom, create_client, etc.)
4. Existing tests pass unchanged -- they test behavior, not internal string storage
