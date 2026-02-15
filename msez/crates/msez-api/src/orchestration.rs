//! # Mass API Orchestration Logic
//!
//! Shared orchestration functions that transform passthrough mass proxy
//! handlers into jurisdiction-aware orchestration endpoints. Each write
//! operation through the Mass APIs is wrapped with:
//!
//! 1. **Pre-flight compliance evaluation** — build a compliance tensor for
//!    the target jurisdiction, evaluate all 20 domains, and reject if any
//!    hard-block domain (Sanctions) is `NonCompliant`.
//!
//! 2. **Mass API delegation** — the primitive operation proceeds via
//!    `msez-mass-client` (the sole authorized gateway to Mass APIs).
//!
//! 3. **Post-operation credential issuance** — issue a W3C Verifiable
//!    Credential attesting to the compliance evaluation at the time of
//!    the operation.
//!
//! 4. **Attestation storage** — persist an attestation record for regulator
//!    queries.
//!
//! ## Architecture
//!
//! This module is the SEZ Stack's core value-add. Without it, Mass is a
//! generic formation/payment API. With it, Mass knows about jurisdictional
//! tax law, sanctions, and compliance.
//!
//! ## Hard-Block vs. Soft-Warning
//!
//! - **Hard block**: Only `Sanctions` domain `NonCompliant` blocks the
//!   operation (legal requirement).
//! - **Soft warning**: All other non-compliant or pending domains are
//!   included in the response as warnings but do not block. This allows
//!   entities to be formed and then brought into compliance iteratively.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use msez_core::ComplianceDomain;
use msez_tensor::{ComplianceState, ComplianceTensor, DefaultJurisdiction, JurisdictionConfig};
use msez_vc::credential::{ContextValue, CredentialTypeValue, ProofValue, VerifiableCredential};
use msez_vc::proof::ProofType;

use crate::state::{AppState, AttestationRecord, AttestationStatus};

// ---------------------------------------------------------------------------
// Compliance evaluation types
// ---------------------------------------------------------------------------

/// Compliance evaluation summary included in orchestration responses.
///
/// Captures the full 20-domain tensor evaluation result for the jurisdiction.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComplianceSummary {
    /// Jurisdiction evaluated against.
    pub jurisdiction_id: String,
    /// Aggregate compliance status across all 20 domains.
    pub overall_status: String,
    /// Per-domain compliance status.
    pub domain_results: HashMap<String, String>,
    /// Domains that are in a passing state (compliant, exempt, not_applicable).
    pub passing_domains: Vec<String>,
    /// Domains that are blocking (non_compliant or pending).
    pub blocking_domains: Vec<String>,
    /// Domains that are hard-blocking (non_compliant on critical domains like sanctions).
    pub hard_blocks: Vec<String>,
    /// SHA-256 tensor commitment digest (hex).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tensor_commitment: Option<String>,
    /// When the evaluation was performed.
    pub evaluated_at: DateTime<Utc>,
}

/// Enriched response envelope for orchestration endpoints.
///
/// Wraps the Mass API response with compliance evaluation and VC data.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrchestrationEnvelope {
    /// The Mass API response data for the primitive operation.
    pub mass_response: serde_json::Value,
    /// Compliance evaluation summary (20-domain tensor).
    pub compliance: ComplianceSummary,
    /// The issued Verifiable Credential (if compliance allows).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<serde_json::Value>,
    /// ID of the stored attestation record (for regulator queries).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation_id: Option<Uuid>,
}

// ---------------------------------------------------------------------------
// Domains applicable to each primitive
// ---------------------------------------------------------------------------

/// Compliance domains relevant to entity formation.
const ENTITY_DOMAINS: &[ComplianceDomain] = &[
    ComplianceDomain::Aml,
    ComplianceDomain::Kyc,
    ComplianceDomain::Sanctions,
    ComplianceDomain::Tax,
    ComplianceDomain::Corporate,
    ComplianceDomain::Licensing,
];

