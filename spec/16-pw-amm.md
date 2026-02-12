# Chapter 16: Prediction-Weighted Automated Market Maker (PW-AMM) Settlement

**MSEZ Stack Specification v0.4.44**

This chapter specifies the mathematical foundations of the Prediction-Weighted Automated Market Maker (PW-AMM), a mechanism for coupling prediction market signals to spot liquidity pools used in cross-jurisdictional settlement corridors.

> **Errata notice (v0.4.44):** Sections 16.1.3 through 16.1.7 replace the prior formulation from v0.32, which contained a critical mathematical error in the spot price derivation (P1-CRITICAL-01). The original invariant `R_B * R_M * E = k` was paired with an incorrect spot price claim of `P = R_M * E / R_B`. This chapter provides the corrected derivation using the effective reserve model. See Section 16.1.6 for the honest assessment of E-coupling limitations.

> **Traceability:** Sections 16.1.3--16.1.7 in this chapter correspond to the external whitepaper sections 17.1.3--17.1.7. The chapter number was assigned to avoid collision with Chapter 17 (Agentic Execution Framework) in this specification.

---

## 16.1 PW-AMM Protocol

### 16.1.1 Motivation

Cross-jurisdictional settlement corridors require liquidity mechanisms that can respond to exogenous signals---prediction market outcomes, macroeconomic indicators, regulatory regime changes---without manual intervention. The PW-AMM extends the constant-product automated market maker with an event signal parameter `E` that couples external prediction market beliefs to the liquidity surface.

**Design goals:**
1. Maintain the simplicity and composability of constant-product AMMs.
2. Allow an external signal `E` to influence the economic equilibrium of the pool.
3. Preserve the key AMM invariant: every swap is path-independent and immediately settled.
4. Ensure mathematical coherence between the stated invariant, the spot price formula, and the swap output formula.

### 16.1.2 Original Formulation (v0.32) --- Error Identification

The v0.32 whitepaper defined the PW-AMM as follows:

**Definitions:**
- `R_B` --- reserve of the base token (e.g., settlement currency)
- `R_M` --- reserve of the minted token (e.g., corridor-specific asset)
- `E` --- event signal from an external prediction market, `E in (0, +inf)`
- `k` --- invariant constant, fixed at pool initialization

**v0.32 invariant:**

```
R_B * R_M * E = k                                    ... (v0.32-eq1)
```

**v0.32 spot price claim:**

```
P = R_M * E / R_B                                    ... (v0.32-eq2)
```

where `P` is the number of minted tokens received per unit of base token at the margin.

**The error.** Equation (v0.32-eq2) does not follow from equation (v0.32-eq1). The correct derivation from the stated invariant:

Holding `E` constant (as a parameter, not a traded variable), implicit differentiation of `R_B * R_M * E = k` gives:

```
E * (R_M * dR_B + R_B * dR_M) = 0
```

Since `E > 0`:

```
R_M * dR_B + R_B * dR_M = 0
dR_M / dR_B = -R_M / R_B
```

The spot price (marginal rate of exchange, M received per B deposited):

```
P = |dR_M / dR_B| = R_M / R_B                        ... (correct)
```

**The factor of `E` does not appear.** The v0.32 derivation erroneously computed the spot price in the effective reserve space (`R_M / R_{B,eff}`) without applying the Jacobian correction to convert back to real-token units. See Section 16.1.4 for the complete derivation showing where this error occurs.

---

### 16.1.3 Effective Reserve Model (Corrected)

**Definition 16.1 (Effective Base Reserve).**

```
R_{B,eff} := R_B / E
```

where `E in (0, +inf)` is the event signal parameter. When `E = 1`, the effective reserve equals the actual reserve.

**Definition 16.2 (PW-AMM Invariant).**

The PW-AMM invariant is defined as a constant-product over the effective base reserve and the minted reserve:

```
R_{B,eff} * R_M = k

equivalently:

(R_B / E) * R_M = k

equivalently:

R_B * R_M = k * E                                    ... (16.1)
```

where `k > 0` is the invariant constant, fixed at pool initialization:

