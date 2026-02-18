# MSEZ Specification: Licensepacks

**Spec ID:** 98-licensepacks
**Version:** 1.0.0
**Status:** Draft
**Stack Version:** 0.4.44+

---

## 1. Overview

A **licensepack** is a content-addressed snapshot of jurisdictional licensing state — the registry of who holds what licenses, under what conditions, with what permissions and restrictions. While lawpacks capture static law (statutes that change over months/years) and regpacks capture dynamic regulatory guidance (sanctions lists, examination schedules that change daily/weekly), licensepacks capture the **live licensing registry** — the current state of granted licenses, their holders, conditions, and compliance status.

### 1.1 The Pack Trilogy

| Pack Type | What It Captures | Change Frequency | Primary Use |
|-----------|-----------------|------------------|-------------|
| **Lawpack** | Statutes, regulations, legal text (AKN XML) | Months/Years | Legal basis validation |
| **Regpack** | Regulatory guidance, sanctions, calendars | Days/Weeks | Compliance checking |
| **Licensepack** | License registry, holders, conditions, permits | Hours/Days | Authorization verification |

### 1.2 Why Licensepacks?

In an Economic Zone, every significant economic activity requires a license:
- Corporate Service Providers (CSPs) must hold CSP licenses
- Banks must hold banking licenses with specific activity permissions
- Exchanges must hold exchange licenses with trading pair approvals
- Custody providers must hold custody licenses with asset class authorizations

The licensepack provides a **cryptographically verifiable snapshot** of this registry, enabling:
1. **Offline verification** — verify a counterparty's license without querying the regulator
2. **Audit trails** — prove licensing state at any historical point in time
3. **Cross-zone settlement** — verify counterparty authorization before settlement
4. **Compliance tensors** — populate the LICENSING domain of compliance tensors

---

## 2. Licensepack Structure

### 2.1 Archive Format

Licensepacks are distributed as `.licensepack.zip` archives:

```
<digest>.licensepack.zip
├── licensepack.yaml           # Semantic metadata
├── digest.sha256              # Content-addressed digest
├── index.json                 # Master index of all licenses
├── license-types/
│   ├── index.json             # Registry of license type definitions
│   └── <license_type_id>.json # Per-type definition, requirements, fees
├── licenses/
│   ├── index.json             # Registry of granted licenses
│   └── <license_id>/
│       ├── license.json       # License record
│       ├── holder.json        # Holder profile
│       ├── conditions.json    # License conditions
│       ├── permissions.json   # Permitted activities
│       ├── restrictions.json  # Activity restrictions
│       └── audit-trail.json   # Compliance audit history
├── permits/
│   ├── index.json             # Sub-permits and approvals
│   └── <permit_id>.json       # Individual permit records
├── suspensions/
│   ├── index.json             # Active suspensions
│   └── <suspension_id>.json   # Suspension details
├── revocations/
│   ├── index.json             # Historical revocations
│   └── <revocation_id>.json   # Revocation records
└── delta/
    └── from-<prev_digest>.json # Delta from previous snapshot
```

### 2.2 licensepack.yaml

The metadata manifest:

