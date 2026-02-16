//! # Tax Collection Pipeline API
//!
//! HTTP surface for the tax collection pipeline. Implements:
//!
//! - **POST `/v1/tax/events`** — Record a tax event from an observed economic activity
//! - **GET `/v1/tax/events`** — List tax events with filtering
//! - **GET `/v1/tax/events/:id`** — Get a specific tax event by ID
//! - **POST `/v1/tax/withhold`** — Compute withholding for a tax event
//! - **GET `/v1/tax/obligations/:entity_id`** — Get tax obligations for an entity
//! - **POST `/v1/tax/report`** — Generate a tax report for FBR IRIS submission
//! - **GET `/v1/tax/rules`** — List loaded withholding rules for a jurisdiction
//!
//! ## Architecture
//!
//! The tax pipeline is SEZ-Stack-owned: it provides jurisdictional tax awareness
//! on top of Mass fiscal operations. Tax events are generated when Mass fiscal
//! activity is observed, withholding is computed using jurisdiction-specific rules
//! from regpacks, and reports are generated for tax authority submission.
//!
//! This module does NOT duplicate Mass fiscal CRUD. Payments, accounts, and
//! transaction records live in Mass treasury-info API and are accessed via
//! `msez-mass-client`.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use msez_agentic::tax::{
    self, format_amount, parse_amount, FilerStatus, TaxEvent, TaxEventType, WithholdingResult,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use crate::extractors::{extract_validated_json, Validate};
use crate::state::{AppState, TaxEventRecord};
use axum::extract::rejection::JsonRejection;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Request to record a tax event.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTaxEventRequest {
    /// Entity subject to the tax event.
    pub entity_id: Uuid,
    /// Event type (e.g., "payment_for_goods", "salary_payment").
    pub event_type: String,
    /// Jurisdiction where the obligation arises (e.g., "PK").
    pub jurisdiction_id: String,
    /// Gross amount of the economic activity.
    pub gross_amount: String,
    /// Currency code (ISO 4217).
    pub currency: String,
    /// Tax year (e.g., "2025-2026").
    pub tax_year: String,
    /// Entity NTN, if registered.
    pub ntn: Option<String>,
    /// Filing status: "filer", "late_filer", "non_filer".
    pub filer_status: Option<String>,
    /// Statutory section reference.
    pub statutory_section: Option<String>,
    /// Reference to Mass payment ID.
    pub mass_payment_id: Option<Uuid>,
    /// Counterparty entity ID.
    pub counterparty_entity_id: Option<Uuid>,
    /// Additional metadata.
    pub metadata: Option<serde_json::Value>,
}

impl Validate for CreateTaxEventRequest {
    fn validate(&self) -> Result<(), String> {
        if self.jurisdiction_id.trim().is_empty() {
            return Err("jurisdiction_id must not be empty".to_string());
        }
        if self.jurisdiction_id.len() > 10 {
            return Err("jurisdiction_id must not exceed 10 characters".to_string());
        }
        if self.gross_amount.trim().is_empty() {
            return Err("gross_amount must not be empty".to_string());
        }
        // Validate gross_amount is a parseable non-negative number.
        let trimmed = self.gross_amount.trim();
        match trimmed.parse::<f64>() {
            Ok(v) if v < 0.0 => {
                return Err("gross_amount must not be negative".to_string());
            }
            Err(_) => {
                return Err("gross_amount must be a valid number".to_string());
            }
            _ => {}
        }
        if self.currency.trim().is_empty() || self.currency.len() > 5 {
            return Err("currency must be 1-5 characters".to_string());
        }
        if self.tax_year.trim().is_empty() || self.tax_year.len() > 20 {
            return Err("tax_year must be 1-20 characters".to_string());
        }
        if self.event_type.trim().is_empty() {
            return Err("event_type must not be empty".to_string());
        }
        if self.event_type.len() > 100 {
            return Err("event_type must not exceed 100 characters".to_string());
        }
        // Validate NTN format if provided (7 digits).
        if let Some(ref ntn) = self.ntn {
            if ntn.len() != 7 || !ntn.chars().all(|c| c.is_ascii_digit()) {
                return Err("ntn must be exactly 7 digits".to_string());
            }
        }
        // Validate filer_status if provided.
        if let Some(ref fs) = self.filer_status {
            if !matches!(fs.as_str(), "filer" | "late_filer" | "non_filer") {
                return Err("filer_status must be one of: filer, late_filer, non_filer".to_string());
            }
        }
        Ok(())
    }
}

