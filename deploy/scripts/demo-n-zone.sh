#!/bin/bash
# ============================================================================
# MEZ Stack — N-Zone Corridor Mesh Demo
# ============================================================================
#
# Generalized N-zone deployment: deploys any number of zones, establishes
# N*(N-1)/2 pairwise corridors, and verifies receipt exchange across the
# full corridor mesh.
#
# Usage:
#   ./deploy/scripts/demo-n-zone.sh pk-sifc ae-abudhabi-adgm sg hk ky
#   ./deploy/scripts/demo-n-zone.sh pk-sifc ae-abudhabi-adgm synth-atlantic-fintech
#   ./deploy/scripts/demo-n-zone.sh --no-teardown pk-sifc sg hk
#   ./deploy/scripts/demo-n-zone.sh --test-only pk-sifc ae-abudhabi-adgm sg
#
# Requires: docker compose, curl, openssl, jq (optional for JSON output)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
COMPOSE_DIR="$PROJECT_ROOT/deploy/docker"

# ── Parse Arguments ───────────────────────────────────────────────────────

NO_TEARDOWN=false
TEST_ONLY=false
ZONES=()

for arg in "$@"; do
    case $arg in
        --no-teardown) NO_TEARDOWN=true ;;
        --test-only) TEST_ONLY=true ;;
        --*) echo "Unknown option: $arg"; exit 1 ;;
        *) ZONES+=("$arg") ;;
    esac
done