```yaml
licensepack_format_version: "1"
licensepack_id: "licensepack:ae-dubai-difc:financial:2026-02-03T00:00:00Z"

jurisdiction_id: "ae-dubai-difc"
domain: "financial"                    # financial, corporate, professional, trade
as_of_date: "2026-02-03"
snapshot_timestamp: "2026-02-03T00:00:00Z"
snapshot_type: "daily"                 # hourly, daily, weekly, on_demand

sources:
  - source_id: "dfsa-registry"
    uri: "https://register.dfsa.ae/api/v2/licenses"
    media_type: "application/json"
    retrieved_at: "2026-02-03T00:05:00Z"
    sha256: "a1b2c3d4..."
    record_count: 450
    license: "government-open-data"
    notes: "DFSA public register API"
    artifact_ref:
      artifact_type: "blob"
      digest_sha256: "..."
      uri: "dist/artifacts/blob/..."
      media_type: "application/json"
      byte_length: 1234567

regulator:
  regulator_id: "dfsa"
  name: "Dubai Financial Services Authority"
  jurisdiction_id: "ae-dubai-difc"
  registry_url: "https://register.dfsa.ae"
  api_capabilities:
    - license_verification
    - holder_lookup
    - condition_query

includes:
  license_types: 16
  licenses_active: 423
  licenses_suspended: 12
  licenses_total: 435
  permits: 1247
  conditions: 2891
  holders: 398

license: "CC0-1.0"

normalization:
  recipe_id: "dfsa-to-msez-v1"
  tool: "msez"
  tool_version: "0.4.44"
  inputs:
    - source_id: "dfsa-registry"
  notes: "Normalized from DFSA public register format"

previous_licensepack_digest: "sha256:9f8e7d6c..."

delta:
  licenses_granted: 3
  licenses_revoked: 1
  licenses_suspended: 2
  licenses_reinstated: 1
  conditions_added: 15
  conditions_removed: 4
  permits_issued: 23
  permits_revoked: 2
```

### 2.3 Digest Computation

The licensepack digest is computed deterministically:

```
licensepack_digest = SHA256(
  "msez-licensepack-v1\0" ||
  canonical_bytes("licensepack.yaml") || "\0" ||
  canonical_bytes("index.json") || "\0" ||
  canonical_bytes("license-types/index.json") || "\0" ||
  Σ(sorted(license_type_files)(path || "\0" || canonical_bytes(path) || "\0")) ||
  canonical_bytes("licenses/index.json") || "\0" ||
  Σ(sorted(license_dirs)(
    path || "/license.json\0" || canonical_bytes(path + "/license.json") || "\0" ||
    path || "/holder.json\0" || canonical_bytes(path + "/holder.json") || "\0" ||
    path || "/conditions.json\0" || canonical_bytes(path + "/conditions.json") || "\0" ||
    path || "/permissions.json\0" || canonical_bytes(path + "/permissions.json") || "\0" ||
    path || "/restrictions.json\0" || canonical_bytes(path + "/restrictions.json") || "\0"
  )) ||
  canonical_bytes("permits/index.json") || "\0" ||
  Σ(sorted(permit_files)(path || "\0" || canonical_bytes(path) || "\0")) ||
  canonical_bytes("suspensions/index.json") || "\0" ||
  Σ(sorted(suspension_files)(path || "\0" || canonical_bytes(path) || "\0")) ||
  canonical_bytes("revocations/index.json") || "\0" ||
  Σ(sorted(revocation_files)(path || "\0" || canonical_bytes(path) || "\0"))
)
```

Where `canonical_bytes()` applies JCS (JSON Canonicalization Scheme) for JSON/YAML files.

**Determinism requirements:**
- No timestamps in content files (only in metadata)
- No floating point numbers (use string decimals for amounts)
- All arrays sorted by deterministic keys
- All object keys sorted alphabetically

---

## 3. License Record Schema

### 3.1 license.json

```json
{
  "$schema": "https://momentum-sez.org/schemas/licensepack.license.schema.json",
  "license_id": "lic:dfsa:2024-001234",
  "license_type_id": "dfsa:category-4",
  "license_number": "CL001234",

  "status": "active",
  "status_effective_date": "2024-06-15",

  "issued_date": "2024-01-15",
  "effective_date": "2024-02-01",
  "expiry_date": "2029-01-31",

  "holder_id": "entity:difc:12345",
  "holder_legal_name": "Acme Financial Services Ltd",
  "holder_registration_number": "CL-12345",
  "holder_did": "did:key:z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH",

  "regulator_id": "dfsa",
  "issuing_authority": "Dubai Financial Services Authority",

  "permitted_activities": [
    "accepting_deposits",
    "providing_credit",
    "arranging_credit",
    "managing_assets"
  ],

  "asset_classes_authorized": [
    "securities",
    "derivatives",
    "crypto_tokens"
  ],

  "client_types_permitted": [
    "professional_client",
    "market_counterparty"
  ],

  "geographic_scope": ["ae-dubai-difc", "ae-dubai", "ae"],

  "prudential_category": "category-4",
  "capital_requirement": {
    "minimum_base_capital": "2000000",
    "currency": "USD",
    "risk_based_capital_ratio": "0.12"
  },

  "conditions_ref": "conditions.json",
  "permissions_ref": "permissions.json",
  "restrictions_ref": "restrictions.json",
  "audit_trail_ref": "audit-trail.json",

  "verification": {
    "verifiable_credential_id": "vc:dfsa:license:2024-001234",
    "vc_digest_sha256": "abc123...",
    "issuer_did": "did:web:dfsa.ae",
    "issued_at": "2024-01-15T10:00:00Z"
  }
}
```

