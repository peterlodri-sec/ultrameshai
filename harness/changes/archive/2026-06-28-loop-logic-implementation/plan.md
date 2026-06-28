# Implementation Plan

## Phase 1a: Cleanup (done)
- [x] Remove `/tmp/gen_stubs.sh`
- [x] Create ECL change directory

## Phases 1b-1k: Parallel implementation (6 batches)

Each batch upgrades a group of stub files to the deepwork.rs pattern.
Batches are independent and run in parallel.

### Batch A — SWE-bench Pipeline (7 loops)
- issue-analyzer, codebase-explorer, fix-planner, fix-implementer, edge-case-analyzer, regression-checker, diff-builder

### Batch B — Testing Expanded (5 loops)
- tester-unit, tester-integration, tester-benchmark, tester-property, tester-mutation

### Batch C — DevOps + Quality (9 loops)
- devops-build, devops-package, devops-deploy, devops-cache, quality-lint, quality-audit, quality-coverage, quality-typecheck, quality-style

### Batch D — Memory + Ralph Expanded (6 loops)
- memory-indexer, memory-pattern-miner, memory-summarizer, ralph-coder, ralph-research, ralph-meta

### Batch E — Infra + Utility + Math (8 loops)
- infra-provisioner, infra-monitor, infra-balancer, reporter, validator, notebook, math-solver, math-verify

### Batch F — Research (3 loops)
- research-web, research-docs, research-patterns

## Phase 1l: Verification
- Run `cargo check`
- Run `cargo test`
- Run `nu scripts/lint-ecl.nu`
- Archive ECL change
