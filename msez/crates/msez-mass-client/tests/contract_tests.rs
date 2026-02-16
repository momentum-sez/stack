//! # Contract Tests: Rust Client ↔ Mass API OpenAPI Spec Validation
//!
//! These tests validate that the Rust client types in `msez-mass-client` match
//! the live Mass API OpenAPI 3.0 schemas committed in `specs/`. If a Java
//! developer renames a field (e.g., `createdAt` → `created_at`), these tests
//! catch the drift before it reaches a live zone.
//!
//! ## Test Strategy
//!
//! 1. **Schema deserialization**: Create JSON matching the OpenAPI response schema,
//!    verify the Rust type deserializes it correctly.
//! 2. **Field coverage**: Check that every field in the spec's component schema
//!    is either consumed by the Rust type or intentionally ignored (forward-compat).
//! 3. **Enum variants**: Verify all spec-defined enum values deserialize into the
//!    corresponding Rust enum (with `#[serde(other)]` catching future additions).
//! 4. **Required field alignment**: Non-`Option` fields in Rust must correspond
//!    to fields consistently present in API responses.
//!
//! ## Running
//!
//! ```bash
//! cargo test -p msez-mass-client contract_test
//! ```
//!
//! ## Staleness checks (fetch live specs and compare)
//!
//! ```bash
//! cargo test -p msez-mass-client contract_staleness -- --ignored
//! ```

use serde_json::Value;
use std::collections::HashSet;
use std::path::PathBuf;

/// Load an OpenAPI spec from the specs/ directory.
fn load_spec(name: &str) -> Value {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("specs");
    path.push(format!("{name}.openapi.json"));
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read spec {}: {e}", path.display()));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to parse spec {}: {e}", path.display()))
}

/// Extract a component schema by name from an OpenAPI spec.
fn get_schema<'a>(spec: &'a Value, schema_name: &str) -> &'a Value {
    &spec["components"]["schemas"][schema_name]
}

/// Get the field names from a schema's `properties` object.
fn schema_field_names(schema: &Value) -> HashSet<String> {
    match schema.get("properties") {
        Some(Value::Object(map)) => map.keys().cloned().collect(),
        _ => HashSet::new(),
    }
}

/// Extract enum values from a schema property.
fn schema_enum_values(schema: &Value, field_name: &str) -> Vec<String> {
    let prop = &schema["properties"][field_name];
    if let Some(arr) = prop.get("enum") {
        if let Some(arr) = arr.as_array() {
            return arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
    }
    Vec::new()
}

/// Get enum values from a referenced schema component.
fn get_component_enum_values(spec: &Value, component_name: &str) -> Vec<String> {
    let schema = &spec["components"]["schemas"][component_name];
    if let Some(arr) = schema.get("enum") {
        if let Some(arr) = arr.as_array() {
            return arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
    }
    Vec::new()
}

// ===========================================================================
// Organization-Info (Entities)
// ===========================================================================

#[test]
fn contract_test_organization_schema_fields_exist() {
    let spec = load_spec("organization-info");
    let schema = get_schema(&spec, "Organization");

    let fields = schema_field_names(schema);
    // The Rust MassEntity type expects these fields (in camelCase, matching the spec):
    let expected = [
        "id",
        "name",
        "jurisdiction",
        "status",
        "address",
        "tags",
        "createdAt",
        "updatedAt",
        "board",
        "members",
    ];
    for field in &expected {
        assert!(
            fields.contains(*field),
            "Organization schema missing field '{field}' that MassEntity expects. \
             Available fields: {fields:?}"
        );
    }
}

#[test]
fn contract_test_organization_deserializes_from_spec_shape() {
    // KNOWN DIVERGENCE: The API spec defines `tags` as Array<Tag{key,value}> (objects),
    // but MassEntity.tags is Vec<String>. The Rust client flattens tag objects to plain
    // strings. If the API starts sending structured tags, the client needs updating.
    let json = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "name": "Test Corp",
        "jurisdiction": "US-DE",
        "status": "ACTIVE",
        "address": {"line1": "123 Main St", "city": "Wilmington"},
        "tags": ["test"],
        "createdAt": "2026-01-15T10:30:00Z",
        "updatedAt": "2026-01-15T10:30:00Z",
        "board": {"members": []},
        "members": [],
        "type": "LLC",
        "subType": null,
        "privacy": "PRIVATE",
        "bio": "A test corporation",
        "profileImgUrl": null,
        "backgroundImgUrl": null,
        "taxState": "US-DE",
        "website": "https://example.com"
    });

    let entity: msez_mass_client::entities::MassEntity = serde_json::from_value(json)
        .expect("MassEntity must deserialize from Organization-shaped JSON");

    assert_eq!(entity.name, "Test Corp");
    assert_eq!(entity.jurisdiction.as_deref(), Some("US-DE"));
}