### 3.2 holder.json

```json
{
  "$schema": "https://momentum-sez.org/schemas/licensepack.holder.schema.json",
  "holder_id": "entity:difc:12345",
  "entity_type": "company",
  "legal_name": "Acme Financial Services Ltd",
  "trading_names": ["Acme Finance", "AFS"],
  "registration_number": "CL-12345",
  "incorporation_date": "2023-06-01",
  "jurisdiction_of_incorporation": "ae-dubai-difc",

  "did": "did:key:z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH",

  "registered_address": {
    "line1": "Level 14, Gate Building",
    "line2": "DIFC",
    "city": "Dubai",
    "country": "AE",
    "postal_code": "507211"
  },

  "contact": {
    "compliance_officer": "Jane Smith",
    "compliance_email": "compliance@acmefinance.ae",
    "general_email": "info@acmefinance.ae",
    "phone": "+971-4-123-4567"
  },

  "controllers": [
    {
      "controller_id": "ctrl:001",
      "name": "John Doe",
      "role": "CEO",
      "approved_date": "2023-06-01",
      "fitness_assessment": "approved"
    }
  ],

  "beneficial_owners": [
    {
      "owner_id": "ubo:001",
      "name": "Investment Holdings Ltd",
      "ownership_percentage": "75.5",
      "jurisdiction": "ky",
      "verified_date": "2023-05-15"
    }
  ],

  "group_structure": {
    "parent_entity": "Investment Holdings Ltd",
    "parent_jurisdiction": "ky",
    "group_supervisor": "CIMA"
  }
}
```

### 3.3 conditions.json

```json
{
  "$schema": "https://momentum-sez.org/schemas/licensepack.conditions.schema.json",
  "license_id": "lic:dfsa:2024-001234",
  "conditions": [
    {
      "condition_id": "cond:001",
      "condition_type": "capital",
      "description": "Maintain minimum base capital of USD 2,000,000",
      "metric": "base_capital",
      "threshold": "2000000",
      "currency": "USD",
      "operator": ">=",
      "frequency": "continuous",
      "reporting_frequency": "quarterly",
      "effective_date": "2024-02-01",
      "status": "active"
    },
    {
      "condition_id": "cond:002",
      "condition_type": "operational",
      "description": "Maintain adequate professional indemnity insurance",
      "metric": "pi_insurance_coverage",
      "threshold": "5000000",
      "currency": "USD",
      "operator": ">=",
      "frequency": "annual",
      "effective_date": "2024-02-01",
      "status": "active"
    },
    {
      "condition_id": "cond:003",
      "condition_type": "activity_restriction",
      "description": "May not conduct business with retail clients for first 12 months",
      "restriction_type": "client_type",
      "restricted_value": "retail_client",
      "effective_date": "2024-02-01",
      "expiry_date": "2025-02-01",
      "status": "active"
    }
  ]
}
```

### 3.4 permissions.json

