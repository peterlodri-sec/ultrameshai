# Requirements

## Functional
1. `crates/cognition/config/models.toml` must define all 11 tier aliases exactly matching current values
2. TOML must define all 48 loop-to-tier mappings exactly matching current hardcoded mappings
3. `ModelRouter::new()` must load TOML and populate `default_models` identically to current behavior
4. All public methods must remain unchanged:
   - `get_model(&self, loop_type) -> Option<&String>`
   - `register_custom(&mut self, loop_type, model_id)`
   - `create_client(&self, loop_type, api_key, base_url) -> Option<LlmClient>`
   - `create_client_for_deepwork`, `_bruteforce_coder`, `_juniors` (convenience methods)
   - `all_loops_mapped(&self) -> bool`

## Non-functional
1. Zero behaviour change -- all public API consumers (orchestrator, actor system, tests) work without modification
2. TOML file must be bundled in the crate (read at runtime, not compile-time macro)
3. File path resolution: use `include_str!` for compile-time embedding, or runtime path relative to crate root
4. Error on malformed TOML: `ModelRouter::new()` should panic with clear message if config is invalid

## Out of scope
- Runtime hot-reload of model config
- Per-environment config overrides
- Removing oh-my-opencode-slim.json fixer model slug (separate concern)
