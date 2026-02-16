//! # GovOS Console API (M-009)
//!
//! Route definitions for the Pakistan GovOS experience layer dashboards.
//! The GovOS architecture specifies 5 dashboards:
//!
//! 1. **GovOS Console** — 40+ ministries overview and operational dashboard
//! 2. **Tax & Revenue Dashboard** — Tax collection metrics, withholding reports,
//!    GDP contribution tracking
//! 3. **Digital Free Zone** — Free zone entity registry, incentive tracking,
//!    zone-specific compliance posture
//! 4. **Citizen Tax & Services** — Citizen-facing tax obligations, filing status,
//!    service availability
//!
//! The **Regulator Console** (dashboard #5) is already implemented in
//! [`super::regulator`].
//!
//! These routes provide read-only views aggregating data from the SEZ Stack's
//! in-memory stores and, where configured, from Mass APIs via `msez-mass-client`.

use std::collections::HashMap;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Datelike, Utc};
use msez_agentic::tax::{format_amount, parse_amount};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::{require_role, CallerIdentity, Role};
use crate::error::AppError;
use crate::state::AppState;

// ── GovOS Console Types ─────────────────────────────────────────────────────

/// Ministry summary for the GovOS Console.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MinistrySummary {
    /// Ministry identifier (e.g., "pk-mof", "pk-mofa").
    pub ministry_id: String,
    /// Human-readable ministry name.
    pub name: String,
    /// Number of entities registered under this ministry's jurisdiction.
    pub entity_count: usize,
    /// Number of active licenses issued by regulators under this ministry.
    pub license_count: usize,
    /// Current compliance posture (percentage of entities fully compliant).
    pub compliance_rate: f64,
}

/// GovOS Console dashboard — top-level overview for 40+ ministries.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GovosConsoleDashboard {
    /// Snapshot timestamp.
    pub snapshot_at: DateTime<Utc>,
    /// Jurisdiction identifier.
    pub jurisdiction_id: String,
    /// Total entities across all ministries (derived from attestations).
    pub total_entities: usize,
    /// Total corridors managed.
    pub total_corridors: usize,
    /// Total attestations issued.
    pub total_attestations: usize,
    /// Per-ministry summaries.
    pub ministries: Vec<MinistrySummary>,
    /// Aggregate compliance rate across all entities.
    pub aggregate_compliance_rate: f64,
}

// ── Tax & Revenue Dashboard Types ────────────────────────────────────────────

/// Tax & Revenue dashboard for GovOS.
#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TaxRevenueDashboard {
    /// Snapshot timestamp.
    pub snapshot_at: DateTime<Utc>,
    /// Jurisdiction identifier.
    pub jurisdiction_id: String,
    /// Total tax events recorded.
    pub total_tax_events: usize,
    /// Tax events by category.
    #[schema(value_type = Object)]
    pub events_by_category: HashMap<String, usize>,
    /// Total withholding amount (formatted string, e.g., "1,234,567.89 PKR").
    pub total_withholding: String,
    /// Number of FBR IRIS reports generated.
    pub reports_generated: usize,
    /// Number of registered entities with NTN.
    pub entities_with_ntn: usize,
    /// Current GDP contribution rate target.
    pub gdp_target_rate: String,
}

// ── Digital Free Zone Types ──────────────────────────────────────────────────

/// Digital Free Zone dashboard.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FreeZoneDashboard {
    /// Snapshot timestamp.
    pub snapshot_at: DateTime<Utc>,
    /// Zone identifier (e.g., "PK-PSEZ", "PK-RSEZ").
    pub zone_id: String,
    /// Number of entities registered in this free zone.
    pub registered_entities: usize,
    /// Active incentive programs.
    pub active_incentives: Vec<IncentiveProgram>,
    /// Corridor activity relevant to this zone.
    pub corridor_count: usize,
    /// Zone-level compliance posture.
    pub compliance_posture: ZoneCompliancePosture,
}

/// An active incentive program within a digital free zone.
#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IncentiveProgram {
    /// Program identifier.
    pub program_id: String,
    /// Program name.
    pub name: String,
    /// Tax holiday duration in years (0 = no holiday).
    pub tax_holiday_years: u32,
    /// Whether customs duty exemption applies.
    pub customs_duty_exempt: bool,
    /// Number of entities enrolled.
    pub enrolled_entities: usize,
}

/// Zone-level compliance posture.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZoneCompliancePosture {
    /// Percentage of entities fully compliant.
    pub compliance_rate: f64,
    /// Number of entities with blocking compliance issues.
    pub entities_with_issues: usize,
    /// Compliance domains with lowest pass rates.
    pub weakest_domains: Vec<String>,
}

// ── Citizen Tax & Services Types ─────────────────────────────────────────────

