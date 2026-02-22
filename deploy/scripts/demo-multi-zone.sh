#!/bin/bash
# ============================================================================
# MEZ Stack — End-to-End Multi-Zone (N=3) Corridor Demo
# ============================================================================
#
# Phase 2 demonstration: Deploy 3 zones → establish pairwise corridors →
# exchange receipts across all corridor pairs → cross-zone compliance →
# watcher attestation delivery.
#
# Architecture:
#   Zone A (pk-sifc)  ←—— corridor AB ——→  Zone B (ae-abudhabi-adgm)
#        ↕                                         ↕
#    corridor AC                              corridor BC
#        ↕                                         ↕
#               Zone C (sg) ←—————————————→
#
# Three pairwise corridors:
#   AB: pk-sifc <-> ae-abudhabi-adgm
#   AC: pk-sifc <-> sg
#   BC: ae-abudhabi-adgm <-> sg
#
# Usage:
#   ./deploy/scripts/demo-multi-zone.sh              # Full demo
#   ./deploy/scripts/demo-multi-zone.sh --no-teardown # Keep running
#   ./deploy/scripts/demo-multi-zone.sh --test-only   # Skip deploy

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
COMPOSE_DIR="$PROJECT_ROOT/deploy/docker"
COMPOSE_FILE="$COMPOSE_DIR/docker-compose.three-zone.yaml"

ZONE_A="http://localhost:8080"
ZONE_B="http://localhost:8081"
ZONE_C="http://localhost:8082"

ZONE_A_JURISDICTION="${ZONE_A_JURISDICTION:-pk}"
ZONE_A_ZONE_ID="${ZONE_A_ZONE_ID:-org.momentum.mez.zone.pk-sifc}"
ZONE_A_PROFILE="${ZONE_A_PROFILE:-sovereign-govos-pk}"

ZONE_B_JURISDICTION="${ZONE_B_JURISDICTION:-ae-abudhabi-adgm}"
ZONE_B_ZONE_ID="${ZONE_B_ZONE_ID:-org.momentum.mez.zone.ae.abudhabi.adgm}"
ZONE_B_PROFILE="${ZONE_B_PROFILE:-sovereign-govos-ae}"

ZONE_C_JURISDICTION="${ZONE_C_JURISDICTION:-sg}"
ZONE_C_ZONE_ID="${ZONE_C_ZONE_ID:-org.momentum.mez.zone.sg}"
ZONE_C_PROFILE="${ZONE_C_PROFILE:-sovereign-govos-sg}"

# Generate corridor IDs from sorted jurisdiction pairs
corridor_id() {
    local a="$1" b="$2"
    local sorted_a sorted_b
    sorted_a=$(echo -e "$a\n$b" | sort | head -1)
    sorted_b=$(echo -e "$a\n$b" | sort | tail -1)
    echo "org.momentum.mez.corridor.${sorted_a}--${sorted_b}.cross-border"
}

CORRIDOR_AB=$(corridor_id "$ZONE_A_JURISDICTION" "$ZONE_B_JURISDICTION")
CORRIDOR_AC=$(corridor_id "$ZONE_A_JURISDICTION" "$ZONE_C_JURISDICTION")
CORRIDOR_BC=$(corridor_id "$ZONE_B_JURISDICTION" "$ZONE_C_JURISDICTION")

export ZONE_A_ZONE_ID ZONE_A_JURISDICTION ZONE_A_PROFILE
export ZONE_B_ZONE_ID ZONE_B_JURISDICTION ZONE_B_PROFILE
export ZONE_C_ZONE_ID ZONE_C_JURISDICTION ZONE_C_PROFILE

# Parse arguments
NO_TEARDOWN=false
TEST_ONLY=false
for arg in "$@"; do
    case $arg in
        --no-teardown) NO_TEARDOWN=true ;;
        --test-only) TEST_ONLY=true ;;
    esac
done

# ── Colors ─────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

PASS=0
FAIL=0
STEP=0

step() {
    STEP=$((STEP + 1))
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}Step $STEP: $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

check() {
    local name="$1"
    local expected_status="$2"
    local actual_status="$3"
    local body="${4:-}"

    if [ "$actual_status" = "$expected_status" ]; then
        echo -e "  ${GREEN}✓ PASS${NC} $name (HTTP $actual_status)"
        PASS=$((PASS + 1))
    else
        echo -e "  ${RED}✗ FAIL${NC} $name (expected $expected_status, got $actual_status)"
        if [ -n "$body" ]; then
            echo -e "    ${RED}Body: $body${NC}"
        fi
        FAIL=$((FAIL + 1))
    fi
}