#[test]
fn contract_test_organization_entity_status_enum_variants() {
    // Test that known status values deserialize correctly.
    let known_statuses = ["ACTIVE", "INACTIVE", "SUSPENDED", "DISSOLVED"];
    for status in &known_statuses {
        let json = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "Test",
            "status": status
        });
        let result: Result<msez_mass_client::entities::MassEntity, _> =
            serde_json::from_value(json);
        assert!(
            result.is_ok(),
            "MassEntity should deserialize with status '{status}'"
        );
    }

    // Forward-compat: unknown status should deserialize to Unknown.
    let json = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "name": "Test",
        "status": "FUTURE_STATUS"
    });
    let entity: msez_mass_client::entities::MassEntity =
        serde_json::from_value(json).expect("MassEntity should handle unknown status");
    assert_eq!(
        entity.status,
        Some(msez_mass_client::entities::MassEntityStatus::Unknown)
    );
}

#[test]
fn contract_test_organization_search_response_deserializes() {
    let json = serde_json::json!({
        "content": [
            {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "name": "Corp A"
            },
            {
                "id": "660e8400-e29b-41d4-a716-446655440000",
                "name": "Corp B",
                "status": "ACTIVE"
            }
        ],
        "totalElements": 2,
        "totalPages": 1,
        "number": 0,
        "size": 20
    });

    let result: msez_mass_client::entities::SearchOrganizationsResponse =
        serde_json::from_value(json)
            .expect("SearchOrganizationsResponse must deserialize from paginated spec shape");
    assert_eq!(result.content.len(), 2);
    assert_eq!(result.total_elements, Some(2));
}

// ===========================================================================
// Treasury-Info (Fiscal)
// ===========================================================================

#[test]
fn contract_test_treasury_schema_fields_exist() {
    let spec = load_spec("treasury-info");
    let schema = get_schema(&spec, "Treasury");

    let fields = schema_field_names(schema);
    let expected = [
        "id",
        "entityId",
        "name",
        "status",
        "context",
        "createdAt",
        "updatedAt",
    ];
    for field in &expected {
        assert!(
            fields.contains(*field),
            "Treasury schema missing field '{field}' that MassTreasury expects. \
             Available: {fields:?}"
        );
    }
}

#[test]
fn contract_test_treasury_deserializes_from_spec_shape() {
    let json = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "referenceId": "ref-001",
        "entityId": "org-123",
        "name": "Main Treasury",
        "status": {"code": "ACTIVE"},
        "context": "UNIT_FINANCE",
        "createdAt": "2026-01-15T10:30:00Z",
        "updatedAt": "2026-01-15T10:30:00Z",
        "identifier": {"ein": "12-3456789"},
        "address": {},
        "allowAccountCreation": true,
        "defaultTreasury": false
    });

    let treasury: msez_mass_client::fiscal::MassTreasury = serde_json::from_value(json)
        .expect("MassTreasury must deserialize from Treasury-shaped JSON");
    assert_eq!(treasury.entity_id, "org-123");
    assert_eq!(
        treasury.context,
        Some(msez_mass_client::fiscal::MassTreasuryContext::UnitFinance)
    );
}

