# Evolutionary Change Log (ECL)

This document defines the process for tracking and auditing changes to the UltraMeshAI repository.

## Directory Structure

*   `harness/changes/active/` — Current active change (maximum 1).
*   `harness/changes/parking/` — Paused changes.
*   `harness/changes/archive/` — Completed and merged changes.

## Change File Format

Every change must have a directory under `harness/changes/` named `YYYY-MM-DD-short-description/` containing:
1.  `summary.md` — What/why/how overview and verification commands.
2.  `spec.md` — Detailed requirements and success criteria.
3.  `plan.md` — Step-by-step implementation plan.
4.  `tasks.md` — Atomic task list with completion status.

## Verification

Before archiving a change:
1.  Run `cargo test` to ensure all tests pass.
2.  Run `nu scripts/lint-ecl.nu` to verify ECL compliance.
