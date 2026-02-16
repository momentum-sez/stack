#!/usr/bin/env bash
# refresh-specs.sh — Fetch live Mass API OpenAPI specs into the specs/ directory.
#
# Usage: ./msez/crates/msez-mass-client/scripts/refresh-specs.sh
#
# Requires curl and network access to Mass API hosts (may need VPN).

set -euo pipefail

SPEC_DIR="$(cd "$(dirname "$0")/../specs" && pwd)"
echo "Refreshing OpenAPI specs into: ${SPEC_DIR}"

declare -A SPECS=(
    ["organization-info"]="https://organization-info.api.mass.inc/organization-info/v3/api-docs"
    ["investment-info"]="https://investment-info-production-4f3779c81425.herokuapp.com/investment-info/v3/api-docs"
    ["treasury-info"]="https://treasury-info.api.mass.inc/treasury-info/v3/api-docs"
    ["consent-info"]="https://consent.api.mass.inc/consent-info/v3/api-docs"
    ["templating-engine"]="https://templating-engine-prod-5edc768c1f80.herokuapp.com/templating-engine/v3/api-docs"
)

FAILED=0

for name in "${!SPECS[@]}"; do
    url="${SPECS[$name]}"
    target="${SPEC_DIR}/${name}.openapi.json"
    echo -n "  ${name}... "
    if curl -s --connect-timeout 10 --max-time 30 "${url}" -o "${target}.tmp" 2>/dev/null; then
        # Validate it's JSON
        if python3 -c "import json; json.load(open('${target}.tmp'))" 2>/dev/null; then
            mv "${target}.tmp" "${target}"
            echo "OK ($(wc -c < "${target}" | tr -d ' ') bytes)"
        else
            rm -f "${target}.tmp"
            echo "FAILED (invalid JSON)"
            FAILED=$((FAILED + 1))
        fi
    else
        rm -f "${target}.tmp"
        echo "FAILED (network error)"
        FAILED=$((FAILED + 1))
    fi
done

echo ""
if [ "$FAILED" -gt 0 ]; then
    echo "WARNING: ${FAILED} spec(s) failed to fetch. Check network/VPN access."
    exit 1
else
    echo "All specs refreshed successfully."
    echo "Run: cargo test -p msez-mass-client contract_test"
fi