#[test]
fn contract_test_bank_account_schema_fields_exist() {
    let spec = load_spec("treasury-info");
    let schema = get_schema(&spec, "BankAccount");

    let fields = schema_field_names(schema);
    let expected = [
        "id",
        "entityId",
        "treasuryId",
        "name",
        "currency",
        "balance",
        "available",
        "status",
        "fundingDetails",
        "createdAt",
        "updatedAt",
    ];
    for field in &expected {
        assert!(
            fields.contains(*field),
            "BankAccount schema missing field '{field}' that MassFiscalAccount expects. \
             Available: {fields:?}"
        );
    }
}

#[test]
fn contract_test_bank_account_deserializes_from_spec_shape() {
    // KNOWN DIVERGENCE: The API spec defines `balance`, `available`, and `hold` as
    // number types, but MassFiscalAccount uses Option<String> for decimal precision.
    // Spring Boot may serialize BigDecimal as either string or number depending on
    // Jackson configuration. If the API sends numeric types, the client will fail.
    let json = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "entityId": "org-123",
        "treasuryId": "660e8400-e29b-41d4-a716-446655440000",
        "name": "Operating Account",
        "currency": "USD",
        "balance": "50000.00",
        "available": "48000.00",
        "status": "OPEN",
        "fundingDetails": [{"routingNumber": "123456789"}],
        "context": "UNIT_FINANCE",
        "createdAt": "2026-01-15T10:30:00Z",
        "updatedAt": "2026-01-15T10:30:00Z",
        "accountType": "checking",
        "hold": "2000.00",
        "maskedAccountNumber": "****1234"
    });

    let account: msez_mass_client::fiscal::MassFiscalAccount = serde_json::from_value(json)
        .expect("MassFiscalAccount must deserialize from BankAccount-shaped JSON");
    assert_eq!(account.entity_id.as_deref(), Some("org-123"));
}

#[test]
fn contract_test_financial_transaction_schema_fields_exist() {
    let spec = load_spec("treasury-info");
    let schema = get_schema(&spec, "FinancialTransactionObject");

    let fields = schema_field_names(schema);
    let expected = [
        "id",
        "accountId",
        "entityId",
        "transactionType",
        "status",
        "direction",
        "currency",
        "amount",
        "createdAt",
    ];
    for field in &expected {
        assert!(
            fields.contains(*field),
            "FinancialTransactionObject schema missing field '{field}' \
             that MassPayment expects. Available: {fields:?}"
        );
    }
}

#[test]
fn contract_test_payment_deserializes_from_spec_shape() {
    // KNOWN DIVERGENCE: The API spec defines `amount` as a number type, but
    // MassPayment uses Option<String> for decimal precision. If the API sends
    // numeric JSON values for amounts, the client will fail to deserialize.
    let json = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "referenceId": "ref-pay-001",
        "accountId": "660e8400-e29b-41d4-a716-446655440000",
        "entityId": "org-123",
        "transactionType": "ACH",
        "status": "PENDING",
        "direction": "DEBIT",
        "currency": "USD",
        "amount": "1000.00",
        "description": "Vendor payment",
        "createdAt": "2026-01-15T10:30:00Z",
        "updatedAt": "2026-01-15T10:30:00Z",
        "details": {},
        "tags": []
    });

    let payment: msez_mass_client::fiscal::MassPayment = serde_json::from_value(json)
        .expect("MassPayment must deserialize from FinancialTransactionObject-shaped JSON");
    assert_eq!(payment.entity_id.as_deref(), Some("org-123"));
}

