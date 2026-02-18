#!/bin/bash
# MSEZ Zone Deployment Script
# Deploy a complete Special Economic Zone from a profile
#
# Usage:
#   ./deploy-zone.sh [profile] [zone-id] [jurisdiction]
#
# Examples:
#   ./deploy-zone.sh digital-financial-center my-zone ae-dubai-difc
#   ./deploy-zone.sh minimal-mvp test-zone ex

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
PROFILE="${1:-digital-financial-center}"
ZONE_ID="${2:-org.momentum.msez.zone.local}"
JURISDICTION="${3:-ex}"

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEPLOY_DIR="$SCRIPT_DIR/../docker"

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}   MSEZ Zone Deployment Script${NC}"
echo -e "${BLUE}   Special Economic Zone in a Box${NC}"
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

# Create config directory
CONFIG_DIR="$DEPLOY_DIR/config"
mkdir -p "$CONFIG_DIR"

# Generate zone.yaml
echo -e "${YELLOW}Generating zone configuration...${NC}"

cat > "$CONFIG_DIR/zone.yaml" << EOF
# Auto-generated zone configuration
# Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")

zone_id: $ZONE_ID
jurisdiction_id: $JURISDICTION
zone_name: "${ZONE_ID##*.} Zone"

profile:
  profile_id: org.momentum.msez.profile.$PROFILE
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
  - org.momentum.msez.corridor.local

trust_anchors: []

key_rotation_policy:
  default:
    rotation_days: 90
    grace_days: 14

lockfile_path: stack.lock
EOF

echo -e "${GREEN}Zone configuration generated${NC}"

# Create .env file
echo -e "${YELLOW}Creating environment configuration...${NC}"

# Generate random credentials — never use hardcoded defaults
GENERATED_PG_PASSWORD="$(openssl rand -base64 24 | tr -d '/+=' | head -c 32)"
GENERATED_GRAFANA_PASSWORD="$(openssl rand -base64 24 | tr -d '/+=' | head -c 32)"

cat > "$DEPLOY_DIR/.env" << EOF
# MSEZ Zone Environment Configuration
# Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Zone Configuration
MSEZ_ZONE_ID=$ZONE_ID
MSEZ_JURISDICTION=$JURISDICTION
MSEZ_PROFILE=$PROFILE
MSEZ_LOG_LEVEL=info

# Corridor Configuration
MSEZ_CORRIDOR_ID=org.momentum.msez.corridor.local

# Watcher Configuration
MSEZ_WATCHER_ID=watcher-local-001
MSEZ_BOND_AMOUNT=100000

# Database Configuration
POSTGRES_USER=msez
POSTGRES_PASSWORD=$GENERATED_PG_PASSWORD
POSTGRES_DB=msez

# Observability
GRAFANA_PASSWORD=$GENERATED_GRAFANA_PASSWORD
EOF

echo -e "${GREEN}Environment configuration created${NC}"

# Create keys directory
KEYS_DIR="$DEPLOY_DIR/keys"
mkdir -p "$KEYS_DIR"

# Generate zone keys if they don't exist
if [ ! -f "$KEYS_DIR/zone-authority.ed25519.jwk" ]; then
    echo -e "${YELLOW}Generating zone authority keys...${NC}"

    # Locate or build msez-cli for real Ed25519 key generation.
    MSEZ_CLI=""
    if command -v msez &> /dev/null; then
        MSEZ_CLI="msez"
    elif [ -f "$PROJECT_ROOT/target/release/msez" ]; then
        MSEZ_CLI="$PROJECT_ROOT/target/release/msez"
    elif [ -f "$PROJECT_ROOT/target/debug/msez" ]; then
        MSEZ_CLI="$PROJECT_ROOT/target/debug/msez"
    else
        echo -e "${YELLOW}msez-cli not found — building...${NC}"
        (cd "$PROJECT_ROOT/msez" && cargo build --release --bin msez 2>&1) || {
            echo -e "${RED}Error: Failed to build msez-cli. Cannot generate real Ed25519 keys.${NC}"
            exit 1
        }
        MSEZ_CLI="$PROJECT_ROOT/target/release/msez"
    fi

    "$MSEZ_CLI" vc keygen --format jwk --output "$KEYS_DIR" --prefix zone-authority.ed25519

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

    echo -e "${GREEN}Zone authority keys generated (Ed25519 via msez-cli)${NC}"
fi

# Pull/build images
echo ""
echo -e "${YELLOW}Building Docker images...${NC}"
cd "$DEPLOY_DIR"

# Create requirements.txt if it doesn't exist
if [ ! -f "$PROJECT_ROOT/requirements.txt" ]; then
    cat > "$PROJECT_ROOT/requirements.txt" << EOF
# MSEZ Stack Dependencies
pyyaml>=6.0
jsonschema>=4.0
cryptography>=41.0
pynacl>=1.5
aiohttp>=3.9
asyncio>=3.4
redis>=5.0
psycopg2-binary>=2.9
prometheus-client>=0.19
structlog>=24.0
EOF
fi

docker compose build --parallel

# Start services
echo ""
echo -e "${YELLOW}Starting MSEZ Zone services...${NC}"
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
echo -e "${GREEN}   MSEZ Zone Deployment Complete${NC}"
echo -e "${GREEN}======================================${NC}"
echo ""
docker compose ps
echo ""
echo -e "${BLUE}Service Endpoints:${NC}"
echo -e "  MSEZ API (all services): http://localhost:8080"
echo -e "  Health check:            http://localhost:8080/health/liveness"
echo ""
echo -e "${BLUE}Observability:${NC}"
echo -e "  Prometheus:              http://localhost:9090"
echo -e "  Grafana:                 http://localhost:3000"
echo ""
echo -e "${BLUE}Databases:${NC}"
echo -e "  PostgreSQL:              localhost:5432"
echo ""
echo -e "${YELLOW}To view logs:${NC}"
echo -e "  docker compose logs -f"
echo ""
echo -e "${YELLOW}To stop the zone:${NC}"
echo -e "  docker compose down"
echo ""
echo -e "${GREEN}Zone '$ZONE_ID' is now operational!${NC}"
