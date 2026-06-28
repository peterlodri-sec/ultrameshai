# Routing Matrix

**Purpose:** Map task types to loop types. When you have a task, find which loop to use.

---

## SWE-Bench Pipeline

| Task | Primary Loop | Model Tier | Fallback |
|------|-------------|------------|----------|
| Analyze GitHub issue | `issue-analyzer` | FRONTIER | `librarian` |
| Explore codebase structure | `codebase-explorer` | CODE_BEST | `juniors` |
| Plan a fix | `fix-planner` | REASONING | `deepwork` |
| Implement a fix | `fix-implementer` | CODE_BEST | `bruteforce-coder` |
| Identify edge cases | `edge-case-analyzer` | FRONTIER | `tester-unit` |
| Check for regressions | `regression-checker` | CODE_MID | `testers` |
| Build clean diff | `diff-builder` | CODE_SPEC | `fix-implementer` |

---

## Testing

| Task | Primary Loop | Model Tier | Fallback |
|------|-------------|------------|----------|
| Write unit tests | `tester-unit` | CODE_SMALL | `testers` |
| Write integration tests | `tester-integration` | CODE_MID | `testers` |
| Run benchmarks | `tester-benchmark` | CODE_SMALL | — |
| Property-based testing | `tester-property` | CODE_MID | `tester-unit` |
| Mutation testing | `tester-mutation` | CODE_BEST | `tester-unit` |

---

## DevOps

| Task | Primary Loop | Model Tier | Fallback |
|------|-------------|------------|----------|
| Build project | `devops-build` | CODE_SMALL | `devops` |
| Package artifacts | `devops-package` | CODE_SMALL | `devops` |
| Deploy to environment | `devops-deploy` | CODE_SMALL | `devops` |
| Manage cache | `devops-cache` | GEN_SMALL | `devops` |

---

## Quality

| Task | Primary Loop | Model Tier | Fallback |
|------|-------------|------------|----------|
| Lint code | `quality-lint` | CODE_SMALL | — |
| Security audit | `quality-audit` | CODE_MID | `red-team` |
| Coverage report | `quality-coverage` | CODE_SMALL | `tester-unit` |
| Type checking | `quality-typecheck` | CODE_SMALL | — |
| Style enforcement | `quality-style` | CODE_SMALL | `quality-lint` |

---

## Research / Knowledge

| Task | Primary Loop | Model Tier | Fallback |
|------|-------------|------------|----------|
| Web research | `research-web` | FRONTIER | `librarian` |
| Documentation lookup | `research-docs` | GEN_LARGE | `librarian` |
| Find similar issues/PRs | `research-patterns` | REASONING | `memory-pattern-miner` |
| External knowledge retrieval | `librarian` | FRONTIER | `deep-research` |

---

## Memory / Learning

| Task | Primary Loop | Model Tier | Fallback |
|------|-------------|------------|----------|
| Index findings to milvus | `memory-indexer` | GEN_SMALL | — |
| Mine patterns from history | `memory-pattern-miner` | REASONING | `honcho` detector |
| Summarize for context | `memory-summarizer` | GEN_SMALL | — |

---

## Ralph Observation

| Task | Primary Loop | Model Tier | Fallback |
|------|-------------|------------|----------|
| Observe coder-tester pair | `ralph-coder` | GEN_TINY | — |
| Observe research-redteam pair | `ralph-research` | GEN_TINY | — |
| Observe entire pipeline | `ralph-meta` | FRONTIER | `yardmaster` |

---

## Infrastructure

| Task | Primary Loop | Model Tier | Fallback |
|------|-------------|------------|----------|
| Provision mesh node | `infra-provisioner` | GEN_SMALL | `devops` |
| Monitor node health | `infra-monitor` | GEN_SMALL | — |
| Balance load across nodes | `infra-balancer` | GEN_MID | — |

---

## Utility

| Task | Primary Loop | Model Tier | Fallback |
|------|-------------|------------|----------|
| Generate reports | `reporter` | FRONTIER | `ui` |
| Validate outputs | `validator` | REASONING | `quality-typecheck` |
| Interactive analysis | `notebook` | CODE_MID | — |
| Math problems | `math-solver` | MATH | `deepwork` |
| Verify proofs | `math-verify` | MATH | `validator` |

---

## Original 10 (General Purpose)

| Task | Primary Loop | Model Tier | Fallback |
|------|-------------|------------|----------|
| Complex reasoning | `deepwork` | FRONTIER | `yardmaster` |
| Rapid code generation | `bruteforce-coder` | CODE_BEST | `fix-implementer` |
| Deep research | `deep-research` | FRONTIER | `librarian` |
| Testing tasks | `testers` | CODE_MID | `tester-unit` |
| Task decomposition | `yardmaster` | FRONTIER | `deepwork` |
| DevOps tasks | `devops` | CODE_MID | `devops-build` |
| UI tasks | `ui` | FRONTIER | `reporter` |
| Red team / security | `red-team` | CODE_BEST | `quality-audit` |
| Junior research bursts | `juniors` | CODE_SMALL | `librarian` |
| Pair observation | `ralph` | GEN_TINY | `ralph-meta` |

---

## Model Tier Reference

| Tier | Virtual Query | Use For |
|------|--------------|---------|
| FRONTIER | `meta-llama@latest?params>=70` | Complex reasoning, planning, high-stakes decisions |
| REASONING | `deepseek@latest` | Logic, verification, analysis |
| CODE_BEST | `code_chat@latest` | Code generation, review, refactoring |
| CODE_MID | `code_chat@latest?params>=14` | Testing, DevOps, medium complexity code |
| CODE_SMALL | `mistral@latest` | Simple code tasks, linting, unit tests |
| CODE_SPEC | `codestral@latest` | Specialized code tasks, diff generation |
| GEN_LARGE | `qwen@latest` | Documentation, general knowledge |
| GEN_MID | `qwen@latest?params>=32` | Medium complexity generation |
| GEN_SMALL | `mistral@latest` | Simple generation, infra tasks |
| GEN_TINY | `mistral@latest` | Observation, lightweight tasks |
| MATH | `mistral@latest` | Mathematical reasoning |

---

**Source of truth:** `crates/cognition/config/models.toml`