/// Request to generate a tax report.
#[derive(Debug, Deserialize, ToSchema)]
pub struct GenerateReportRequest {
    /// Entity this report covers.
    pub entity_id: Uuid,
    /// Entity NTN.
    pub ntn: Option<String>,
    /// Jurisdiction.
    pub jurisdiction_id: String,
    /// Tax year.
    pub tax_year: String,
    /// Report period start (YYYY-MM-DD).
    pub period_start: String,
    /// Report period end (YYYY-MM-DD).
    pub period_end: String,
    /// Report type (e.g., "monthly_withholding", "annual_return").
    pub report_type: String,
}

impl Validate for GenerateReportRequest {
    fn validate(&self) -> Result<(), String> {
        if self.jurisdiction_id.trim().is_empty() {
            return Err("jurisdiction_id must not be empty".to_string());
        }
        if self.tax_year.trim().is_empty() {
            return Err("tax_year must not be empty".to_string());
        }
        if self.period_start.trim().is_empty() {
            return Err("period_start must not be empty".to_string());
        }
        if self.period_end.trim().is_empty() {
            return Err("period_end must not be empty".to_string());
        }
        if self.report_type.trim().is_empty() {
            return Err("report_type must not be empty".to_string());
        }
        Ok(())
    }
}

/// Query parameters for listing tax events.
#[derive(Debug, Deserialize, Default)]
pub struct TaxEventQueryParams {
    /// Filter by entity ID.
    pub entity_id: Option<Uuid>,
    /// Filter by jurisdiction.
    pub jurisdiction_id: Option<String>,
    /// Filter by tax year.
    pub tax_year: Option<String>,
    /// Maximum number of items to return (default: 100, max: 1000).
    pub limit: Option<usize>,
    /// Number of items to skip (default: 0).
    pub offset: Option<usize>,
}

/// Query parameters for listing withholding rules.
#[derive(Debug, Deserialize, Default)]
pub struct RulesQueryParams {
    /// Jurisdiction to list rules for (e.g., "PK").
    pub jurisdiction_id: Option<String>,
}

/// Response for a tax event with withholding details.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TaxEventResponse {
    /// The persisted tax event record.
    pub event: TaxEventRecord,
    /// Withholding results computed by the pipeline.
    pub withholdings: Vec<WithholdingResultResponse>,
}

/// Simplified withholding result for API responses.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WithholdingResultResponse {
    pub rule_id: String,
    pub rate_percent: String,
    pub withholding_amount: String,
    pub net_amount: String,
    pub statutory_section: String,
    pub tax_category: String,
    pub is_final_tax: bool,
}

impl From<&WithholdingResult> for WithholdingResultResponse {
    fn from(r: &WithholdingResult) -> Self {
        Self {
            rule_id: r.rule_id.clone(),
            rate_percent: r.rate_percent.clone(),
            withholding_amount: r.withholding_amount.clone(),
            net_amount: r.net_amount.clone(),
            statutory_section: r.statutory_section.clone(),
            tax_category: r.tax_category.to_string(),
            is_final_tax: r.is_final_tax,
        }
    }
}

/// Response for the report generation endpoint.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TaxReportResponse {
    pub report_id: Uuid,
    pub entity_id: Uuid,
    pub jurisdiction_id: String,
    pub tax_year: String,
    pub period_start: String,
    pub period_end: String,
    pub report_type: String,
    pub total_gross: String,
    pub total_withholding: String,
    pub currency: String,
    pub event_count: usize,
    pub line_item_count: usize,
    pub status: String,
    pub generated_at: String,
}