```json
{
  "$schema": "https://momentum-sez.org/schemas/licensepack.permissions.schema.json",
  "license_id": "lic:dfsa:2024-001234",
  "permissions": [
    {
      "permission_id": "perm:001",
      "activity": "accepting_deposits",
      "scope": {
        "client_types": ["professional_client", "market_counterparty"],
        "currencies": ["USD", "EUR", "AED"],
        "jurisdictions": ["ae-dubai-difc"]
      },
      "limits": {
        "single_deposit_max": "10000000",
        "aggregate_deposits_max": "500000000",
        "currency": "USD"
      },
      "effective_date": "2024-02-01",
      "status": "active"
    },
    {
      "permission_id": "perm:002",
      "activity": "providing_credit",
      "scope": {
        "client_types": ["professional_client"],
        "secured_only": true,
        "jurisdictions": ["ae-dubai-difc", "ae-dubai"]
      },
      "limits": {
        "single_facility_max": "50000000",
        "total_book_max": "200000000",
        "currency": "USD"
      },
      "effective_date": "2024-02-01",
      "status": "active"
    }
  ]
}
```

### 3.5 restrictions.json

```json
{
  "$schema": "https://momentum-sez.org/schemas/licensepack.restrictions.schema.json",
  "license_id": "lic:dfsa:2024-001234",
  "restrictions": [
    {
      "restriction_id": "rest:001",
      "restriction_type": "geographic",
      "description": "May not solicit clients from jurisdictions outside GCC",
      "blocked_jurisdictions": ["*"],
      "allowed_jurisdictions": ["ae", "sa", "qa", "kw", "bh", "om"],
      "effective_date": "2024-02-01",
      "status": "active"
    },
    {
      "restriction_id": "rest:002",
      "restriction_type": "activity",
      "description": "May not act as principal in proprietary trading",
      "blocked_activities": ["proprietary_trading"],
      "effective_date": "2024-02-01",
      "status": "active"
    },
    {
      "restriction_id": "rest:003",
      "restriction_type": "product",
      "description": "May not offer leveraged products exceeding 10:1",
      "product_type": "leveraged_derivatives",
      "max_leverage": "10",
      "effective_date": "2024-02-01",
      "status": "active"
    }
  ]
}
```

---

## 4. Licensepack Lock File

### 4.1 licensepack.lock.json

Created in the module directory referencing the licensepack:

```json
{
  "$schema": "https://momentum-sez.org/schemas/licensepack.lock.schema.json",
  "lock_version": "1",
  "generated_at": "2026-02-03T00:15:00Z",
  "generator": "msez",
  "generator_version": "0.4.44",

  "licensepack": {
    "licensepack_id": "licensepack:ae-dubai-difc:financial:2026-02-03T00:00:00Z",
    "jurisdiction_id": "ae-dubai-difc",
    "domain": "financial",
    "as_of_date": "2026-02-03",
    "digest_sha256": "e4f5a6b7c8d9..."
  },

  "artifact": {
    "artifact_type": "licensepack",
    "digest_sha256": "e4f5a6b7c8d9...",
    "uri": "dist/artifacts/licensepack/e4f5a6b7c8d9....licensepack.zip",
    "media_type": "application/zip",
    "byte_length": 2345678
  },

  "component_digests": {
    "metadata": "sha256:aaa...",
    "index": "sha256:bbb...",
    "license_types": "sha256:ccc...",
    "licenses": "sha256:ddd...",
    "permits": "sha256:eee...",
    "suspensions": "sha256:fff...",
    "revocations": "sha256:ggg..."
  },

  "provenance": {
    "sources": [
      {
        "source_id": "dfsa-registry",
        "uri": "https://register.dfsa.ae/api/v2/licenses",
        "retrieved_at": "2026-02-03T00:05:00Z",
        "digest_sha256": "hhh..."
      }
    ],
    "normalization": {
      "recipe_id": "dfsa-to-msez-v1",
      "tool": "msez",
      "tool_version": "0.4.44"
    }
  },

  "verification": {
    "verified_at": "2026-02-03T00:15:00Z",
    "verifier": "msez-cli",
    "digest_verified": true,
    "schema_valid": true
  }
}
```

---

## 5. Stack Lock Integration

### 5.1 licensepacks Section

Add to `stack.lock.schema.json` and `stack.lock`:

```json
{
  "stack_spec_version": "0.4.44",
  "...": "...",

  "licensepacks": [
    {
      "jurisdiction_id": "ae-dubai-difc",
      "domain": "financial",
      "licensepack_digest_sha256": "e4f5a6b7c8d9...",
      "licensepack_lock_path": "modules/licensing/registry/ae-dubai-difc/licensepack.lock.json",
      "licensepack_lock_sha256": "ijk...",
      "licensepack_artifact_path": "dist/artifacts/licensepack/e4f5a6b7c8d9....licensepack.zip",
      "as_of_date": "2026-02-03"
    }
  ]
}
```

### 5.2 Zone Manifest Integration

Add to `zone.yaml` and `zone.schema.json`:

```yaml
zone_id: org.momentum.msez.zone.difc-fintech
jurisdiction_id: ae-dubai-difc
zone_name: DIFC Fintech Zone

profile:
  profile_id: org.momentum.msez.profile.digital-financial-center
  version: 0.4.44

lawpack_domains:
  - civil
  - financial

licensepack_domains:           # NEW FIELD
  - financial
  - corporate
  - professional

licensepack_refresh_policy:    # NEW FIELD
  default:
    refresh_frequency: daily
    max_staleness_hours: 24
  financial:
    refresh_frequency: hourly
    max_staleness_hours: 4
```

---

## 6. CLI Commands

### 6.1 Licensepack Management

```bash
# Fetch and create licensepack from regulator API
msez licensepack fetch \
  --jurisdiction ae-dubai-difc \
  --domain financial \
  --source dfsa-registry

# Verify licensepack integrity
msez licensepack verify <digest>.licensepack.zip

# Lock licensepack to module
msez licensepack lock \
  --jurisdiction ae-dubai-difc \
  --domain financial \
  --module-path modules/licensing/registry/ae-dubai-difc/

# Compute delta between two licensepacks
msez licensepack delta \
  --from <old_digest>.licensepack.zip \
  --to <new_digest>.licensepack.zip

# Query license status
msez licensepack query \
  --jurisdiction ae-dubai-difc \
  --holder-did did:key:z6Mkp... \
  --activity accepting_deposits

# Export license as VC
msez licensepack export-vc \
  --license-id lic:dfsa:2024-001234 \
  --issuer-key zone-authority.ed25519.jwk
```

### 6.2 License Verification

```bash
# Verify counterparty license for transaction
msez license verify \
  --counterparty-did did:key:z6Mkp... \
  --activity settlement \
  --amount 1000000 \
  --currency USD \
  --jurisdiction ae-dubai-difc

# Check license against compliance tensor
msez tensor check-license \
  --asset-id asset:001 \
  --license-id lic:dfsa:2024-001234 \
  --domain LICENSING
```

---

## 7. Verifiable Credentials

### 7.1 License VC Schema

Licenses can be exported as verifiable credentials:

```json
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://momentum-sez.org/credentials/license/v1"
  ],
  "type": ["VerifiableCredential", "LicenseCredential"],
  "id": "vc:dfsa:license:2024-001234",
  "issuer": "did:web:dfsa.ae",
  "issuanceDate": "2024-01-15T10:00:00Z",
  "expirationDate": "2029-01-31T23:59:59Z",
  "credentialSubject": {
    "id": "did:key:z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH",
    "license": {
      "license_id": "lic:dfsa:2024-001234",
      "license_type": "dfsa:category-4",
      "license_number": "CL001234",
      "holder_legal_name": "Acme Financial Services Ltd",
      "permitted_activities": ["accepting_deposits", "providing_credit"],
      "jurisdiction": "ae-dubai-difc",
      "status": "active"
    }
  },
  "credentialStatus": {
    "id": "https://register.dfsa.ae/status/2024-001234",
    "type": "LicenseStatusList2024"
  },
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2024-01-15T10:00:00Z",
    "verificationMethod": "did:web:dfsa.ae#key-1",
    "proofPurpose": "assertionMethod",
    "proofValue": "z..."
  }
}
```

### 7.2 License Attestation for Compliance Tensor

