#!/bin/bash
# MEZ Zone Deployment Script
# Deploy a complete Economic Zone from a profile
#
# Usage:
#   ./deploy-zone.sh [profile] [zone-id] [jurisdiction]
#
# Examples:
#   ./deploy-zone.sh sovereign-govos org.momentum.mez.zone.pk-sifc pk
#   ./deploy-zone.sh digital-financial-center my-zone ae-dubai-difc
#   ./deploy-zone.sh minimal-mvp test-zone ex
#
# Environment variables (optional, prompted if not set):
#   MASS_API_TOKEN         — Bearer token for Mass API authentication
#   MASS_ORG_INFO_URL      — Mass organization-info API URL
#   MASS_TREASURY_INFO_URL — Mass treasury-info API URL
#   MASS_CONSENT_INFO_URL  — Mass consent-info API URL

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
PROFILE="${1:-digital-financial-center}"
ZONE_ID="${2:-org.momentum.mez.zone.local}"
JURISDICTION="${3:-ex}"

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEPLOY_DIR="$SCRIPT_DIR/../docker"

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}   MEZ Zone Deployment Script${NC}"
echo -e "${BLUE}   Economic Zone in a Box${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""
echo -e "Profile:      ${GREEN}$PROFILE${NC}"
echo -e "Zone ID:      ${GREEN}$ZONE_ID${NC}"
echo -e "Jurisdiction: ${GREEN}$JURISDICTION${NC}"
echo ""

# Check prerequisites
echo -e "${YELLOW}Checking prerequisites...${NC}"

if ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: Docker is not installed${NC}"
    exit 1
fi

if ! command -v docker compose &> /dev/null; then
    echo -e "${RED}Error: Docker Compose is not installed${NC}"
    exit 1
fi

echo -e "${GREEN}Prerequisites OK${NC}"
echo ""

# Validate profile exists
PROFILE_PATH="$PROJECT_ROOT/profiles/$PROFILE/profile.yaml"
if [ ! -f "$PROFILE_PATH" ]; then
    echo -e "${RED}Error: Profile '$PROFILE' not found at $PROFILE_PATH${NC}"
    echo "Available profiles:"
    ls -1 "$PROJECT_ROOT/profiles/"
    exit 1
fi

echo -e "${GREEN}Profile found: $PROFILE_PATH${NC}"

# Check for jurisdiction-specific zone.yaml
JURISDICTION_ZONE="$PROJECT_ROOT/jurisdictions/$JURISDICTION/zone.yaml"
if [ -f "$JURISDICTION_ZONE" ]; then
    echo -e "${GREEN}Jurisdiction zone manifest found: $JURISDICTION_ZONE${NC}"
    USE_JURISDICTION_ZONE=true
else
    # Also check for hyphenated jurisdiction dirs (e.g., pk-sifc)
    JURISDICTION_ZONE="$PROJECT_ROOT/jurisdictions/${ZONE_ID##*.}/zone.yaml"
    if [ -f "$JURISDICTION_ZONE" ]; then
        echo -e "${GREEN}Jurisdiction zone manifest found: $JURISDICTION_ZONE${NC}"
        USE_JURISDICTION_ZONE=true
    else
        USE_JURISDICTION_ZONE=false
    fi
fi

# Create config directory
CONFIG_DIR="$DEPLOY_DIR/config"
mkdir -p "$CONFIG_DIR"

# Generate or copy zone.yaml
echo -e "${YELLOW}Preparing zone configuration...${NC}"

if [ "$USE_JURISDICTION_ZONE" = true ]; then
    cp "$JURISDICTION_ZONE" "$CONFIG_DIR/zone.yaml"
    echo -e "${GREEN}Copied jurisdiction zone manifest${NC}"
else
    # Generate a basic zone.yaml for jurisdictions without a dedicated manifest
    cat > "$CONFIG_DIR/zone.yaml" << EOF
# Auto-generated zone configuration
# Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")

zone_id: $ZONE_ID
jurisdiction_id: $JURISDICTION
zone_name: "${ZONE_ID##*.} Zone"

profile:
  profile_id: org.momentum.mez.profile.$PROFILE
  version: "0.4.44"

jurisdiction_stack:
  - $JURISDICTION

lawpack_domains:
  - civil
  - financial