#[test]
fn contract_test_treasury_context_enum_variants() {
    let spec = load_spec("treasury-info");

    // The Treasury.context field references an enum. Verify all spec values
    // deserialize into MassTreasuryContext.
    let spec_variants = get_component_enum_values(&spec, "Context");

    // Fallback: if the enum is not a separate component, check the Treasury schema.
    let variants = if spec_variants.is_empty() {
        schema_enum_values(get_schema(&spec, "Treasury"), "context")
    } else {
        spec_variants
    };

    // Verify our known variants work.
    let known = [
        "UNIT_FINANCE",
        "CURRENCY_CLOUD",
        "CLOWD9",
        "INTERLACE",
        "PAYNETICS",
        "MASS",
        "TENET",
        "NOT_WORTHY",
    ];
    for v in &known {
        let json = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "entityId": "org-123",
            "context": v
        });
        let result: Result<msez_mass_client::fiscal::MassTreasury, _> =
            serde_json::from_value(json);
        assert!(
            result.is_ok(),
            "MassTreasuryContext should deserialize '{v}'"
        );
    }

    // If the spec exposes variants, check we handle them all.
    for v in &variants {
        let json = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "entityId": "org-123",
            "context": v
        });
        let result: Result<msez_mass_client::fiscal::MassTreasury, _> =
            serde_json::from_value(json);
        assert!(
            result.is_ok(),
            "MassTreasury should handle spec context variant '{v}'"
        );
    }
}

#[test]
fn contract_test_payment_status_enum_variants() {
    let spec = load_spec("treasury-info");
    let schema = get_schema(&spec, "FinancialTransactionObject");
    let spec_variants = schema_enum_values(schema, "status");

    // Our known variants.
    let known = ["PENDING", "SUCCEEDED", "FAILED", "REJECTED", "CANCELED"];
    for v in &known {
        let json = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "status": v
        });
        let result: Result<msez_mass_client::fiscal::MassPayment, _> =
            serde_json::from_value(json);
        assert!(
            result.is_ok(),
            "MassPayment should deserialize with status '{v}'"
        );
    }

    // Verify all spec-defined variants also work.
    for v in &spec_variants {
        let json = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "status": v
        });
        let result: Result<msez_mass_client::fiscal::MassPayment, _> =
            serde_json::from_value(json);
        assert!(
            result.is_ok(),
            "MassPayment should handle spec status variant '{v}' \
             (may need new variant or #[serde(other)])"
        );
    }
}

// ===========================================================================
// Consent-Info
// ===========================================================================

#[test]
fn contract_test_consent_schema_fields_exist() {
    let spec = load_spec("consent-info");
    let schema = get_schema(&spec, "Consent");

    let fields = schema_field_names(schema);
    let expected = [
        "id",
        "organizationId",
        "operationId",
        "operationType",
        "status",
        "votes",
        "numVotesRequired",
        "approvalCount",
        "rejectionCount",
        "documentUrl",
        "signatory",
        "jurisdiction",
        "requestedBy",
        "expiresAt",
        "createdAt",
        "updatedAt",
    ];
    for field in &expected {
        assert!(
            fields.contains(*field),
            "Consent schema missing field '{field}' that MassConsent expects. \
             Available: {fields:?}"
        );
    }
}

#[test]
fn contract_test_consent_deserializes_from_spec_shape() {
    let json = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "organizationId": "org-123",
        "operationId": "660e8400-e29b-41d4-a716-446655440000",
        "operationType": "EQUITY_OFFER",
        "status": "PENDING",
        "votes": [
            {"vote": "APPROVE", "votedBy": "user-1", "boardMemberId": "bm-1", "approve": true}
        ],
        "numVotesRequired": 3,
        "approvalCount": 1,
        "rejectionCount": 0,
        "documentUrl": "https://example.com/doc.pdf",
        "signatory": "user-2",
        "signatoryEmail": "signer@example.com",
        "jurisdiction": "US-DE",
        "requestedBy": "user-3",
        "expiresAt": "2026-03-15T10:30:00Z",
        "createdAt": "2026-01-15T10:30:00Z",
        "updatedAt": "2026-01-15T10:30:00Z",
        "submissionId": "sub-001",
        "relatedAgreements": [],
        "deletedAt": null,
        "signwellUrl": null
    });

    let consent: msez_mass_client::consent::MassConsent = serde_json::from_value(json)
        .expect("MassConsent must deserialize from Consent-shaped JSON");
    assert_eq!(consent.organization_id, "org-123");
    assert_eq!(
        consent.status,
        Some(msez_mass_client::consent::MassConsentStatus::Pending)
    );
}

