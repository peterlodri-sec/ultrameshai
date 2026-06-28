# Adopt oh-my-openagent-toolkit Governance Patterns

## What
Adopt the governance and documentation patterns from oh-my-openagent-toolkit: routing matrix, support tiers, validated workflows, and CLI tooling.

## Why
We have 48 loops but no honest documentation about which are production-ready. No routing guide for "which loop for which task". No CLI for ECL management or config validation. This creates friction for new users and makes it hard to trust the system.

## How
1. Create `docs/routing-matrix.md` — task type → loop type mapping
2. Create `docs/support-policy.md` — validated/guided/experimental tiers
3. Create `docs/workflow-catalog.md` — validated end-to-end workflows
4. Build `ultrameshai-cli` — ECL lint, config validate, doctor commands
5. Add `docs/capability-matrix.json` — machine-readable support levels

## Verification
- Routing matrix covers all 48 loops
- Support tiers are honest (validated = tested in production)
- CLI has `ecl-lint`, `config-validate`, `doctor` commands
- All docs are thin, link to code as source of truth
