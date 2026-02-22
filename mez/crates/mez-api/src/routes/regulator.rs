//! # Regulator Console API
//!
//! Provides read-only query access for regulatory authorities
//! to monitor zone activity, compliance status, and audit trails.
//! Route structure based on apis/regulator-console.openapi.yaml.

use std::collections::BTreeMap;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::{require_role, CallerIdentity, Role};
use crate::error::AppError;
use crate::extractors::{extract_validated_json, Validate};
use crate::middleware::metrics::ApiMetrics;
use crate::state::{AppState, AssetComplianceStatus, AttestationRecord, AttestationStatus};
#[cfg(test)]
use crate::state::{AssetStatus, SmartAssetType};
use axum::extract::rejection::JsonRejection;

/// Query attestations request.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryAttestationsRequest {
    #[serde(default)]
    pub jurisdiction_id: Option<String>,
    #[serde(default)]
    pub entity_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub attestation_type: Option<String>,
    #[serde(default)]
    pub status: Option<AttestationStatus>,
    /// Maximum number of results to return (default: 100, max: 1000).
    #[serde(default)]
    pub limit: Option<usize>,
    /// Number of results to skip (default: 0).
    #[serde(default)]
    pub offset: Option<usize>,
}

const DEFAULT_QUERY_LIMIT: usize = 100;
const MAX_QUERY_LIMIT: usize = 1000;

impl Validate for QueryAttestationsRequest {
    fn validate(&self) -> Result<(), String> {
        if let Some(limit) = self.limit {
            if limit == 0 {
                return Err("limit must be >= 1".to_string());
            }
            if limit > MAX_QUERY_LIMIT {
                return Err(format!("limit must be <= {MAX_QUERY_LIMIT}"));
            }
        }
        if let Some(ref jid) = self.jurisdiction_id {
            if jid.len() > 255 {
                return Err("jurisdiction_id must not exceed 255 characters".to_string());
            }
        }
        if let Some(ref at) = self.attestation_type {
            if at.len() > 255 {
                return Err("attestation_type must not exceed 255 characters".to_string());
            }
        }
        Ok(())
    }
}

/// Query results response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QueryResultsResponse {
    /// Number of results in this page.
    pub count: usize,
    /// Total number of matching results (before pagination).
    pub total: usize,
    pub results: Vec<AttestationRecord>,
}

/// Compliance summary for regulator dashboard.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ComplianceSummary {
    pub total_entities: usize,
    pub total_corridors: usize,
    pub total_assets: usize,
    pub total_attestations: usize,
}

// ── Regulator Dashboard Types ───────────────────────────────────────────────

/// Comprehensive zone operational dashboard.
///
/// Assembles zone identity, compliance posture, corridor health,
/// agentic policy activity, and system health into a single response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RegulatorDashboard {
    /// Zone identity and operational status.
    pub zone: ZoneStatus,
    /// Compliance posture across all assets.
    pub compliance: CompliancePosture,
    /// Corridor health and activity.
    pub corridors: CorridorOverview,
    /// Recent agentic policy activity.
    pub policy_activity: PolicyActivity,
    /// System health indicators.
    pub health: SystemHealth,
}

/// Zone identity and counts.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZoneStatus {
    /// Zone identifier (from zone manifest, if bootstrapped).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_id: Option<String>,
    /// Jurisdiction identifier (from zone manifest, if bootstrapped).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jurisdiction_id: Option<String>,
    /// Zone operator's DID (from the zone signing key).
    pub zone_did: String,
    /// Zone operator's public key (hex).
    pub zone_public_key: String,
    /// Timestamp of this dashboard snapshot.
    pub snapshot_at: DateTime<Utc>,
    /// Number of applicable compliance domains (from zone manifest).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_count: Option<usize>,
    /// Applicable compliance domain names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicable_domains: Option<Vec<String>>,
    /// Number of unique entities known via attestations.
    pub entity_count: usize,
    /// Number of corridors.
    pub corridor_count: usize,
    /// Number of smart assets.
    pub asset_count: usize,
    /// Number of attestation records.
    pub attestation_count: usize,
}

/// Aggregate compliance posture across all assets.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CompliancePosture {
    /// Per-asset compliance summary.
    pub assets: Vec<AssetComplianceSnapshot>,
    /// Number of assets with all domains passing.
    pub fully_compliant_count: usize,
    /// Number of assets with at least one blocking domain.
    pub has_blocking_count: usize,
    /// Number of assets with all domains pending (or no status).
    pub all_pending_count: usize,
}

/// Per-asset compliance status snapshot for the regulator dashboard.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AssetComplianceSnapshot {
    /// Asset ID.
    pub asset_id: Uuid,
    /// Asset type (equity, bond, etc.).
    pub asset_type: String,
    /// Jurisdiction.
    pub jurisdiction_id: String,
    /// Last known compliance status.
    pub compliance_status: AssetComplianceStatus,
    /// When compliance was last evaluated (from asset metadata).
    pub last_evaluated: Option<String>,
}

/// Corridor health and activity overview.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CorridorOverview {
    /// Per-corridor status.
    pub corridors: Vec<CorridorStatus>,
    /// Count by typestate (e.g. {"ACTIVE": 3, "HALTED": 1}).
    #[schema(value_type = Object)]
    pub by_state: BTreeMap<String, usize>,
    /// Total receipts across all corridors.
    pub total_receipts: usize,
}