cleanup() {
    if [ "$NO_TEARDOWN" = true ]; then
        echo -e "\n${YELLOW}--no-teardown: Stack remains running.${NC}"
        echo -e "  Zone A: $ZONE_A ($ZONE_A_JURISDICTION)"
        echo -e "  Zone B: $ZONE_B ($ZONE_B_JURISDICTION)"
        echo -e "  Zone C: $ZONE_C ($ZONE_C_JURISDICTION)"
        echo -e "  Tear down manually: docker compose -f $COMPOSE_FILE down -v"
        return
    fi
    echo -e "\n${YELLOW}Tearing down...${NC}"
    cd "$COMPOSE_DIR"
    docker compose -f "$COMPOSE_FILE" down -v 2>/dev/null || true
}

# Helper: establish a corridor between two zones
establish_corridor() {
    local zone_x_url="$1" zone_x_token="$2" zone_x_jid="$3" zone_x_zid="$4"
    local zone_y_url="$5" zone_y_token="$6" zone_y_jid="$7" zone_y_zid="$8"
    local corridor_id="$9"
    local label_x="${10}" label_y="${11}"

    local genesis_root
    genesis_root=$(openssl rand -hex 32)

    # Create corridor on both zones
    local create='{"jurisdiction_a":"'"$zone_x_jid"'","jurisdiction_b":"'"$zone_y_jid"'"}'

    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$zone_x_url/v1/corridors" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $zone_x_token" \
        -d "$create" 2>/dev/null || echo "000")
    check "$label_x creates corridor ($label_x↔$label_y)" "201" "$STATUS"

    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$zone_y_url/v1/corridors" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $zone_y_token" \
        -d "$create" 2>/dev/null || echo "000")
    check "$label_y creates corridor ($label_x↔$label_y)" "201" "$STATUS"

    # Propose
    local proposal='{"corridor_id":"'"$corridor_id"'","proposer_jurisdiction_id":"'"$zone_x_jid"'","proposer_zone_id":"'"$zone_x_zid"'","proposer_verifying_key_hex":"'"$(openssl rand -hex 32)"'","proposer_did":"did:mass:zone:'"$zone_x_jid"'-demo","responder_jurisdiction_id":"'"$zone_y_jid"'","proposed_at":"'"$(date -u +%Y-%m-%dT%H:%M:%SZ)"'","parameters":{},"signature":"'"$(openssl rand -hex 64)"'"}'

    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$zone_y_url/v1/corridors/peers/propose" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $zone_y_token" \
        -d "$proposal" 2>/dev/null || echo "000")
    check "$label_y receives proposal from $label_x" "200" "$STATUS"

    # Accept (bidirectional)
    local accept_xy='{"corridor_id":"'"$corridor_id"'","responder_zone_id":"'"$zone_y_zid"'","responder_verifying_key_hex":"'"$(openssl rand -hex 32)"'","responder_did":"did:mass:zone:'"$zone_y_jid"'-demo","genesis_root_hex":"'"$genesis_root"'","accepted_at":"'"$(date -u +%Y-%m-%dT%H:%M:%SZ)"'","signature":"'"$(openssl rand -hex 64)"'"}'

    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$zone_x_url/v1/corridors/peers/accept" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $zone_x_token" \
        -d "$accept_xy" 2>/dev/null || echo "000")
    check "$label_x receives acceptance" "200" "$STATUS"

    local accept_yx='{"corridor_id":"'"$corridor_id"'","responder_zone_id":"'"$zone_x_zid"'","responder_verifying_key_hex":"'"$(openssl rand -hex 32)"'","responder_did":"did:mass:zone:'"$zone_x_jid"'-demo","genesis_root_hex":"'"$genesis_root"'","accepted_at":"'"$(date -u +%Y-%m-%dT%H:%M:%SZ)"'","signature":"'"$(openssl rand -hex 64)"'"}'

    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$zone_y_url/v1/corridors/peers/accept" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $zone_y_token" \
        -d "$accept_yx" 2>/dev/null || echo "000")
    check "$label_y registers $label_x as peer" "200" "$STATUS"

    # Return genesis root for receipt exchange
    echo "$genesis_root"
}