/// Summary of tax obligations for an entity.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TaxObligationSummary {
    pub entity_id: Uuid,
    pub jurisdiction_id: String,
    pub total_events: usize,
    pub total_gross: String,
    pub total_withholding: String,
    pub currency: String,
    pub by_category: Vec<CategorySummary>,
}

/// Per-category tax obligation summary.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CategorySummary {
    pub tax_category: String,
    pub event_count: usize,
    pub total_gross: String,
    pub total_withholding: String,
}

/// Withholding rule as returned by the API.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WithholdingRuleResponse {
    pub rule_id: String,
    pub applicable_event_types: Vec<String>,
    pub applicable_filer_status: Vec<String>,
    pub tax_category: String,
    pub rate_percent: String,
    pub statutory_section: String,
    pub description: String,
    pub effective_from: String,
    pub is_final_tax: bool,
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Construct the tax collection pipeline router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/tax/events",
            get(list_tax_events).post(create_tax_event),
        )
        .route("/v1/tax/events/{id}", get(get_tax_event))
        .route("/v1/tax/withhold", post(compute_withholding))
        .route("/v1/tax/obligations/{entity_id}", get(get_tax_obligations))
        .route("/v1/tax/report", post(generate_tax_report))
        .route("/v1/tax/rules", get(list_withholding_rules))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /v1/tax/events — Record a tax event and compute withholding.
///
/// Observes an economic activity, classifies it as a tax event, runs it
/// through the withholding pipeline, and persists the result.
async fn create_tax_event(
    State(state): State<AppState>,
    body: Result<Json<CreateTaxEventRequest>, JsonRejection>,
) -> Result<Json<TaxEventResponse>, AppError> {
    let req = extract_validated_json(body)?;

    let event_type = parse_event_type(&req.event_type)?;
    let filer_status = parse_filer_status(req.filer_status.as_deref())?;

    let mut event = TaxEvent::new(
        req.entity_id,
        event_type,
        &req.jurisdiction_id,
        &req.gross_amount,
        &req.currency,
        &req.tax_year,
    );

    if let Some(ref ntn) = req.ntn {
        event = event.with_ntn(ntn, filer_status);
    } else {
        event.filer_status = filer_status;
    }

    if let Some(ref section) = req.statutory_section {
        event = event.with_statutory_section(section);
    }

    if let Some(pay_id) = req.mass_payment_id {
        event = event.with_mass_payment(pay_id);
    }

    if let Some(counter_id) = req.counterparty_entity_id {
        event = event.with_counterparty(counter_id);
    }

    if let Some(ref meta) = req.metadata {
        event = event.with_metadata(meta.clone());
    }

    // Run through the withholding pipeline.
    let withholdings = {
        let pipeline = state.tax_pipeline.lock();
        pipeline.process_event(&event)
    };

    // Compute totals from withholding results.
    let (total_wht, total_net) = aggregate_withholdings(&withholdings, &req.gross_amount);

    let record = TaxEventRecord {
        id: event.event_id,
        entity_id: event.entity_id,
        event_type: event.event_type.to_string(),
        tax_category: event.tax_category.to_string(),
        jurisdiction_id: event.jurisdiction_id.clone(),
        gross_amount: event.gross_amount.clone(),
        withholding_amount: total_wht,
        net_amount: total_net,
        currency: event.currency.clone(),
        tax_year: event.tax_year.clone(),
        ntn: event.ntn.clone(),
        filer_status: event.filer_status.to_string(),
        statutory_section: event.statutory_section.clone(),
        withholding_executed: false,
        mass_payment_id: event.mass_payment_id,
        rules_applied: withholdings.len(),
        created_at: Utc::now(),
    };

    let withholding_responses: Vec<WithholdingResultResponse> = withholdings
        .iter()
        .map(WithholdingResultResponse::from)
        .collect();

    state.tax_events.insert(record.id, record.clone());

    // Persist to database (write-through). Failure is surfaced to the client
    // because the in-memory record would be lost on restart, causing silent data loss.
    if let Some(pool) = &state.db_pool {
        if let Err(e) = crate::db::tax_events::insert(pool, &record).await {
            tracing::error!(tax_event_id = %record.id, error = %e, "failed to persist tax event to database");
            return Err(AppError::Internal(
                "tax event recorded in-memory but database persist failed".to_string(),
            ));
        }
    }

    Ok(Json(TaxEventResponse {
        event: record,
        withholdings: withholding_responses,
    }))
}

