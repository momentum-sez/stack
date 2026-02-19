#!/bin/bash
# MEZ Two-Zone Integration Test
#
# Tests the inter-zone corridor protocol (P0-CORRIDOR-NET-001):
#   1. Health check both zones
#   2. Zone A proposes a corridor to Zone B
#   3. Zone B accepts the proposal
#   4. Zone A sends a receipt to Zone B
#   5. Zone B sends a watcher attestation to Zone A
#   6. Verify replay protection (duplicate receipt rejected)
#
# Prerequisites:
#   docker compose -f deploy/docker/docker-compose.two-zone.yaml up -d
#
# Usage:
#   ./deploy/scripts/test-two-zone.sh

set -euo pipefail

ZONE_A="http://localhost:8080"
ZONE_B="http://localhost:8081"
ZONE_A_TOKEN="${ZONE_A_AUTH_TOKEN:-zone-a-test-token}"
ZONE_B_TOKEN="${ZONE_B_AUTH_TOKEN:-zone-b-test-token}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PASS=0
FAIL=0

check() {
    local name="$1"
    local expected_status="$2"
    local actual_status="$3"

    if [ "$actual_status" = "$expected_status" ]; then
        echo -e "  ${GREEN}PASS${NC} $name (HTTP $actual_status)"
        PASS=$((PASS + 1))
    else
        echo -e "  ${RED}FAIL${NC} $name (expected $expected_status, got $actual_status)"
        FAIL=$((FAIL + 1))
    fi
}

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}  MEZ Two-Zone Integration Test${NC}"
echo -e "${BLUE}  P0-CORRIDOR-NET-001${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

# ---------------------------------------------------------------------------
# Step 1: Health checks
# ---------------------------------------------------------------------------
echo -e "${YELLOW}Step 1: Health checks${NC}"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$ZONE_A/health/liveness" 2>/dev/null || echo "000")
check "Zone A liveness" "200" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$ZONE_B/health/liveness" 2>/dev/null || echo "000")
check "Zone B liveness" "200" "$STATUS"

echo ""

# ---------------------------------------------------------------------------
# Step 2: Zone A proposes a corridor to Zone B
# ---------------------------------------------------------------------------
echo -e "${YELLOW}Step 2: Zone A proposes corridor to Zone B${NC}"

PROPOSAL='{
    "corridor_id": "org.momentum.mez.corridor.pk-ae.cross-border",
    "proposer_jurisdiction_id": "pk",
    "proposer_zone_id": "org.momentum.mez.zone.pk-sifc",
    "proposer_verifying_key_hex": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "proposer_did": "did:mass:zone:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "responder_jurisdiction_id": "ae-difc",
    "proposed_at": "2026-01-15T00:00:00Z",
    "parameters": {},
    "signature": "proposal-sig-placeholder"
}'

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$ZONE_B/v1/corridors/peers/propose" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ZONE_B_TOKEN" \
    -d "$PROPOSAL" 2>/dev/null || echo "000")
check "Zone B receives proposal" "200" "$STATUS"

echo ""

# ---------------------------------------------------------------------------
# Step 3: Zone B accepts the corridor
# ---------------------------------------------------------------------------
echo -e "${YELLOW}Step 3: Zone B accepts corridor${NC}"

ACCEPTANCE='{
    "corridor_id": "org.momentum.mez.corridor.pk-ae.cross-border",
    "responder_zone_id": "org.momentum.mez.zone.ae-difc",
    "responder_verifying_key_hex": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "responder_did": "did:mass:zone:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "genesis_root_hex": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    "accepted_at": "2026-01-15T00:01:00Z",
    "signature": "acceptance-sig-placeholder"
}'

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$ZONE_A/v1/corridors/peers/accept" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ZONE_A_TOKEN" \
    -d "$ACCEPTANCE" 2>/dev/null || echo "000")
check "Zone A receives acceptance" "200" "$STATUS"

echo ""

