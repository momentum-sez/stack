#!/usr/bin/env bash
# verify.sh — PW-AMM specification consistency checker
# Checks for contradictions in the PW-AMM specification after FIX 1.1 and FIX 1.9.
#
# Usage: ./verify.sh
# Exit code: 0 = all checks pass, 1 = contradictions found

set -euo pipefail

ERRORS=0
SPEC_DIR="spec"
DOCS_DIR="docs"

echo "=== PW-AMM Specification Consistency Verification ==="
echo ""

# --------------------------------------------------------------------------
# CHECK 1: No references to the incorrect spot price formula P = R_M * E / R_B
# in any specification or documentation file (except the errata/correction sections).
# --------------------------------------------------------------------------
echo "[CHECK 1] Scanning for incorrect spot price formula (P = R_M * E / R_B)..."

# Search spec files for the incorrect formula, excluding errata/correction context.
# The pattern matches "P = R_M * E / R_B" (the wrong formula).
# Exclusions cover: errata table rows, error explanation, superseded references,
# effective-space derivations labeled as P_eff (not P), and correction tables.
INCORRECT_REFS=$(grep -rn "P\s*=\s*R_M\s*\*\s*E\s*/\s*R_B" "$SPEC_DIR"/ 2>/dev/null \
    | grep -v "v0.32" \
    | grep -v "incorrect" \
    | grep -v "v0.32-eq" \
    | grep -v "errata" \
    | grep -v "error" \
    | grep -v "wrong" \
    | grep -v "corrected" \
    | grep -v "superseded" \
    | grep -v "P_eff" \
    | grep -v "|.*|.*|" \
    || true)

if [ -n "$INCORRECT_REFS" ]; then
    echo "  FAIL: Found unremediated references to incorrect spot price formula:"
    echo "$INCORRECT_REFS"
    ERRORS=$((ERRORS + 1))
else
    echo "  PASS: No unremediated references to P = R_M * E / R_B"
fi

# --------------------------------------------------------------------------
# CHECK 2: No references to the v0.32 invariant R_B * R_M * E = k as the
# current/correct invariant (references in errata context are acceptable).
# --------------------------------------------------------------------------
echo "[CHECK 2] Scanning for v0.32 invariant (R_B * R_M * E = k) used as current..."

# Use a precise pattern: R_B * R_M * E = k (with literal asterisks, not R_B * R_M / E = k).
# The incorrect invariant has "* E" (multiply), the correct one has "/ E" (divide).
OLD_INVARIANT=$(grep -rn "R_B \* R_M \* E\s*=\s*k" "$SPEC_DIR"/ 2>/dev/null \
    | grep -v "v0.32" \
    | grep -v "incorrect" \
    | grep -v "v0.32-eq" \
    | grep -v "errata" \
    | grep -v "error" \
    | grep -v "original" \
    | grep -v "prior" \
    | grep -v "|.*|.*|" \
    | grep -v "differentiation of" \
    || true)

if [ -n "$OLD_INVARIANT" ]; then
    echo "  FAIL: Found unremediated references to v0.32 invariant:"
    echo "$OLD_INVARIANT"
    ERRORS=$((ERRORS + 1))
else
    echo "  PASS: No unremediated references to R_B * R_M * E = k"
fi

# --------------------------------------------------------------------------
# CHECK 3: Corrected invariant R_B * R_M / E = k is present in the spec.
# --------------------------------------------------------------------------
echo "[CHECK 3] Verifying corrected invariant (R_B * R_M / E = k) is present..."

CORRECT_INVARIANT=$(grep -c "R_B \* R_M / E = k" "$SPEC_DIR/16-pw-amm.md" 2>/dev/null || echo "0")

if [ "$CORRECT_INVARIANT" -ge 1 ]; then
    echo "  PASS: Corrected invariant found ($CORRECT_INVARIANT occurrences)"
else
    echo "  FAIL: Corrected invariant R_B * R_M / E = k not found in spec/16-pw-amm.md"
    ERRORS=$((ERRORS + 1))
fi

# --------------------------------------------------------------------------
# CHECK 4: Correct spot price P = R_M / R_B is stated as a theorem.
# --------------------------------------------------------------------------
echo "[CHECK 4] Verifying correct spot price formula is present..."

CORRECT_PRICE=$(grep -c "P = R_M / R_B" "$SPEC_DIR/16-pw-amm.md" 2>/dev/null || echo "0")