```
k = R_B(0) * R_M(0) / E(0)                           ... (16.2)
```

**Interpretation.** The effective reserve `R_{B,eff} = R_B / E` represents the base token reserve as "seen" by the invariant surface. When `E > 1`, the effective reserve is smaller than the actual reserve, tightening the invariant curve. When `E < 1`, the effective reserve is larger, loosening it. The actual tokens held by the pool are always `R_B` and `R_M`.

**Remark.** The v0.32 invariant `R_B * R_M * E = k` is algebraically equivalent to `R_B * R_M = k / E`, which places `E` in the denominator of the product target. The corrected invariant `R_B * R_M = k * E` places `E` in the numerator. These are distinct models with different economic behavior when `E` changes. The corrected model is adopted because it gives `R_{B,eff} = R_B / E` the natural interpretation: higher event probability reduces the effective base reserve, tightening the pool around a new equilibrium.

---

### 16.1.4 Spot Price Derivation

**Theorem 16.1 (PW-AMM Spot Price).** Under the invariant of Definition 16.2, the spot price of the minted token in terms of the base token is:

```
P = R_M / R_B                                        ... (16.3)
```

The event signal `E` does not appear in the spot price formula.

**Proof.** Two independent derivations are provided to confirm the result.

*Derivation 1 (Direct implicit differentiation).*

From `R_B * R_M = k * E`, with `E` held constant during a swap:

```
d(R_B * R_M) = 0
R_M * dR_B + R_B * dR_M = 0
dR_M / dR_B = -R_M / R_B
```

The spot price (M tokens received per B token deposited, at the margin):

```
P = |dR_M / dR_B| = R_M / R_B                        QED (Derivation 1)
```

*Derivation 2 (Effective reserve space with Jacobian correction).*

Working in the effective reserve space `(R_{B,eff}, R_M)` where `R_{B,eff} * R_M = k`:

The "effective spot price" (M per effective-B):

```
P_eff = R_M / R_{B,eff} = R_M / (R_B / E) = R_M * E / R_B
```

This is the expression that v0.32 incorrectly reported as the real spot price. However, a trader deposits **real** base tokens `dR_B`, not effective tokens `dR_{B,eff}`. The Jacobian of the transformation is:

```
dR_{B,eff} / dR_B = 1/E
```

Therefore the real spot price is:

```
P_real = P_eff * (dR_{B,eff} / dR_B)
       = (R_M * E / R_B) * (1 / E)
       = R_M / R_B                                    QED (Derivation 2)
```

**Remark (Where v0.32 went wrong).** The v0.32 derivation computed `P_eff = R_M * E / R_B` in the effective reserve space and reported it as the real-token price without applying the `1/E` Jacobian correction. This is the "dropped factor of E" identified in the audit finding P1-CRITICAL-01. The factor of `E` that enters through `R_{B,eff}` in the numerator is exactly cancelled by the `1/E` chain rule factor from the coordinate transformation.

---

### 16.1.5 Swap Mechanics Under the Effective Reserve Model

**Theorem 16.2 (Swap Output).** For a trader depositing `Delta_B > 0` base tokens to receive `Delta_M` minted tokens:

```
Delta_M = R_M * Delta_B / (R_B + Delta_B)             ... (16.4)
```

For the reverse direction (depositing `Delta_M > 0` minted tokens to receive `Delta_B` base tokens):

```
Delta_B = R_B * Delta_M / (R_M + Delta_M)             ... (16.5)
```

**The event signal `E` does not appear in either swap formula.**

**Proof (forward direction).**

After deposit of `Delta_B`, the new reserves must satisfy the invariant:

```
(R_B + Delta_B) * (R_M - Delta_M) / E = k = R_B * R_M / E
```

Since `E > 0`, we can multiply both sides by `E`:

```
(R_B + Delta_B) * (R_M - Delta_M) = R_B * R_M
```

Solving for `Delta_M`:

```
R_M - Delta_M = R_B * R_M / (R_B + Delta_B)
Delta_M = R_M - R_B * R_M / (R_B + Delta_B)
Delta_M = R_M * [1 - R_B / (R_B + Delta_B)]
Delta_M = R_M * Delta_B / (R_B + Delta_B)             QED
```