/// GET /v1/tax/events — List tax events with optional filtering.
async fn list_tax_events(
    State(state): State<AppState>,
    Query(params): Query<TaxEventQueryParams>,
) -> Result<Json<Vec<TaxEventRecord>>, AppError> {
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);

    let all = state.tax_events.list();
    let filtered: Vec<TaxEventRecord> = all
        .into_iter()
        .filter(|e| {
            if let Some(ref eid) = params.entity_id {
                if e.entity_id != *eid {
                    return false;
                }
            }
            if let Some(ref jid) = params.jurisdiction_id {
                if e.jurisdiction_id != *jid {
                    return false;
                }
            }
            if let Some(ref ty) = params.tax_year {
                if e.tax_year != *ty {
                    return false;
                }
            }
            true
        })
        .skip(offset)
        .take(limit)
        .collect();

    Ok(Json(filtered))
}

/// GET /v1/tax/events/:id — Get a tax event by ID.
async fn get_tax_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<TaxEventRecord>, AppError> {
    state
        .tax_events
        .get(&id)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("tax event {id} not found")))
}

/// POST /v1/tax/withhold — Compute withholding for an event without persisting.
///
/// Dry-run endpoint: evaluates withholding rules against the provided event
/// data and returns the computed results without recording a tax event.
async fn compute_withholding(
    State(state): State<AppState>,
    body: Result<Json<CreateTaxEventRequest>, JsonRejection>,
) -> Result<Json<Vec<WithholdingResultResponse>>, AppError> {
    let req = extract_validated_json(body)?;

    let event_type = parse_event_type(&req.event_type)?;
    let filer_status = parse_filer_status(req.filer_status.as_deref())?;

    let mut event = TaxEvent::new(
        req.entity_id,
        event_type,
        &req.jurisdiction_id,
        &req.gross_amount,
        &req.currency,
        &req.tax_year,
    );

    if let Some(ref ntn) = req.ntn {
        event = event.with_ntn(ntn, filer_status);
    } else {
        event.filer_status = filer_status;
    }

    let withholdings = {
        let pipeline = state.tax_pipeline.lock();
        pipeline.process_event(&event)
    };

    let responses: Vec<WithholdingResultResponse> = withholdings
        .iter()
        .map(WithholdingResultResponse::from)
        .collect();

    Ok(Json(responses))
}