/// Citizen-facing tax and services dashboard.
#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CitizenDashboard {
    /// Snapshot timestamp.
    pub snapshot_at: DateTime<Utc>,
    /// Number of available government services.
    pub available_services: usize,
    /// Services by category.
    pub service_categories: Vec<ServiceCategory>,
    /// Tax filing status summary.
    pub filing_status: FilingStatusSummary,
}

/// A government service category.
#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ServiceCategory {
    /// Category identifier.
    pub category_id: String,
    /// Category name.
    pub name: String,
    /// Number of services in this category.
    pub service_count: usize,
    /// Whether online filing is available.
    pub online_filing: bool,
}

/// Tax filing status summary for citizen dashboard.
#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct FilingStatusSummary {
    /// Current tax year.
    pub tax_year: String,
    /// Filing deadline.
    pub filing_deadline: String,
    /// Number of entities that have filed.
    pub filed_count: usize,
    /// Number of entities with outstanding obligations.
    pub outstanding_count: usize,
}

// ── Router ──────────────────────────────────────────────────────────────────

/// Build the GovOS console router.
///
/// Mounts four dashboard endpoints corresponding to the Pakistan GovOS
/// experience layer (minus the Regulator Console, which lives in
/// [`super::regulator`]).
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/govos/console", get(govos_console))
        .route("/v1/govos/tax-revenue", get(tax_revenue_dashboard))
        .route("/v1/govos/freezone", get(freezone_dashboard))
        .route("/v1/govos/citizen", get(citizen_dashboard))
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// GET /v1/govos/console — GovOS Console dashboard for 40+ ministries.
#[utoipa::path(
    get,
    path = "/v1/govos/console",
    responses(
        (status = 200, description = "GovOS Console dashboard", body = GovosConsoleDashboard),
    ),
    tag = "govos"
)]
async fn govos_console(
    State(state): State<AppState>,
    caller: CallerIdentity,
) -> Result<Json<GovosConsoleDashboard>, AppError> {
    require_role(&caller, Role::Regulator)?;

    let now = Utc::now();
    let unique_entities: std::collections::HashSet<uuid::Uuid> = state
        .attestations
        .list()
        .iter()
        .map(|a| a.entity_id)
        .collect();

    // Jurisdiction from zone config or default to PK.
    let jurisdiction_id = state
        .zone
        .as_ref()
        .map(|z| z.jurisdiction_id.clone())
        .unwrap_or_else(|| "PK".to_string());

    // In a full deployment, ministry data would be sourced from Mass
    // organization-info grouped by regulatory authority. For now, provide
    // the structural response with SEZ-Stack-owned counts.
    let ministries = vec![
        MinistrySummary {
            ministry_id: "pk-mof".to_string(),
            name: "Ministry of Finance".to_string(),
            entity_count: 0,
            license_count: 0,
            compliance_rate: 0.0,
        },
        MinistrySummary {
            ministry_id: "pk-moc".to_string(),
            name: "Ministry of Commerce".to_string(),
            entity_count: 0,
            license_count: 0,
            compliance_rate: 0.0,
        },
        MinistrySummary {
            ministry_id: "pk-moit".to_string(),
            name: "Ministry of IT & Telecom".to_string(),
            entity_count: 0,
            license_count: 0,
            compliance_rate: 0.0,
        },
    ];

    Ok(Json(GovosConsoleDashboard {
        snapshot_at: now,
        jurisdiction_id,
        total_entities: unique_entities.len(),
        total_corridors: state.corridors.len(),
        total_attestations: state.attestations.len(),
        ministries,
        aggregate_compliance_rate: 0.0,
    }))
}