licensepack_domains:
  - financial
  - corporate

licensepack_refresh_policy:
  default:
    refresh_frequency: daily
    max_staleness_hours: 24
  financial:
    refresh_frequency: hourly
    max_staleness_hours: 4

corridors:
  - org.momentum.mez.corridor.swift.iso20022-cross-border

trust_anchors: []

key_rotation_policy:
  default:
    rotation_days: 90
    grace_days: 14

lockfile_path: stack.lock
EOF

    echo -e "${GREEN}Zone configuration generated${NC}"
fi

# Create .env file
echo -e "${YELLOW}Creating environment configuration...${NC}"

# Generate random credentials — never use hardcoded defaults
GENERATED_PG_PASSWORD="$(openssl rand -base64 24 | tr -d '/+=' | head -c 32)"
GENERATED_GRAFANA_PASSWORD="$(openssl rand -base64 24 | tr -d '/+=' | head -c 32)"
GENERATED_AUTH_TOKEN="$(openssl rand -base64 32 | tr -d '/+=' | head -c 48)"

cat > "$DEPLOY_DIR/.env" << EOF
# MEZ Zone Environment Configuration
# Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Zone Configuration
MEZ_ZONE_ID=$ZONE_ID
MEZ_JURISDICTION=$JURISDICTION
MEZ_PROFILE=$PROFILE
MEZ_LOG_LEVEL=info

# Zone manifest path (inside container)
ZONE_CONFIG=/app/config/zone.yaml

# Authentication
AUTH_TOKEN=$GENERATED_AUTH_TOKEN

# Corridor Configuration
MEZ_CORRIDOR_ID=org.momentum.mez.corridor.swift.iso20022-cross-border

# Watcher Configuration
MEZ_WATCHER_ID=watcher-${JURISDICTION}-001
MEZ_BOND_AMOUNT=100000

# Database Configuration
POSTGRES_USER=mez
POSTGRES_PASSWORD=$GENERATED_PG_PASSWORD
POSTGRES_DB=mez

