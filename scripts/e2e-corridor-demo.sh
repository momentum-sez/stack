#!/usr/bin/env bash
# ============================================================================
# MEZ Stack — End-to-End Corridor Lifecycle Demo
# ============================================================================
#
# Proves the "AWS of Economic Zones" value proposition by exercising:
#
#   1. Zone bootstrapping (PK-SIFC + AE-DIFC)
#   2. Corridor establishment (DRAFT → PENDING → ACTIVE)
#   3. Cross-border receipt exchange (5 receipts with dual-commitment)
#   4. Receipt chain query + hash-chain integrity verification
#   5. Checkpoint creation (MMR + hash-chain snapshot)
#   6. Compliance tensor query (20 domains, jurisdiction-scoped)
#   7. Bilateral corridor compliance evaluation
#
# Prerequisites:
#   - The mez-api server running on localhost:8080 (or $MEZ_API_URL)
#     Start with: cargo run -p mez-api
#   - curl and jq installed
#
# Usage:
#   ./scripts/e2e-corridor-demo.sh
#   MEZ_API_URL=http://localhost:3000 ./scripts/e2e-corridor-demo.sh
#
# Exit codes:
#   0 — All assertions passed
#   1 — Assertion failure (with diagnostic output)
#
# ============================================================================

set -euo pipefail

# Configuration
API="${MEZ_API_URL:-http://localhost:8080}"
AUTH_HEADER=""
if [ -n "${MEZ_AUTH_TOKEN:-}" ]; then
    AUTH_HEADER="-H \"Authorization: Bearer ${MEZ_AUTH_TOKEN}\""
fi