/// Compliance domains relevant to ownership/cap table operations.
const OWNERSHIP_DOMAINS: &[ComplianceDomain] = &[
    ComplianceDomain::Aml,
    ComplianceDomain::Kyc,
    ComplianceDomain::Sanctions,
    ComplianceDomain::Securities,
    ComplianceDomain::Corporate,
];

/// Compliance domains relevant to fiscal account creation.
const FISCAL_ACCOUNT_DOMAINS: &[ComplianceDomain] = &[
    ComplianceDomain::Aml,
    ComplianceDomain::Kyc,
    ComplianceDomain::Sanctions,
    ComplianceDomain::Tax,
    ComplianceDomain::Banking,
];

/// Compliance domains relevant to payment initiation.
const PAYMENT_DOMAINS: &[ComplianceDomain] = &[
    ComplianceDomain::Aml,
    ComplianceDomain::Sanctions,
    ComplianceDomain::Tax,
    ComplianceDomain::Payments,
    ComplianceDomain::Banking,
];

/// Compliance domains relevant to identity verification.
const IDENTITY_DOMAINS: &[ComplianceDomain] = &[
    ComplianceDomain::Kyc,
    ComplianceDomain::Sanctions,
    ComplianceDomain::DataPrivacy,
];

/// Compliance domains relevant to consent/governance workflows.
const CONSENT_DOMAINS: &[ComplianceDomain] = &[
    ComplianceDomain::Aml,
    ComplianceDomain::Sanctions,
    ComplianceDomain::Corporate,
];

/// Domains where `NonCompliant` is a hard block (legal requirement).
const HARD_BLOCK_DOMAINS: &[ComplianceDomain] = &[ComplianceDomain::Sanctions];

// ---------------------------------------------------------------------------
// Compliance evaluation
// ---------------------------------------------------------------------------

/// Build a compliance tensor for a jurisdiction and evaluate all 20 domains.
///
/// Returns the tensor and a compliance summary. Falls back to `"UNKNOWN"`
/// jurisdiction if the input fails validation.
pub fn evaluate_compliance(
    jurisdiction_id: &str,
    entity_id: &str,
    relevant_domains: &[ComplianceDomain],
) -> (ComplianceTensor<DefaultJurisdiction>, ComplianceSummary) {
    let tensor = crate::compliance::build_tensor(jurisdiction_id);

    // Evaluate all domains for this entity.
    let all_results = tensor.evaluate_all(entity_id);

    let mut domain_results = HashMap::new();
    let mut passing_domains = Vec::new();
    let mut blocking_domains = Vec::new();
    let mut hard_blocks = Vec::new();

    // Report on the relevant domains for this operation.
    for &domain in relevant_domains {
        let state = all_results
            .get(&domain)
            .copied()
            .unwrap_or(ComplianceState::Pending);

        let state_str = format!("{state}");
        domain_results.insert(domain.as_str().to_string(), state_str);

        if state.is_passing() {
            passing_domains.push(domain.as_str().to_string());
        } else {
            blocking_domains.push(domain.as_str().to_string());
            if HARD_BLOCK_DOMAINS.contains(&domain) && state == ComplianceState::NonCompliant {
                hard_blocks.push(domain.as_str().to_string());
            }
        }
    }

    // Also include non-relevant domains for completeness.
    for &domain in ComplianceDomain::all() {
        if relevant_domains.contains(&domain) {
            continue;
        }
        let state = all_results
            .get(&domain)
            .copied()
            .unwrap_or(ComplianceState::Pending);
        domain_results.insert(domain.as_str().to_string(), format!("{state}"));
    }

    passing_domains.sort();
    blocking_domains.sort();
    hard_blocks.sort();

    // Aggregate state across relevant domains only.
    let aggregate = relevant_domains
        .iter()
        .map(|d| {
            all_results
                .get(d)
                .copied()
                .unwrap_or(ComplianceState::Pending)
        })
        .fold(ComplianceState::Compliant, ComplianceState::meet);

    let tensor_commitment = tensor
        .commit()
        .map_err(|e| {
            tracing::warn!(error = %e, "tensor commitment failed — response will omit commitment");
            e
        })
        .ok()
        .map(|c| c.to_hex());

    let summary = ComplianceSummary {
        jurisdiction_id: tensor.jurisdiction().jurisdiction_id().as_str().to_string(),
        overall_status: format!("{aggregate}"),
        domain_results,
        passing_domains,
        blocking_domains,
        hard_blocks,
        tensor_commitment,
        evaluated_at: Utc::now(),
    };

    (tensor, summary)
}

