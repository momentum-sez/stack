//! # OpenAPI Specification Assembly
//!
//! Assembles all utoipa-documented routes into a single OpenAPI 3.1 spec.
//! Serves at `/openapi.json`. Optionally includes Swagger UI at `/swagger-ui`
//! when the `swagger` feature is enabled.

use axum::routing::get;
use axum::{Json, Router};
use utoipa::OpenApi;

use crate::state::AppState;

/// Assembled OpenAPI spec for the entire API surface.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "MSEZ API — Five Programmable Primitives",
        version = "0.1.0",
        description = "Axum API services for the Momentum SEZ Stack: Entities, Ownership, Fiscal, Identity, Consent, Corridors, Smart Assets, and Regulator Console.",
        license(name = "BUSL-1.1")
    ),
    paths(
        // Entities
        crate::routes::entities::create_entity,
        crate::routes::entities::list_entities,
        crate::routes::entities::get_entity,
        crate::routes::entities::update_entity,
        crate::routes::entities::get_beneficial_owners,
        crate::routes::entities::initiate_dissolution,
        crate::routes::entities::get_dissolution_status,
        // Ownership
        crate::routes::ownership::create_cap_table,
        crate::routes::ownership::get_cap_table,
        crate::routes::ownership::record_transfer,
        crate::routes::ownership::get_share_classes,
        // Fiscal
        crate::routes::fiscal::create_account,
        crate::routes::fiscal::initiate_payment,
        crate::routes::fiscal::calculate_withholding,
        crate::routes::fiscal::get_tax_events,
        crate::routes::fiscal::generate_report,
        // Identity
        crate::routes::identity::verify_identity,
        crate::routes::identity::get_identity,
        crate::routes::identity::link_external_id,
        crate::routes::identity::submit_attestation,
        // Consent
        crate::routes::consent::create_consent,
        crate::routes::consent::get_consent,
        crate::routes::consent::sign_consent,
        crate::routes::consent::get_audit_trail,
        // Corridors
        crate::routes::corridors::create_corridor,
        crate::routes::corridors::list_corridors,
        crate::routes::corridors::get_corridor,
        crate::routes::corridors::transition_corridor,
        crate::routes::corridors::propose_receipt,
        crate::routes::corridors::fork_resolve,
        crate::routes::corridors::anchor_commitment,
        crate::routes::corridors::finality_status,
        // Smart Assets
        crate::routes::smart_assets::create_asset,
        crate::routes::smart_assets::submit_registry,
        crate::routes::smart_assets::get_asset,
        crate::routes::smart_assets::evaluate_compliance,
        crate::routes::smart_assets::verify_anchor,
        // Regulator
        crate::routes::regulator::query_attestations,
        crate::routes::regulator::compliance_summary,
    ),
    components(schemas(
        // State record types
        crate::state::EntityRecord,
        crate::state::BeneficialOwner,
        crate::state::CapTableRecord,
        crate::state::ShareClass,
        crate::state::OwnershipTransfer,
        crate::state::FiscalAccountRecord,
        crate::state::PaymentRecord,
        crate::state::TaxEventRecord,
        crate::state::IdentityRecord,
        crate::state::LinkedExternalId,
        crate::state::IdentityAttestation,
        crate::state::ConsentRecord,
        crate::state::ConsentParty,
        crate::state::ConsentAuditEntry,
        crate::state::CorridorRecord,
        crate::state::CorridorTransitionEntry,
        crate::state::SmartAssetRecord,
        crate::state::AttestationRecord,
        // Error types
        crate::error::ErrorBody,
        crate::error::ErrorDetail,
        // Entity DTOs
        crate::routes::entities::CreateEntityRequest,
        crate::routes::entities::UpdateEntityRequest,
        crate::routes::entities::DissolutionStatusResponse,
        // Ownership DTOs
        crate::routes::ownership::CreateCapTableRequest,
        crate::routes::ownership::RecordTransferRequest,
        // Fiscal DTOs
        crate::routes::fiscal::CreateAccountRequest,
        crate::routes::fiscal::InitiatePaymentRequest,
        crate::routes::fiscal::WithholdingCalculateRequest,
        crate::routes::fiscal::WithholdingResponse,
        // Identity DTOs
        crate::routes::identity::VerifyIdentityRequest,
        crate::routes::identity::LinkExternalIdRequest,
        crate::routes::identity::SubmitAttestationRequest,
        // Consent DTOs
        crate::routes::consent::CreateConsentRequest,
        crate::routes::consent::ConsentPartyInput,
        crate::routes::consent::SignConsentRequest,
        // Corridor DTOs
        crate::routes::corridors::CreateCorridorRequest,
        crate::routes::corridors::TransitionCorridorRequest,
        crate::routes::corridors::ProposeReceiptRequest,
        crate::routes::corridors::ReceiptResponse,
        // Smart Asset DTOs
        crate::routes::smart_assets::CreateAssetRequest,
        crate::routes::smart_assets::ComplianceEvalRequest,
        crate::routes::smart_assets::ComplianceEvalResponse,
        crate::routes::smart_assets::AnchorVerifyRequest,
        // Regulator DTOs
        crate::routes::regulator::QueryAttestationsRequest,
        crate::routes::regulator::QueryResultsResponse,
        crate::routes::regulator::ComplianceSummary,
    )),
    tags(
        (name = "entities", description = "ENTITIES primitive — Organization Info API"),
        (name = "ownership", description = "OWNERSHIP primitive — Investment Info API"),
        (name = "fiscal", description = "FISCAL primitive — Treasury Info API"),
        (name = "identity", description = "IDENTITY primitive — Identity Verification API"),
        (name = "consent", description = "CONSENT primitive — Consent Info API"),
        (name = "corridors", description = "Corridor Operations API"),
        (name = "smart_assets", description = "Smart Asset API"),
        (name = "regulator", description = "Regulator Console API"),
    )
)]
pub struct ApiDoc;