/// Per-corridor status snapshot.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CorridorStatus {
    /// Corridor ID.
    pub corridor_id: Uuid,
    /// Source jurisdiction.
    pub jurisdiction_a: String,
    /// Destination jurisdiction.
    pub jurisdiction_b: String,
    /// Current typestate (DRAFT, PENDING, ACTIVE, HALTED, SUSPENDED, DEPRECATED).
    pub state: String,
    /// Number of transitions in the corridor's log.
    pub transition_count: usize,
    /// Last transition timestamp (if any).
    pub last_transition: Option<DateTime<Utc>>,
    /// Receipt chain height (number of receipts), if receipt chain exists.
    pub receipt_chain_height: Option<u64>,
    /// Current MMR root hex, if receipt chain exists and non-empty.
    pub mmr_root: Option<String>,
}

/// Recent agentic policy activity.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PolicyActivity {
    /// Number of registered policies.
    pub policy_count: usize,
    /// Number of audit trail entries.
    pub audit_trail_size: usize,
    /// Most recent audit entries (up to 50).
    pub recent_entries: Vec<AuditEntrySummary>,
}

/// Audit trail entry summary.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuditEntrySummary {
    /// Entry type (trigger_received, policy_evaluated, action_scheduled, etc.).
    pub entry_type: String,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// Associated asset ID (if any).
    pub asset_id: Option<String>,
    /// Content digest of the entry (for tamper evidence).
    pub digest: Option<String>,
}

/// System health indicators.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SystemHealth {
    /// Corridors stuck in DRAFT for more than 7 days.
    pub stale_draft_corridors: usize,
    /// Corridors in HALTED state.
    pub halted_corridors: usize,
    /// Assets with compliance_status of `NonCompliant`.
    pub assets_with_blocking_compliance: usize,
    /// Whether the zone signing key is ephemeral (dev mode).
    pub zone_key_ephemeral: bool,
    /// API request count (from metrics middleware).
    pub total_requests: u64,
    /// API error count (from metrics middleware).
    pub total_errors: u64,
}

/// Per-entity compliance posture across all domains.
///
/// Returns the compliance status for a specific entity, aggregated
/// from attestation records and smart asset compliance evaluations.
/// This is the key endpoint for sovereign regulators to query entity
/// compliance status (Phase G #33: `GET /compliance/{entity_id}`).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EntityComplianceResponse {
    /// Entity ID queried.
    pub entity_id: Uuid,
    /// Jurisdiction of the entity (from first attestation, if available).
    pub jurisdiction_id: Option<String>,
    /// Number of attestations for this entity.
    pub attestation_count: usize,
    /// Per-domain attestation summary.
    pub domains: Vec<DomainComplianceStatus>,
    /// Assets owned by or associated with this entity.
    pub assets: Vec<EntityAssetCompliance>,
    /// Overall entity compliance state.
    pub overall_status: String,
    /// Per-corridor compliance summaries. Present when `?corridors=true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corridor_compliance: Option<Vec<CorridorComplianceStatus>>,
}

/// Compliance status for a single domain.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DomainComplianceStatus {
    /// Domain name (kyc, aml, sanctions, tax, etc.).
    pub domain: String,
    /// Most recent attestation status for this domain.
    pub status: String,
    /// Issuer of the most recent attestation.
    pub issuer: Option<String>,
    /// When the attestation was issued.
    pub issued_at: Option<DateTime<Utc>>,
    /// When the attestation expires (if applicable).
    pub expires_at: Option<DateTime<Utc>>,
}

/// Compliance status for an asset owned by the entity.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EntityAssetCompliance {
    /// Asset ID.
    pub asset_id: Uuid,
    /// Asset type.
    pub asset_type: String,
    /// Current compliance status.
    pub compliance_status: AssetComplianceStatus,
}

/// Per-corridor compliance status for cross-zone queries.
///
/// Returned when `?corridors=true` is passed to `GET /v1/compliance/{entity_id}`.
/// Summarizes the compliance posture visible through each corridor the entity's
/// zone participates in.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CorridorComplianceStatus {
    /// Corridor UUID.
    pub corridor_id: Uuid,
    /// Peer zone jurisdiction (the other end of the corridor).
    pub peer_zone: String,
    /// Corridor lifecycle state (ACTIVE, HALTED, etc.).
    pub corridor_state: String,
    /// Height of the receipt chain for this corridor (number of receipts).
    pub last_receipt_height: u64,
    /// Timestamp of the last checkpoint, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checkpoint_time: Option<DateTime<Utc>>,
    /// Current MMR root of the receipt chain (64-char hex), if non-empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mmr_root: Option<String>,
}

/// Query parameters for the entity compliance endpoint.
#[derive(Debug, Deserialize)]
pub struct EntityComplianceQuery {
    /// When `true`, include per-corridor compliance summaries in the response.
    #[serde(default)]
    pub corridors: Option<bool>,
}

/// Build the regulator router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/regulator/query/attestations", post(query_attestations))
        .route("/v1/regulator/summary", get(compliance_summary))
        .route("/v1/regulator/dashboard", get(dashboard))
        .route("/v1/compliance/{entity_id}", get(entity_compliance))
}

/// POST /v1/regulator/query/attestations — Query attestations.
#[utoipa::path(
    post,
    path = "/v1/regulator/query/attestations",
    request_body = QueryAttestationsRequest,
    responses(
        (status = 200, description = "Query results", body = QueryResultsResponse),
    ),
    tag = "regulator"
)]
async fn query_attestations(
    State(state): State<AppState>,
    caller: CallerIdentity,
    body: Result<Json<QueryAttestationsRequest>, JsonRejection>,
) -> Result<Json<QueryResultsResponse>, AppError> {
    require_role(&caller, Role::Regulator)?;
    let req = extract_validated_json(body)?;
    let all = state.attestations.list();
    let filtered: Vec<_> = all
        .into_iter()
        .filter(|a| {
            if let Some(ref jid) = req.jurisdiction_id {
                if a.jurisdiction_id != *jid {
                    return false;
                }
            }
            if let Some(ref eid) = req.entity_id {
                if a.entity_id != *eid {
                    return false;
                }
            }
            if let Some(ref at) = req.attestation_type {
                if a.attestation_type != *at {
                    return false;
                }
            }
            if let Some(ref s) = req.status {
                if a.status != *s {
                    return false;
                }
            }
            true
        })
        .collect();

    let total = filtered.len();
    let limit = req
        .limit
        .unwrap_or(DEFAULT_QUERY_LIMIT)
        .min(MAX_QUERY_LIMIT);
    let offset = req.offset.unwrap_or(0);
    let page: Vec<_> = filtered.into_iter().skip(offset).take(limit).collect();
    let count = page.len();
    Ok(Json(QueryResultsResponse {
        count,
        total,
        results: page,
    }))
}