**Corollary.** The swap output formula (16.4) is identical to the standard constant-product (Uniswap v2) formula `x * y = k`. The effective reserve model does not alter the swap mechanics for any given state `(R_B, R_M)`.

---

**Worked Example (Sanity Check).**

Parameters: `R_B = 1000`, `R_M = 100`, `E = 1.1`

Step 1 --- Compute invariant constant:
```
k = R_B * R_M / E = 1000 * 100 / 1.1 = 90,909.0909...
```

Step 2 --- Swap `Delta_B = 10` base tokens for minted tokens:
```
Delta_M = R_M * Delta_B / (R_B + Delta_B)
        = 100 * 10 / (1000 + 10)
        = 1000 / 1010
        = 0.990099...
```

Step 3 --- New reserves:
```
R_B' = 1010
R_M' = 100 - 1000/1010 = 100000/1010 = 99.009900...
```

Step 4 --- Verify invariant preserved:
```
R_B' * R_M' / E = 1010 * (100000/1010) / 1.1
                = 100000 / 1.1
                = 90,909.0909...
                = k                                    CONFIRMED
```

Step 5 --- Verify effective execution price:
```
P_exec = Delta_B / Delta_M = 10 / 0.990099 = 10.10 B per M
P_spot = R_B / R_M = 1000 / 100 = 10.00 B per M
Price impact = (10.10 - 10.00) / 10.00 = 1.0%
```

---

### 16.1.6 E-Signal Coupling: Honest Assessment

**Remark 16.1 (Fundamental Limitation).** Under any constant-product invariant of the form `f(R_B, E) * R_M = k` where `f` is a function of `R_B` and `E`, the spot price in real-token units is always:

```
P = R_M / R_B
```

regardless of the choice of `f`. This is because `E` is a parameter (not a traded reserve), so it factors out of the implicit differentiation. The Jacobian of the effective-to-real coordinate transformation always cancels the `E` factor introduced by the effective reserve definition.

**Consequence:** The design goal "E couples prediction market beliefs to spot prices" **cannot be achieved** through a constant-product invariant alone. The event signal `E` does not enter the spot price formula, does not alter the swap output for a given reserve state, and does not change the slippage profile.

**How E actually couples to the system.** The event signal `E` affects the PW-AMM through two indirect mechanisms:

1. **Invariant surface adjustment.** When `E` changes (see Section 16.3 for the state transition specification), the invariant constant `k` must be recalculated as `k_new = R_B * R_M / E_new` to keep the current reserves on the invariant curve. If `k` is instead held fixed, the current reserves `(R_B, R_M)` are no longer on the invariant surface and the pool enters an inconsistent state (see Section 16.2.1 for analysis).

2. **Equilibrium rebalancing.** If an external mechanism adjusts the reserves when `E` changes (see Section 16.3, Options A and B), then the reserve ratio `R_M / R_B` changes, which **does** change the spot price. In this case, `E` influences the price indirectly through the rebalancing mechanism, not through the spot price formula itself.

**Design implication.** For `E` to have a meaningful effect on market prices, the PW-AMM requires an explicit **E-update state transition** (Section 16.3) that prescribes how reserves are adjusted when `E` changes. Without this specification, `E` is a no-op parameter that has no observable effect on trading.

---

**Sanity Check Table.** The following table confirms that for fixed reserves `R_B = 1000`, `R_M = 100`, the swap of `Delta_B = 10` produces identical results regardless of `E`:

| Parameter | E = 0.8 | E = 1.0 | E = 1.2 |
|---|---|---|---|
| k = R_B * R_M / E | 125,000.00 | 100,000.00 | 83,333.33 |
| R_{B,eff} = R_B / E | 1,250.00 | 1,000.00 | 833.33 |
| Spot price P = R_M / R_B | 0.1000 | 0.1000 | 0.1000 |
| Swap output Delta_M (for Delta_B = 10) | 0.9901 | 0.9901 | 0.9901 |
| Effective price (B per M) | 10.10 | 10.10 | 10.10 |
| Slippage | 1.00% | 1.00% | 1.00% |
| LP value (at P_ext = 10 B/M) | 2,000 B | 2,000 B | 2,000 B |