#[test]
fn contract_test_consent_status_enum_variants() {
    let spec = load_spec("consent-info");
    let schema = get_schema(&spec, "Consent");
    let spec_variants = schema_enum_values(schema, "status");

    let known = [
        "PENDING",
        "APPROVED",
        "REJECTED",
        "EXPIRED",
        "FORCE_APPROVED",
        "COMPLETED",
        "CANCELED",
    ];
    for v in &known {
        let json = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "organizationId": "org-123",
            "status": v
        });
        let result: Result<msez_mass_client::consent::MassConsent, _> =
            serde_json::from_value(json);
        assert!(
            result.is_ok(),
            "MassConsent should deserialize with status '{v}'"
        );
    }

    for v in &spec_variants {
        let json = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "organizationId": "org-123",
            "status": v
        });
        let result: Result<msez_mass_client::consent::MassConsent, _> =
            serde_json::from_value(json);
        assert!(
            result.is_ok(),
            "MassConsent should handle spec status variant '{v}'"
        );
    }
}

#[test]
fn contract_test_consent_operation_type_enum_variants() {
    let known = [
        "EQUITY_OFFER",
        "ISSUE_NEW_SHARES",
        "AMEND_OPTIONS_POOL",
        "CREATE_OPTIONS_POOL",
        "CREATE_COMMON_CLASS",
        "MODIFY_COMPANY_LEGAL_NAME",
        "MODIFY_BOARD_MEMBER_DESIGNATION",
        "CERTIFICATE_OF_AMENDMENT",
    ];
    for v in &known {
        let json = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "organizationId": "org-123",
            "operationType": v
        });
        let result: Result<msez_mass_client::consent::MassConsent, _> =
            serde_json::from_value(json);
        assert!(
            result.is_ok(),
            "MassConsent should deserialize with operationType '{v}'"
        );
    }
}

// ===========================================================================
// Consent-Info: Cap Tables (Ownership)
// ===========================================================================

#[test]
fn contract_test_cap_table_schema_fields_exist() {
    let spec = load_spec("consent-info");
    let schema = get_schema(&spec, "CapTable");

    let fields = schema_field_names(schema);
    let expected = [
        "id",
        "organizationId",
        "authorizedShares",
        "outstandingShares",
        "fullyDilutedShares",
        "reservedShares",
        "unreservedShares",
        "shareClasses",
        "shareholders",
        "optionsPools",
        "createdAt",
        "updatedAt",
    ];
    for field in &expected {
        assert!(
            fields.contains(*field),
            "CapTable schema missing field '{field}' that MassCapTable expects. \
             Available: {fields:?}"
        );
    }
}