/// Check if the compliance summary contains any hard blocks.
///
/// Hard blocks (e.g., sanctions match) prevent the Mass API operation
/// from proceeding. Returns `Some(reason)` if blocked, `None` if clear.
pub fn check_hard_blocks(summary: &ComplianceSummary) -> Option<String> {
    if summary.hard_blocks.is_empty() {
        return None;
    }
    Some(format!(
        "operation blocked by compliance hard-block on: {}",
        summary.hard_blocks.join(", ")
    ))
}

// ---------------------------------------------------------------------------
// VC issuance
// ---------------------------------------------------------------------------

/// VC type constants for each primitive.
pub mod vc_types {
    pub const FORMATION_COMPLIANCE: &str = "MsezFormationComplianceCredential";
    pub const OWNERSHIP_COMPLIANCE: &str = "MsezOwnershipComplianceCredential";
    pub const FISCAL_COMPLIANCE: &str = "MsezFiscalComplianceCredential";
    pub const PAYMENT_COMPLIANCE: &str = "MsezPaymentComplianceCredential";
    pub const IDENTITY_COMPLIANCE: &str = "MsezIdentityComplianceCredential";
    pub const CONSENT_COMPLIANCE: &str = "MsezConsentComplianceCredential";
}

/// Issue a compliance attestation VC for a primitive operation.
///
/// The VC attests that the compliance tensor was evaluated at the time of
/// the operation, capturing the domain states as credential subject claims.
///
/// ## Security Invariant
///
/// Uses `VerifiableCredential::sign_ed25519()` which enforces
/// canonicalization via `CanonicalBytes` — never raw serialization.
pub fn issue_compliance_vc(
    state: &AppState,
    vc_type: &str,
    jurisdiction_id: &str,
    entity_reference: &str,
    summary: &ComplianceSummary,
) -> Result<VerifiableCredential, String> {
    let subject = serde_json::json!({
        "entity_reference": entity_reference,
        "jurisdiction_id": jurisdiction_id,
        "overall_status": summary.overall_status,
        "domain_results": summary.domain_results,
        "passing_domains": summary.passing_domains,
        "blocking_domains": summary.blocking_domains,
        "tensor_commitment": summary.tensor_commitment,
        "evaluated_at": summary.evaluated_at.to_rfc3339(),
    });

    let mut vc = VerifiableCredential {
        context: ContextValue::Array(vec![serde_json::Value::String(
            "https://www.w3.org/2018/credentials/v1".to_string(),
        )]),
        id: Some(format!("urn:msez:vc:{vc_type}:{}", Uuid::new_v4())),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            vc_type.to_string(),
        ]),
        issuer: state.zone_did.clone(),
        issuance_date: Utc::now(),
        expiration_date: None,
        credential_subject: subject,
        proof: ProofValue::default(),
    };

    let verification_method = format!("{}#key-1", state.zone_did);
    vc.sign_ed25519(
        &state.zone_signing_key,
        verification_method,
        ProofType::MsezEd25519Signature2025,
        None,
    )
    .map_err(|e| format!("VC signing failed: {e}"))?;

    Ok(vc)
}

// ---------------------------------------------------------------------------
// Attestation storage
// ---------------------------------------------------------------------------