**Observation.** All trading parameters are identical across `E` values. This is the expected and mathematically necessary outcome under the constant-product invariant. `E` only has observable effects when it **changes** and the pool undergoes a state transition (Section 16.3).

---

### 16.1.7 Downstream Equation Corrections

**Equations changed from v0.32.** The following equations from the v0.32 whitepaper are corrected in this chapter:

| Location | v0.32 (incorrect) | v0.4.44 (correct) | Reason |
|---|---|---|---|
| Invariant | R_B * R_M * E = k | R_B * R_M / E = k (eq. 16.1) | Effective reserve model; `E` in denominator gives natural interpretation |
| Spot price | P = R_M * E / R_B | P = R_M / R_B (eq. 16.3) | Jacobian correction; `E` cancels in real-token units |
| Swap output (forward) | Delta_M = R_M * E * Delta_B / (R_B + Delta_B) | Delta_M = R_M * Delta_B / (R_B + Delta_B) (eq. 16.4) | `E` was erroneously retained in swap formula |
| Swap output (reverse) | Delta_B = R_B * Delta_M / (E * R_M + Delta_M) | Delta_B = R_B * Delta_M / (R_M + Delta_M) (eq. 16.5) | Same error in reverse direction |

**Downstream references.** All references to `P = R_M * E / R_B` in the specification are superseded by Theorem 16.1 (eq. 16.3). No downstream references to this formula exist in the current MSEZ specification chapters 10--97. The roadmap document `docs/roadmap/ROADMAP_V043_HARD_MODE.md` Section 8.2 (EquilibriumAMM Settlement Integration) references AMM settlement but does not contain the erroneous formula.

---

## 16.2 No-Arbitrage Coupling Condition

> **Added by FIX 1.9 (P3-CRITICAL-03).** This section formalizes the conditions under which the PW-AMM event signal `E` achieves no-arbitrage alignment with external markets.

**Proposition 16.1 (No-Arbitrage E-Coupling).** For a PW-AMM with invariant `R_B * R_M / E = k` to achieve prediction-market-coupled pricing such that the pool's spot price tracks the external market's assessment of the event signal, the following conditions are **necessary and sufficient**:

**(C1) E-Update Reserve Adjustment.** When the event signal changes from `E_old` to `E_new`, the pool MUST execute exactly one of the following reserve adjustment strategies:

- **Strategy A (Base-side adjustment):** Adjust the base reserve:
  ```
  R_B' = R_B * (E_new / E_old)
  R_M' = R_M                          (unchanged)
  ```
  Resulting spot price change: `P_new = P_old * (E_old / E_new)`

- **Strategy B (Mint-side adjustment):** Adjust the minted reserve:
  ```
  R_B' = R_B                          (unchanged)
  R_M' = R_M * (E_new / E_old)
  ```
  Resulting spot price change: `P_new = P_old * (E_new / E_old)`

Both strategies preserve the invariant: `R_B' * R_M' / E_new = R_B * R_M / E_old = k`.

**(C2) External Price Alignment.** At equilibrium, the PW-AMM spot price must equal the external market price:
```
R_M / R_B = P_external
```
The E-update mechanism (C1) must converge to this condition. Under Strategy A, convergence requires that `E_new / E_old` tracks the inverse of the external price movement. Under Strategy B, it tracks the direct price movement.

**(C3) Monotonic E-Price Coupling.** The mapping from `E` to the induced spot price change must be monotonic. For Strategy A (recommended for settlement corridors): higher `E` implies lower `P` (base token appreciates relative to minted token when the prediction market favors the event). For Strategy B: higher `E` implies higher `P`.

**(C4) Bounded E-Updates.** For any single E-update, the ratio `E_new / E_old` MUST satisfy:
```
1 / E_MAX_RATIO <= E_new / E_old <= E_MAX_RATIO
```
where `E_MAX_RATIO` is a protocol parameter (recommended: `E_MAX_RATIO = 2.0` for production corridors). This prevents catastrophic reserve adjustments from oracle manipulation.