/// Build the OpenAPI router.
///
/// Serves the OpenAPI JSON spec at `/openapi.json`.
pub fn router() -> Router<AppState> {
    Router::new().route("/openapi.json", get(openapi_json))
}

/// GET /openapi.json — Return the generated OpenAPI specification.
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── OpenAPI spec generation ───────────────────────────────────

    #[test]
    fn test_openapi_spec_generates_successfully() {
        let spec = ApiDoc::openapi();
        assert_eq!(spec.info.title, "MSEZ API — Five Programmable Primitives");
        assert_eq!(spec.info.version, "0.1.0");
    }

    #[test]
    fn test_openapi_spec_has_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            !spec.paths.paths.is_empty(),
            "OpenAPI spec should contain at least one path"
        );
    }

    #[test]
    fn test_openapi_spec_has_entity_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/entities"),
            "OpenAPI spec should contain /v1/entities path"
        );
    }

    #[test]
    fn test_openapi_spec_has_corridor_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/corridors"),
            "OpenAPI spec should contain /v1/corridors path"
        );
    }

    #[test]
    fn test_openapi_spec_has_fiscal_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/fiscal/accounts"),
            "OpenAPI spec should contain /v1/fiscal/accounts path"
        );
    }

    #[test]
    fn test_openapi_spec_has_identity_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/identity/verify"),
            "OpenAPI spec should contain /v1/identity/verify path"
        );
    }

    #[test]
    fn test_openapi_spec_has_consent_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/consent/request"),
            "OpenAPI spec should contain /v1/consent/request path"
        );
    }

    #[test]
    fn test_openapi_spec_has_ownership_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/ownership/cap-table"),
            "OpenAPI spec should contain /v1/ownership/cap-table path"
        );
    }

    #[test]
    fn test_openapi_spec_has_smart_asset_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/assets/genesis"),
            "OpenAPI spec should contain /v1/assets/genesis path"
        );
    }

    #[test]
    fn test_openapi_spec_has_regulator_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/regulator/summary"),
            "OpenAPI spec should contain /v1/regulator/summary path"
        );
    }

    #[test]
    fn test_openapi_spec_has_tags() {
        let spec = ApiDoc::openapi();
        let tags = &spec.tags;
        assert!(tags.is_some(), "OpenAPI spec should have tags");
        let tags = tags.as_ref().unwrap();
        let tag_names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
        assert!(
            tag_names.contains(&"entities"),
            "should contain entities tag"
        );
        assert!(
            tag_names.contains(&"ownership"),
            "should contain ownership tag"
        );
        assert!(tag_names.contains(&"fiscal"), "should contain fiscal tag");
        assert!(
            tag_names.contains(&"identity"),
            "should contain identity tag"
        );
        assert!(tag_names.contains(&"consent"), "should contain consent tag");
        assert!(
            tag_names.contains(&"corridors"),
            "should contain corridors tag"
        );
        assert!(
            tag_names.contains(&"smart_assets"),
            "should contain smart_assets tag"
        );
        assert!(
            tag_names.contains(&"regulator"),
            "should contain regulator tag"
        );
    }

    #[test]
    fn test_openapi_spec_has_components() {
        let spec = ApiDoc::openapi();
        let components = &spec.components;
        assert!(components.is_some(), "OpenAPI spec should have components");
        let schemas = &components.as_ref().unwrap().schemas;
        assert!(
            !schemas.is_empty(),
            "OpenAPI spec should have schema components"
        );
    }

    #[test]
    fn test_openapi_spec_serializes_to_json() {
        let spec = ApiDoc::openapi();
        let json = serde_json::to_string(&spec);
        assert!(json.is_ok(), "OpenAPI spec should serialize to JSON");
        let json_str = json.unwrap();
        assert!(
            json_str.contains("openapi"),
            "JSON should contain openapi key"
        );
    }

    // ── Router construction ───────────────────────────────────────

    #[test]
    fn test_router_builds_successfully() {
        let _router = router();
    }
}