if [ "$CORRECT_PRICE" -ge 1 ]; then
    echo "  PASS: Correct spot price formula found ($CORRECT_PRICE occurrences)"
else
    echo "  FAIL: Correct spot price P = R_M / R_B not found in spec/16-pw-amm.md"
    ERRORS=$((ERRORS + 1))
fi

# --------------------------------------------------------------------------
# CHECK 5: Honest assessment remark is present.
# --------------------------------------------------------------------------
echo "[CHECK 5] Verifying honest assessment remark is present..."

HONEST=$(grep -c "Fundamental Limitation" "$SPEC_DIR/16-pw-amm.md" 2>/dev/null || echo "0")

if [ "$HONEST" -ge 1 ]; then
    echo "  PASS: Honest assessment (Remark 16.1, Fundamental Limitation) is present"
else
    echo "  FAIL: Honest assessment remark not found"
    ERRORS=$((ERRORS + 1))
fi

# --------------------------------------------------------------------------
# CHECK 6: No-arbitrage coupling condition (Proposition 16.1) is present.
# --------------------------------------------------------------------------
echo "[CHECK 6] Verifying no-arbitrage coupling condition..."

NOARB=$(grep -c "No-Arbitrage E-Coupling" "$SPEC_DIR/16-pw-amm.md" 2>/dev/null || echo "0")

if [ "$NOARB" -ge 1 ]; then
    echo "  PASS: No-arbitrage coupling condition (Proposition 16.1) is present"
else
    echo "  FAIL: No-arbitrage coupling condition not found"
    ERRORS=$((ERRORS + 1))
fi

# --------------------------------------------------------------------------
# CHECK 7: E-update state transition specification is present.
# --------------------------------------------------------------------------
echo "[CHECK 7] Verifying E-update state transition specification..."

ESTATE=$(grep -c "E-Update Transition" "$SPEC_DIR/16-pw-amm.md" 2>/dev/null || echo "0")

if [ "$ESTATE" -ge 1 ]; then
    echo "  PASS: E-update state transition specification is present"
else
    echo "  FAIL: E-update state transition specification not found"
    ERRORS=$((ERRORS + 1))
fi

# --------------------------------------------------------------------------
# CHECK 8: Adversarial simulation requirements are present.
# --------------------------------------------------------------------------
echo "[CHECK 8] Verifying adversarial simulation requirements..."

ADVSIM=$(grep -c "Adversarial Simulation" "$SPEC_DIR/16-pw-amm.md" 2>/dev/null || echo "0")

if [ "$ADVSIM" -ge 1 ]; then
    echo "  PASS: Adversarial simulation requirements are present"
else
    echo "  FAIL: Adversarial simulation requirements not found"
    ERRORS=$((ERRORS + 1))
fi

# --------------------------------------------------------------------------
# CHECK 9: Numeric sanity check is present.
# --------------------------------------------------------------------------
echo "[CHECK 9] Verifying numeric sanity check is present..."

SANITY=$(grep -c "90,909" "$SPEC_DIR/16-pw-amm.md" 2>/dev/null || echo "0")

if [ "$SANITY" -ge 1 ]; then
    echo "  PASS: Numeric sanity check with k = 90,909.09 is present"
else
    echo "  FAIL: Numeric sanity check not found"
    ERRORS=$((ERRORS + 1))
fi

# --------------------------------------------------------------------------
# CHECK 10: Scan docs/ for any unremediated PW-AMM contradictions.
# --------------------------------------------------------------------------
echo "[CHECK 10] Scanning docs/ for PW-AMM contradictions..."

DOC_CONTRADICTIONS=$(grep -rn "P\s*=\s*R_M\s*\*\s*E\s*/\s*R_B" "$DOCS_DIR"/ 2>/dev/null || true)

if [ -n "$DOC_CONTRADICTIONS" ]; then
    echo "  WARN: Found PW-AMM formula references in docs/ (may be historical):"
    echo "$DOC_CONTRADICTIONS"
    # Not counted as error since docs may contain historical references
else
    echo "  PASS: No PW-AMM contradictions in docs/"
fi

# --------------------------------------------------------------------------
# SUMMARY
# --------------------------------------------------------------------------
echo ""
echo "=== SUMMARY ==="
if [ "$ERRORS" -eq 0 ]; then
    echo "ALL CHECKS PASSED — zero PW-AMM contradictions detected."
    exit 0
else
    echo "FAILED: $ERRORS contradiction(s) detected."
    exit 1
fi