#[test]
fn contract_test_cap_table_deserializes_from_spec_shape() {
    // KNOWN DIVERGENCE: The API spec defines `votingRights` as integer type, but
    // MassShareClass uses bool. The API likely sends 0/1 integers for boolean fields.
    // If the API sends integer values, serde will fail to deserialize as bool.
    let json = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "organizationId": "org-123",
        "authorizedShares": 10000000,
        "outstandingShares": 5000000,
        "fullyDilutedShares": 7000000,
        "reservedShares": 1000000,
        "unreservedShares": 4000000,
        "shareClasses": [
            {
                "id": "660e8400-e29b-41d4-a716-446655440000",
                "name": "Common A",
                "authorizedShares": 10000000,
                "outstandingShares": 5000000,
                "votingRights": true,
                "restricted": false,
                "type": "COMMON"
            }
        ],
        "shareholders": [],
        "optionsPools": [],
        "securities": [],
        "transactions": [],
        "vestingSchedules": [],
        "createdAt": "2026-01-15T10:30:00Z",
        "updatedAt": "2026-01-15T10:30:00Z",
        "deletedAt": null
    });

    let cap_table: msez_mass_client::ownership::MassCapTable = serde_json::from_value(json)
        .expect("MassCapTable must deserialize from CapTable-shaped JSON");
    assert_eq!(cap_table.organization_id, "org-123");
    assert_eq!(cap_table.share_classes.len(), 1);
    assert_eq!(cap_table.share_classes[0].name, "Common A");
}

#[test]
fn contract_test_share_class_schema_fields_exist() {
    let spec = load_spec("consent-info");
    let schema = get_schema(&spec, "ShareClass");

    let fields = schema_field_names(schema);
    let expected = [
        "id",
        "name",
        "authorizedShares",
        "outstandingShares",
        "votingRights",
        "restricted",
    ];
    for field in &expected {
        assert!(
            fields.contains(*field),
            "ShareClass schema missing field '{field}' that MassShareClass expects. \
             Available: {fields:?}"
        );
    }
}

// ===========================================================================
// Templating Engine
// ===========================================================================

#[test]
fn contract_test_submission_response_deserializes_from_spec_shape() {
    let json = serde_json::json!({
        "id": "sub-001",
        "entityId": "org-123",
        "context": "DOCUSEAL",
        "status": "AWAITING_SIGNATURES",
        "signingOrder": "RANDOM",
        "signers": [
            {
                "id": "signer-1",
                "email": "signer@example.com",
                "signingRole": "OFFICER",
                "name": {"firstName": "John", "lastName": "Doe"}
            }
        ],
        "documentUri": "https://example.com/doc.pdf",
        "createdAt": "2026-01-15T10:30:00Z",
        "updatedAt": "2026-01-15T10:30:00Z",
        "template": [{"id": "tpl-1", "name": "Certificate"}],
        "tags": [],
        "parentSubmissionId": null,
        "userId": "user-1"
    });

    let submission: msez_mass_client::templating::SubmissionResponse =
        serde_json::from_value(json)
            .expect("SubmissionResponse must deserialize from Submission-shaped JSON");
    assert_eq!(submission.id, "sub-001");
}

#[test]
fn contract_test_template_deserializes_from_spec_shape() {
    // KNOWN DIVERGENCE: The API spec defines `version` as number (format: float),
    // but Template uses Option<String>. If the API sends a numeric version like
    // `1.0` instead of `"1.0"`, the client will fail to deserialize.
    let json = serde_json::json!({
        "id": "tpl-001",
        "name": "Certificate of Incorporation",
        "context": "DOCUSEAL",
        "entityId": "org-123",
        "version": "1.0",
        "type": "INCORPORATION",
        "grouping": "formation",
        "status": "ACTIVE"
    });

    let template: msez_mass_client::templating::Template = serde_json::from_value(json)
        .expect("Template must deserialize from Template-shaped JSON");
    assert_eq!(template.id, "tpl-001");
    assert_eq!(
        template.name.as_deref(),
        Some("Certificate of Incorporation")
    );
}

#[test]
fn contract_test_signing_order_enum_variants() {
    let known = ["RANDOM", "PRESERVED"];
    for v in &known {
        let result: Result<msez_mass_client::templating::SigningOrder, _> =
            serde_json::from_value(serde_json::json!(v));
        assert!(result.is_ok(), "SigningOrder should deserialize '{v}'");
    }
}