/// GET /v1/govos/tax-revenue — Tax & Revenue Dashboard.
#[utoipa::path(
    get,
    path = "/v1/govos/tax-revenue",
    responses(
        (status = 200, description = "Tax & Revenue dashboard", body = TaxRevenueDashboard),
    ),
    tag = "govos"
)]
async fn tax_revenue_dashboard(
    State(state): State<AppState>,
    caller: CallerIdentity,
) -> Result<Json<TaxRevenueDashboard>, AppError> {
    require_role(&caller, Role::Regulator)?;

    let now = Utc::now();
    let jurisdiction_id = state
        .zone
        .as_ref()
        .map(|z| z.jurisdiction_id.clone())
        .unwrap_or_else(|| "PK".to_string());

    // Aggregate tax event data from AppState's tax_events store.
    let events = state.tax_events.list();
    let total_tax_events = events.len();

    let mut events_by_category: HashMap<String, usize> = HashMap::new();
    let mut total_withholding_cents: i64 = 0;

    for event in &events {
        *events_by_category
            .entry(event.event_type.clone())
            .or_insert(0) += 1;
        // Parse the withholding_amount string to cents using fixed-precision parsing.
        // The withholding_amount is stored as a decimal string (e.g., "100.00")
        // from format_amount(). Using parse_amount() avoids f64 precision loss.
        if let Some(cents) = parse_amount(&event.withholding_amount) {
            total_withholding_cents = total_withholding_cents.saturating_add(cents);
        } else {
            tracing::warn!(
                event_id = %event.id,
                withholding_amount = %event.withholding_amount,
                "failed to parse withholding amount for GovOS tax revenue dashboard — \
                 event excluded from total withholding aggregate"
            );
        }
    }

    let reports_generated = 0usize; // Report count tracked externally.

    // Derive NTN count from attestations with "NTN_VERIFICATION" type.
    // Count unique entities, not attestation records — the same entity
    // verified multiple times should be counted once.
    let ntn_entities: std::collections::HashSet<uuid::Uuid> = state
        .attestations
        .list()
        .iter()
        .filter(|a| a.attestation_type == "NTN_VERIFICATION")
        .map(|a| a.entity_id)
        .collect();
    let entities_with_ntn = ntn_entities.len();

    let total_withholding = format!("{} PKR", format_amount(total_withholding_cents));

    Ok(Json(TaxRevenueDashboard {
        snapshot_at: now,
        jurisdiction_id,
        total_tax_events,
        events_by_category,
        total_withholding,
        reports_generated,
        entities_with_ntn,
        gdp_target_rate: "15.0%".to_string(),
    }))
}

/// GET /v1/govos/freezone — Digital Free Zone dashboard.
#[utoipa::path(
    get,
    path = "/v1/govos/freezone",
    responses(
        (status = 200, description = "Digital Free Zone dashboard", body = FreeZoneDashboard),
    ),
    tag = "govos"
)]
async fn freezone_dashboard(
    State(state): State<AppState>,
    caller: CallerIdentity,
) -> Result<Json<FreeZoneDashboard>, AppError> {
    require_role(&caller, Role::Regulator)?;

    let now = Utc::now();
    let zone_id = state
        .zone
        .as_ref()
        .map(|z| z.zone_id.clone())
        .unwrap_or_else(|| "PK-PSEZ".to_string());

    let unique_entities: std::collections::HashSet<uuid::Uuid> = state
        .attestations
        .list()
        .iter()
        .map(|a| a.entity_id)
        .collect();

    let assets_list = state.smart_assets.list();

    // Count entities (not assets) with at least one non-compliant asset.
    // Previous implementation subtracted non-compliant ASSET count from
    // ENTITY count — a category error (one entity can own many assets).
    let entities_with_issues: std::collections::HashSet<uuid::Uuid> = assets_list
        .iter()
        .filter(|a| {
            a.compliance_status == crate::state::AssetComplianceStatus::NonCompliant
        })
        .filter_map(|a| a.owner_entity_id)
        .collect();
    let non_compliant = entities_with_issues.len();

    let compliance_rate = if unique_entities.is_empty() {
        0.0
    } else {
        let compliant = unique_entities.len().saturating_sub(non_compliant);
        (compliant as f64 / unique_entities.len() as f64) * 100.0
    };

    // Pakistan SEZ incentive programs (structural data).
    let active_incentives = vec![
        IncentiveProgram {
            program_id: "pk-sez-tax-holiday".to_string(),
            name: "SEZ Tax Holiday (10 years)".to_string(),
            tax_holiday_years: 10,
            customs_duty_exempt: true,
            enrolled_entities: 0,
        },
        IncentiveProgram {
            program_id: "pk-digital-fz-exempt".to_string(),
            name: "Digital Free Zone Exemption".to_string(),
            tax_holiday_years: 5,
            customs_duty_exempt: false,
            enrolled_entities: 0,
        },
    ];

    Ok(Json(FreeZoneDashboard {
        snapshot_at: now,
        zone_id,
        registered_entities: unique_entities.len(),
        active_incentives,
        corridor_count: state.corridors.len(),
        compliance_posture: ZoneCompliancePosture {
            compliance_rate,
            entities_with_issues: non_compliant,
            weakest_domains: vec![],
        },
    }))
}

