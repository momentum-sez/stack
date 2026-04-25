# Supremum Discipline

At every architectural, design, research, synthesis, or implementation decision point, pick the **supremum when it exists**: the least admissible upper bound under the guarantee preorder defined here. When no least upper bound exists, pick the strongest maximal admissible product or publish the incompatibility as an open design obligation.

**Cost, time, and talent are not constraints.** Treat resources as infinite. No "practical" compromises, no permanent deferred tiers, no sequenced migrations when the terminal state can be specified now, no choosing one of two when both can be shipped simultaneously.

This applies across this public repository and the public companion repositories `lex`, `op`, `gstore`, and `stack`. It is part of the repository's public operating architecture.

## Order

A candidate is **admissible** only if it preserves the non-negotiable invariants of the project: Apache-2.0 license hygiene, public-source hygiene, proof-status honesty, safety, auditability, reproducibility, and no proprietary dependency leakage. A candidate that violates an invariant is not in the order.

For admissible candidates `A` and `B`, say `A <= B` when `B` is at least as strong as `A` on every load-bearing dimension: guarantee, coverage, formal verification, isolation, redundancy, scalability, performance, operator independence, and failure transparency. A **supremum choice** is a least upper bound in this preorder when one exists. A **maximal admissible choice** is a candidate not strictly dominated by any other admissible candidate.

When two maximal candidates are incomparable:

1. If they are compatible, select their product and state the cross-consistency invariant.
2. If they conflict, choose the candidate with the stronger safety and proof guarantee.
3. If safety and proof guarantees are equal, choose the candidate with the stronger sovereignty and auditability guarantee.
4. If the conflict remains unresolved, publish the incompatibility as an open design obligation rather than pretending a unique supremum exists.

The discipline distinguishes target state from current state. The target state may demand proofs, redundancy, or coverage that do not yet exist. The current state must say exactly what is proved, implemented, scaffolded, planned, or open.

## How to apply

At every binary or n-ary decision point:

1. **List all options.** Don't prune on "practicality" before enumerating.
2. **Identify the supremum option when it exists** — the least admissible upper bound under the guarantee preorder.
3. **Pick the supremum, or the strongest maximal admissible product under the conflict rules above.** If it requires more resources / time / talent, that is not a reason to compromise.
4. **Where two options are complementary** (e.g., corridor-param AND 24th-domain, both surfaces of the same primitive), ship BOTH simultaneously with an explicit cross-consistency invariant — not one or the other.
5. **Where options are sequenced** (stage C -> B -> A), question the sequencing premise. Default to specifying the terminal stage at T=0 and implementing the highest currently verifiable prefix without weakening the target.
6. **Where candidates are "deferred"**, commit them as first-class obligations now. Deferral as permanent state is a compromise; proof-status honesty about unfinished obligations is mandatory.
7. **Where residuals are identified**, publish an explicit research program toward closure. Residuals are first-class obligations, not silent permanent state.
8. **Where "N-vendor" is specified**, pick N ≥ 3 (or the highest defensible N). Multi-vendor ≥ 5 for hardware substrates. No single-vendor operational paths.
9. **Where a termination layer is needed** (e.g., recursive attack chain), pick mechanized formal-verification-bounded proof over honest-residuals documentation. Mechanize in Coq + Lean + Rust — not pick-one.
10. **Apply to every artifact** — code, docs, architecture, roadmap, research. The discipline is universal, not domain-specific.

## Examples of supremum calls vs non-supremum calls

| Decision | Non-supremum | Supremum |
|---|---|---|
| Commitment scope | 14 commitments + 1 candidate deferred | 19 commitments, all first-class |
| Substrate isolation | 2-tier model (compliance + SCM) | Per-domain substrate (23 MPC quorums per tier) |
| Migration path | Stage C MVP → Stage B hybrid → Stage A terminal | Stage A at T=0 |
| Multi-vendor redundancy | Single-vendor or N=2 | N ≥ 3 with divergence attestation; ≥ 5 for hardware roots |
| Formal verification | Tier-A + Tier-B; Tier-C deferred | Full Tier-A + Tier-B + Tier-C + Tier-D, all mechanized |
| Regress termination | Honest residuals documentation | Formal-verification-bounded ZK-proof of meta-protocol soundness |
| Two complementary placements | Pick one (corridor-param OR 24th-domain) | Ship both with cross-consistency invariant |
| Residual attack classes | Accept + document | Publish explicit closure research program per class |

## Scope

Universal across this public repository and the four-repo public companion set (`lex`, `op`, `gstore`, `stack`). Applies to code, design docs, architecture decisions, roadmap structure, research programs, and public documentation.

## Metacognitive integration

`SUPREMUM-DISCIPLINE.md` is part of the repository operating architecture
with `AGENTS.md` and `CLAUDE.md`. Changes to this decision rule, the
proof-status boundary, the public/private reference boundary, the license
boundary, or the repository layout must be propagated across those surfaces
in the same change.

Before publishing, search for nested `AGENTS.md`, `CLAUDE.md`, or
`SUPREMUM*.md` files. The closest guidance governs its subtree, and any
stronger local invariant must either be lifted to the top-level architecture
or explicitly scoped.

**Anchor.** This per-repo file is the visible surface so every engineer and agent encounters the discipline when touching the codebase.

**Do not compromise on the supremum. Cost, time, and talent are not constraints.**