/// Store a compliance attestation record for regulator queries.
///
/// Returns the attestation UUID.
pub fn store_attestation(
    state: &AppState,
    entity_id: Uuid,
    attestation_type: &str,
    jurisdiction_id: &str,
    details: serde_json::Value,
) -> Uuid {
    let id = Uuid::new_v4();
    let record = AttestationRecord {
        id,
        entity_id,
        attestation_type: attestation_type.to_string(),
        issuer: state.zone_did.clone(),
        status: AttestationStatus::Active,
        jurisdiction_id: jurisdiction_id.to_string(),
        issued_at: Utc::now(),
        expires_at: None,
        details,
    };
    state.attestations.insert(id, record);
    id
}

// ---------------------------------------------------------------------------
// Orchestration helpers per primitive
// ---------------------------------------------------------------------------

/// Run the full orchestration pipeline for an entity creation.
///
/// 1. Evaluate compliance tensor for the entity's jurisdiction
/// 2. Check for hard blocks (sanctions)
/// 3. Issue a formation compliance VC
/// 4. Store an attestation record
pub fn orchestrate_entity_creation(
    state: &AppState,
    jurisdiction_id: &str,
    legal_name: &str,
    mass_response: serde_json::Value,
) -> OrchestrationEnvelope {
    let entity_id_str = mass_response
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let (_tensor, summary) = evaluate_compliance(jurisdiction_id, entity_id_str, ENTITY_DOMAINS);

    let (credential, attestation_id) = issue_and_store(
        state,
        vc_types::FORMATION_COMPLIANCE,
        jurisdiction_id,
        entity_id_str,
        legal_name,
        &summary,
    );

    OrchestrationEnvelope {
        mass_response,
        compliance: summary,
        credential,
        attestation_id,
    }
}

/// Run the full orchestration pipeline for an entity update.
///
/// Same pipeline as creation: compliance evaluation → VC issuance →
/// attestation storage. Updates carry the same regulatory risk as creation
/// (e.g., changing jurisdiction or beneficial owners triggers sanctions check).
pub fn orchestrate_entity_update(
    state: &AppState,
    entity_id: uuid::Uuid,
    jurisdiction_id: &str,
    mass_response: serde_json::Value,
) -> OrchestrationEnvelope {
    let entity_id_str = entity_id.to_string();

    let (_tensor, summary) = evaluate_compliance(jurisdiction_id, &entity_id_str, ENTITY_DOMAINS);

    let (credential, attestation_id) = issue_and_store(
        state,
        vc_types::FORMATION_COMPLIANCE,
        jurisdiction_id,
        &entity_id_str,
        &entity_id_str,
        &summary,
    );

    OrchestrationEnvelope {
        mass_response,
        compliance: summary,
        credential,
        attestation_id,
    }
}

/// Run the full orchestration pipeline for cap table creation.
pub fn orchestrate_cap_table_creation(
    state: &AppState,
    entity_id: uuid::Uuid,
    mass_response: serde_json::Value,
) -> OrchestrationEnvelope {
    let entity_id_str = entity_id.to_string();

    // TODO(P1-004): Fetch entity's jurisdiction from Mass organization-info
    // by entity_id. For now, ownership operations use GLOBAL-scope evaluation
    // (same as the handler's pre-flight check in mass_proxy.rs).
    let jurisdiction_id = "GLOBAL";

    let (_tensor, summary) =
        evaluate_compliance(jurisdiction_id, &entity_id_str, OWNERSHIP_DOMAINS);

    let (credential, attestation_id) = issue_and_store(
        state,
        vc_types::OWNERSHIP_COMPLIANCE,
        jurisdiction_id,
        &entity_id_str,
        &entity_id_str,
        &summary,
    );

    OrchestrationEnvelope {
        mass_response,
        compliance: summary,
        credential,
        attestation_id,
    }
}