/// GET /v1/regulator/summary — Compliance summary dashboard.
#[utoipa::path(
    get,
    path = "/v1/regulator/summary",
    responses(
        (status = 200, description = "Compliance summary", body = ComplianceSummary),
    ),
    tag = "regulator"
)]
async fn compliance_summary(
    State(state): State<AppState>,
    caller: CallerIdentity,
) -> Result<Json<ComplianceSummary>, AppError> {
    require_role(&caller, Role::Regulator)?;
    // Entity count is no longer stored locally — entities live in Mass APIs.
    // The regulator summary reports EZ-Stack-owned counts plus attestation-derived
    // entity count (unique entity_ids across attestations).
    let unique_entities: std::collections::HashSet<uuid::Uuid> = state
        .attestations
        .list()
        .iter()
        .map(|a| a.entity_id)
        .collect();

    Ok(Json(ComplianceSummary {
        total_entities: unique_entities.len(),
        total_corridors: state.corridors.list().len(),
        total_assets: state.smart_assets.list().len(),
        total_attestations: state.attestations.list().len(),
    }))
}

/// GET /v1/regulator/dashboard — Comprehensive zone operational dashboard.
///
/// Reads from all domain stores to assemble a complete picture of zone
/// health, compliance posture, corridor activity, and policy operations.
/// Read-only; computationally cheap (iterates in-memory stores).
#[utoipa::path(
    get,
    path = "/v1/regulator/dashboard",
    responses(
        (status = 200, description = "Zone operational dashboard", body = RegulatorDashboard),
    ),
    tag = "regulator"
)]
async fn dashboard(
    State(state): State<AppState>,
    caller: CallerIdentity,
    metrics: Option<Extension<ApiMetrics>>,
) -> Result<Json<RegulatorDashboard>, AppError> {
    require_role(&caller, Role::Regulator)?;
    let now = Utc::now();

    // ── Zone Status ─────────────────────────────────────────────
    let unique_entities: std::collections::HashSet<Uuid> = state
        .attestations
        .list()
        .iter()
        .map(|a| a.entity_id)
        .collect();

    let (zone_id, jurisdiction_id, domain_count, applicable_domains) = match &state.zone {
        Some(zc) => (
            Some(zc.zone_id.clone()),
            Some(zc.jurisdiction_id.clone()),
            Some(zc.applicable_domains.len()),
            Some(
                zc.applicable_domains
                    .iter()
                    .map(|d| d.as_str().to_string())
                    .collect(),
            ),
        ),
        None => (None, None, None, None),
    };

    let zone = ZoneStatus {
        zone_id,
        jurisdiction_id,
        zone_did: state.zone_did.clone(),
        zone_public_key: state.zone_signing_key.verifying_key().to_hex(),
        snapshot_at: now,
        domain_count,
        applicable_domains,
        entity_count: unique_entities.len(),
        corridor_count: state.corridors.len(),
        asset_count: state.smart_assets.len(),
        attestation_count: state.attestations.len(),
    };

    // ── Compliance Posture ──────────────────────────────────────
    let assets_list = state.smart_assets.list();
    let asset_statuses: Vec<AssetComplianceSnapshot> = assets_list
        .iter()
        .map(|a| AssetComplianceSnapshot {
            asset_id: a.id,
            asset_type: a.asset_type.to_string(),
            jurisdiction_id: a.jurisdiction_id.clone(),
            compliance_status: a.compliance_status,
            last_evaluated: a
                .metadata
                .get("last_evaluated")
                .and_then(|v| v.as_str())
                .map(String::from),
        })
        .collect();

    let fully_compliant_count = asset_statuses
        .iter()
        .filter(|a| a.compliance_status == AssetComplianceStatus::Compliant)
        .count();
    let has_blocking_count = asset_statuses
        .iter()
        .filter(|a| a.compliance_status == AssetComplianceStatus::NonCompliant)
        .count();
    let all_pending_count = asset_statuses
        .iter()
        .filter(|a| {
            matches!(
                a.compliance_status,
                AssetComplianceStatus::Pending | AssetComplianceStatus::Unevaluated
            )
        })
        .count();

    let compliance = CompliancePosture {
        assets: asset_statuses,
        fully_compliant_count,
        has_blocking_count,
        all_pending_count,
    };

    // ── Corridor Overview ───────────────────────────────────────
    let corridors_list = state.corridors.list();
    let mut by_state: BTreeMap<String, usize> = BTreeMap::new();
    let mut total_receipts: usize = 0;
    let mut corridor_statuses: Vec<CorridorStatus> = Vec::with_capacity(corridors_list.len());

    {
        let chains_guard = state.receipt_chains.read();
        for c in &corridors_list {
            *by_state.entry(c.state.as_str().to_string()).or_insert(0) += 1;

            let (chain_height, mmr_root) = match chains_guard.get(&c.id) {
                Some(chain) => {
                    let h = chain.height();
                    let root = chain.mmr_root().ok().filter(|s| !s.is_empty());
                    (Some(h), root)
                }
                None => (None, None),
            };

            if let Some(h) = chain_height {
                total_receipts += h as usize;
            }

            corridor_statuses.push(CorridorStatus {
                corridor_id: c.id,
                jurisdiction_a: c.jurisdiction_a.clone(),
                jurisdiction_b: c.jurisdiction_b.clone(),
                state: c.state.as_str().to_string(),
                transition_count: c.transition_log.len(),
                last_transition: c.transition_log.last().map(|t| t.timestamp),
                receipt_chain_height: chain_height,
                mmr_root,
            });
        }
    }

    let corridors_overview = CorridorOverview {
        corridors: corridor_statuses,
        by_state,
        total_receipts,
    };

    // ── Policy Activity ─────────────────────────────────────────
    let policy_activity = {
        let engine = state.policy_engine.lock();
        let recent = engine.audit_trail.last_n(50);
        PolicyActivity {
            policy_count: engine.policy_count(),
            audit_trail_size: engine.audit_trail.len(),
            recent_entries: recent
                .iter()
                .map(|entry| AuditEntrySummary {
                    entry_type: entry.entry_type.as_str().to_string(),
                    timestamp: entry.timestamp,
                    asset_id: entry.asset_id.clone(),
                    digest: entry.digest().map(|d| d.to_hex()),
                })
                .collect(),
        }
    };

    // ── System Health ───────────────────────────────────────────
    let seven_days_ago = now - chrono::Duration::days(7);
    let stale_drafts = corridors_list
        .iter()
        .filter(|c| c.state.as_str() == "DRAFT" && c.created_at < seven_days_ago)
        .count();
    let halted = corridors_list
        .iter()
        .filter(|c| c.state.as_str() == "HALTED")
        .count();

    let zone_key_ephemeral = match &state.zone {
        Some(zc) => zc.key_ephemeral,
        None => std::env::var("ZONE_SIGNING_KEY_HEX").is_err(),
    };

    let (total_requests, total_errors) = metrics
        .map(|Extension(m)| (m.requests(), m.errors()))
        .unwrap_or((0, 0));

    let health = SystemHealth {
        stale_draft_corridors: stale_drafts,
        halted_corridors: halted,
        assets_with_blocking_compliance: compliance.has_blocking_count,
        zone_key_ephemeral,
        total_requests,
        total_errors,
    };

    Ok(Json(RegulatorDashboard {
        zone,
        compliance,
        corridors: corridors_overview,
        policy_activity,
        health,
    }))
}

