# Audit Tracker — SEZ Stack Fortification

**Version:** 0.4.44 GENESIS
**Last updated:** 2026-02-12

This document tracks the status of audit findings and their remediation.

---

## Critical Findings

### P1-CRITICAL-01: PW-AMM Invariant/Spot Price Contradiction

| Field | Value |
|---|---|
| **Severity** | P1 — Critical (Mathematical Correctness) |
| **Status** | RESOLVED |
| **Fix** | FIX 1.1 |
| **Commit** | `fix(P1-CRITICAL-01): rewrite PW-AMM from coherent invariant — effective reserve model with honest assessment` |
| **Spec Section** | Chapter 16 (spec/16-pw-amm.md), Sections 16.1.3--16.1.7 |

**Finding:** The v0.32 whitepaper stated invariant `R_B * R_M * E = k` but claimed spot price `P = R_M * E / R_B`. Correct derivation from the stated invariant yields `P = R_M / R_B` (no E factor). The proof dropped a factor of E by computing the spot price in the effective reserve coordinate space without applying the Jacobian correction back to real-token units.

**Resolution:**
1. Rewrote Sections 16.1.3--16.1.7 using the effective reserve model (`R_{B,eff} = R_B / E`).
2. Derived spot price correctly as `P = R_M / R_B` with two independent proofs (implicit differentiation and Jacobian-corrected effective reserve derivation).
3. Included honest assessment (Remark 16.1): E cannot appear in the spot price under any constant-product invariant. E couples through invariant surface adjustment and reserve rebalancing, not through the spot price formula.
4. Added numeric sanity check: `R_B=1000, R_M=100, E=1.1, Delta_B=10` confirms invariant preservation and E-independence of swap output.
5. Added comparison table for `E = 0.8, 1.0, 1.2` confirming identical trading parameters.
6. Documented all v0.32 equation corrections in Section 16.1.7.

**Verification:** `verify.sh` confirms zero PW-AMM contradictions (no references to `P = R_M * E / R_B` in the specification).

---

### P3-CRITICAL-03: Missing No-Arbitrage Coupling Condition for PW-AMM

| Field | Value |
|---|---|
| **Severity** | P3 — Critical (Protocol Completeness) |
| **Status** | RESOLVED |
| **Fix** | FIX 1.9 |
| **Commit** | `fix(P3-CRITICAL-03): add no-arbitrage coupling condition and simulation requirements for PW-AMM` |
| **Spec Section** | Chapter 16, Sections 16.2--16.4 |

**Finding:** The PW-AMM specification lacked a formal no-arbitrage condition specifying how the event signal E achieves alignment with external market prices. Without this, E was a no-op parameter with no observable effect on trading behavior.

**Resolution:**
1. Added Proposition 16.1 (No-Arbitrage E-Coupling) with four necessary and sufficient conditions (C1--C4): E-update reserve adjustment, external price alignment, monotonic E-price coupling, and bounded E-updates.
2. Added Definition 16.4 (E-Update Transition) with full state transition specification including pre-conditions, execution steps, post-conditions, and event emission.
3. Added Definition 16.5 (Swap Transition) confirming swap mechanics are unchanged from standard constant-product AMM.
4. Added Section 16.4 (Adversarial Simulation Requirements) with five mandatory simulation scenarios: oracle manipulation (S1), sandwich attacks (S2), rapid E oscillation (S3), liquidity drain (S4), and clock skew (S5).
5. Added protocol parameter recommendations: `E_UPDATE_COOLDOWN = 300s`, `E_MAX_RATIO = 2.0`, `E_ORACLE_QUORUM = 3-of-5`.

**Verification:** Spec completeness check confirms all required sections present.

---

## Summary

| Finding | Severity | Status | Fix | Spec Section |
|---|---|---|---|---|
| P1-CRITICAL-01 | P1 Critical | RESOLVED | FIX 1.1 | 16.1.3--16.1.7 |
| P3-CRITICAL-03 | P3 Critical | RESOLVED | FIX 1.9 | 16.2--16.4 |