**Proof sketch (sufficiency).** Given (C1), each E-update produces a deterministic reserve adjustment that changes the spot price. Given (C2), the target price is well-defined. Given (C3), the adjustment is monotonic in `E`, ensuring convergence. Given (C4), no single update can drain the pool. Together, these conditions ensure that the PW-AMM's spot price tracks the external prediction market signal through discrete reserve adjustments, achieving no-arbitrage alignment at each E-update epoch.

**Proof sketch (necessity).** Without (C1), E-updates have no effect on spot price (Remark 16.1). Without (C2), there is no target for convergence and arbitrage persists. Without (C3), oscillation is possible. Without (C4), a single oracle manipulation can drain the pool. Each condition is therefore necessary.

---

## 16.3 State Transition Specification for E-Updates

> **Added by FIX 1.9 (P3-CRITICAL-03).**

**Definition 16.3 (PW-AMM State).** The complete state of a PW-AMM pool at time `t` is the tuple:

```
S(t) = (R_B, R_M, E, k, t_last_update)
```

where:
- `R_B > 0` --- actual base token reserve
- `R_M > 0` --- actual minted token reserve
- `E > 0` --- current event signal
- `k > 0` --- invariant constant, `k = R_B * R_M / E`
- `t_last_update` --- timestamp of the last E-update

**Definition 16.4 (E-Update Transition).** An E-update transition transforms the pool state upon receiving a new event signal from the oracle:

```
TRANSITION E_UPDATE(S, E_new, t_now):

  PRE-CONDITIONS:
    (P1) E_new > 0
    (P2) t_now >= t_last_update + E_UPDATE_COOLDOWN
    (P3) 1/E_MAX_RATIO <= E_new/E <= E_MAX_RATIO
    (P4) E_new != E  (no-op guard)

  EXECUTION (Strategy A --- base-side adjustment):
    alpha := E_new / E
    R_B' := R_B * alpha
    R_M' := R_M
    k'   := R_B' * R_M' / E_new   (= R_B * R_M / E = k, invariant preserved)
    E'   := E_new
    t'   := t_now

  POST-CONDITIONS:
    (Q1) R_B' * R_M' / E' = k     (invariant preserved)
    (Q2) R_B' > 0 AND R_M' > 0    (reserves positive)
    (Q3) |R_B' - R_B| / R_B <= E_MAX_RATIO - 1   (bounded adjustment)

  EMIT EVENT:
    EUpdateApplied {
      pool_id,
      E_old: E,
      E_new: E_new,
      alpha: alpha,
      R_B_old: R_B,
      R_B_new: R_B',
      spot_price_old: R_M / R_B,
      spot_price_new: R_M' / R_B',
      timestamp: t_now
    }

  RETURN S' = (R_B', R_M', E', k', t')
```

**Protocol Parameters:**

| Parameter | Recommended Value | Description |
|---|---|---|
| `E_UPDATE_COOLDOWN` | 300 seconds | Minimum time between E-updates |
| `E_MAX_RATIO` | 2.0 | Maximum single-step E change ratio |
| `E_ORACLE_QUORUM` | 3-of-5 | Oracle quorum for E-update validity |

**Definition 16.5 (Swap Transition).** Swap transitions are unchanged from the standard constant-product AMM:

```
TRANSITION SWAP_B_TO_M(S, Delta_B):

  PRE-CONDITIONS:
    (P1) Delta_B > 0
    (P2) Delta_B <= R_B * MAX_SWAP_FRACTION   (circuit breaker)

  EXECUTION:
    Delta_M := R_M * Delta_B / (R_B + Delta_B)
    R_B'    := R_B + Delta_B
    R_M'    := R_M - Delta_M

  POST-CONDITIONS:
    (Q1) R_B' * R_M' / E = k       (invariant preserved, E unchanged)
    (Q2) Delta_M > 0

  RETURN (S' = (R_B', R_M', E, k, t_last_update), Delta_M)
```