# Helper: send a receipt from zone X to zone Y
send_receipt() {
    local zone_x_zid="$1" zone_y_url="$2" zone_y_token="$3"
    local corridor_id="$4" genesis_root="$5"
    local label_x="$6" label_y="$7"

    local receipt='{"corridor_id":"'"$corridor_id"'","origin_zone_id":"'"$zone_x_zid"'","sequence":0,"receipt_json":{"type":"CorridorReceipt","corridor_id":"'"$corridor_id"'","sequence":0,"timestamp":"'"$(date -u +%Y-%m-%dT%H:%M:%SZ)"'","prev_root":"'"$genesis_root"'","next_root":"'"$(openssl rand -hex 32)"'","lawpack_digest_set":["'"$(openssl rand -hex 32)"'"],"ruleset_digest_set":["'"$(openssl rand -hex 32)"'"]},"receipt_digest":"'"$(openssl rand -hex 32)"'","signature":"'"$(openssl rand -hex 64)"'","produced_at":"'"$(date -u +%Y-%m-%dT%H:%M:%SZ)"'"}'

    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$zone_y_url/v1/corridors/peers/receipts" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $zone_y_token" \
        -d "$receipt" 2>/dev/null || echo "000")
    check "$label_y accepts receipt from $label_x" "200" "$STATUS"
}

# ── Banner ─────────────────────────────────────────────────────────────────

echo -e "${BLUE}"
echo "  ╔══════════════════════════════════════════════════════════╗"
echo "  ║  MEZ Stack — Multi-Zone (N=3) Corridor Demo              ║"
echo "  ║  Phase 2: AWS of Economic Zones                          ║"
echo "  ║  v0.4.44 GENESIS                                        ║"
echo "  ╚══════════════════════════════════════════════════════════╝"
echo -e "${NC}"
echo -e "  Zone A: ${BOLD}$ZONE_A_JURISDICTION${NC} ($ZONE_A_ZONE_ID) [$ZONE_A_PROFILE]"
echo -e "  Zone B: ${BOLD}$ZONE_B_JURISDICTION${NC} ($ZONE_B_ZONE_ID) [$ZONE_B_PROFILE]"
echo -e "  Zone C: ${BOLD}$ZONE_C_JURISDICTION${NC} ($ZONE_C_ZONE_ID) [$ZONE_C_PROFILE]"
echo -e ""
echo -e "  Corridor AB: $CORRIDOR_AB"
echo -e "  Corridor AC: $CORRIDOR_AC"
echo -e "  Corridor BC: $CORRIDOR_BC"
echo ""

# ── Step 1: Generate Credentials ──────────────────────────────────────────

if [ "$TEST_ONLY" = false ]; then

step "Generate credentials"

export POSTGRES_PASSWORD="$(openssl rand -base64 24 | tr -d '/+=' | head -c 32)"
export ZONE_A_AUTH_TOKEN="$(openssl rand -base64 32 | tr -d '/+=' | head -c 48)"
export ZONE_B_AUTH_TOKEN="$(openssl rand -base64 32 | tr -d '/+=' | head -c 48)"
export ZONE_C_AUTH_TOKEN="$(openssl rand -base64 32 | tr -d '/+=' | head -c 48)"

echo -e "  ${GREEN}✓${NC} POSTGRES_PASSWORD generated (${#POSTGRES_PASSWORD} chars)"
echo -e "  ${GREEN}✓${NC} ZONE_A_AUTH_TOKEN generated (${#ZONE_A_AUTH_TOKEN} chars)"
echo -e "  ${GREEN}✓${NC} ZONE_B_AUTH_TOKEN generated (${#ZONE_B_AUTH_TOKEN} chars)"
echo -e "  ${GREEN}✓${NC} ZONE_C_AUTH_TOKEN generated (${#ZONE_C_AUTH_TOKEN} chars)"

echo "$ZONE_A_AUTH_TOKEN" > /tmp/mez-demo-zone-a-token
chmod 600 /tmp/mez-demo-zone-a-token
echo "$ZONE_B_AUTH_TOKEN" > /tmp/mez-demo-zone-b-token
chmod 600 /tmp/mez-demo-zone-b-token
echo "$ZONE_C_AUTH_TOKEN" > /tmp/mez-demo-zone-c-token
chmod 600 /tmp/mez-demo-zone-c-token

# ── Step 2: Deploy Three-Zone Stack ────────────────────────────────────────

step "Deploy three-zone Docker Compose stack"

cd "$COMPOSE_DIR"
echo -e "  Building and starting services..."
docker compose -f "$COMPOSE_FILE" up -d --build 2>&1 | while read -r line; do
    echo -e "    $line"
done

trap cleanup EXIT

# ── Step 3: Wait for Health Checks ─────────────────────────────────────────

step "Wait for all three zones to be healthy"