# Mass API Configuration
# Set MASS_API_TOKEN to connect to live Mass APIs.
# Without it, the zone operates in standalone mode (no Mass proxy).
MASS_API_TOKEN=\${MASS_API_TOKEN:-}
MASS_ORG_INFO_URL=\${MASS_ORG_INFO_URL:-https://organization-info.api.mass.inc}
MASS_TREASURY_INFO_URL=\${MASS_TREASURY_INFO_URL:-https://treasury-info.api.mass.inc}
MASS_CONSENT_INFO_URL=\${MASS_CONSENT_INFO_URL:-https://consent.api.mass.inc}
MASS_IDENTITY_INFO_URL=\${MASS_IDENTITY_INFO_URL:-}
MASS_TIMEOUT_SECS=30

# Observability
GRAFANA_PASSWORD=$GENERATED_GRAFANA_PASSWORD
EOF

echo -e "${GREEN}Environment configuration created${NC}"
echo -e "${YELLOW}Auth token saved to .env (use this for API access)${NC}"

# Create keys directory
KEYS_DIR="$DEPLOY_DIR/keys"
mkdir -p "$KEYS_DIR"

# Generate zone keys if they don't exist
if [ ! -f "$KEYS_DIR/zone-authority.ed25519.jwk" ]; then
    echo -e "${YELLOW}Generating zone authority keys...${NC}"

    # Locate or build mez-cli for real Ed25519 key generation.
    MEZ_CLI=""
    if command -v mez &> /dev/null; then
        MEZ_CLI="mez"
    elif [ -f "$PROJECT_ROOT/target/release/mez" ]; then
        MEZ_CLI="$PROJECT_ROOT/target/release/mez"
    elif [ -f "$PROJECT_ROOT/target/debug/mez" ]; then
        MEZ_CLI="$PROJECT_ROOT/target/debug/mez"
    else
        echo -e "${YELLOW}mez-cli not found — building...${NC}"
        (cd "$PROJECT_ROOT/mez" && cargo build --release --bin mez 2>&1) || {
            echo -e "${RED}Error: Failed to build mez-cli. Cannot generate real Ed25519 keys.${NC}"
            exit 1
        }
        MEZ_CLI="$PROJECT_ROOT/target/release/mez"
    fi

    "$MEZ_CLI" vc keygen --format jwk --output "$KEYS_DIR" --prefix zone-authority.ed25519

    # Verify the generated key file is valid.
    if [ ! -f "$KEYS_DIR/zone-authority.ed25519.jwk" ]; then
        echo -e "${RED}Error: Key generation failed — JWK file not created${NC}"
        exit 1
    fi

    # Validate the JWK contains required fields.
    for field in kty crv x d; do
        if ! grep -q "\"$field\"" "$KEYS_DIR/zone-authority.ed25519.jwk"; then
            echo -e "${RED}Error: Generated JWK missing required field: $field${NC}"
            exit 1
        fi
    done

    echo -e "${GREEN}Zone authority keys generated (Ed25519 via mez-cli)${NC}"
fi

# Extract the signing key hex for the container environment.
# The zone signing key is the 'd' (private key) field from the JWK, base64url-decoded to hex.
if command -v python3 &> /dev/null; then
    ZONE_KEY_HEX=$(python3 -c "
import json, base64
with open('$KEYS_DIR/zone-authority.ed25519.jwk') as f:
    jwk = json.load(f)
d = jwk['d']
# base64url decode
d += '=' * (4 - len(d) % 4) if len(d) % 4 else ''
raw = base64.urlsafe_b64decode(d)
print(raw.hex())
" 2>/dev/null || echo "")
    if [ -n "$ZONE_KEY_HEX" ]; then
        echo "ZONE_SIGNING_KEY_HEX=$ZONE_KEY_HEX" >> "$DEPLOY_DIR/.env"
        echo -e "${GREEN}Zone signing key extracted to .env${NC}"
    else
        echo -e "${YELLOW}Warning: Could not extract signing key hex. Zone will use ephemeral key.${NC}"
    fi
else
    echo -e "${YELLOW}Warning: python3 not available — zone will use ephemeral signing key${NC}"
fi

# Pull/build images
echo ""
echo -e "${YELLOW}Building Docker images...${NC}"
cd "$DEPLOY_DIR"

docker compose build --parallel

# Start services
echo ""
echo -e "${YELLOW}Starting MEZ Zone services...${NC}"
docker compose up -d

# Wait for services to be healthy
echo ""
echo -e "${YELLOW}Waiting for services to be healthy...${NC}"

MAX_RETRIES=30
RETRY_COUNT=0

while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    if docker compose ps | grep -q "unhealthy\|starting"; then
        echo -n "."
        sleep 2
        RETRY_COUNT=$((RETRY_COUNT + 1))
    else
        echo ""
        break
    fi
done

if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
    echo ""
    echo -e "${YELLOW}Warning: Some services may still be starting${NC}"
fi

# Display status
echo ""
echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}   MEZ Zone Deployment Complete${NC}"
echo -e "${GREEN}======================================${NC}"
echo ""
docker compose ps
echo ""
echo -e "${BLUE}Zone:${NC}"
echo -e "  Zone ID:      $ZONE_ID"
echo -e "  Jurisdiction: $JURISDICTION"
echo -e "  Profile:      $PROFILE"
echo ""
echo -e "${BLUE}Service Endpoints:${NC}"
echo -e "  MEZ API:       http://localhost:8080"
echo -e "  Health check:  http://localhost:8080/health/liveness"
echo -e "  Readiness:     http://localhost:8080/health/readiness"
echo ""
echo -e "${BLUE}API Access:${NC}"
echo -e "  Auth token:    (see deploy/docker/.env AUTH_TOKEN)"
echo -e "  Example:       curl -H 'Authorization: Bearer <token>' http://localhost:8080/v1/corridors"
echo ""
echo -e "${BLUE}Observability:${NC}"
echo -e "  Prometheus:    http://localhost:9090"
echo -e "  Grafana:       http://localhost:3000 (admin / <see .env>)"
echo ""
echo -e "${BLUE}Database:${NC}"
echo -e "  PostgreSQL:    localhost:5432 (mez / <see .env>)"
echo ""
echo -e "${YELLOW}To view logs:${NC}"
echo -e "  docker compose logs -f mez-api"
echo ""
echo -e "${YELLOW}To stop the zone:${NC}"
echo -e "  cd deploy/docker && docker compose down"
echo ""
echo -e "${GREEN}Zone '$ZONE_ID' is now operational!${NC}"
