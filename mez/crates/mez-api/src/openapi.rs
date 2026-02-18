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
        title = "MEZ API — EZ Stack Orchestration Layer",
        version = "0.4.44",
        description = "Axum API services for the Momentum EZ Stack: orchestration layer above Mass APIs. Provides Mass API proxy for primitive operations (entities, ownership, fiscal, identity, consent), plus EZ-Stack-native corridor lifecycle, smart asset management, and regulator console.",
        license(name = "BUSL-1.1")
    ),
    paths(
        // Mass API proxy — entities (organization-info)
        crate::routes::mass_proxy::create_entity,
        crate::routes::mass_proxy::get_entity,
        crate::routes::mass_proxy::update_entity,
        crate::routes::mass_proxy::list_entities,
        // Mass API proxy — ownership (investment-info)
        crate::routes::mass_proxy::create_cap_table,
        crate::routes::mass_proxy::get_cap_table,
        // Mass API proxy — fiscal (treasury-info)
        crate::routes::mass_proxy::create_account,
        crate::routes::mass_proxy::initiate_payment,
        // Mass API proxy — identity
        crate::routes::mass_proxy::verify_identity,
        crate::routes::mass_proxy::get_identity,
        // Mass API proxy — consent (consent-info)
        crate::routes::mass_proxy::create_consent,
        crate::routes::mass_proxy::get_consent,
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
        crate::routes::regulator::dashboard,
    ),
    components(schemas(
        // State record types (EZ-Stack-owned)
        crate::state::CorridorRecord,
        crate::state::SmartAssetRecord,
        crate::state::AttestationRecord,
        // Error types
        crate::error::ErrorBody,
        crate::error::ErrorDetail,
        // Mass proxy DTOs — entities
        crate::routes::mass_proxy::CreateEntityProxyRequest,
        crate::routes::mass_proxy::BeneficialOwnerInput,
        // Mass proxy DTOs — ownership
        crate::routes::mass_proxy::CreateCapTableProxyRequest,
        crate::routes::mass_proxy::ShareClassInput,
        // Mass proxy DTOs — fiscal
        crate::routes::mass_proxy::CreateAccountProxyRequest,
        crate::routes::mass_proxy::CreatePaymentProxyRequest,
        // Mass proxy DTOs — identity
        crate::routes::mass_proxy::VerifyIdentityProxyRequest,
        crate::routes::mass_proxy::LinkedIdInput,
        // Mass proxy DTOs — consent
        crate::routes::mass_proxy::CreateConsentProxyRequest,
        crate::routes::mass_proxy::ConsentPartyInput,
        // Corridor DTOs
        crate::routes::corridors::CreateCorridorRequest,
        crate::routes::corridors::TransitionCorridorRequest,
        crate::routes::corridors::ProposeReceiptRequest,
        crate::routes::corridors::ReceiptProposalResponse,
        crate::routes::corridors::ForkResolveRequest,
        crate::routes::corridors::ForkBranchInput,
        crate::routes::corridors::ForkResolveResponse,
        // Smart Asset DTOs
        crate::routes::smart_assets::CreateAssetRequest,
        crate::routes::smart_assets::ComplianceEvalRequest,
        crate::routes::smart_assets::ComplianceEvalResponse,
        crate::routes::smart_assets::AnchorVerifyRequest,
        // Regulator DTOs
        crate::routes::regulator::QueryAttestationsRequest,
        crate::routes::regulator::QueryResultsResponse,
        crate::routes::regulator::ComplianceSummary,
        // Regulator Dashboard DTOs
        crate::routes::regulator::RegulatorDashboard,
        crate::routes::regulator::ZoneStatus,
        crate::routes::regulator::CompliancePosture,
        crate::routes::regulator::AssetComplianceSnapshot,
        crate::state::AssetComplianceStatus,
        crate::routes::regulator::CorridorOverview,
        crate::routes::regulator::CorridorStatus,
        crate::routes::regulator::PolicyActivity,
        crate::routes::regulator::AuditEntrySummary,
        crate::routes::regulator::SystemHealth,
    )),
    tags(
        (name = "entities", description = "ENTITIES primitive — proxied to Mass organization-info API"),
        (name = "ownership", description = "OWNERSHIP primitive — proxied to Mass investment-info API"),
        (name = "fiscal", description = "FISCAL primitive — proxied to Mass treasury-info API"),
        (name = "identity", description = "IDENTITY primitive — proxied to Mass identity services"),
        (name = "consent", description = "CONSENT primitive — proxied to Mass consent-info API"),
        (name = "corridors", description = "Corridor Operations API (EZ Stack domain)"),
        (name = "smart_assets", description = "Smart Asset API (EZ Stack domain)"),
        (name = "regulator", description = "Regulator Console API (EZ Stack domain)"),
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

    #[test]
    fn test_openapi_spec_generates_successfully() {
        let spec = ApiDoc::openapi();
        assert_eq!(spec.info.title, "MEZ API — EZ Stack Orchestration Layer");
        assert_eq!(spec.info.version, "0.4.44");
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

    #[test]
    fn test_router_builds_successfully() {
        let _router = router();
    }
}