# Colors (disabled if not a terminal)
if [ -t 1 ]; then
    GREEN='\033[0;32m'
    RED='\033[0;31m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    RESET='\033[0m'
else
    GREEN='' RED='' BLUE='' BOLD='' RESET=''
fi

pass_count=0
fail_count=0

# Assertion helpers
assert_eq() {
    local desc="$1" expected="$2" actual="$3"
    if [ "$expected" = "$actual" ]; then
        echo -e "  ${GREEN}✓${RESET} $desc"
        ((pass_count++))
    else
        echo -e "  ${RED}✗${RESET} $desc: expected '$expected', got '$actual'"
        ((fail_count++))
    fi
}

assert_ne() {
    local desc="$1" unexpected="$2" actual="$3"
    if [ "$unexpected" != "$actual" ]; then
        echo -e "  ${GREEN}✓${RESET} $desc"
        ((pass_count++))
    else
        echo -e "  ${RED}✗${RESET} $desc: got unexpected '$actual'"
        ((fail_count++))
    fi
}

assert_len() {
    local desc="$1" expected_len="$2" actual="$3"
    local actual_len="${#actual}"
    if [ "$actual_len" -eq "$expected_len" ]; then
        echo -e "  ${GREEN}✓${RESET} $desc (${actual_len} chars)"
        ((pass_count++))
    else
        echo -e "  ${RED}✗${RESET} $desc: expected ${expected_len} chars, got ${actual_len}"
        ((fail_count++))
    fi
}

assert_gt() {
    local desc="$1" threshold="$2" actual="$3"
    if [ "$actual" -gt "$threshold" ]; then
        echo -e "  ${GREEN}✓${RESET} $desc ($actual > $threshold)"
        ((pass_count++))
    else
        echo -e "  ${RED}✗${RESET} $desc: expected > $threshold, got $actual"
        ((fail_count++))
    fi
}

section() {
    echo ""
    echo -e "${BOLD}${BLUE}── $1 ──${RESET}"
}

# ── Preflight: check API is reachable ─────────────────────────────────

section "Step 0: Preflight Check"

HEALTH=$(curl -sf "${API}/health/liveness" 2>/dev/null || echo "UNREACHABLE")
if [ "$HEALTH" = "UNREACHABLE" ]; then
    echo -e "${RED}ERROR: MEZ API not reachable at ${API}${RESET}"
    echo "Start the server with: cargo run -p mez-api"
    echo "Or set MEZ_API_URL to the correct URL."
    exit 1
fi
assert_eq "API liveness" "ok" "$HEALTH"

READY=$(curl -sf "${API}/health/readiness" 2>/dev/null || echo "NOT_READY")
echo -e "  ${GREEN}✓${RESET} API readiness: ${READY}"

# ── Step 1: Create a PK ↔ AE corridor ────────────────────────────────

section "Step 1: Create Corridor (PK ↔ AE)"

CORRIDOR_RESP=$(curl -sf -X POST "${API}/v1/corridors" \
    -H "Content-Type: application/json" \
    -d '{"jurisdiction_a":"pk","jurisdiction_b":"ae"}')

CORRIDOR_ID=$(echo "$CORRIDOR_RESP" | jq -r '.id')
CORRIDOR_STATE=$(echo "$CORRIDOR_RESP" | jq -r '.state')

assert_ne "Corridor ID assigned" "null" "$CORRIDOR_ID"
assert_eq "Initial state is DRAFT" "DRAFT" "$CORRIDOR_STATE"

echo "  Corridor ID: ${CORRIDOR_ID}"

# ── Step 2: DRAFT → PENDING → ACTIVE ─────────────────────────────────

section "Step 2: Corridor Lifecycle (DRAFT → PENDING → ACTIVE)"

EVIDENCE=$(printf '%0.sa' {1..64})  # 64-char hex evidence digest

PENDING_RESP=$(curl -sf -X PUT "${API}/v1/corridors/${CORRIDOR_ID}/transition" \
    -H "Content-Type: application/json" \
    -d "{\"target_state\":\"PENDING\",\"evidence_digest\":\"${EVIDENCE}\",\"reason\":\"bilateral agreement signed\"}")

PENDING_STATE=$(echo "$PENDING_RESP" | jq -r '.state')
assert_eq "Transition to PENDING" "PENDING" "$PENDING_STATE"

ACTIVE_RESP=$(curl -sf -X PUT "${API}/v1/corridors/${CORRIDOR_ID}/transition" \
    -H "Content-Type: application/json" \
    -d "{\"target_state\":\"ACTIVE\",\"evidence_digest\":\"${EVIDENCE}\",\"reason\":\"regulatory approval\"}")

ACTIVE_STATE=$(echo "$ACTIVE_RESP" | jq -r '.state')
TRANSITION_COUNT=$(echo "$ACTIVE_RESP" | jq '.transition_log | length')
assert_eq "Transition to ACTIVE" "ACTIVE" "$ACTIVE_STATE"
assert_eq "Transition log has 2 entries" "2" "$TRANSITION_COUNT"

# ── Step 3: Exchange 5 cross-border receipts ──────────────────────────

section "Step 3: Exchange 5 Cross-Border Receipts"

PREV_MMR=""
for i in $(seq 0 4); do
    AMOUNT=$(( (i + 1) * 10000 ))
    RECEIPT_RESP=$(curl -sf -X POST "${API}/v1/corridors/state/propose" \
        -H "Content-Type: application/json" \
        -d "{
            \"corridor_id\": \"${CORRIDOR_ID}\",
            \"payload\": {
                \"type\": \"cross_border_transfer\",
                \"from_zone\": \"pk-sifc\",
                \"to_zone\": \"ae-difc\",
                \"amount\": \"${AMOUNT}.00\",
                \"currency\": \"USD\",
                \"reference\": \"XB-PK-AE-$(printf '%04d' $i)\"
            }
        }")

    SEQ=$(echo "$RECEIPT_RESP" | jq -r '.sequence')
    CHAIN_HEIGHT=$(echo "$RECEIPT_RESP" | jq -r '.chain_height')
    NEXT_ROOT=$(echo "$RECEIPT_RESP" | jq -r '.next_root')
    MMR_ROOT=$(echo "$RECEIPT_RESP" | jq -r '.mmr_root')

    assert_eq "Receipt #${i} sequence" "$i" "$SEQ"
    assert_eq "Chain height after receipt #${i}" "$(( i + 1 ))" "$CHAIN_HEIGHT"
    assert_len "Receipt #${i} next_root is 64-char hex" 64 "$NEXT_ROOT"

    if [ -n "$PREV_MMR" ]; then
        assert_ne "MMR root changes with receipt #${i}" "$PREV_MMR" "$MMR_ROOT"
    fi
    PREV_MMR="$MMR_ROOT"