/// Run the full orchestration pipeline for fiscal account creation.
pub fn orchestrate_account_creation(
    state: &AppState,
    entity_id: uuid::Uuid,
    currency: &str,
    mass_response: serde_json::Value,
) -> OrchestrationEnvelope {
    let entity_id_str = entity_id.to_string();

    // Infer jurisdiction from currency for fiscal operations.
    let jurisdiction_id = infer_jurisdiction_from_currency(currency);

    let (_tensor, summary) =
        evaluate_compliance(jurisdiction_id, &entity_id_str, FISCAL_ACCOUNT_DOMAINS);

    let (credential, attestation_id) = issue_and_store(
        state,
        vc_types::FISCAL_COMPLIANCE,
        jurisdiction_id,
        &entity_id_str,
        &entity_id_str,
        &summary,
    );

    OrchestrationEnvelope {
        mass_response,
        compliance: summary,
        credential,
        attestation_id,
    }
}

/// Run the full orchestration pipeline for payment initiation.
pub fn orchestrate_payment(
    state: &AppState,
    from_account_id: uuid::Uuid,
    currency: &str,
    mass_response: serde_json::Value,
) -> OrchestrationEnvelope {
    let account_id_str = from_account_id.to_string();
    let jurisdiction_id = infer_jurisdiction_from_currency(currency);

    let (_tensor, summary) = evaluate_compliance(jurisdiction_id, &account_id_str, PAYMENT_DOMAINS);

    let (credential, attestation_id) = issue_and_store(
        state,
        vc_types::PAYMENT_COMPLIANCE,
        jurisdiction_id,
        &account_id_str,
        &account_id_str,
        &summary,
    );

    OrchestrationEnvelope {
        mass_response,
        compliance: summary,
        credential,
        attestation_id,
    }
}

/// Run the full orchestration pipeline for identity verification.
pub fn orchestrate_identity_verification(
    state: &AppState,
    identity_type: &str,
    mass_response: serde_json::Value,
) -> OrchestrationEnvelope {
    let entity_ref = mass_response
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Identity verification is jurisdiction-agnostic at this layer.
    let jurisdiction_id = "GLOBAL";

    let (_tensor, summary) = evaluate_compliance(jurisdiction_id, entity_ref, IDENTITY_DOMAINS);

    let (credential, attestation_id) = issue_and_store(
        state,
        vc_types::IDENTITY_COMPLIANCE,
        jurisdiction_id,
        entity_ref,
        identity_type,
        &summary,
    );

    OrchestrationEnvelope {
        mass_response,
        compliance: summary,
        credential,
        attestation_id,
    }
}

/// Run the full orchestration pipeline for consent request creation.
pub fn orchestrate_consent_creation(
    state: &AppState,
    consent_type: &str,
    mass_response: serde_json::Value,
) -> OrchestrationEnvelope {
    let consent_ref = mass_response
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let jurisdiction_id = "GLOBAL";

    let (_tensor, summary) = evaluate_compliance(jurisdiction_id, consent_ref, CONSENT_DOMAINS);

    let (credential, attestation_id) = issue_and_store(
        state,
        vc_types::CONSENT_COMPLIANCE,
        jurisdiction_id,
        consent_ref,
        consent_type,
        &summary,
    );

    OrchestrationEnvelope {
        mass_response,
        compliance: summary,
        credential,
        attestation_id,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Issue a VC and store an attestation record. Returns both or logs warnings
/// on failure (graceful degradation — never blocks the Mass API response).
fn issue_and_store(
    state: &AppState,
    vc_type: &str,
    jurisdiction_id: &str,
    entity_reference: &str,
    description: &str,
    summary: &ComplianceSummary,
) -> (Option<serde_json::Value>, Option<Uuid>) {
    let credential = match issue_compliance_vc(
        state,
        vc_type,
        jurisdiction_id,
        entity_reference,
        summary,
    ) {
        Ok(vc) => match serde_json::to_value(&vc) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::warn!(error = %e, "failed to serialize compliance VC");
                None
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "failed to issue compliance VC — response will omit credential");
            None
        }
    };

    let entity_uuid = Uuid::parse_str(entity_reference).unwrap_or_else(|_| Uuid::new_v4());

    let attestation_id = store_attestation(
        state,
        entity_uuid,
        &format!("{vc_type}:{description}"),
        jurisdiction_id,
        serde_json::json!({
            "operation": vc_type,
            "overall_status": summary.overall_status,
            "blocking_domains": summary.blocking_domains,
        }),
    );

    (credential, Some(attestation_id))
}