if [ ${#ZONES[@]} -lt 2 ]; then
    echo "Usage: $0 [--no-teardown] [--test-only] <zone1> <zone2> [zone3 ...]"
    echo ""
    echo "Example: $0 pk-sifc ae-abudhabi-adgm sg hk ky"
    exit 1
fi

N=${#ZONES[@]}
CORRIDORS=$(( N * (N - 1) / 2 ))

# ── Colors ────────────────────────────────────────────────────────────────

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

# Generate corridor ID from sorted jurisdiction pair
corridor_id() {
    local a="$1" b="$2"
    local sorted_a sorted_b
    sorted_a=$(echo -e "$a\n$b" | sort | head -1)
    sorted_b=$(echo -e "$a\n$b" | sort | tail -1)
    echo "org.momentum.mez.corridor.${sorted_a}--${sorted_b}"
}

# ── Banner ────────────────────────────────────────────────────────────────

echo -e "${BLUE}"
echo "  ╔══════════════════════════════════════════════════════════╗"
echo "  ║  MEZ Stack — N-Zone Corridor Mesh Demo                    ║"
echo "  ║  AWS of Economic Zones — Compositional Zone Algebra       ║"
echo "  ║  v0.4.44 GENESIS                                        ║"
echo "  ╚══════════════════════════════════════════════════════════╝"
echo -e "${NC}"
echo -e "  Zones: ${BOLD}$N${NC}"
echo -e "  Corridors: ${BOLD}$CORRIDORS${NC} (N*(N-1)/2)"
echo ""

for i in $(seq 0 $((N - 1))); do
    echo -e "  Zone $((i + 1)): ${BOLD}${ZONES[$i]}${NC} (API port $((8080 + i)))"
done
echo ""

# ── Step 1: Generate Compose File ────────────────────────────────────────

COMPOSE_FILE="$COMPOSE_DIR/docker-compose.n-zone-generated.yaml"
ZONE_LIST=$(IFS=,; echo "${ZONES[*]}")

if [ "$TEST_ONLY" = false ]; then

step "Generate Docker Compose for $N zones"

# Use mez deploy generate if available, otherwise generate inline
if command -v cargo &> /dev/null; then
    cargo run --package mez-cli -- deploy generate \
        --zones "$ZONE_LIST" \
        --output "$COMPOSE_FILE" 2>&1 | while read -r line; do
        echo -e "  $line"
    done
else
    echo -e "  ${YELLOW}cargo not available; using pre-generated compose file${NC}"
fi

# ── Step 2: Generate Credentials ─────────────────────────────────────────

step "Generate credentials for $N zones"

export POSTGRES_PASSWORD="$(openssl rand -base64 24 | tr -d '/+=' | head -c 32)"
echo -e "  ${GREEN}✓${NC} POSTGRES_PASSWORD generated (${#POSTGRES_PASSWORD} chars)"

declare -A ZONE_TOKENS
for i in $(seq 0 $((N - 1))); do
    jid="${ZONES[$i]}"
    env_name="ZONE_${jid^^}_AUTH_TOKEN"
    env_name="${env_name//-/_}"
    token="$(openssl rand -base64 32 | tr -d '/+=' | head -c 48)"
    export "$env_name=$token"
    ZONE_TOKENS[$i]="$token"
    echo -e "  ${GREEN}✓${NC} $env_name generated (${#token} chars)"
    echo "$token" > "/tmp/mez-demo-zone-${i}-token"
    chmod 600 "/tmp/mez-demo-zone-${i}-token"
done

# ── Step 3: Deploy N-Zone Stack ──────────────────────────────────────────

step "Deploy $N-zone Docker Compose stack"

cd "$COMPOSE_DIR"
echo -e "  Building and starting services..."
docker compose -f "$COMPOSE_FILE" up -d --build 2>&1 | while read -r line; do
    echo -e "    $line"
done

cleanup() {
    if [ "$NO_TEARDOWN" = true ]; then
        echo -e "\n${YELLOW}--no-teardown: Stack remains running.${NC}"
        for i in $(seq 0 $((N - 1))); do
            echo -e "  Zone $((i + 1)): ${ZONES[$i]} -> http://localhost:$((8080 + i))"
        done
        echo -e "  Tear down manually: docker compose -f $COMPOSE_FILE down -v"
        return
    fi
    echo -e "\n${YELLOW}Tearing down...${NC}"
    cd "$COMPOSE_DIR"
    docker compose -f "$COMPOSE_FILE" down -v 2>/dev/null || true
}
trap cleanup EXIT

# ── Step 4: Wait for Health ──────────────────────────────────────────────

step "Wait for all $N zones to be healthy"

MAX_WAIT=180
WAITED=0
while [ $WAITED -lt $MAX_WAIT ]; do
    ALL_OK=true
    for i in $(seq 0 $((N - 1))); do
        port=$((8080 + i))
        status=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:$port/health/liveness" 2>/dev/null || echo "000")
        if [ "$status" != "200" ]; then
            ALL_OK=false
            break
        fi
    done
    if [ "$ALL_OK" = true ]; then
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
    declare -A ZONE_TOKENS
    for i in $(seq 0 $((N - 1))); do
        if [ -f "/tmp/mez-demo-zone-${i}-token" ]; then
            ZONE_TOKENS[$i]=$(cat "/tmp/mez-demo-zone-${i}-token")
        else
            echo -e "${RED}Token file not found for zone $i. Run without --test-only first.${NC}"
            exit 1
        fi
    done
fi

# ── Step 5: Health Verification ──────────────────────────────────────────

step "Verify zone health endpoints"

for i in $(seq 0 $((N - 1))); do
    port=$((8080 + i))
    jid="${ZONES[$i]}"
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:$port/health/liveness" 2>/dev/null || echo "000")
    check "Zone $jid liveness" "200" "$STATUS"
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:$port/health/readiness" 2>/dev/null || echo "000")
    check "Zone $jid readiness" "200" "$STATUS"
done

# ── Step 6: Establish N*(N-1)/2 Pairwise Corridors ───────────────────────

step "Establish $CORRIDORS pairwise corridors"

declare -A GENESIS_ROOTS
corridor_count=0

for i in $(seq 0 $((N - 1))); do
    for j in $(seq $((i + 1)) $((N - 1))); do
        corridor_count=$((corridor_count + 1))
        jid_i="${ZONES[$i]}"
        jid_j="${ZONES[$j]}"
        port_i=$((8080 + i))
        port_j=$((8080 + j))
        token_i="${ZONE_TOKENS[$i]}"
        token_j="${ZONE_TOKENS[$j]}"
        cid=$(corridor_id "$jid_i" "$jid_j")

        genesis_root=$(openssl rand -hex 32)
        GENESIS_ROOTS["${i}_${j}"]="$genesis_root"

        # Create corridor on both zones
        create='{"jurisdiction_a":"'"$jid_i"'","jurisdiction_b":"'"$jid_j"'"}'

        STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
            -X POST "http://localhost:$port_i/v1/corridors" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $token_i" \
            -d "$create" 2>/dev/null || echo "000")
        check "Corridor $corridor_count/$CORRIDORS: $jid_i creates ($jid_i<->$jid_j)" "201" "$STATUS"

        STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
            -X POST "http://localhost:$port_j/v1/corridors" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $token_j" \
            -d "$create" 2>/dev/null || echo "000")
        check "Corridor $corridor_count/$CORRIDORS: $jid_j creates ($jid_i<->$jid_j)" "201" "$STATUS"
    done
done

# ── Step 7: Receipt Exchange ─────────────────────────────────────────────

step "Receipt exchange across all $CORRIDORS corridors"

for i in $(seq 0 $((N - 1))); do
    for j in $(seq $((i + 1)) $((N - 1))); do
        jid_i="${ZONES[$i]}"
        jid_j="${ZONES[$j]}"
        port_j=$((8080 + j))
        token_j="${ZONE_TOKENS[$j]}"
        cid=$(corridor_id "$jid_i" "$jid_j")
        genesis_root="${GENESIS_ROOTS["${i}_${j}"]}"

        receipt='{"corridor_id":"'"$cid"'","origin_zone_id":"org.momentum.mez.zone.'"$jid_i"'","sequence":0,"receipt_json":{"type":"CorridorReceipt","corridor_id":"'"$cid"'","sequence":0,"timestamp":"'"$(date -u +%Y-%m-%dT%H:%M:%SZ)"'","prev_root":"'"$genesis_root"'","next_root":"'"$(openssl rand -hex 32)"'","lawpack_digest_set":["'"$(openssl rand -hex 32)"'"],"ruleset_digest_set":["'"$(openssl rand -hex 32)"'"]},"receipt_digest":"'"$(openssl rand -hex 32)"'","signature":"'"$(openssl rand -hex 64)"'","produced_at":"'"$(date -u +%Y-%m-%dT%H:%M:%SZ)"'"}'

        STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
            -X POST "http://localhost:$port_j/v1/corridors/peers/receipts" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $token_j" \
            -d "$receipt" 2>/dev/null || echo "000")
        check "$jid_j accepts receipt from $jid_i" "200" "$STATUS"
    done
done

# ── Results ───────────────────────────────────────────────────────────────

echo ""
echo -e "${BLUE}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║               N-ZONE MESH DEMO RESULTS                   ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""
TOTAL=$((PASS + FAIL))
echo -e "  Tests:     ${GREEN}$PASS passed${NC}, ${RED}$FAIL failed${NC} out of $TOTAL"
echo -e "  Zones:     $N"
echo -e "  Corridors: $CORRIDORS"
echo ""
echo -e "  ${BOLD}Zone Deployment:${NC}"
for i in $(seq 0 $((N - 1))); do
    echo -e "    ${GREEN}✓${NC} ${ZONES[$i]} (port $((8080 + i)))"
done
echo ""
echo -e "  ${BOLD}Corridor Mesh:${NC}"
for i in $(seq 0 $((N - 1))); do
    for j in $(seq $((i + 1)) $((N - 1))); do
        echo -e "    ${GREEN}✓${NC} ${ZONES[$i]} <-> ${ZONES[$j]}"
    done
done
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo -e "${RED}SOME TESTS FAILED — N-zone mesh demo NOT fully passing${NC}"
    exit 1
else
    echo -e "${GREEN}ALL TESTS PASSED — $N-zone corridor mesh VERIFIED${NC}"
    echo ""
    echo -e "  ${BOLD}The MEZ Stack supports N-zone corridor mesh topology.${NC}"
    exit 0
fi