/// GET /v1/tax/obligations/:entity_id — Get tax obligation summary for an entity.
///
/// Aggregates all recorded tax events for the entity across all jurisdictions
/// and categories, producing a summary of total obligations and withholdings.
async fn get_tax_obligations(
    State(state): State<AppState>,
    Path(entity_id): Path<Uuid>,
    Query(params): Query<TaxEventQueryParams>,
) -> Result<Json<TaxObligationSummary>, AppError> {
    let all = state.tax_events.list();
    let entity_events: Vec<&TaxEventRecord> = all
        .iter()
        .filter(|e| {
            if e.entity_id != entity_id {
                return false;
            }
            if let Some(ref jid) = params.jurisdiction_id {
                if e.jurisdiction_id != *jid {
                    return false;
                }
            }
            if let Some(ref ty) = params.tax_year {
                if e.tax_year != *ty {
                    return false;
                }
            }
            true
        })
        .collect();

    if entity_events.is_empty() {
        return Err(AppError::NotFound(format!(
            "no tax events found for entity {entity_id}"
        )));
    }

    // Aggregate by category.
    let mut by_category: std::collections::BTreeMap<String, (usize, i64, i64)> =
        std::collections::BTreeMap::new();
    let mut total_gross_cents: i64 = 0;
    let mut total_wht_cents: i64 = 0;
    let mut currency = String::new();
    let mut jurisdiction_id = String::new();

    for e in &entity_events {
        if currency.is_empty() {
            currency.clone_from(&e.currency);
        }
        if jurisdiction_id.is_empty() {
            jurisdiction_id.clone_from(&e.jurisdiction_id);
        }

        let gross = parse_amount_or_zero(&e.gross_amount);
        let wht = parse_amount_or_zero(&e.withholding_amount);

        total_gross_cents = total_gross_cents.saturating_add(gross);
        total_wht_cents = total_wht_cents.saturating_add(wht);

        let entry = by_category
            .entry(e.tax_category.clone())
            .or_insert((0, 0, 0));
        entry.0 = entry.0.saturating_add(1);
        entry.1 = entry.1.saturating_add(gross);
        entry.2 = entry.2.saturating_add(wht);
    }

    let category_summaries: Vec<CategorySummary> = by_category
        .into_iter()
        .map(|(cat, (count, gross, wht))| CategorySummary {
            tax_category: cat,
            event_count: count,
            total_gross: format_amount(gross),
            total_withholding: format_amount(wht),
        })
        .collect();

    Ok(Json(TaxObligationSummary {
        entity_id,
        jurisdiction_id,
        total_events: entity_events.len(),
        total_gross: format_amount(total_gross_cents),
        total_withholding: format_amount(total_wht_cents),
        currency,
        by_category: category_summaries,
    }))
}

/// POST /v1/tax/report — Generate a tax report for FBR IRIS submission.
///
/// Aggregates all tax events for the specified entity and period into a
/// report suitable for submission to the tax authority.
async fn generate_tax_report(
    State(state): State<AppState>,
    body: Result<Json<GenerateReportRequest>, JsonRejection>,
) -> Result<Json<TaxReportResponse>, AppError> {
    let req = extract_validated_json(body)?;

    // Collect all events matching the entity, jurisdiction, and tax year.
    let all = state.tax_events.list();
    let matching_events: Vec<&TaxEventRecord> = all
        .iter()
        .filter(|e| {
            e.entity_id == req.entity_id
                && e.jurisdiction_id == req.jurisdiction_id
                && e.tax_year == req.tax_year
        })
        .collect();

    if matching_events.is_empty() {
        return Err(AppError::NotFound(format!(
            "no tax events found for entity {} in {} for {}",
            req.entity_id, req.jurisdiction_id, req.tax_year
        )));
    }

    // Re-compute withholdings for all matching events to get full results.
    let pipeline = state.tax_pipeline.lock();
    let mut all_withholdings: Vec<WithholdingResult> = Vec::new();

    for record in &matching_events {
        let event_type = match parse_event_type(&record.event_type) {
            Ok(t) => t,
            Err(_) => {
                tracing::warn!(
                    tax_event_id = %record.id,
                    event_type = %record.event_type,
                    "skipping tax event with unparseable event_type during report generation"
                );
                continue;
            }
        };
        let filer_status = match parse_filer_status(Some(&record.filer_status)) {
            Ok(s) => s,
            Err(_) => FilerStatus::NonFiler,
        };

        let mut event = TaxEvent::new(
            record.entity_id,
            event_type,
            &record.jurisdiction_id,
            &record.gross_amount,
            &record.currency,
            &record.tax_year,
        );

        if let Some(ref ntn) = record.ntn {
            event = event.with_ntn(ntn, filer_status);
        } else {
            event.filer_status = filer_status;
        }

        all_withholdings.extend(pipeline.process_event(&event));
    }

    drop(pipeline);

    let report = tax::generate_report(
        &tax::ReportParams {
            entity_id: req.entity_id,
            ntn: req.ntn,
            jurisdiction_id: req.jurisdiction_id.clone(),
            tax_year: req.tax_year.clone(),
            period_start: req.period_start.clone(),
            period_end: req.period_end.clone(),
            report_type: req.report_type.clone(),
        },
        &all_withholdings,
    );

    Ok(Json(TaxReportResponse {
        report_id: report.report_id,
        entity_id: report.entity_id,
        jurisdiction_id: report.jurisdiction_id,
        tax_year: report.tax_year,
        period_start: report.period_start,
        period_end: report.period_end,
        report_type: report.report_type,
        total_gross: report.total_gross,
        total_withholding: report.total_withholding,
        currency: report.currency,
        event_count: report.event_count,
        line_item_count: report.line_items.len(),
        status: report.status.to_string(),
        generated_at: report.generated_at.to_rfc3339(),
    }))
}