/// Infer a jurisdiction from a currency code.
///
/// This is a heuristic for fiscal operations where the request doesn't
/// carry an explicit jurisdiction. In production, the entity's registered
/// jurisdiction should be fetched from Mass.
///
/// Coverage: all current and planned deployment corridors — USA, BVI,
/// Cayman, UAE, Pakistan, China (Hainan), Seychelles, Kazakhstan, KSA,
/// Hong Kong, and common international currencies.
pub fn infer_jurisdiction(currency: &str) -> &str {
    infer_jurisdiction_from_currency(currency)
}

fn infer_jurisdiction_from_currency(currency: &str) -> &str {
    match currency.to_uppercase().as_str() {
        // Active deployment targets
        "PKR" => "PK",   // Pakistan
        "AED" => "AE",   // UAE (ADGM, DIFC, 27 free zones)
        "USD" => "US",   // United States (also used in BVI)
        "GBP" => "GB",   // United Kingdom
        "EUR" => "EU",   // European Union
        "SGD" => "SG",   // Singapore
        "CNY" | "RMB" => "CN", // China (Hainan SEZ)
        "SAR" => "SA",   // Saudi Arabia (PAK↔KSA corridor)
        "KYD" => "KY",   // Cayman Islands
        "SCR" => "SC",   // Seychelles
        "KZT" => "KZ",   // Kazakhstan (Alatau City / AIFC)
        "HKD" => "HK",   // Hong Kong
        // Additional corridors and common currencies
        "BHD" => "BH",   // Bahrain
        "OMR" => "OM",   // Oman
        "QAR" => "QA",   // Qatar
        "KWD" => "KW",   // Kuwait
        "INR" => "IN",   // India
        "JPY" => "JP",   // Japan
        "CHF" => "CH",   // Switzerland
        "CAD" => "CA",   // Canada
        "AUD" => "AU",   // Australia
        "MYR" => "MY",   // Malaysia
        "TRY" => "TR",   // Türkiye
        _ => "UNKNOWN",
    }
}

// ---------------------------------------------------------------------------
// Convenience accessors for domain lists (used in tests)
// ---------------------------------------------------------------------------

/// Domain list for entity operations.
pub fn entity_domains() -> &'static [ComplianceDomain] {
    ENTITY_DOMAINS
}

/// Domain list for ownership operations.
pub fn ownership_domains() -> &'static [ComplianceDomain] {
    OWNERSHIP_DOMAINS
}

/// Domain list for fiscal account operations.
pub fn fiscal_account_domains() -> &'static [ComplianceDomain] {
    FISCAL_ACCOUNT_DOMAINS
}

/// Domain list for payment operations.
pub fn payment_domains() -> &'static [ComplianceDomain] {
    PAYMENT_DOMAINS
}

/// Domain list for identity operations.
pub fn identity_domains() -> &'static [ComplianceDomain] {
    IDENTITY_DOMAINS
}