# ---------------------------------------------------------------------------
# Step 4: Verify peers are registered
# ---------------------------------------------------------------------------
echo -e "${YELLOW}Step 4: Verify peer registration${NC}"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X GET "$ZONE_A/v1/corridors/peers" \
    -H "Authorization: Bearer $ZONE_A_TOKEN" 2>/dev/null || echo "000")
check "Zone A lists peers" "200" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X GET "$ZONE_B/v1/corridors/peers/org.momentum.mez.zone.pk-sifc" \
    -H "Authorization: Bearer $ZONE_B_TOKEN" 2>/dev/null || echo "000")
check "Zone B finds Zone A peer" "200" "$STATUS"

echo ""

# ---------------------------------------------------------------------------
# Step 5: Zone A sends a receipt to Zone B
# ---------------------------------------------------------------------------
echo -e "${YELLOW}Step 5: Receipt exchange${NC}"

# First register Zone A as active peer on Zone B (via acceptance on Zone B)
ACCEPTANCE_ON_B='{
    "corridor_id": "org.momentum.mez.corridor.pk-ae.cross-border",
    "responder_zone_id": "org.momentum.mez.zone.pk-sifc",
    "responder_verifying_key_hex": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "responder_did": "did:mass:zone:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "genesis_root_hex": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    "accepted_at": "2026-01-15T00:01:00Z",
    "signature": "acceptance-sig-placeholder"
}'

curl -s -o /dev/null \
    -X POST "$ZONE_B/v1/corridors/peers/accept" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ZONE_B_TOKEN" \
    -d "$ACCEPTANCE_ON_B" 2>/dev/null || true

RECEIPT='{
    "corridor_id": "org.momentum.mez.corridor.pk-ae.cross-border",
    "origin_zone_id": "org.momentum.mez.zone.pk-sifc",
    "sequence": 0,
    "receipt_json": {"corridor_id": "pk-ae", "height": 0, "test": true},
    "receipt_digest": "1111111111111111111111111111111111111111111111111111111111111111",
    "signature": "receipt-sig-placeholder",
    "produced_at": "2026-01-15T00:02:00Z"
}'

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$ZONE_B/v1/corridors/peers/receipts" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ZONE_B_TOKEN" \
    -d "$RECEIPT" 2>/dev/null || echo "000")
check "Zone B accepts receipt from Zone A" "200" "$STATUS"

echo ""

# ---------------------------------------------------------------------------
# Step 6: Replay protection — same receipt rejected
# ---------------------------------------------------------------------------
echo -e "${YELLOW}Step 6: Replay protection${NC}"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$ZONE_B/v1/corridors/peers/receipts" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ZONE_B_TOKEN" \
    -d "$RECEIPT" 2>/dev/null || echo "000")
check "Zone B rejects replay" "409" "$STATUS"

echo ""

# ---------------------------------------------------------------------------
# Step 7: Watcher attestation delivery
# ---------------------------------------------------------------------------
echo -e "${YELLOW}Step 7: Watcher attestation${NC}"

ATTESTATION='{
    "corridor_id": "org.momentum.mez.corridor.pk-ae.cross-border",
    "watcher_id": "watcher-pk-001",
    "attested_height": 1,
    "attested_root_hex": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
    "signature": "attestation-sig-placeholder",
    "attested_at": "2026-01-15T00:03:00Z"
}'

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$ZONE_A/v1/corridors/peers/attestations" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ZONE_A_TOKEN" \
    -d "$ATTESTATION" 2>/dev/null || echo "000")
check "Zone A receives attestation" "200" "$STATUS"

echo ""

# ---------------------------------------------------------------------------
# Results
# ---------------------------------------------------------------------------
echo -e "${BLUE}======================================${NC}"
TOTAL=$((PASS + FAIL))
echo -e "  Results: ${GREEN}$PASS passed${NC}, ${RED}$FAIL failed${NC} out of $TOTAL tests"
echo -e "${BLUE}======================================${NC}"

if [ "$FAIL" -gt 0 ]; then
    echo -e "${RED}SOME TESTS FAILED${NC}"
    exit 1
else
    echo -e "${GREEN}ALL TESTS PASSED${NC}"
    exit 0
fi
