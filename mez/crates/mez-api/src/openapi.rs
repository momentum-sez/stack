//! # OpenAPI Specification Assembly
//!
//! Assembles all utoipa-documented routes into a single OpenAPI 3.1 spec.
//! Serves at `/openapi.json`. Optionally includes Swagger UI at `/swagger-ui`
//! when the `swagger` feature is enabled.

use axum::routing::get;
use axum::{Json, Router};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::state::AppState;

/// Adds Bearer token security scheme to the OpenAPI spec.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some(
                            "Bearer token authentication. Set via MEZ_AUTH_TOKEN env var.",
                        ))
                        .build(),
                ),
            );
        }
    }
}

/// Assembled OpenAPI spec for the entire API surface.
///
/// Registers all utoipa-documented routes, schemas, tags, and security
/// definitions. Serves as the single source of truth for integrators.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "MEZ API — EZ Stack Orchestration Layer",
        version = "0.4.44",
        description = "Axum API services for the Momentum EZ Stack: orchestration layer above Mass APIs.\n\nProvides:\n- **Mass API proxy** for primitive operations (entities, ownership, fiscal, identity, consent) with compliance orchestration\n- **Corridor lifecycle** management, receipt chains, checkpoints, and fork resolution\n- **Settlement** computation, route optimization, and SWIFT pacs.008 generation\n- **Compliance tensor** evaluation across 20 domains, jurisdiction-scoped\n- **Verifiable Credential** issuance and verification (Ed25519Signature2020)\n- **Smart asset** management with compliance-gated registration\n- **Inter-zone corridor peering** protocol (proposal/acceptance/receipt exchange)\n- **Identity orchestration** (NADRA CNIC, FBR IRIS NTN verification)\n- **Regulator console** with zone dashboard and entity compliance queries\n\nAuthentication: Bearer token via `Authorization: Bearer <token>` header.\nAll `/v1/*` endpoints require authentication. Health probes (`/health/*`) are unauthenticated.",
        license(name = "BUSL-1.1"),
        contact(name = "Momentum", url = "https://momentum.inc")
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
    ),
    security(
        ("bearer_auth" = [])
    ),
    paths(
        // ── Mass API proxy — entities (organization-info) ────────────────
        crate::routes::mass_proxy::create_entity,
        crate::routes::mass_proxy::get_entity,
        crate::routes::mass_proxy::update_entity,
        crate::routes::mass_proxy::list_entities,
        // ── Mass API proxy — ownership (investment-info) ─────────────────
        crate::routes::mass_proxy::create_cap_table,
        crate::routes::mass_proxy::get_cap_table,
        // ── Mass API proxy — fiscal (treasury-info) ──────────────────────
        crate::routes::mass_proxy::create_account,
        crate::routes::mass_proxy::initiate_payment,
        // ── Mass API proxy — identity ────────────────────────────────────
        crate::routes::mass_proxy::verify_identity,
        crate::routes::mass_proxy::get_identity,
        // ── Mass API proxy — consent (consent-info) ──────────────────────
        crate::routes::mass_proxy::create_consent,
        crate::routes::mass_proxy::get_consent,
        // ── Corridors ────────────────────────────────────────────────────
        crate::routes::corridors::create_corridor,
        crate::routes::corridors::list_corridors,
        crate::routes::corridors::get_corridor,
        crate::routes::corridors::transition_corridor,
        crate::routes::corridors::propose_receipt,
        crate::routes::corridors::get_receipts,
        crate::routes::corridors::get_checkpoint,
        crate::routes::corridors::create_checkpoint,
        crate::routes::corridors::fork_resolve,
        crate::routes::corridors::anchor_commitment,
        crate::routes::corridors::finality_status,
        crate::routes::corridors::corridor_health,
        // ── Settlement ──────────────────────────────────────────────────
        crate::routes::settlement::compute_settlement,
        crate::routes::settlement::find_route,
        crate::routes::settlement::generate_instructions,
        // ── Credentials ─────────────────────────────────────────────────
        crate::routes::credentials::issue_compliance_credential,
        crate::routes::credentials::verify_credential,
        // ── Smart Assets ────────────────────────────────────────────────
        crate::routes::smart_assets::create_asset,
        crate::routes::smart_assets::submit_registry,
        crate::routes::smart_assets::get_asset,
        crate::routes::smart_assets::evaluate_compliance,
        crate::routes::smart_assets::verify_anchor,
        // ── Inter-Zone Corridor Peering ─────────────────────────────────
        crate::routes::peers::list_peers,
        crate::routes::peers::get_peer,
        crate::routes::peers::propose_corridor,
        crate::routes::peers::accept_corridor,
        crate::routes::peers::receive_receipt,
        crate::routes::peers::receive_attestation,
        // ── Identity Orchestration ──────────────────────────────────────
        crate::routes::identity::verify_cnic,
        crate::routes::identity::verify_ntn,
        crate::routes::identity::get_entity_identity,
        crate::routes::identity::identity_service_status,
        // ── Regulator Console ───────────────────────────────────────────
        crate::routes::regulator::query_attestations,
        crate::routes::regulator::compliance_summary,
        crate::routes::regulator::dashboard,
        // ── Compliance Query ────────────────────────────────────────────
        crate::routes::compliance_query::list_domains,
        crate::routes::compliance_query::query_jurisdiction,
        crate::routes::compliance_query::query_entity,
        crate::routes::compliance_query::query_corridor,
        // ── Trade Flow Instruments ────────────────────────────────────────
        crate::routes::trade::create_trade_flow,
        crate::routes::trade::list_trade_flows,
        crate::routes::trade::get_trade_flow,
        crate::routes::trade::submit_transition,
        crate::routes::trade::list_transitions,
        // ── Agentic Policy Engine ────────────────────────────────────────
        crate::routes::agentic::submit_trigger,
        crate::routes::agentic::list_policies,
        crate::routes::agentic::delete_policy,
        // ── Watcher Economy ──────────────────────────────────────────────
        crate::routes::watchers::register_watcher,
        crate::routes::watchers::list_watchers,
        crate::routes::watchers::get_watcher,
        crate::routes::watchers::bond_watcher,
        crate::routes::watchers::activate_watcher,
        crate::routes::watchers::slash_watcher,
        crate::routes::watchers::rebond_watcher,
        crate::routes::watchers::unbond_watcher,
        crate::routes::watchers::complete_unbond_watcher,
        crate::routes::watchers::record_attestation,
    ),
    components(
        schemas(
            // ── State record types (EZ-Stack-owned) ─────────────────────
            crate::state::CorridorRecord,
            crate::state::SmartAssetRecord,
            crate::state::AttestationRecord,
            crate::state::AssetComplianceStatus,
            // ── Error types ─────────────────────────────────────────────
            crate::error::ErrorBody,
            crate::error::ErrorDetail,
            // ── Mass proxy DTOs — entities ───────────────────────────────
            crate::routes::mass_proxy::CreateEntityProxyRequest,
            crate::routes::mass_proxy::BeneficialOwnerInput,
            // ── Mass proxy DTOs — ownership ──────────────────────────────
            crate::routes::mass_proxy::CreateCapTableProxyRequest,
            crate::routes::mass_proxy::ShareClassInput,
            // ── Mass proxy DTOs — fiscal ─────────────────────────────────
            crate::routes::mass_proxy::CreateAccountProxyRequest,
            crate::routes::mass_proxy::CreatePaymentProxyRequest,
            // ── Mass proxy DTOs — identity ───────────────────────────────
            crate::routes::mass_proxy::VerifyIdentityProxyRequest,
            crate::routes::mass_proxy::LinkedIdInput,
            // ── Mass proxy DTOs — consent ────────────────────────────────
            crate::routes::mass_proxy::CreateConsentProxyRequest,
            crate::routes::mass_proxy::ConsentPartyInput,
            // ── Corridor DTOs ───────────────────────────────────────────
            crate::routes::corridors::PaginationParams,
            crate::routes::corridors::CreateCorridorRequest,
            crate::routes::corridors::TransitionCorridorRequest,
            crate::routes::corridors::ProposeReceiptRequest,
            crate::routes::corridors::ReceiptProposalResponse,
            crate::routes::corridors::ForkResolveRequest,
            crate::routes::corridors::ForkBranchInput,
            crate::routes::corridors::ForkResolveResponse,
            crate::routes::corridors::ReceiptChainResponse,
            crate::routes::corridors::ReceiptEntry,
            crate::routes::corridors::CheckpointResponse,
            crate::routes::corridors::CorridorHealthResponse,
            crate::routes::corridors::CorridorHealthEntry,
            // ── Settlement DTOs ─────────────────────────────────────────
            crate::routes::settlement::SettlementComputeRequest,
            crate::routes::settlement::ObligationInput,
            crate::routes::settlement::SettlementPlanResponse,
            crate::routes::settlement::NetPositionResponse,
            crate::routes::settlement::SettlementLegResponse,
            crate::routes::settlement::RouteRequest,
            crate::routes::settlement::RouteResponse,
            crate::routes::settlement::RouteHopResponse,
            crate::routes::settlement::InstructionRequest,
            crate::routes::settlement::InstructionLeg,
            crate::routes::settlement::InstructionResponse,
            crate::routes::settlement::Pacs008Output,
            // ── Credential DTOs ─────────────────────────────────────────
            crate::routes::credentials::ComplianceCredentialRequest,
            crate::routes::credentials::ComplianceCredentialResponse,
            crate::routes::credentials::VerificationResponse,
            crate::routes::credentials::ProofVerificationResult,
            // ── Smart Asset DTOs ────────────────────────────────────────
            crate::routes::smart_assets::CreateAssetRequest,
            crate::routes::smart_assets::ComplianceEvalRequest,
            crate::routes::smart_assets::ComplianceEvalResponse,
            crate::routes::smart_assets::AnchorVerifyRequest,
            // ── Peer DTOs ───────────────────────────────────────────────
            crate::routes::peers::ProposalResponse,
            crate::routes::peers::AcceptanceResponse,
            crate::routes::peers::PeerSummary,
            // ── Identity DTOs ───────────────────────────────────────────
            crate::routes::identity::CnicVerifyRequest,
            crate::routes::identity::CnicVerifyResponse,
            crate::routes::identity::NtnVerifyRequest,
            crate::routes::identity::NtnVerifyResponse,
            crate::routes::identity::EntityIdentityResponse,
            crate::routes::identity::IdentityServiceStatus,
            // ── Regulator DTOs ──────────────────────────────────────────
            crate::routes::regulator::QueryAttestationsRequest,
            crate::routes::regulator::QueryResultsResponse,
            crate::routes::regulator::ComplianceSummary,
            crate::routes::regulator::RegulatorDashboard,
            crate::routes::regulator::ZoneStatus,
            crate::routes::regulator::CompliancePosture,
            crate::routes::regulator::AssetComplianceSnapshot,
            crate::routes::regulator::CorridorOverview,
            crate::routes::regulator::CorridorStatus,
            crate::routes::regulator::PolicyActivity,
            crate::routes::regulator::AuditEntrySummary,
            crate::routes::regulator::SystemHealth,
            // ── Compliance Query DTOs ───────────────────────────────────
            crate::routes::compliance_query::JurisdictionComplianceResponse,
            crate::routes::compliance_query::DomainComplianceEntry,
            crate::routes::compliance_query::CorridorComplianceResponse,
            crate::routes::compliance_query::JurisdictionComplianceSummary,
            crate::routes::compliance_query::CrossBlockingDomain,
            crate::routes::compliance_query::ComplianceDomainInfo,
            crate::routes::compliance_query::EntityComplianceResponse,
            crate::routes::compliance_query::EntityDomainEntry,
            // ── Trade Flow DTOs ──────────────────────────────────────────
            crate::routes::trade::CreateTradeFlowRequest,
            crate::routes::trade::SubmitTransitionRequest,
            crate::routes::trade::TradeFlowResponse,
            // ── Agentic DTOs ────────────────────────────────────────────
            crate::routes::agentic::TriggerRequest,
            crate::routes::agentic::TriggerResponse,
            crate::routes::agentic::ActionResult,
            crate::routes::agentic::ActionStatus,
            // ── Watcher Economy DTOs ────────────────────────────────────
            crate::routes::watchers::WatcherResponse,
            crate::routes::watchers::WatcherListResponse,
            crate::routes::watchers::BondRequest,
            crate::routes::watchers::SlashRequest,
            crate::routes::watchers::RebondRequest,
            crate::routes::watchers::SlashResponse,
            crate::routes::watchers::UnbondResponse,
        ),
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "entities", description = "ENTITIES primitive — proxied to Mass organization-info API"),
        (name = "ownership", description = "OWNERSHIP primitive — proxied to Mass investment-info API"),
        (name = "fiscal", description = "FISCAL primitive — proxied to Mass treasury-info API"),
        (name = "identity", description = "IDENTITY primitive — proxied to Mass identity services"),
        (name = "identity-orchestration", description = "Identity Orchestration — NADRA CNIC and FBR IRIS NTN verification"),
        (name = "consent", description = "CONSENT primitive — proxied to Mass consent-info API"),
        (name = "corridors", description = "Corridor lifecycle, receipt chains, checkpoints, and fork resolution"),
        (name = "corridor-peers", description = "Inter-zone corridor peering protocol — proposal, acceptance, receipt and attestation exchange"),
        (name = "settlement", description = "Settlement computation, route optimization, and SWIFT pacs.008 instruction generation"),
        (name = "credentials", description = "Verifiable Credential issuance (compliance attestations) and verification"),
        (name = "smart_assets", description = "Smart Asset lifecycle — genesis, registry submission, compliance evaluation"),
        (name = "regulator", description = "Regulator Console — attestation queries, compliance summaries, zone dashboard"),
        (name = "compliance", description = "Compliance Query API — tensor evaluation, entity compliance, bilateral corridor compliance"),
        (name = "trade", description = "Trade Flow Instruments — export, import, letter of credit, and open account lifecycle"),
        (name = "agentic", description = "Agentic Policy Engine — trigger evaluation, policy management, autonomous corridor reactions"),
        (name = "watchers", description = "Watcher Economy — bonding, slashing, activation, attestation lifecycle for corridor watchers"),
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
            "should contain /v1/corridors"
        );
        assert!(
            spec.paths.paths.contains_key("/v1/corridors/{id}/receipts"),
            "should contain receipt chain query path"
        );
        assert!(
            spec.paths.paths.contains_key("/v1/corridors/{id}/checkpoint"),
            "should contain checkpoint path"
        );
    }

    #[test]
    fn test_openapi_spec_has_settlement_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/corridors/{id}/settlement/compute"),
            "should contain settlement compute path"
        );
        assert!(
            spec.paths.paths.contains_key("/v1/corridors/route"),
            "should contain settlement route path"
        );
        assert!(
            spec.paths.paths.contains_key("/v1/corridors/{id}/settlement/instruct"),
            "should contain settlement instruct path"
        );
    }

    #[test]
    fn test_openapi_spec_has_credential_paths() {
        let spec = ApiDoc::openapi();
        let has_credential_path = spec
            .paths
            .paths
            .keys()
            .any(|k| k.contains("credentials"));
        assert!(has_credential_path, "should contain credential paths");
    }

    #[test]
    fn test_openapi_spec_has_peer_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/corridors/peers"),
            "should contain peer listing path"
        );
    }

    #[test]
    fn test_openapi_spec_has_identity_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/identity/cnic/verify"),
            "should contain CNIC verification path"
        );
        assert!(
            spec.paths.paths.contains_key("/v1/identity/ntn/verify"),
            "should contain NTN verification path"
        );
    }

    #[test]
    fn test_openapi_spec_has_compliance_entity_path() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/compliance/entity/{entity_id}"),
            "should contain entity compliance query path"
        );
    }

    #[test]
    fn test_openapi_spec_has_smart_asset_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/assets/genesis"),
            "should contain /v1/assets/genesis path"
        );
    }

    #[test]
    fn test_openapi_spec_has_regulator_paths() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/v1/regulator/summary"),
            "should contain /v1/regulator/summary path"
        );
    }

    #[test]
    fn test_openapi_spec_has_tags() {
        let spec = ApiDoc::openapi();
        let tags = &spec.tags;
        assert!(tags.is_some(), "OpenAPI spec should have tags");
        let tags = tags.as_ref().unwrap();
        let tag_names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
        for expected in &[
            "entities",
            "corridors",
            "corridor-peers",
            "settlement",
            "credentials",
            "smart_assets",
            "regulator",
            "compliance",
            "identity-orchestration",
        ] {
            assert!(
                tag_names.contains(expected),
                "should contain {expected} tag"
            );
        }
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
        // Verify key schemas are present.
        for name in &[
            "ReceiptChainResponse",
            "ReceiptEntry",
            "CheckpointResponse",
            "SettlementComputeRequest",
            "SettlementPlanResponse",
            "ComplianceCredentialRequest",
            "PeerSummary",
            "CnicVerifyRequest",
            "EntityComplianceResponse",
        ] {
            assert!(
                schemas.contains_key(*name),
                "should contain {name} schema"
            );
        }
    }

    #[test]
    fn test_openapi_spec_has_security_scheme() {
        let spec = ApiDoc::openapi();
        let components = spec.components.as_ref().unwrap();
        let security_schemes = &components.security_schemes;
        assert!(
            security_schemes.contains_key("bearer_auth"),
            "should contain bearer_auth security scheme"
        );
    }

    #[test]
    fn test_openapi_spec_has_servers() {
        let spec = ApiDoc::openapi();
        let servers = &spec.servers;
        assert!(servers.is_some(), "should have server definitions");
        let servers = servers.as_ref().unwrap();
        assert!(!servers.is_empty(), "should have at least one server");
    }

    #[test]
    fn test_openapi_spec_path_count() {
        let spec = ApiDoc::openapi();
        let path_count = spec.paths.paths.len();
        // We registered 40+ endpoints across all modules.
        assert!(
            path_count >= 30,
            "expected at least 30 paths, got {path_count}"
        );
    }

    #[test]
    fn test_openapi_spec_serializes_to_json() {
        let spec = ApiDoc::openapi();
        let json = serde_json::to_string(&spec);
        assert!(json.is_ok(), "OpenAPI spec should serialize to JSON");
        let json_str = json.unwrap();
        assert!(json_str.contains("openapi"), "should contain openapi key");
        assert!(
            json_str.contains("bearer_auth"),
            "should contain security scheme"
        );
    }

    #[test]
    fn test_router_builds_successfully() {
        let _router = router();
    }
}