/// Domain list for consent operations.
pub fn consent_domains() -> &'static [ComplianceDomain] {
    CONSENT_DOMAINS
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_compliance_returns_20_domain_results() {
        let (_tensor, summary) = evaluate_compliance("PK-RSEZ", "entity-1", ENTITY_DOMAINS);
        assert_eq!(summary.domain_results.len(), 20);
        assert!(!summary.jurisdiction_id.is_empty());
    }

    #[test]
    fn evaluate_compliance_entity_domains_pending() {
        let (_tensor, summary) = evaluate_compliance("PK-RSEZ", "entity-1", ENTITY_DOMAINS);
        // With no attestations, relevant domains should be pending.
        for domain in ENTITY_DOMAINS {
            let state = summary.domain_results.get(domain.as_str());
            assert!(state.is_some(), "missing domain {domain}");
        }
    }

    #[test]
    fn no_hard_blocks_on_fresh_evaluation() {
        let (_tensor, summary) = evaluate_compliance("PK-RSEZ", "entity-1", ENTITY_DOMAINS);
        // Fresh tensor has no NonCompliant sanctions — no hard blocks.
        assert!(summary.hard_blocks.is_empty());
        assert!(check_hard_blocks(&summary).is_none());
    }

    #[test]
    fn check_hard_blocks_detects_sanctions() {
        let summary = ComplianceSummary {
            jurisdiction_id: "PK".to_string(),
            overall_status: "non_compliant".to_string(),
            domain_results: HashMap::new(),
            passing_domains: vec![],
            blocking_domains: vec!["sanctions".to_string()],
            hard_blocks: vec!["sanctions".to_string()],
            tensor_commitment: None,
            evaluated_at: Utc::now(),
        };
        let result = check_hard_blocks(&summary);
        assert!(result.is_some());
        assert!(result.as_ref().map_or(false, |r| r.contains("sanctions")));
    }

    #[test]
    fn issue_compliance_vc_signs_with_zone_key() {
        let state = AppState::new();
        let (_tensor, summary) = evaluate_compliance("PK-RSEZ", "entity-1", ENTITY_DOMAINS);

        let vc = issue_compliance_vc(
            &state,
            vc_types::FORMATION_COMPLIANCE,
            "PK-RSEZ",
            "entity-1",
            &summary,
        );

        assert!(vc.is_ok());
        let vc = vc.expect("VC should be issued");
        assert_eq!(vc.issuer, state.zone_did);
        assert!(!vc.proof.is_empty());
    }

    #[test]
    fn store_attestation_returns_uuid() {
        let state = AppState::new();
        let id = store_attestation(
            &state,
            Uuid::new_v4(),
            "test_attestation",
            "PK-RSEZ",
            serde_json::json!({"test": true}),
        );
        let record = state.attestations.get(&id);
        assert!(record.is_some());
        let record = record.expect("attestation should exist");
        assert_eq!(record.status, AttestationStatus::Active);
    }

    #[test]
    fn orchestrate_entity_creation_produces_envelope() {
        let state = AppState::new();
        let mass_response = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "legal_name": "Test Corp",
            "jurisdiction_id": "pk-sez-01"
        });

        let envelope =
            orchestrate_entity_creation(&state, "pk-sez-01", "Test Corp", mass_response.clone());

        assert_eq!(envelope.mass_response, mass_response);
        assert_eq!(envelope.compliance.domain_results.len(), 20);
        assert!(envelope.credential.is_some());
        assert!(envelope.attestation_id.is_some());
    }

    #[test]
    fn orchestrate_cap_table_creation_produces_envelope() {
        let state = AppState::new();
        let mass_response = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "entity_id": "550e8400-e29b-41d4-a716-446655440001"
        });

        let envelope = orchestrate_cap_table_creation(
            &state,
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").expect("valid uuid"),
            mass_response,
        );

        assert_eq!(envelope.compliance.domain_results.len(), 20);
        assert!(envelope.credential.is_some());
    }

    #[test]
    fn orchestrate_account_creation_produces_envelope() {
        let state = AppState::new();
        let mass_response = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "currency": "PKR"
        });

        let envelope = orchestrate_account_creation(
            &state,
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").expect("valid uuid"),
            "PKR",
            mass_response,
        );

        assert_eq!(envelope.compliance.jurisdiction_id, "PK");
        assert!(envelope.credential.is_some());
    }

    #[test]
    fn orchestrate_payment_produces_envelope() {
        let state = AppState::new();
        let mass_response = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "amount": "50000.00"
        });

        let envelope = orchestrate_payment(
            &state,
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").expect("valid uuid"),
            "PKR",
            mass_response,
        );

        assert_eq!(envelope.compliance.jurisdiction_id, "PK");
        assert!(envelope.credential.is_some());
    }

    #[test]
    fn orchestrate_identity_verification_produces_envelope() {
        let state = AppState::new();
        let mass_response = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "status": "verified"
        });

        let envelope = orchestrate_identity_verification(&state, "individual", mass_response);

        assert_eq!(envelope.compliance.jurisdiction_id, "GLOBAL");
        assert!(envelope.credential.is_some());
    }

    #[test]
    fn orchestrate_consent_creation_produces_envelope() {
        let state = AppState::new();
        let mass_response = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "consent_type": "board_resolution"
        });

        let envelope = orchestrate_consent_creation(&state, "board_resolution", mass_response);

        assert_eq!(envelope.compliance.jurisdiction_id, "GLOBAL");
        assert!(envelope.credential.is_some());
    }

    #[test]
    fn infer_jurisdiction_pkr() {
        assert_eq!(infer_jurisdiction_from_currency("PKR"), "PK");
        assert_eq!(infer_jurisdiction_from_currency("pkr"), "PK");
    }

    #[test]
    fn infer_jurisdiction_deployment_targets() {
        // All active and planned deployment corridors must resolve.
        assert_eq!(infer_jurisdiction_from_currency("AED"), "AE");
        assert_eq!(infer_jurisdiction_from_currency("USD"), "US");
        assert_eq!(infer_jurisdiction_from_currency("GBP"), "GB");
        assert_eq!(infer_jurisdiction_from_currency("EUR"), "EU");
        assert_eq!(infer_jurisdiction_from_currency("SGD"), "SG");
        assert_eq!(infer_jurisdiction_from_currency("CNY"), "CN");
        assert_eq!(infer_jurisdiction_from_currency("RMB"), "CN");
        assert_eq!(infer_jurisdiction_from_currency("SAR"), "SA");
        assert_eq!(infer_jurisdiction_from_currency("KYD"), "KY");
        assert_eq!(infer_jurisdiction_from_currency("SCR"), "SC");
        assert_eq!(infer_jurisdiction_from_currency("KZT"), "KZ");
        assert_eq!(infer_jurisdiction_from_currency("HKD"), "HK");
    }

    #[test]
    fn infer_jurisdiction_case_insensitive() {
        assert_eq!(infer_jurisdiction_from_currency("cny"), "CN");
        assert_eq!(infer_jurisdiction_from_currency("Sar"), "SA");
        assert_eq!(infer_jurisdiction_from_currency("kyd"), "KY");
    }

    #[test]
    fn infer_jurisdiction_unknown() {
        assert_eq!(infer_jurisdiction_from_currency("XYZ"), "UNKNOWN");
    }

    #[test]
    fn domain_list_accessors() {
        assert_eq!(entity_domains().len(), 6);
        assert_eq!(ownership_domains().len(), 5);
        assert_eq!(fiscal_account_domains().len(), 5);
        assert_eq!(payment_domains().len(), 5);
        assert_eq!(identity_domains().len(), 3);
        assert_eq!(consent_domains().len(), 3);
    }

    #[test]
    fn vc_type_constants_are_non_empty() {
        assert!(!vc_types::FORMATION_COMPLIANCE.is_empty());
        assert!(!vc_types::OWNERSHIP_COMPLIANCE.is_empty());
        assert!(!vc_types::FISCAL_COMPLIANCE.is_empty());
        assert!(!vc_types::PAYMENT_COMPLIANCE.is_empty());
        assert!(!vc_types::IDENTITY_COMPLIANCE.is_empty());
        assert!(!vc_types::CONSENT_COMPLIANCE.is_empty());
    }
}