```json
{
  "attestation_id": "att:license:2024-001234:2026-02-03",
  "attestation_type": "LICENSE_VALID",
  "asset_id": "asset:settlement:001",
  "jurisdiction_id": "ae-dubai-difc",
  "domain": "LICENSING",
  "timestamp": "2026-02-03T00:15:00Z",
  "validity_window": {
    "not_before": "2026-02-03T00:00:00Z",
    "not_after": "2026-02-04T00:00:00Z"
  },
  "evidence": {
    "license_id": "lic:dfsa:2024-001234",
    "licensepack_digest": "e4f5a6b7c8d9...",
    "verification_path": ["licenses", "lic:dfsa:2024-001234", "license.json"]
  },
  "attester": {
    "attester_id": "watcher:difc:001",
    "attester_did": "did:key:z6Mkw..."
  },
  "signature": "..."
}
```

---

## 8. Compliance Tensor Integration

### 8.1 LICENSING Domain

The compliance tensor includes LICENSING as a core domain:

```python
class ComplianceDomain(Enum):
    AML = "AML"
    KYC = "KYC"
    SANCTIONS = "SANCTIONS"
    TAX = "TAX"
    SECURITIES = "SECURITIES"
    CORPORATE = "CORPORATE"
    LICENSING = "LICENSING"      # Licensepack-backed domain
```

### 8.2 License State Evaluation

```python
def evaluate_license_compliance(
    tensor: ComplianceTensor,
    license_id: str,
    activity: str,
    licensepack: LicensePack
) -> ComplianceState:
    """
    Evaluate licensing compliance for an activity.

    Returns:
        COMPLIANT: Valid license with permission for activity
        NON_COMPLIANT: No license, expired, or activity not permitted
        PENDING: License application in progress
        SUSPENDED: License temporarily suspended
    """
    license = licensepack.get_license(license_id)

    if not license:
        return ComplianceState.NON_COMPLIANT

    if license.status == "suspended":
        return ComplianceState.SUSPENDED

    if license.status == "pending":
        return ComplianceState.PENDING

    if license.is_expired():
        return ComplianceState.NON_COMPLIANT

    if not license.permits_activity(activity):
        return ComplianceState.NON_COMPLIANT

    if license.has_blocking_restriction(activity):
        return ComplianceState.NON_COMPLIANT

    return ComplianceState.COMPLIANT
```

---

## 9. Security Considerations

### 9.1 Freshness

Licensepacks must include staleness checks:
- Settlement operations MUST use licensepacks no older than `max_staleness_hours`
- The compliance tensor MUST track licensepack timestamp in attestation
- Corridor watchers MUST verify licensepack freshness before attestation

### 9.2 Regulator Authority

Only licensepacks signed by recognized regulators are valid:
- Each jurisdiction defines its regulatory authority DIDs
- Licensepack sources must chain to regulator authority
- Zone manifests specify trusted regulator DIDs

### 9.3 Revocation Propagation

License revocations propagate with priority:
- Revocation events trigger immediate licensepack refresh
- Compliance tensors transition to NON_COMPLIANT on revocation
- Settlement operations blocked for revoked licenses

---

## 10. Implementation Checklist

### 10.1 Schemas
- [ ] `licensepack.schema.json` — Main licensepack structure
- [ ] `licensepack.license.schema.json` — License record
- [ ] `licensepack.holder.schema.json` — Holder profile
- [ ] `licensepack.conditions.schema.json` — License conditions
- [ ] `licensepack.permissions.schema.json` — Permitted activities
- [ ] `licensepack.restrictions.schema.json` — Activity restrictions
- [ ] `licensepack.lock.schema.json` — Lock file format
- [ ] `vc.license-credential.schema.json` — License VC

### 10.2 Tools
- [ ] `tools/licensepack.py` — Pack management (fetch, verify, lock, delta, query)
- [ ] Update `tools/phoenix/tensor.py` — LICENSING domain support
- [ ] Update `tools/vc.py` — License credential issuance

### 10.3 Schemas Updates
- [ ] `zone.schema.json` — Add `licensepack_domains`, `licensepack_refresh_policy`
- [ ] `stack.lock.schema.json` — Add `licensepacks` section

### 10.4 Modules
- [ ] `modules/licensing/registry/` — License registry module

---

*Specification Version: 1.0.0*
*Author: Momentum Engineering*
*Date: February 2026*