/// GET /v1/govos/citizen — Citizen Tax & Services dashboard.
#[utoipa::path(
    get,
    path = "/v1/govos/citizen",
    responses(
        (status = 200, description = "Citizen Tax & Services dashboard", body = CitizenDashboard),
    ),
    tag = "govos"
)]
async fn citizen_dashboard(
    State(state): State<AppState>,
    caller: CallerIdentity,
) -> Result<Json<CitizenDashboard>, AppError> {
    require_role(&caller, Role::EntityOperator)?;

    let now = Utc::now();

    // Service categories available through the GovOS citizen portal.
    let service_categories = vec![
        ServiceCategory {
            category_id: "tax-filing".to_string(),
            name: "Tax Filing & Returns".to_string(),
            service_count: 4,
            online_filing: true,
        },
        ServiceCategory {
            category_id: "entity-registration".to_string(),
            name: "Business Registration".to_string(),
            service_count: 3,
            online_filing: true,
        },
        ServiceCategory {
            category_id: "licensing".to_string(),
            name: "License Applications".to_string(),
            service_count: 6,
            online_filing: true,
        },
        ServiceCategory {
            category_id: "compliance".to_string(),
            name: "Compliance Certificates".to_string(),
            service_count: 5,
            online_filing: false,
        },
    ];

    let available_services: usize = service_categories.iter().map(|c| c.service_count).sum();

    // Tax events for the caller's entity (if bound).
    let events = state.tax_events.list();
    let (filed_count, outstanding_count) = match caller.entity_id {
        Some(eid) => {
            let entity_events: Vec<_> = events
                .iter()
                .filter(|e| e.entity_id == eid)
                .collect();
            let filed = entity_events
                .iter()
                .filter(|e| e.withholding_executed)
                .count();
            let outstanding = entity_events.len().saturating_sub(filed);
            (filed, outstanding)
        }
        None => (0, 0),
    };

    Ok(Json(CitizenDashboard {
        snapshot_at: now,
        available_services,
        service_categories,
        filing_status: FilingStatusSummary {
            tax_year: super::mass_proxy::current_pk_fiscal_year(),
            filing_deadline: {
                // Pakistan filing deadline is Sep 30 of the calendar year
                // in which the fiscal year ends (July-June cycle).
                let now = chrono::Utc::now();
                let deadline_year = if now.month() >= 7 { now.year() + 1 } else { now.year() };
                format!("{deadline_year}-09-30")
            },
            filed_count,
            outstanding_count,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::CallerIdentity;
    use crate::state::AppState;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn zone_admin() -> CallerIdentity {
        CallerIdentity {
            role: Role::ZoneAdmin,
            entity_id: None,
            jurisdiction_id: None,
        }
    }

    fn entity_operator() -> CallerIdentity {
        CallerIdentity {
            role: Role::EntityOperator,
            entity_id: Some(uuid::Uuid::new_v4()),
            jurisdiction_id: Some("PK".to_string()),
        }
    }

    fn test_app() -> Router<()> {
        router()
            .layer(axum::Extension(zone_admin()))
            .with_state(AppState::new())
    }

    fn test_app_citizen() -> Router<()> {
        router()
            .layer(axum::Extension(entity_operator()))
            .with_state(AppState::new())
    }

    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn govos_console_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/govos/console")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let dashboard: GovosConsoleDashboard = body_json(resp).await;
        assert!(!dashboard.jurisdiction_id.is_empty());
        assert!(dashboard.ministries.len() >= 3);
        assert_eq!(dashboard.total_entities, 0);
    }

    #[tokio::test]
    async fn tax_revenue_dashboard_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/govos/tax-revenue")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let dashboard: TaxRevenueDashboard = body_json(resp).await;
        assert_eq!(dashboard.gdp_target_rate, "15.0%");
        assert_eq!(dashboard.total_tax_events, 0);
    }

    #[tokio::test]
    async fn freezone_dashboard_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/govos/freezone")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let dashboard: FreeZoneDashboard = body_json(resp).await;
        assert!(dashboard.active_incentives.len() >= 2);
        assert_eq!(dashboard.registered_entities, 0);
    }

    #[tokio::test]
    async fn citizen_dashboard_returns_200() {
        let app = test_app_citizen();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/govos/citizen")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let dashboard: CitizenDashboard = body_json(resp).await;
        assert!(dashboard.available_services > 0);
        assert!(dashboard.service_categories.len() >= 4);
        assert_eq!(dashboard.filing_status.tax_year, "2025-2026");
    }

    #[tokio::test]
    async fn govos_console_serialization_roundtrip() {
        let dashboard = GovosConsoleDashboard {
            snapshot_at: Utc::now(),
            jurisdiction_id: "PK".to_string(),
            total_entities: 42,
            total_corridors: 3,
            total_attestations: 100,
            ministries: vec![MinistrySummary {
                ministry_id: "pk-mof".to_string(),
                name: "Ministry of Finance".to_string(),
                entity_count: 10,
                license_count: 5,
                compliance_rate: 85.0,
            }],
            aggregate_compliance_rate: 85.0,
        };
        let json = serde_json::to_string(&dashboard).unwrap();
        let deserialized: GovosConsoleDashboard = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_entities, 42);
        assert_eq!(deserialized.ministries.len(), 1);
    }

    #[test]
    fn router_builds_successfully() {
        let _router = router();
    }
}