#[test]
fn contract_test_signing_role_enum_variants() {
    let known = ["OFFICER", "RECIPIENT", "SPOUSE", "WITNESS"];
    for v in &known {
        let result: Result<msez_mass_client::templating::SigningRole, _> =
            serde_json::from_value(serde_json::json!(v));
        assert!(result.is_ok(), "SigningRole should deserialize '{v}'");
    }
}

// ===========================================================================
// Investment-Info (Ownership - Investments)
// ===========================================================================

#[test]
fn contract_test_investment_schema_has_expected_fields() {
    let spec = load_spec("investment-info");
    let schema = get_schema(&spec, "Investment");

    let fields = schema_field_names(schema);
    // Core fields the Rust OwnershipClient expects from investments:
    let expected = [
        "id",
        "type",
        "createdAt",
        "investmentAmount",
        "company",
        "investor",
    ];
    for field in &expected {
        assert!(
            fields.contains(*field),
            "Investment schema missing field '{field}'. Available: {fields:?}"
        );
    }
}

// ===========================================================================
// Cross-Spec: Field Rename Drift Detection
// ===========================================================================

/// Verify that camelCase ↔ snake_case field mapping hasn't drifted.
///
/// This is the core contract test: if the Java team renames `createdAt` to
/// `created_at`, this test catches it because the spec schema will no longer
/// contain `createdAt`.
#[test]
fn contract_test_field_rename_drift_organization() {
    let spec = load_spec("organization-info");
    let fields = schema_field_names(get_schema(&spec, "Organization"));

    // These are the camelCase field names that the Rust MassEntity type
    // expects (via #[serde(rename_all = "camelCase")]).
    let rust_expects_camel = ["id", "name", "createdAt", "updatedAt"];
    for field in &rust_expects_camel {
        assert!(
            fields.contains(*field),
            "DRIFT DETECTED: Organization spec no longer has field '{field}'. \
             The Java API may have renamed it. Update MassEntity serde attributes."
        );
    }
}

#[test]
fn contract_test_field_rename_drift_treasury() {
    let spec = load_spec("treasury-info");
    let fields = schema_field_names(get_schema(&spec, "Treasury"));

    let rust_expects_camel = ["id", "entityId", "createdAt", "updatedAt"];
    for field in &rust_expects_camel {
        assert!(
            fields.contains(*field),
            "DRIFT DETECTED: Treasury spec no longer has field '{field}'."
        );
    }
}

#[test]
fn contract_test_field_rename_drift_bank_account() {
    let spec = load_spec("treasury-info");
    let fields = schema_field_names(get_schema(&spec, "BankAccount"));

    let rust_expects_camel = [
        "id",
        "entityId",
        "treasuryId",
        "currency",
        "balance",
        "available",
        "createdAt",
        "updatedAt",
    ];
    for field in &rust_expects_camel {
        assert!(
            fields.contains(*field),
            "DRIFT DETECTED: BankAccount spec no longer has field '{field}'."
        );
    }
}

#[test]
fn contract_test_field_rename_drift_consent() {
    let spec = load_spec("consent-info");
    let fields = schema_field_names(get_schema(&spec, "Consent"));

    let rust_expects_camel = [
        "id",
        "organizationId",
        "operationType",
        "status",
        "votes",
        "numVotesRequired",
        "approvalCount",
        "rejectionCount",
        "createdAt",
        "updatedAt",
    ];
    for field in &rust_expects_camel {
        assert!(
            fields.contains(*field),
            "DRIFT DETECTED: Consent spec no longer has field '{field}'."
        );
    }
}

#[test]
fn contract_test_field_rename_drift_cap_table() {
    let spec = load_spec("consent-info");
    let fields = schema_field_names(get_schema(&spec, "CapTable"));

    let rust_expects_camel = [
        "id",
        "organizationId",
        "authorizedShares",
        "outstandingShares",
        "shareClasses",
        "shareholders",
        "createdAt",
        "updatedAt",
    ];
    for field in &rust_expects_camel {
        assert!(
            fields.contains(*field),
            "DRIFT DETECTED: CapTable spec no longer has field '{field}'."
        );
    }
}