MAX_WAIT=180
WAITED=0
while [ $WAITED -lt $MAX_WAIT ]; do
    A_OK=$(curl -s -o /dev/null -w "%{http_code}" "$ZONE_A/health/liveness" 2>/dev/null || echo "000")
    B_OK=$(curl -s -o /dev/null -w "%{http_code}" "$ZONE_B/health/liveness" 2>/dev/null || echo "000")
    C_OK=$(curl -s -o /dev/null -w "%{http_code}" "$ZONE_C/health/liveness" 2>/dev/null || echo "000")
    if [ "$A_OK" = "200" ] && [ "$B_OK" = "200" ] && [ "$C_OK" = "200" ]; then
        break
    fi
    echo -n "."
    sleep 2
    WAITED=$((WAITED + 2))
done
echo ""

if [ "$WAITED" -ge "$MAX_WAIT" ]; then
    echo -e "  ${RED}Timeout waiting for zones to become healthy${NC}"
    docker compose -f "$COMPOSE_FILE" logs --tail=30
    exit 1
fi

else
    # --test-only: load saved tokens
    if [ -f /tmp/mez-demo-zone-a-token ] && [ -f /tmp/mez-demo-zone-b-token ] && [ -f /tmp/mez-demo-zone-c-token ]; then
        ZONE_A_AUTH_TOKEN=$(cat /tmp/mez-demo-zone-a-token)
        ZONE_B_AUTH_TOKEN=$(cat /tmp/mez-demo-zone-b-token)
        ZONE_C_AUTH_TOKEN=$(cat /tmp/mez-demo-zone-c-token)
    else
        ZONE_A_AUTH_TOKEN="${ZONE_A_AUTH_TOKEN:?ZONE_A_AUTH_TOKEN must be set}"
        ZONE_B_AUTH_TOKEN="${ZONE_B_AUTH_TOKEN:?ZONE_B_AUTH_TOKEN must be set}"
        ZONE_C_AUTH_TOKEN="${ZONE_C_AUTH_TOKEN:?ZONE_C_AUTH_TOKEN must be set}"
    fi
    export ZONE_A_AUTH_TOKEN ZONE_B_AUTH_TOKEN ZONE_C_AUTH_TOKEN
fi

# ── Step 4: Health Verification ────────────────────────────────────────────

step "Verify zone health endpoints"

for zone_label zone_url in A "$ZONE_A" B "$ZONE_B" C "$ZONE_C"; do
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$zone_url/health/liveness" 2>/dev/null || echo "000")
    check "Zone $zone_label liveness" "200" "$STATUS"
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$zone_url/health/readiness" 2>/dev/null || echo "000")
    check "Zone $zone_label readiness" "200" "$STATUS"
done

# ── Step 5: Establish 3 Pairwise Corridors ─────────────────────────────────

step "Establish corridor AB ($ZONE_A_JURISDICTION ↔ $ZONE_B_JURISDICTION)"
GENESIS_AB=$(establish_corridor \
    "$ZONE_A" "$ZONE_A_AUTH_TOKEN" "$ZONE_A_JURISDICTION" "$ZONE_A_ZONE_ID" \
    "$ZONE_B" "$ZONE_B_AUTH_TOKEN" "$ZONE_B_JURISDICTION" "$ZONE_B_ZONE_ID" \
    "$CORRIDOR_AB" "Zone-A" "Zone-B")

step "Establish corridor AC ($ZONE_A_JURISDICTION ↔ $ZONE_C_JURISDICTION)"
GENESIS_AC=$(establish_corridor \
    "$ZONE_A" "$ZONE_A_AUTH_TOKEN" "$ZONE_A_JURISDICTION" "$ZONE_A_ZONE_ID" \
    "$ZONE_C" "$ZONE_C_AUTH_TOKEN" "$ZONE_C_JURISDICTION" "$ZONE_C_ZONE_ID" \
    "$CORRIDOR_AC" "Zone-A" "Zone-C")

step "Establish corridor BC ($ZONE_B_JURISDICTION ↔ $ZONE_C_JURISDICTION)"
GENESIS_BC=$(establish_corridor \
    "$ZONE_B" "$ZONE_B_AUTH_TOKEN" "$ZONE_B_JURISDICTION" "$ZONE_B_ZONE_ID" \
    "$ZONE_C" "$ZONE_C_AUTH_TOKEN" "$ZONE_C_JURISDICTION" "$ZONE_C_ZONE_ID" \
    "$CORRIDOR_BC" "Zone-B" "Zone-C")

# ── Step 6: Receipt Exchange Across All Corridors ──────────────────────────

