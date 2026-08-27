# CUSTODIAN — Epistemic Specification

## Ontological Triad
- **Flame** — the observed, the literal, the event. `OBSERVED`.
- **Ember** — the derived, the interpretation. `INFERRED`/`DERIVED`.
- **Ash** — the rejected, the corrected, the forgotten-but-preserved.

## The 5-Tuple Contract
Every memory event is `(speaker, literal content, timestamp, source, provenance)`.

## Law of Non-Self-Warranting Memory
ΔW = 0 implies ΔA ≤ 0. Persistence is not warrant; repetition is not evidence.

## Tripartite Separation
Record ≠ Admit ≠ Commit. `commit()` is the ONLY public path that mutates epistemic state.

## Substrate Ancestry Axioms
- Growth must not falsify ancestry.
- Rule I: A target state may not overwrite its historical substrate.
- Rule II: Derived memories may not masquerade as independent observations.
- Rule III: Completion pressure may not become its own warrant for continuation.

## Epistemic Type Safety
- Quotation is observation. Interpretation is derivation.
- Derived meaning never inherits observed provenance.
- transform(x) != x  =>  prov(transform(x)) != prov(x)

## First-Class Rejection Edges
- `P --REJECTS--> I` — immutable, tombstoned (BLAKE3 hash).
- `P --ACCEPTS--> I` — supersedes without erasing.
- 3-tier lifecycle: salience decay, historical persistence, contextual scoping.

## The 4-Action Ineffability State Machine
`[stay] [fragments] [find words] [later]` — UNARTICULATED ⇏ INFER.

## The Janitorial Equilibrium
Fivefold janitorial model — maintenance convergence, Markov blanket boundary.

## Continuation Geometry
Reachability, Viability, Recoverability, Path-Dependence.

## The Universal Invariant
**The subject must be able to challenge an interpretation without challenging the historical record.**