---

## 16.4 Adversarial Simulation Requirements

> **Added by FIX 1.9 (P3-CRITICAL-03).**

**Remark 16.2 (Adversarial Simulation).** Before any production deployment of the PW-AMM mechanism, the following adversarial scenarios MUST be simulated and the results documented:

**(S1) Oracle Manipulation Attack.** Simulate an attacker who controls the prediction market oracle and can set arbitrary `E` values (within the `E_MAX_RATIO` bound). Verify:
- The bounded E-update condition (C4 of Proposition 16.1) limits the maximum single-step reserve adjustment.
- The cooldown period `E_UPDATE_COOLDOWN` limits the rate of adjustment.
- Compute the maximum extractable value (MEV) from `n` consecutive manipulated E-updates.
- **Acceptance criterion:** MEV from `n = 10` consecutive manipulated updates MUST be less than 5% of pool TVL.

**(S2) Sandwich Attack on E-Updates.** Simulate an attacker who observes a pending E-update and executes trades immediately before and after:
```
1. Attacker observes pending E_UPDATE(E_new)
2. Attacker executes SWAP_B_TO_M(Delta_B_front) at old reserves
3. E_UPDATE executes, adjusting reserves
4. Attacker executes SWAP_M_TO_B(Delta_M_back) at new reserves
```
Verify:
- Compute attacker profit as a function of `Delta_B_front` and `|E_new - E_old|`.
- **Acceptance criterion:** Maximum profit from sandwich MUST be less than 1% of trade size for `E_MAX_RATIO <= 2.0`.

**(S3) Rapid E Oscillation.** Simulate `E` oscillating between `E_low` and `E_high` every `E_UPDATE_COOLDOWN` seconds for 24 hours. Verify:
- LP positions do not lose more than 10% of initial value (beyond normal impermanent loss).
- Pool reserves remain within operational bounds (neither token approaches zero).
- **Acceptance criterion:** LP drawdown from oscillation MUST be bounded by `2 * |E_high/E_low - 1| * num_oscillations * avg_trade_volume / TVL`.

**(S4) Liquidity Drain.** Simulate an attacker attempting to extract maximum liquidity through repeated trades across E-updates. Verify:
- The constant invariant `k` prevents total extraction.
- Emergency halt conditions trigger before reserve depletion.
- **Acceptance criterion:** Reserves MUST NOT fall below 10% of initial values under any attack sequence of length 1000.

**(S5) Clock Skew and Timestamp Manipulation.** Simulate E-updates with manipulated timestamps (up to 5 minutes in the future). Verify:
- The cooldown check uses monotonic chain time, not wall clock.
- Future-dated updates are rejected.
- **Acceptance criterion:** Zero successful cooldown bypasses under any timestamp manipulation.

**Implementation note.** These simulations SHOULD be implemented as property-based tests using a framework such as Hypothesis (Python) or proptest (Rust). Each simulation SHOULD run for at least 10,000 iterations with randomized parameters.

---

## 16.5 Security Considerations

### 16.5.1 Oracle Trust Model

The PW-AMM's security depends critically on the integrity of the event signal `E`. A compromised oracle can manipulate reserves through E-updates (bounded by `E_MAX_RATIO`). Implementations MUST:

1. Require oracle quorum (`E_ORACLE_QUORUM`) for E-updates.
2. Implement oracle rotation and slashing for misbehavior.
3. Log all E-updates in the corridor audit trail (Chapter 17, Definition 17.7).

### 16.5.2 Front-Running Protection

E-updates SHOULD be committed using a commit-reveal scheme:
1. Oracle commits `H(E_new || nonce)` at time `t`.
2. Oracle reveals `E_new` and `nonce` at time `t + REVEAL_DELAY`.
3. Pool applies E-update only after reveal.

This prevents front-running of E-updates by MEV extractors.

---

## Schema References

- `schemas/pw-amm.pool-state.schema.json` (to be created)
- `schemas/pw-amm.e-update-event.schema.json` (to be created)
- `schemas/pw-amm.swap-receipt.schema.json` (to be created)