done

# ── Step 4: Query receipt chain ───────────────────────────────────────

section "Step 4: Query Receipt Chain"

CHAIN_RESP=$(curl -sf "${API}/v1/corridors/${CORRIDOR_ID}/receipts")

CHAIN_HEIGHT=$(echo "$CHAIN_RESP" | jq -r '.chain_height')
RECEIPT_COUNT=$(echo "$CHAIN_RESP" | jq '.receipts | length')
GENESIS_ROOT=$(echo "$CHAIN_RESP" | jq -r '.genesis_root')
FINAL_ROOT=$(echo "$CHAIN_RESP" | jq -r '.final_state_root')
CHAIN_MMR=$(echo "$CHAIN_RESP" | jq -r '.mmr_root')

assert_eq "Chain height is 5" "5" "$CHAIN_HEIGHT"
assert_eq "5 receipts returned" "5" "$RECEIPT_COUNT"
assert_len "Genesis root is 64-char hex" 64 "$GENESIS_ROOT"
assert_len "Final state root is 64-char hex" 64 "$FINAL_ROOT"
assert_ne "Final root differs from genesis (chain advanced)" "$GENESIS_ROOT" "$FINAL_ROOT"
assert_len "MMR root is 64-char hex" 64 "$CHAIN_MMR"

# Verify chain linkage: unique next_roots
UNIQUE_ROOTS=$(echo "$CHAIN_RESP" | jq '[.receipts[].next_root] | unique | length')
assert_eq "All 5 next_roots are unique" "5" "$UNIQUE_ROOTS"

# Verify receipt pagination
PAGE_RESP=$(curl -sf "${API}/v1/corridors/${CORRIDOR_ID}/receipts?limit=2&offset=3")
PAGE_COUNT=$(echo "$PAGE_RESP" | jq '.receipts | length')
PAGE_FIRST_SEQ=$(echo "$PAGE_RESP" | jq -r '.receipts[0].sequence')
assert_eq "Pagination returns 2 receipts" "2" "$PAGE_COUNT"
assert_eq "Pagination offset correct (starts at seq 3)" "3" "$PAGE_FIRST_SEQ"

# ── Step 5: Create checkpoint ─────────────────────────────────────────

section "Step 5: Create Checkpoint"

CP_RESP=$(curl -sf -X POST "${API}/v1/corridors/${CORRIDOR_ID}/checkpoint")

CP_TYPE=$(echo "$CP_RESP" | jq -r '.checkpoint_type')
CP_COUNT=$(echo "$CP_RESP" | jq -r '.receipt_count')
CP_TOTAL=$(echo "$CP_RESP" | jq -r '.checkpoint_count')
CP_GENESIS=$(echo "$CP_RESP" | jq -r '.genesis_root')
CP_FINAL=$(echo "$CP_RESP" | jq -r '.final_state_root')
CP_MMR=$(echo "$CP_RESP" | jq -r '.mmr_root')

assert_eq "Checkpoint type" "MEZCorridorStateCheckpoint" "$CP_TYPE"
assert_eq "Checkpoint receipt count is 5" "5" "$CP_COUNT"
assert_eq "Total checkpoints is 1" "1" "$CP_TOTAL"
assert_eq "Checkpoint genesis = chain genesis" "$GENESIS_ROOT" "$CP_GENESIS"
assert_eq "Checkpoint final_state = chain final_state" "$FINAL_ROOT" "$CP_FINAL"
assert_eq "Checkpoint MMR = chain MMR" "$CHAIN_MMR" "$CP_MMR"

# Verify GET checkpoint returns same data
GET_CP_RESP=$(curl -sf "${API}/v1/corridors/${CORRIDOR_ID}/checkpoint")
GET_CP_FINAL=$(echo "$GET_CP_RESP" | jq -r '.final_state_root')
assert_eq "GET checkpoint matches POST result" "$CP_FINAL" "$GET_CP_FINAL"