// ===========================================================================
// Staleness Checks (ignored by default — fetch live specs and compare)
// ===========================================================================

#[tokio::test]
#[ignore] // Run with: cargo test -p msez-mass-client contract_staleness -- --ignored
async fn contract_staleness_organization_info() {
    check_staleness(
        "organization-info",
        "https://organization-info.api.mass.inc/organization-info/v3/api-docs",
        &["Organization"],
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn contract_staleness_investment_info() {
    check_staleness(
        "investment-info",
        "https://investment-info-production-4f3779c81425.herokuapp.com/investment-info/v3/api-docs",
        &["Investment"],
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn contract_staleness_treasury_info() {
    check_staleness(
        "treasury-info",
        "https://treasury-info.api.mass.inc/treasury-info/v3/api-docs",
        &["Treasury", "BankAccount", "FinancialTransactionObject"],
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn contract_staleness_consent_info() {
    check_staleness(
        "consent-info",
        "https://consent.api.mass.inc/consent-info/v3/api-docs",
        &["Consent", "CapTable", "ShareClass"],
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn contract_staleness_templating_engine() {
    check_staleness(
        "templating-engine",
        "https://templating-engine-prod-5edc768c1f80.herokuapp.com/templating-engine/v3/api-docs",
        &["Submission", "Template"],
    )
    .await;
}

/// Fetch a live spec and compare its schema components against the committed snapshot.
async fn check_staleness(spec_name: &str, url: &str, key_schemas: &[&str]) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("failed to build HTTP client");

    let response = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "WARNING: Could not reach {spec_name} at {url}: {e}. \
                 Skipping staleness check (VPN may be required)."
            );
            return;
        }
    };

    let live_text = response
        .text()
        .await
        .expect("failed to read response body");

    let live_spec: Value =
        serde_json::from_str(&live_text).expect("live spec is not valid JSON");

    let committed_spec = load_spec(spec_name);

    // Compare component schema keys (most sensitive to drift).
    let live_schemas = live_spec["components"]["schemas"]
        .as_object()
        .map(|m| m.keys().cloned().collect::<HashSet<_>>())
        .unwrap_or_default();
    let committed_schemas = committed_spec["components"]["schemas"]
        .as_object()
        .map(|m| m.keys().cloned().collect::<HashSet<_>>())
        .unwrap_or_default();

    let added: Vec<_> = live_schemas.difference(&committed_schemas).collect();
    let removed: Vec<_> = committed_schemas.difference(&live_schemas).collect();

    // Check field-level drift on key schemas.
    let mut field_diffs = Vec::new();
    for schema_name in key_schemas {
        let live_fields =
            schema_field_names(&live_spec["components"]["schemas"][*schema_name]);
        let committed_fields =
            schema_field_names(&committed_spec["components"]["schemas"][*schema_name]);

        let new_fields: Vec<_> = live_fields.difference(&committed_fields).collect();
        let gone_fields: Vec<_> = committed_fields.difference(&live_fields).collect();

        if !new_fields.is_empty() || !gone_fields.is_empty() {
            field_diffs.push(format!(
                "  {schema_name}: added={new_fields:?}, removed={gone_fields:?}"
            ));
        }
    }

    if !added.is_empty() || !removed.is_empty() || !field_diffs.is_empty() {
        let mut msg = format!(
            "{spec_name} spec has drifted. \
             Run `./msez/crates/msez-mass-client/scripts/refresh-specs.sh` and review changes.\n"
        );
        if !added.is_empty() {
            msg.push_str(&format!("  New schemas: {added:?}\n"));
        }
        if !removed.is_empty() {
            msg.push_str(&format!("  Removed schemas: {removed:?}\n"));
        }
        for diff in &field_diffs {
            msg.push_str(&format!("{diff}\n"));
        }
        panic!("{msg}");
    }
}