step "Receipt exchange across all 3 corridors"

send_receipt "$ZONE_A_ZONE_ID" "$ZONE_B" "$ZONE_B_AUTH_TOKEN" "$CORRIDOR_AB" "$GENESIS_AB" "Zone-A" "Zone-B"
send_receipt "$ZONE_A_ZONE_ID" "$ZONE_C" "$ZONE_C_AUTH_TOKEN" "$CORRIDOR_AC" "$GENESIS_AC" "Zone-A" "Zone-C"
send_receipt "$ZONE_B_ZONE_ID" "$ZONE_C" "$ZONE_C_AUTH_TOKEN" "$CORRIDOR_BC" "$GENESIS_BC" "Zone-B" "Zone-C"

# ── Step 7: Cross-Zone Compliance Query ────────────────────────────────────

step "Cross-zone compliance query"

for zone_label zone_url zone_token in \
    "A" "$ZONE_A" "$ZONE_A_AUTH_TOKEN" \
    "B" "$ZONE_B" "$ZONE_B_AUTH_TOKEN" \
    "C" "$ZONE_C" "$ZONE_C_AUTH_TOKEN"; do
    BODY=$(curl -s -w "\n%{http_code}" \
        "$zone_url/v1/regulator/summary" \
        -H "Authorization: Bearer $zone_token" 2>/dev/null || echo -e "\n000")
    STATUS=$(echo "$BODY" | tail -1)
    check "Zone $zone_label compliance summary" "200" "$STATUS"
done

# ── Step 8: Watcher Attestation Delivery ───────────────────────────────────

step "Watcher attestation delivery to all zones"

for zone_label zone_url zone_token in \
    "A" "$ZONE_A" "$ZONE_A_AUTH_TOKEN" \
    "B" "$ZONE_B" "$ZONE_B_AUTH_TOKEN" \
    "C" "$ZONE_C" "$ZONE_C_AUTH_TOKEN"; do

    ATTESTATION='{"corridor_id":"'"$CORRIDOR_AB"'","watcher_id":"watcher-'"${zone_label,,}"'-001","attested_height":1,"attested_root_hex":"'"$(openssl rand -hex 32)"'","signature":"'"$(openssl rand -hex 64)"'","attested_at":"'"$(date -u +%Y-%m-%dT%H:%M:%SZ)"'"}'

    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$zone_url/v1/corridors/peers/attestations" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $zone_token" \
        -d "$ATTESTATION" 2>/dev/null || echo "000")
    check "Zone $zone_label receives watcher attestation" "200" "$STATUS"
done

# ── Step 9: List Corridors on All Zones ────────────────────────────────────

step "List corridors on all zones"

for zone_label zone_url zone_token in \
    "A" "$ZONE_A" "$ZONE_A_AUTH_TOKEN" \
    "B" "$ZONE_B" "$ZONE_B_AUTH_TOKEN" \
    "C" "$ZONE_C" "$ZONE_C_AUTH_TOKEN"; do
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        "$zone_url/v1/corridors" \
        -H "Authorization: Bearer $zone_token" 2>/dev/null || echo "000")
    check "Zone $zone_label lists corridors" "200" "$STATUS"
done

# ── Results ────────────────────────────────────────────────────────────────

echo ""
echo -e "${BLUE}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║               MULTI-ZONE DEMO RESULTS                    ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""
TOTAL=$((PASS + FAIL))
echo -e "  Tests:  ${GREEN}$PASS passed${NC}, ${RED}$FAIL failed${NC} out of $TOTAL"
echo ""
echo -e "  ${BOLD}Phase 2 AWS of Economic Zones Demonstrated:${NC}"
echo -e "    ${GREEN}✓${NC} Three sovereign zones deployed ($ZONE_A_JURISDICTION + $ZONE_B_JURISDICTION + $ZONE_C_JURISDICTION)"
echo -e "    ${GREEN}✓${NC} Three pairwise corridors established"
echo -e "    ${GREEN}✓${NC} Receipt exchange across all corridors"
echo -e "    ${GREEN}✓${NC} Cross-zone compliance query"
echo -e "    ${GREEN}✓${NC} Watcher attestations delivered to all zones"
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo -e "${RED}SOME TESTS FAILED — Multi-zone demo NOT fully passing${NC}"
    exit 1
else
    echo -e "${GREEN}ALL TESTS PASSED — Multi-zone corridor topology VERIFIED${NC}"
    echo ""
    echo -e "  ${BOLD}The MEZ Stack supports N-zone corridor mesh topology.${NC}"
    exit 0
fi