# ── Step 6: Query compliance tensor ───────────────────────────────────

section "Step 6: Compliance Tensor Evaluation"

PK_RESP=$(curl -sf "${API}/v1/compliance/pk")
PK_JID=$(echo "$PK_RESP" | jq -r '.jurisdiction_id')
PK_DOMAINS=$(echo "$PK_RESP" | jq -r '.total_domains')
PK_PASSING=$(echo "$PK_RESP" | jq -r '.passing_count')

assert_eq "Pakistan jurisdiction ID" "pk" "$PK_JID"
assert_eq "Total domains is 20" "20" "$PK_DOMAINS"
assert_gt "Pakistan has passing domains" "0" "$PK_PASSING"

AE_RESP=$(curl -sf "${API}/v1/compliance/ae")
AE_JID=$(echo "$AE_RESP" | jq -r '.jurisdiction_id')
AE_DOMAINS=$(echo "$AE_RESP" | jq -r '.total_domains')
assert_eq "UAE jurisdiction ID" "ae" "$AE_JID"
assert_eq "UAE total domains is 20" "20" "$AE_DOMAINS"

# ── Step 7: Bilateral corridor compliance ─────────────────────────────

section "Step 7: Bilateral Corridor Compliance"

BILATERAL_RESP=$(curl -sf "${API}/v1/compliance/corridor/${CORRIDOR_ID}")
BIL_COR_ID=$(echo "$BILATERAL_RESP" | jq -r '.corridor_id')
BIL_JA=$(echo "$BILATERAL_RESP" | jq -r '.jurisdiction_a.jurisdiction_id')
BIL_JB=$(echo "$BILATERAL_RESP" | jq -r '.jurisdiction_b.jurisdiction_id')
BIL_CROSS=$(echo "$BILATERAL_RESP" | jq '.cross_blocking_domains | length')

assert_eq "Bilateral corridor ID matches" "$CORRIDOR_ID" "$BIL_COR_ID"
assert_eq "Jurisdiction A is pk" "pk" "$BIL_JA"
assert_eq "Jurisdiction B is ae" "ae" "$BIL_JB"
assert_gt "Cross-blocking domains identified" "0" "$BIL_CROSS"

# ── Step 8: Query all 20 compliance domains ───────────────────────────

section "Step 8: Compliance Domain Catalog"

DOMAINS_RESP=$(curl -sf "${API}/v1/compliance/domains")
DOMAIN_COUNT=$(echo "$DOMAINS_RESP" | jq '. | length')
assert_eq "20 compliance domains defined" "20" "$DOMAIN_COUNT"

# ── Results ───────────────────────────────────────────────────────────

echo ""
echo "============================================================================"
echo -e "${BOLD}Results: ${pass_count} passed, ${fail_count} failed${RESET}"
echo "============================================================================"
echo ""

if [ "$fail_count" -gt 0 ]; then
    echo -e "${RED}DEMO FAILED — ${fail_count} assertions did not pass.${RESET}"
    exit 1
else
    echo -e "${GREEN}DEMO PASSED — Full corridor lifecycle verified.${RESET}"
    echo ""
    echo "Demonstrated:"
    echo "  1. Zone corridor creation (PK ↔ AE)"
    echo "  2. Typestate lifecycle: DRAFT → PENDING → ACTIVE"
    echo "  3. 5 cross-border receipts with dual-commitment (hash-chain + MMR)"
    echo "  4. Receipt chain query with pagination"
    echo "  5. Checkpoint creation (verifiable snapshot)"
    echo "  6. Compliance tensor evaluation (20 domains, jurisdiction-scoped)"
    echo "  7. Bilateral corridor compliance assessment"
    echo "  8. Compliance domain catalog query"
    echo ""
    echo "Corridor: ${CORRIDOR_ID}"
    echo "Chain height: 5 receipts"
    echo "Checkpoints: 1"
    echo "Genesis root: ${GENESIS_ROOT:0:16}..."
    echo "Final state:  ${FINAL_ROOT:0:16}..."
    echo "MMR root:     ${CHAIN_MMR:0:16}..."
    exit 0
fi