/// GET /v1/compliance/{entity_id} — Per-entity compliance query.
///
/// Returns the compliance posture for a specific entity across all domains.
/// Aggregates attestation records by domain, reports the most recent status
/// for each, and includes compliance status for any associated assets.
///
/// When `?corridors=true` is passed, includes per-corridor compliance
/// summaries showing receipt chain height, checkpoint times, and MMR roots
/// for all corridors the entity's zone participates in.
#[utoipa::path(
    get,
    path = "/v1/compliance/{entity_id}",
    params(
        ("entity_id" = Uuid, Path, description = "Entity identifier"),
        ("corridors" = Option<bool>, Query, description = "Include per-corridor compliance summaries")
    ),
    responses(
        (status = 200, description = "Entity compliance posture", body = EntityComplianceResponse),
        (status = 404, description = "Entity not found"),
    ),
    tag = "regulator"
)]
async fn entity_compliance(
    State(state): State<AppState>,
    caller: CallerIdentity,
    axum::extract::Path(entity_id): axum::extract::Path<Uuid>,
    axum::extract::Query(query): axum::extract::Query<EntityComplianceQuery>,
) -> Result<Json<EntityComplianceResponse>, AppError> {
    require_role(&caller, Role::Regulator)?;

    let all_attestations = state.attestations.list();
    let entity_attestations: Vec<_> = all_attestations
        .into_iter()
        .filter(|a| a.entity_id == entity_id)
        .collect();

    if entity_attestations.is_empty() {
        // Check if the entity has any assets even without attestations.
        let entity_assets: Vec<_> = state
            .smart_assets
            .list()
            .into_iter()
            .filter(|a| a.owner_entity_id.as_ref() == Some(&entity_id))
            .collect();

        if entity_assets.is_empty() {
            return Err(AppError::NotFound(format!(
                "No records found for entity {entity_id}"
            )));
        }
    }

    let jurisdiction_id = entity_attestations
        .first()
        .map(|a| a.jurisdiction_id.clone());

    // Group attestations by domain (attestation_type).
    let mut domain_map: std::collections::BTreeMap<String, Vec<&AttestationRecord>> =
        std::collections::BTreeMap::new();
    for att in &entity_attestations {
        domain_map
            .entry(att.attestation_type.clone())
            .or_default()
            .push(att);
    }

    // For each domain, pick the most recent attestation.
    let mut domains: Vec<DomainComplianceStatus> = domain_map
        .into_iter()
        .filter_map(|(domain, mut atts)| {
            atts.sort_by(|a, b| b.issued_at.cmp(&a.issued_at));
            let latest = atts.first()?;
            Some(DomainComplianceStatus {
                domain,
                status: format!("{}", latest.status),
                issuer: Some(latest.issuer.clone()),
                issued_at: Some(latest.issued_at),
                expires_at: latest.expires_at,
            })
        })
        .collect();
    domains.sort_by(|a, b| a.domain.cmp(&b.domain));

    // Entity assets.
    let entity_assets: Vec<EntityAssetCompliance> = state
        .smart_assets
        .list()
        .into_iter()
        .filter(|a| a.owner_entity_id.as_ref() == Some(&entity_id))
        .map(|a| EntityAssetCompliance {
            asset_id: a.id,
            asset_type: a.asset_type.to_string(),
            compliance_status: a.compliance_status,
        })
        .collect();

    // Determine overall status.
    let has_blocking = entity_assets
        .iter()
        .any(|a| a.compliance_status == AssetComplianceStatus::NonCompliant);
    let all_active = entity_attestations
        .iter()
        .all(|a| a.status == AttestationStatus::Active);
    let overall_status = if has_blocking {
        "non_compliant"
    } else if all_active && !entity_attestations.is_empty() {
        "compliant"
    } else {
        "pending"
    };

    // Corridor compliance (when ?corridors=true).
    let corridor_compliance = if query.corridors.unwrap_or(false) {
        let corridors_list = state.corridors.list();
        let chains_guard = state.receipt_chains.read();
        let entity_jurisdiction = jurisdiction_id.as_deref();

        let mut statuses = Vec::new();
        for c in &corridors_list {
            // Include corridors where the entity's jurisdiction matches either side.
            let matches = entity_jurisdiction.map_or(true, |jid| {
                c.jurisdiction_a == jid || c.jurisdiction_b == jid
            });
            if !matches {
                continue;
            }

            let peer_zone = entity_jurisdiction
                .map(|jid| {
                    if c.jurisdiction_a == jid {
                        c.jurisdiction_b.clone()
                    } else {
                        c.jurisdiction_a.clone()
                    }
                })
                .unwrap_or_else(|| {
                    format!("{} / {}", c.jurisdiction_a, c.jurisdiction_b)
                });

            let (height, mmr_root) = match chains_guard.get(&c.id) {
                Some(chain) => {
                    let h = chain.height();
                    let root = chain.mmr_root().ok().filter(|s| !s.is_empty());
                    (h, root)
                }
                None => (0, None),
            };

            let last_checkpoint_time = chains_guard
                .get(&c.id)
                .and_then(|chain| chain.checkpoints().last())
                .map(|cp| cp.timestamp.as_datetime().to_owned());

            statuses.push(CorridorComplianceStatus {
                corridor_id: c.id,
                peer_zone,
                corridor_state: c.state.as_str().to_string(),
                last_receipt_height: height,
                last_checkpoint_time,
                mmr_root,
            });
        }
        Some(statuses)
    } else {
        None
    };

    Ok(Json(EntityComplianceResponse {
        entity_id,
        jurisdiction_id,
        attestation_count: entity_attestations.len(),
        domains,
        assets: entity_assets,
        overall_status: overall_status.to_string(),
        corridor_compliance,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::CallerIdentity;
    use crate::extractors::Validate;

    // ── QueryAttestationsRequest validation ───────────────────────

    #[test]
    fn test_query_attestations_request_valid_empty() {
        let req = QueryAttestationsRequest {
            jurisdiction_id: None,
            entity_id: None,
            attestation_type: None,
            status: None,
            limit: None,
            offset: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_query_attestations_request_valid_with_filters() {
        let req = QueryAttestationsRequest {
            jurisdiction_id: Some("PK-PEZ".to_string()),
            entity_id: Some(uuid::Uuid::new_v4()),
            attestation_type: Some("identity_verification".to_string()),
            status: Some(AttestationStatus::Active),
            limit: None,
            offset: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_query_attestations_request_valid_partial_filters() {
        let req = QueryAttestationsRequest {
            jurisdiction_id: Some("AE-DIFC".to_string()),
            entity_id: None,
            attestation_type: None,
            status: Some(AttestationStatus::Pending),
            limit: None,
            offset: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_query_attestations_request_limit_too_high() {
        let req = QueryAttestationsRequest {
            jurisdiction_id: None,
            entity_id: None,
            attestation_type: None,
            status: None,
            limit: Some(5000),
            offset: None,
        };
        assert!(req.validate().is_err());
    }

    // ── Router construction ───────────────────────────────────────

    #[test]
    fn test_router_builds_successfully() {
        let _router = router();
    }

    // ── Handler integration tests ──────────────────────────────────

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    /// A zone admin identity for tests that need full access.
    fn zone_admin() -> CallerIdentity {
        CallerIdentity {
            role: Role::ZoneAdmin,
            entity_id: None,
            jurisdiction_id: None,
        }
    }

    /// Helper: build the regulator router with a fresh AppState and
    /// ZoneAdmin identity injected for full access.
    fn test_app() -> Router<()> {
        router()
            .layer(axum::Extension(zone_admin()))
            .with_state(AppState::new())
    }

    /// Helper: build the router with shared state and ZoneAdmin identity.
    fn test_app_with_state(state: AppState) -> Router<()> {
        router()
            .layer(axum::Extension(zone_admin()))
            .with_state(state)
    }

    /// Helper: read the response body as bytes and deserialize from JSON.
    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn handler_query_attestations_empty_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(r#"{}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: QueryResultsResponse = body_json(resp).await;
        assert_eq!(result.count, 0);
        assert!(result.results.is_empty());
    }

    #[tokio::test]
    async fn handler_query_attestations_with_filters_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_id":"PK-PEZ","status":"ACTIVE"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: QueryResultsResponse = body_json(resp).await;
        assert_eq!(result.count, 0);
    }

    #[tokio::test]
    async fn handler_query_attestations_filters_matching_records() {
        let state = AppState::new();

        // Seed the attestations store directly.
        let att1 = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: uuid::Uuid::new_v4(),
            attestation_type: "identity_verification".to_string(),
            issuer: "NADRA".to_string(),
            status: AttestationStatus::Active,
            jurisdiction_id: "PK-PEZ".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        let att2 = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: uuid::Uuid::new_v4(),
            attestation_type: "compliance_check".to_string(),
            issuer: "FBR".to_string(),
            status: AttestationStatus::Pending,
            jurisdiction_id: "AE-DIFC".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        state.attestations.insert(att1.id, att1.clone());
        state.attestations.insert(att2.id, att2.clone());

        let app = test_app_with_state(state.clone());

        // Query filtering by jurisdiction_id.
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"jurisdiction_id":"PK-PEZ"}"#))
            .unwrap();

        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: QueryResultsResponse = body_json(resp).await;
        assert_eq!(result.count, 1);
        assert_eq!(result.results[0].jurisdiction_id, "PK-PEZ");

        // Query with no filters returns all.
        let req_all = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(r#"{}"#))
            .unwrap();
        let resp_all = app.oneshot(req_all).await.unwrap();
        let result_all: QueryResultsResponse = body_json(resp_all).await;
        assert_eq!(result_all.count, 2);
    }

    #[tokio::test]
    async fn handler_compliance_summary_empty_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/regulator/summary")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let summary: ComplianceSummary = body_json(resp).await;
        assert_eq!(summary.total_entities, 0);
        assert_eq!(summary.total_corridors, 0);
        assert_eq!(summary.total_assets, 0);
        assert_eq!(summary.total_attestations, 0);
    }

    #[tokio::test]
    async fn handler_compliance_summary_reflects_state() {
        let state = AppState::new();

        // Entity count is now derived from unique entity_ids in attestations.
        // Add an attestation to represent an entity known to the EZ Stack.
        let entity_id = uuid::Uuid::new_v4();
        let att = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id,
            attestation_type: "kyc".to_string(),
            issuer: "NADRA".to_string(),
            status: AttestationStatus::Active,
            jurisdiction_id: "PK-PEZ".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        state.attestations.insert(att.id, att);

        let corridor = crate::state::CorridorRecord {
            id: uuid::Uuid::new_v4(),
            jurisdiction_a: "PK-PEZ".to_string(),
            jurisdiction_b: "AE-DIFC".to_string(),
            state: mez_state::DynCorridorState::Active,
            transition_log: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        state.corridors.insert(corridor.id, corridor);

        let app = test_app_with_state(state.clone());

        let req = Request::builder()
            .method("GET")
            .uri("/v1/regulator/summary")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let summary: ComplianceSummary = body_json(resp).await;
        assert_eq!(summary.total_entities, 1);
        assert_eq!(summary.total_corridors, 1);
        assert_eq!(summary.total_assets, 0);
        assert_eq!(summary.total_attestations, 1);
    }

    // ── Additional regulator route tests ─────────────────────────

    #[tokio::test]
    async fn handler_query_attestations_invalid_json_returns_422() {
        // BUG-038: JSON parse errors now return 422 (Unprocessable Entity).
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(r#"not valid json"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_query_attestations_filter_by_entity_id() {
        let state = AppState::new();
        let target_entity = uuid::Uuid::new_v4();

        let att1 = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: target_entity,
            attestation_type: "kyc".to_string(),
            issuer: "NADRA".to_string(),
            status: AttestationStatus::Active,
            jurisdiction_id: "PK-PEZ".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        let att2 = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: uuid::Uuid::new_v4(),
            attestation_type: "kyc".to_string(),
            issuer: "FBR".to_string(),
            status: AttestationStatus::Active,
            jurisdiction_id: "PK-PEZ".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        state.attestations.insert(att1.id, att1);
        state.attestations.insert(att2.id, att2);

        let app = test_app_with_state(state);
        let body = serde_json::json!({ "entity_id": target_entity });
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let result: QueryResultsResponse = body_json(resp).await;
        assert_eq!(result.count, 1);
        assert_eq!(result.results[0].entity_id, target_entity);
    }

    #[tokio::test]
    async fn handler_query_attestations_filter_by_attestation_type() {
        let state = AppState::new();

        let att1 = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: uuid::Uuid::new_v4(),
            attestation_type: "identity_verification".to_string(),
            issuer: "NADRA".to_string(),
            status: AttestationStatus::Active,
            jurisdiction_id: "PK-PEZ".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        let att2 = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: uuid::Uuid::new_v4(),
            attestation_type: "compliance_check".to_string(),
            issuer: "FBR".to_string(),
            status: AttestationStatus::Active,
            jurisdiction_id: "PK-PEZ".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        state.attestations.insert(att1.id, att1);
        state.attestations.insert(att2.id, att2);

        let app = test_app_with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"attestation_type":"compliance_check"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let result: QueryResultsResponse = body_json(resp).await;
        assert_eq!(result.count, 1);
        assert_eq!(result.results[0].attestation_type, "compliance_check");
    }

    #[tokio::test]
    async fn handler_query_attestations_combined_filters() {
        let state = AppState::new();

        for i in 0..5 {
            let att = AttestationRecord {
                id: uuid::Uuid::new_v4(),
                entity_id: uuid::Uuid::new_v4(),
                attestation_type: if i % 2 == 0 { "kyc" } else { "aml" }.to_string(),
                issuer: "NADRA".to_string(),
                status: if i < 3 {
                    AttestationStatus::Active
                } else {
                    AttestationStatus::Pending
                },
                jurisdiction_id: if i < 2 { "PK-PEZ" } else { "AE-DIFC" }.to_string(),
                issued_at: chrono::Utc::now(),
                expires_at: None,
                details: serde_json::json!({}),
            };
            state.attestations.insert(att.id, att);
        }

        let app = test_app_with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_id":"PK-PEZ","status":"ACTIVE"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let result: QueryResultsResponse = body_json(resp).await;
        // PK-PEZ (indices 0,1) and ACTIVE (indices 0,1,2) → intersection = indices 0,1
        assert_eq!(result.count, 2);
    }

    #[tokio::test]
    async fn handler_compliance_summary_counts_assets_and_attestations() {
        let state = AppState::new();

        // Add a smart asset
        let asset = crate::state::SmartAssetRecord {
            id: uuid::Uuid::new_v4(),
            asset_type: SmartAssetType::new("CapTable").expect("valid"),
            jurisdiction_id: "PK-PEZ".to_string(),
            status: AssetStatus::Active,
            genesis_digest: None,
            compliance_status: AssetComplianceStatus::Compliant,
            metadata: serde_json::json!({}),
            owner_entity_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        state.smart_assets.insert(asset.id, asset);

        // Add an attestation
        let att = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: uuid::Uuid::new_v4(),
            attestation_type: "kyc".to_string(),
            issuer: "NADRA".to_string(),
            status: AttestationStatus::Active,
            jurisdiction_id: "PK-PEZ".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        state.attestations.insert(att.id, att);

        let app = test_app_with_state(state);
        let req = Request::builder()
            .method("GET")
            .uri("/v1/regulator/summary")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let summary: ComplianceSummary = body_json(resp).await;
        assert_eq!(summary.total_assets, 1);
        assert_eq!(summary.total_attestations, 1);
    }

    #[test]
    fn compliance_summary_serialization() {
        let summary = ComplianceSummary {
            total_entities: 10,
            total_corridors: 3,
            total_assets: 25,
            total_attestations: 100,
        };
        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: ComplianceSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_entities, 10);
        assert_eq!(deserialized.total_corridors, 3);
        assert_eq!(deserialized.total_assets, 25);
        assert_eq!(deserialized.total_attestations, 100);
    }

    #[test]
    fn query_results_response_serialization() {
        let resp = QueryResultsResponse {
            count: 0,
            total: 0,
            results: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: QueryResultsResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.count, 0);
        assert!(deserialized.results.is_empty());
    }

    // ── Dashboard tests ──────────────────────────────────────────

    #[tokio::test]
    async fn dashboard_empty_zone_returns_zeros() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/regulator/dashboard")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let db: RegulatorDashboard = body_json(resp).await;

        // Zone identity is present.
        assert!(db.zone.zone_did.starts_with("did:mass:zone:"));
        assert_eq!(db.zone.zone_public_key.len(), 64);
        assert_eq!(db.zone.entity_count, 0);
        assert_eq!(db.zone.corridor_count, 0);
        assert_eq!(db.zone.asset_count, 0);
        assert_eq!(db.zone.attestation_count, 0);

        // Compliance posture is empty.
        assert!(db.compliance.assets.is_empty());
        assert_eq!(db.compliance.fully_compliant_count, 0);
        assert_eq!(db.compliance.has_blocking_count, 0);
        assert_eq!(db.compliance.all_pending_count, 0);

        // Corridors are empty.
        assert!(db.corridors.corridors.is_empty());
        assert!(db.corridors.by_state.is_empty());
        assert_eq!(db.corridors.total_receipts, 0);

        // Policy engine has extended policies registered by default.
        assert!(db.policy_activity.policy_count > 0);

        // Health is clean.
        assert_eq!(db.health.stale_draft_corridors, 0);
        assert_eq!(db.health.halted_corridors, 0);
        assert_eq!(db.health.assets_with_blocking_compliance, 0);
        assert!(db.health.zone_key_ephemeral); // no ZONE_SIGNING_KEY_HEX in test env
    }

    #[tokio::test]
    async fn dashboard_reflects_populated_state() {
        let state = AppState::new();
        let now = chrono::Utc::now();

        // 2 entities (via attestations with distinct entity_ids).
        let entity1 = uuid::Uuid::new_v4();
        let entity2 = uuid::Uuid::new_v4();
        for (eid, atype) in [(entity1, "kyc"), (entity2, "aml")] {
            let att = AttestationRecord {
                id: uuid::Uuid::new_v4(),
                entity_id: eid,
                attestation_type: atype.to_string(),
                issuer: "NADRA".to_string(),
                status: AttestationStatus::Active,
                jurisdiction_id: "PK-PEZ".to_string(),
                issued_at: now,
                expires_at: None,
                details: serde_json::json!({}),
            };
            state.attestations.insert(att.id, att);
        }

        // 3 corridors: 1 ACTIVE, 1 HALTED, 1 DRAFT.
        for (cs, ja, jb) in [
            (mez_state::DynCorridorState::Active, "PK-PEZ", "AE-DIFC"),
            (mez_state::DynCorridorState::Halted, "PK-PEZ", "SA-NEOM"),
            (mez_state::DynCorridorState::Draft, "AE-DIFC", "SA-NEOM"),
        ] {
            let c = crate::state::CorridorRecord {
                id: uuid::Uuid::new_v4(),
                jurisdiction_a: ja.to_string(),
                jurisdiction_b: jb.to_string(),
                state: cs,
                transition_log: vec![],
                created_at: now,
                updated_at: now,
            };
            state.corridors.insert(c.id, c);
        }

        // 2 assets: 1 compliant, 1 non_compliant.
        let compliant_asset = crate::state::SmartAssetRecord {
            id: uuid::Uuid::new_v4(),
            asset_type: SmartAssetType::new("equity").expect("valid"),
            jurisdiction_id: "PK-PEZ".to_string(),
            status: AssetStatus::Active,
            genesis_digest: None,
            compliance_status: AssetComplianceStatus::Compliant,
            metadata: serde_json::json!({}),
            owner_entity_id: None,
            created_at: now,
            updated_at: now,
        };
        state
            .smart_assets
            .insert(compliant_asset.id, compliant_asset);

        let blocking_asset = crate::state::SmartAssetRecord {
            id: uuid::Uuid::new_v4(),
            asset_type: SmartAssetType::new("bond").expect("valid"),
            jurisdiction_id: "AE-DIFC".to_string(),
            status: AssetStatus::Active,
            genesis_digest: None,
            compliance_status: AssetComplianceStatus::NonCompliant,
            metadata: serde_json::json!({}),
            owner_entity_id: None,
            created_at: now,
            updated_at: now,
        };
        state.smart_assets.insert(blocking_asset.id, blocking_asset);

        let app = test_app_with_state(state);
        let req = Request::builder()
            .method("GET")
            .uri("/v1/regulator/dashboard")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let db: RegulatorDashboard = body_json(resp).await;

        assert_eq!(db.zone.entity_count, 2);
        assert_eq!(db.zone.corridor_count, 3);
        assert_eq!(db.zone.asset_count, 2);
        assert_eq!(db.zone.attestation_count, 2);

        assert_eq!(db.corridors.by_state.get("ACTIVE"), Some(&1));
        assert_eq!(db.corridors.by_state.get("HALTED"), Some(&1));
        assert_eq!(db.corridors.by_state.get("DRAFT"), Some(&1));

        assert_eq!(db.compliance.fully_compliant_count, 1);
        assert_eq!(db.compliance.has_blocking_count, 1);
        assert_eq!(db.compliance.all_pending_count, 0);
        assert_eq!(db.compliance.assets.len(), 2);

        assert_eq!(db.health.halted_corridors, 1);
        assert_eq!(db.health.assets_with_blocking_compliance, 1);
    }

    #[tokio::test]
    async fn dashboard_detects_stale_draft_corridor() {
        let state = AppState::new();
        let now = chrono::Utc::now();

        // Stale: DRAFT created 8 days ago.
        let stale = crate::state::CorridorRecord {
            id: uuid::Uuid::new_v4(),
            jurisdiction_a: "PK-PEZ".to_string(),
            jurisdiction_b: "AE-DIFC".to_string(),
            state: mez_state::DynCorridorState::Draft,
            transition_log: vec![],
            created_at: now - chrono::Duration::days(8),
            updated_at: now - chrono::Duration::days(8),
        };
        state.corridors.insert(stale.id, stale);

        // Fresh: DRAFT created today.
        let fresh = crate::state::CorridorRecord {
            id: uuid::Uuid::new_v4(),
            jurisdiction_a: "AE-DIFC".to_string(),
            jurisdiction_b: "SA-NEOM".to_string(),
            state: mez_state::DynCorridorState::Draft,
            transition_log: vec![],
            created_at: now,
            updated_at: now,
        };
        state.corridors.insert(fresh.id, fresh);

        let app = test_app_with_state(state);
        let req = Request::builder()
            .method("GET")
            .uri("/v1/regulator/dashboard")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let db: RegulatorDashboard = body_json(resp).await;
        assert_eq!(db.health.stale_draft_corridors, 1);
        assert_eq!(db.corridors.by_state.get("DRAFT"), Some(&2));
    }

    #[tokio::test]
    async fn dashboard_policy_activity_after_trigger() {
        let state = AppState::new();

        // Fire a trigger through the policy engine directly.
        {
            let mut engine = state.policy_engine.lock();
            let trigger = mez_agentic::Trigger::new(
                mez_agentic::TriggerType::SanctionsListUpdate,
                serde_json::json!({"affected_parties": ["self"]}),
            );
            let _ = engine.process_trigger(&trigger, "asset-123", None);
        }

        let app = test_app_with_state(state);
        let req = Request::builder()
            .method("GET")
            .uri("/v1/regulator/dashboard")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let db: RegulatorDashboard = body_json(resp).await;
        assert!(db.policy_activity.audit_trail_size > 0);
        assert!(!db.policy_activity.recent_entries.is_empty());

        // At least one entry should be a trigger_received.
        let has_trigger = db
            .policy_activity
            .recent_entries
            .iter()
            .any(|e| e.entry_type == "trigger_received");
        assert!(has_trigger, "expected a trigger_received audit entry");

        // Each entry with a digest should have a 64-char hex string.
        for entry in &db.policy_activity.recent_entries {
            if let Some(ref d) = entry.digest {
                assert_eq!(d.len(), 64, "digest should be 64 hex chars");
                assert!(
                    d.chars().all(|c| c.is_ascii_hexdigit()),
                    "digest should be valid hex"
                );
            }
        }
    }

    #[tokio::test]
    async fn dashboard_corridor_receipt_chains_default_to_none() {
        let state = AppState::new();
        let now = chrono::Utc::now();

        // Corridor with no receipt chain entry.
        let c = crate::state::CorridorRecord {
            id: uuid::Uuid::new_v4(),
            jurisdiction_a: "PK-PEZ".to_string(),
            jurisdiction_b: "AE-DIFC".to_string(),
            state: mez_state::DynCorridorState::Active,
            transition_log: vec![],
            created_at: now,
            updated_at: now,
        };
        state.corridors.insert(c.id, c.clone());

        let app = test_app_with_state(state);
        let req = Request::builder()
            .method("GET")
            .uri("/v1/regulator/dashboard")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let db: RegulatorDashboard = body_json(resp).await;
        assert_eq!(db.corridors.corridors.len(), 1);

        let cs = &db.corridors.corridors[0];
        assert_eq!(cs.corridor_id, c.id);
        assert_eq!(cs.state, "ACTIVE");
        assert!(cs.receipt_chain_height.is_none());
        assert!(cs.mmr_root.is_none());
        assert_eq!(db.corridors.total_receipts, 0);
    }
}