/// GET /v1/tax/rules — List loaded withholding rules.
async fn list_withholding_rules(
    State(state): State<AppState>,
    Query(params): Query<RulesQueryParams>,
) -> Result<Json<Vec<WithholdingRuleResponse>>, AppError> {
    let pipeline = state.tax_pipeline.lock();
    let jurisdiction = params.jurisdiction_id.as_deref().unwrap_or("PK");

    let rules = pipeline.engine.rules_for_jurisdiction(jurisdiction);
    let responses: Vec<WithholdingRuleResponse> = rules
        .iter()
        .map(|r| WithholdingRuleResponse {
            rule_id: r.rule_id.clone(),
            applicable_event_types: r
                .applicable_event_types
                .iter()
                .map(|t| t.to_string())
                .collect(),
            applicable_filer_status: r
                .applicable_filer_status
                .iter()
                .map(|s| s.to_string())
                .collect(),
            tax_category: r.tax_category.to_string(),
            rate_percent: r.rate_percent.clone(),
            statutory_section: r.statutory_section.clone(),
            description: r.description.clone(),
            effective_from: r.effective_from.clone(),
            is_final_tax: r.is_final_tax,
        })
        .collect();

    Ok(Json(responses))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a string event type into the enum.
fn parse_event_type(s: &str) -> Result<TaxEventType, AppError> {
    match s {
        "payment_for_goods" => Ok(TaxEventType::PaymentForGoods),
        "payment_for_services" => Ok(TaxEventType::PaymentForServices),
        "salary_payment" => Ok(TaxEventType::SalaryPayment),
        "profit_on_debt" => Ok(TaxEventType::ProfitOnDebt),
        "dividend_distribution" => Ok(TaxEventType::DividendDistribution),
        "rent_payment" => Ok(TaxEventType::RentPayment),
        "cash_withdrawal" => Ok(TaxEventType::CashWithdrawal),
        "sale_to_unregistered" => Ok(TaxEventType::SaleToUnregistered),
        "cross_border_payment" => Ok(TaxEventType::CrossBorderPayment),
        "capital_gain_disposal" => Ok(TaxEventType::CapitalGainDisposal),
        "import_of_goods" => Ok(TaxEventType::ImportOfGoods),
        "export_of_goods" => Ok(TaxEventType::ExportOfGoods),
        "supply_of_goods" => Ok(TaxEventType::SupplyOfGoods),
        "supply_of_services" => Ok(TaxEventType::SupplyOfServices),
        "formation_fee" => Ok(TaxEventType::FormationFee),
        "annual_filing_fee" => Ok(TaxEventType::AnnualFilingFee),
        other => Err(AppError::Validation(format!(
            "unknown event_type: \"{other}\". Valid types: payment_for_goods, \
             payment_for_services, salary_payment, profit_on_debt, \
             dividend_distribution, rent_payment, cash_withdrawal, \
             sale_to_unregistered, cross_border_payment, capital_gain_disposal, \
             import_of_goods, export_of_goods, supply_of_goods, \
             supply_of_services, formation_fee, annual_filing_fee"
        ))),
    }
}

/// Parse a string filer status.
fn parse_filer_status(s: Option<&str>) -> Result<FilerStatus, AppError> {
    match s {
        None | Some("non_filer") => Ok(FilerStatus::NonFiler),
        Some("filer") => Ok(FilerStatus::Filer),
        Some("late_filer") => Ok(FilerStatus::LateFiler),
        Some(other) => Err(AppError::Validation(format!(
            "unknown filer_status: \"{other}\". Valid values: filer, late_filer, non_filer"
        ))),
    }
}

/// Aggregate withholding results into total withholding and net amounts.
fn aggregate_withholdings(withholdings: &[WithholdingResult], gross: &str) -> (String, String) {
    if withholdings.is_empty() {
        return ("0.00".to_string(), gross.to_string());
    }

    let mut total_wht_cents: i64 = 0;
    for w in withholdings {
        total_wht_cents =
            total_wht_cents.saturating_add(parse_amount_or_zero(&w.withholding_amount));
    }

    let gross_cents = parse_amount_or_zero(gross);
    let net_cents = gross_cents.saturating_sub(total_wht_cents);

    (format_amount(total_wht_cents), format_amount(net_cents))
}

/// Parse a string amount into cents, returning 0 for unparseable input.
///
/// Delegates to [`msez_agentic::tax::parse_amount`] — the canonical
/// fixed-precision parser — with a fallback of 0 for invalid strings.
fn parse_amount_or_zero(s: &str) -> i64 {
    parse_amount(s).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_app() -> Router {
        let state = AppState::new();
        super::router().with_state(state)
    }

    #[tokio::test]
    async fn create_tax_event_goods_filer() {
        let app = test_app();

        let entity_id = Uuid::new_v4();
        let body = serde_json::json!({
            "entity_id": entity_id,
            "event_type": "payment_for_goods",
            "jurisdiction_id": "PK",
            "gross_amount": "100000",
            "currency": "PKR",
            "tax_year": "2025-2026",
            "ntn": "1234567",
            "filer_status": "filer"
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/events")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let result: TaxEventResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.event.entity_id, entity_id);
        assert_eq!(result.event.event_type, "payment_for_goods");
        assert_eq!(result.event.jurisdiction_id, "PK");
        assert_eq!(result.event.withholding_amount, "4500.00");
        assert_eq!(result.withholdings.len(), 1);
        assert_eq!(result.withholdings[0].rate_percent, "4.5");
    }

    #[tokio::test]
    async fn create_tax_event_nonfiler_double_rate() {
        let app = test_app();

        let body = serde_json::json!({
            "entity_id": Uuid::new_v4(),
            "event_type": "payment_for_goods",
            "jurisdiction_id": "PK",
            "gross_amount": "100000",
            "currency": "PKR",
            "tax_year": "2025-2026",
            "filer_status": "non_filer"
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/events")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let result: TaxEventResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.event.withholding_amount, "9000.00");
        assert_eq!(result.withholdings[0].rate_percent, "9.0");
    }

    #[tokio::test]
    async fn create_tax_event_rejects_invalid_event_type() {
        let app = test_app();

        let body = serde_json::json!({
            "entity_id": Uuid::new_v4(),
            "event_type": "invalid_type",
            "jurisdiction_id": "PK",
            "gross_amount": "100000",
            "currency": "PKR",
            "tax_year": "2025-2026"
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/events")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn create_tax_event_rejects_invalid_ntn() {
        let app = test_app();

        let body = serde_json::json!({
            "entity_id": Uuid::new_v4(),
            "event_type": "payment_for_goods",
            "jurisdiction_id": "PK",
            "gross_amount": "100000",
            "currency": "PKR",
            "tax_year": "2025-2026",
            "ntn": "123"
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/events")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn list_tax_events_empty() {
        let app = test_app();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/tax/events")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let events: Vec<TaxEventRecord> = serde_json::from_slice(&body).unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn get_tax_event_not_found() {
        let app = test_app();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/v1/tax/events/{}", Uuid::new_v4()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn compute_withholding_dry_run() {
        let app = test_app();

        let body = serde_json::json!({
            "entity_id": Uuid::new_v4(),
            "event_type": "payment_for_services",
            "jurisdiction_id": "PK",
            "gross_amount": "50000",
            "currency": "PKR",
            "tax_year": "2025-2026",
            "ntn": "1234567",
            "filer_status": "filer"
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tax/withhold")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let results: Vec<WithholdingResultResponse> = serde_json::from_slice(&body).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].rate_percent, "8.0");
        assert_eq!(results[0].withholding_amount, "4000.00");
    }

    #[tokio::test]
    async fn list_withholding_rules() {
        let app = test_app();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/tax/rules?jurisdiction_id=PK")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let rules: Vec<WithholdingRuleResponse> = serde_json::from_slice(&body).unwrap();

        assert!(!rules.is_empty());
        // Pakistan should have the standard rules loaded.
        assert!(rules.iter().any(|r| r.rule_id.contains("S153")));
    }

    #[tokio::test]
    async fn list_withholding_rules_unknown_jurisdiction() {
        let app = test_app();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/tax/rules?jurisdiction_id=XX")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let rules: Vec<WithholdingRuleResponse> = serde_json::from_slice(&body).unwrap();
        assert!(rules.is_empty());
    }

    #[tokio::test]
    async fn tax_obligations_not_found() {
        let app = test_app();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/v1/tax/obligations/{}", Uuid::new_v4()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // -- Helper tests --

    #[test]
    fn parse_event_type_all_variants() {
        let valid_types = [
            "payment_for_goods",
            "payment_for_services",
            "salary_payment",
            "profit_on_debt",
            "dividend_distribution",
            "rent_payment",
            "cash_withdrawal",
            "sale_to_unregistered",
            "cross_border_payment",
            "capital_gain_disposal",
            "import_of_goods",
            "export_of_goods",
            "supply_of_goods",
            "supply_of_services",
            "formation_fee",
            "annual_filing_fee",
        ];

        for t in &valid_types {
            assert!(parse_event_type(t).is_ok(), "failed for: {t}");
        }
    }

    #[test]
    fn parse_event_type_rejects_unknown() {
        assert!(parse_event_type("unknown").is_err());
    }

    #[test]
    fn parse_filer_status_variants() {
        assert_eq!(
            parse_filer_status(Some("filer")).unwrap(),
            FilerStatus::Filer
        );
        assert_eq!(
            parse_filer_status(Some("non_filer")).unwrap(),
            FilerStatus::NonFiler
        );
        assert_eq!(
            parse_filer_status(Some("late_filer")).unwrap(),
            FilerStatus::LateFiler
        );
        assert_eq!(parse_filer_status(None).unwrap(), FilerStatus::NonFiler);
    }

    #[test]
    fn parse_filer_status_rejects_unknown() {
        assert!(parse_filer_status(Some("invalid")).is_err());
    }

    #[test]
    fn amount_parsing_and_formatting() {
        assert_eq!(parse_amount_or_zero("100000"), 10_000_000);
        assert_eq!(parse_amount_or_zero("4500.00"), 450_000);
        assert_eq!(parse_amount_or_zero("0.01"), 1);
        assert_eq!(format_amount(450_000), "4500.00");
        assert_eq!(format_amount(0), "0.00");
    }

    #[test]
    fn aggregate_withholdings_empty() {
        let (wht, net) = aggregate_withholdings(&[], "100000");
        assert_eq!(wht, "0.00");
        assert_eq!(net, "100000");
    }
}
