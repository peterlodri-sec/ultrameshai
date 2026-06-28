# Support Policy

**Purpose:** Honest documentation about which loops are production-ready vs. experimental.

---

## Support Tiers

### `validated` ✅

**Definition:** Production-tested with ECL archive, all tests passing, used in real workflows.

**Requirements for promotion:**
- ECL change archived in `harness/changes/archive/`
- All `cargo test` tests pass
- Used in at least one validated end-to-end workflow
- No known critical bugs

**Current validated loops (10):**
- `deepwork` — Complex reasoning
- `bruteforce-coder` — Rapid code generation
- `yardmaster` — Task decomposition
- `testers` — Testing
- `devops` — DevOps tasks
- `ui` — UI tasks
- `red-team` — Security analysis
- `juniors` — Junior research bursts
- `ralph` — Pair observation
- `deep-research` — Deep research

---

### `guided` 📘

**Definition:** Works correctly (compiles + tests pass) but not yet production-validated.

**Requirements for promotion:**
- Compiles without errors
- All tests pass
- ECL change in progress or archived
- No known critical bugs

**Current guided loops (28):**
- **Coder sub-loops:** `coder-planner`, `coder-editor`, `coder-reviewer`
- **Deepwork sub-loops:** `deepwork-decomposer`, `deepwork-verifier`
- **Red-team sub-loops:** `redteam-fuzzer`, `redteam-analyzer`
- **SWE-bench pipeline:** `issue-analyzer`, `codebase-explorer`, `fix-planner`, `fix-implementer`, `edge-case-analyzer`, `regression-checker`, `diff-builder`
- **Testing expanded:** `tester-unit`, `tester-integration`, `tester-benchmark`, `tester-property`, `tester-mutation`
- **DevOps expanded:** `devops-build`, `devops-package`, `devops-deploy`, `devops-cache`
- **Quality:** `quality-lint`, `quality-audit`, `quality-coverage`, `quality-typecheck`, `quality-style`
- **Memory:** `memory-indexer`, `memory-pattern-miner`, `memory-summarizer`
- **Ralph expanded:** `ralph-coder`, `ralph-research`, `ralph-meta`
- **Research:** `research-web`, `research-docs`, `research-patterns`

---

### `experimental` 🧪

**Definition:** Skeleton implementation, TODOs remain, or untested in real workflows.

**Current experimental loops (10):**
- `infra-provisioner` — Skeleton only
- `infra-monitor` — Skeleton only
- `infra-balancer` — Skeleton only
- `reporter` — Skeleton only
- `validator` — Skeleton only
- `notebook` — Skeleton only
- `math-solver` — Skeleton only
- `math-verify` — Skeleton only
- `librarian` — Skeleton only (web research not wired)
- `ralph-meta` — Orchestrator logic incomplete

---

## Promotion Path

```
experimental → guided → validated
```

**To promote from experimental → guided:**
1. Fill in skeleton implementation
2. Add prompt template to `crates/cognition/src/prompt.rs`
3. Add model mapping to `crates/cognition/config/models.toml`
4. Ensure `cargo test` passes

**To promote from guided → validated:**
1. Create ECL change for production use
2. Run in real workflow (not just tests)
3. Archive ECL change
4. Document in `docs/workflow-catalog.md`

---

## Deprecation Policy

Loops may be deprecated if:
- Superseded by better implementation
- No longer maintained
- Critical bugs with no fix path

Deprecated loops remain in codebase but are marked in `docs/capability-matrix.json`.

---

## Getting Help

- **Validated loops:** File GitHub issue with reproduction
- **Guided loops:** File GitHub issue, expect community support
- **Experimental loops:** PRs welcome, no support guarantee

---

**Source of truth:** `docs/capability-matrix.json` (machine-readable)
