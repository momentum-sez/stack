//! # Tax Collection Pipeline — Pakistan GovOS Reference Implementation
//!
//! Implements the signature Pakistan deployment feature:
//! **Every economic activity on Mass generates a tax event → automatic
//! withholding at source → real-time reporting to FBR IRIS → gap analysis
//! closes evasion.**
//!
//! ## Architecture
//!
//! The tax pipeline sits in the SEZ Stack's jurisdictional orchestration layer.
//! It does NOT duplicate Mass fiscal CRUD — it provides the *tax awareness*
//! that transforms generic payment/transaction operations into tax-compliant
//! operations under Pakistani (or any jurisdiction's) tax law.
//!
//! ### Pipeline stages:
//!
//! 1. **Event Generation** — Observe a Mass fiscal event (payment, transfer,
//!    dividend) and classify it as a [`TaxEvent`] with the applicable
//!    [`TaxEventType`] and [`TaxCategory`].
//!
//! 2. **Withholding Computation** — Given the event, entity NTN status, and
//!    jurisdiction-specific [`WithholdingRule`]s (from regpack), compute the
//!    [`WithholdingResult`] with exact amount, applicable section, and rate.
//!
//! 3. **Reporting** — Generate a [`TaxReport`] for submission to FBR IRIS
//!    (or any jurisdiction's tax authority). Includes event digest for
//!    tamper evidence via [`CanonicalBytes`].
//!
//! ## Pakistan-Specific Context
//!
//! - Income Tax Ordinance 2001: Sections 149 (salary), 151 (profit on debt),
//!   153 (payments for goods/services), 231A (cash withdrawal), 236G (sale of
//!   goods to unregistered persons)
//! - Sales Tax Act 1990: Standard rate, zero-rated, exempt supplies
//! - Federal Excise Act 2005: Excise duties on services and goods
//! - FBR IRIS: Real-time reporting endpoint for withholding certificates
//!
//! ## Determinism
//!
//! Withholding computation is deterministic: given identical inputs
//! (amount, entity status, jurisdiction, applicable rules), the output
//! is always the same. This property is critical for audit reproducibility.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Tax Event Classification
// ---------------------------------------------------------------------------

/// The category of tax applicable to an economic event.
///
/// Maps to major Pakistani tax statutes. Other jurisdictions will use
/// the same categories — the rates and sections differ, not the taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaxCategory {
    /// Income Tax Ordinance 2001 — withholding at source on payments.
    IncomeTax,
    /// Sales Tax Act 1990 — tax on supply of goods and services.
    SalesTax,
    /// Federal Excise Act 2005 — excise duty on goods and services.
    FederalExcise,
    /// Customs Act 1969 — duties on imports/exports.
    CustomsDuty,
    /// Capital gains tax (Schedule I, ITO 2001).
    CapitalGains,
    /// Withholding tax on cross-border payments (Section 152, ITO 2001).
    CrossBorderWithholding,
    /// Provincial sales tax on services (varies by province).
    ProvincialSalesTax,
}

impl TaxCategory {
    /// Return the string representation of this category.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IncomeTax => "income_tax",
            Self::SalesTax => "sales_tax",
            Self::FederalExcise => "federal_excise",
            Self::CustomsDuty => "customs_duty",
            Self::CapitalGains => "capital_gains",
            Self::CrossBorderWithholding => "cross_border_withholding",
            Self::ProvincialSalesTax => "provincial_sales_tax",
        }
    }

    /// Return all tax category variants.
    pub fn all() -> &'static [TaxCategory] {
        &[
            Self::IncomeTax,
            Self::SalesTax,
            Self::FederalExcise,
            Self::CustomsDuty,
            Self::CapitalGains,
            Self::CrossBorderWithholding,
            Self::ProvincialSalesTax,
        ]
    }
}

impl std::fmt::Display for TaxCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The type of economic activity that generated a tax event.
///
/// Each variant maps to a specific withholding section under Pakistani tax law.
/// The mapping is jurisdiction-configurable via regpack data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaxEventType {
    /// Payment for goods or services — ITO 2001 Section 153.
    PaymentForGoods,
    /// Payment for services — ITO 2001 Section 153.
    PaymentForServices,
    /// Salary payment — ITO 2001 Section 149.
    SalaryPayment,
    /// Profit on debt (interest) — ITO 2001 Section 151.
    ProfitOnDebt,
    /// Dividend distribution — ITO 2001 Section 150.
    DividendDistribution,
    /// Rent payment — ITO 2001 Section 155.
    RentPayment,
    /// Cash withdrawal from bank — ITO 2001 Section 231A.
    CashWithdrawal,
    /// Sale to unregistered person — ITO 2001 Section 236G.
    SaleToUnregistered,
    /// Cross-border payment — ITO 2001 Section 152.
    CrossBorderPayment,
    /// Capital gain on disposal of asset.
    CapitalGainDisposal,
    /// Import of goods — Customs Act 1969.
    ImportOfGoods,
    /// Export of goods.
    ExportOfGoods,
    /// Supply of taxable goods — Sales Tax Act 1990.
    SupplyOfGoods,
    /// Supply of taxable services — Sales Tax Act 1990.
    SupplyOfServices,
    /// Entity formation fee.
    FormationFee,
    /// Annual filing fee.
    AnnualFilingFee,
}

impl TaxEventType {
    /// Return the string representation of this event type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PaymentForGoods => "payment_for_goods",
            Self::PaymentForServices => "payment_for_services",
            Self::SalaryPayment => "salary_payment",
            Self::ProfitOnDebt => "profit_on_debt",
            Self::DividendDistribution => "dividend_distribution",
            Self::RentPayment => "rent_payment",
            Self::CashWithdrawal => "cash_withdrawal",
            Self::SaleToUnregistered => "sale_to_unregistered",
            Self::CrossBorderPayment => "cross_border_payment",
            Self::CapitalGainDisposal => "capital_gain_disposal",
            Self::ImportOfGoods => "import_of_goods",
            Self::ExportOfGoods => "export_of_goods",
            Self::SupplyOfGoods => "supply_of_goods",
            Self::SupplyOfServices => "supply_of_services",
            Self::FormationFee => "formation_fee",
            Self::AnnualFilingFee => "annual_filing_fee",
        }
    }

    /// Return the default [`TaxCategory`] for this event type.
    ///
    /// This provides the primary classification. Some events may trigger
    /// obligations in multiple categories (e.g., a goods import triggers
    /// both customs duty and sales tax). The withholding engine handles
    /// multi-category evaluation.
    pub fn default_category(&self) -> TaxCategory {
        match self {
            Self::PaymentForGoods
            | Self::PaymentForServices
            | Self::SalaryPayment
            | Self::ProfitOnDebt
            | Self::DividendDistribution
            | Self::RentPayment
            | Self::CashWithdrawal
            | Self::SaleToUnregistered
            | Self::FormationFee
            | Self::AnnualFilingFee => TaxCategory::IncomeTax,

            Self::CrossBorderPayment | Self::CapitalGainDisposal => TaxCategory::IncomeTax,

            Self::ImportOfGoods | Self::ExportOfGoods => TaxCategory::CustomsDuty,

            Self::SupplyOfGoods | Self::SupplyOfServices => TaxCategory::SalesTax,
        }
    }

    /// Return all tax event type variants.
    pub fn all() -> &'static [TaxEventType] {
        &[
            Self::PaymentForGoods,
            Self::PaymentForServices,
            Self::SalaryPayment,
            Self::ProfitOnDebt,
            Self::DividendDistribution,
            Self::RentPayment,
            Self::CashWithdrawal,
            Self::SaleToUnregistered,
            Self::CrossBorderPayment,
            Self::CapitalGainDisposal,
            Self::ImportOfGoods,
            Self::ExportOfGoods,
            Self::SupplyOfGoods,
            Self::SupplyOfServices,
            Self::FormationFee,
            Self::AnnualFilingFee,
        ]
    }
}

impl std::fmt::Display for TaxEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The filing status of an entity with the tax authority.
///
/// Withholding rates differ based on whether the entity is a registered
/// tax filer. Under Pakistani law, non-filers pay significantly higher
/// withholding rates as an incentive for tax registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilerStatus {
    /// Entity has a valid NTN and is on the Active Taxpayer List (ATL).
    Filer,
    /// Entity has an NTN but is not on the ATL (late filer).
    LateFiler,
    /// Entity does not have an NTN or is not on the ATL.
    NonFiler,
}

impl FilerStatus {
    /// Return the string representation of this filer status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Filer => "filer",
            Self::LateFiler => "late_filer",
            Self::NonFiler => "non_filer",
        }
    }
}

impl std::fmt::Display for FilerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Tax Event — the primary pipeline record
// ---------------------------------------------------------------------------

/// A tax event generated from an economic activity on Mass.
///
/// This is the primary record in the tax collection pipeline. Each event
/// represents an observable economic activity that has tax implications
/// under the applicable jurisdiction's tax law.
///
/// Tax events are immutable once created. They feed into the withholding
/// computation engine and the reporting pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxEvent {
    /// Unique event identifier.
    pub event_id: Uuid,
    /// Entity that is the tax subject (payer or payee).
    pub entity_id: Uuid,
    /// Entity's NTN (National Tax Number), if registered.
    pub ntn: Option<String>,
    /// Filing status of the entity.
    pub filer_status: FilerStatus,
    /// Type of economic activity.
    pub event_type: TaxEventType,
    /// Primary tax category.
    pub tax_category: TaxCategory,
    /// Jurisdiction where the tax obligation arises.
    pub jurisdiction_id: String,
    /// Gross amount of the economic activity (string to avoid floating-point).
    pub gross_amount: String,
    /// Currency code (ISO 4217).
    pub currency: String,
    /// Tax year (e.g., "2025-2026" for Pakistani fiscal year Jul-Jun).
    pub tax_year: String,
    /// Reference to the originating Mass fiscal transaction.
    pub mass_transaction_id: Option<Uuid>,
    /// Reference to the originating Mass payment.
    pub mass_payment_id: Option<Uuid>,
    /// Counterparty entity ID (for B2B transactions).
    pub counterparty_entity_id: Option<Uuid>,
    /// Applicable statutory section (e.g., "ITO2001-S153" for Pakistan).
    pub statutory_section: Option<String>,
    /// Additional event-specific metadata.
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// When the underlying economic activity occurred.
    pub activity_timestamp: DateTime<Utc>,
    /// When this tax event was generated.
    pub created_at: DateTime<Utc>,
}

impl TaxEvent {
    /// Create a new tax event from an observed economic activity.
    pub fn new(
        entity_id: Uuid,
        event_type: TaxEventType,
        jurisdiction_id: impl Into<String>,
        gross_amount: impl Into<String>,
        currency: impl Into<String>,
        tax_year: impl Into<String>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            entity_id,
            ntn: None,
            filer_status: FilerStatus::NonFiler,
            event_type,
            tax_category: event_type.default_category(),
            jurisdiction_id: jurisdiction_id.into(),
            gross_amount: gross_amount.into(),
            currency: currency.into(),
            tax_year: tax_year.into(),
            mass_transaction_id: None,
            mass_payment_id: None,
            counterparty_entity_id: None,
            statutory_section: None,
            metadata: serde_json::Value::Null,
            activity_timestamp: Utc::now(),
            created_at: Utc::now(),
        }
    }

    /// Builder: set NTN and filer status.
    pub fn with_ntn(mut self, ntn: impl Into<String>, filer_status: FilerStatus) -> Self {
        self.ntn = Some(ntn.into());
        self.filer_status = filer_status;
        self
    }

    /// Builder: set the originating Mass transaction ID.
    pub fn with_mass_transaction(mut self, transaction_id: Uuid) -> Self {
        self.mass_transaction_id = Some(transaction_id);
        self
    }

    /// Builder: set the originating Mass payment ID.
    pub fn with_mass_payment(mut self, payment_id: Uuid) -> Self {
        self.mass_payment_id = Some(payment_id);
        self
    }

    /// Builder: set the counterparty entity.
    pub fn with_counterparty(mut self, counterparty_id: Uuid) -> Self {
        self.counterparty_entity_id = Some(counterparty_id);
        self
    }

    /// Builder: set the statutory section reference.
    pub fn with_statutory_section(mut self, section: impl Into<String>) -> Self {
        self.statutory_section = Some(section.into());
        self
    }

    /// Builder: override the tax category.
    pub fn with_tax_category(mut self, category: TaxCategory) -> Self {
        self.tax_category = category;
        self
    }

    /// Builder: set additional metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Builder: set the activity timestamp.
    pub fn with_activity_timestamp(mut self, ts: DateTime<Utc>) -> Self {
        self.activity_timestamp = ts;
        self
    }
}

// ---------------------------------------------------------------------------
// Withholding Rules and Computation
// ---------------------------------------------------------------------------

/// A withholding rule loaded from regpack data.
///
/// Encodes a single applicable withholding rate for a specific combination
/// of event type, filer status, and threshold. Rules are jurisdiction-specific
/// and typically sourced from FBR SROs (Statutory Regulatory Orders) for Pakistan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithholdingRule {
    /// Rule identifier (e.g., "PAK-ITO2001-S153-GOODS-FILER").
    pub rule_id: String,
    /// Applicable tax event types.
    pub applicable_event_types: Vec<TaxEventType>,
    /// Applicable filer status.
    pub applicable_filer_status: Vec<FilerStatus>,
    /// Tax category this rule applies to.
    pub tax_category: TaxCategory,
    /// Withholding rate as a percentage (e.g., "4.5" for 4.5%).
    /// String to avoid floating-point imprecision.
    pub rate_percent: String,
    /// Minimum transaction amount for this rule to apply (inclusive).
    /// "0" means no minimum.
    pub threshold_min: String,
    /// Maximum transaction amount (exclusive). Empty means no maximum.
    pub threshold_max: Option<String>,
    /// Statutory section reference.
    pub statutory_section: String,
    /// Human-readable description of the rule.
    pub description: String,
    /// Effective date of this rule.
    pub effective_from: String,
    /// Expiry date (rules may be superseded by new SROs).
    pub effective_until: Option<String>,
    /// Whether this is a final tax (no further liability) or adjustable.
    pub is_final_tax: bool,
}

/// Result of withholding computation for a single tax event.
///
/// Deterministic: identical inputs always produce identical outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithholdingResult {
    /// The tax event this computation applies to.
    pub event_id: Uuid,
    /// Entity subject to withholding.
    pub entity_id: Uuid,
    /// The rule that was applied.
    pub rule_id: String,
    /// Gross amount from the event.
    pub gross_amount: String,
    /// Withholding rate applied (percentage).
    pub rate_percent: String,
    /// Computed withholding amount (string for precision).
    pub withholding_amount: String,
    /// Net amount after withholding.
    pub net_amount: String,
    /// Currency code.
    pub currency: String,
    /// Tax category.
    pub tax_category: TaxCategory,
    /// Statutory section.
    pub statutory_section: String,
    /// Whether this is a final tax.
    pub is_final_tax: bool,
    /// When the computation was performed.
    pub computed_at: DateTime<Utc>,
}

/// The withholding computation engine.
///
/// Evaluates a [`TaxEvent`] against applicable [`WithholdingRule`]s
/// and produces [`WithholdingResult`]s. The engine is stateless — all
/// state comes from the inputs (event + rules).
///
/// ## Determinism
///
/// Given identical `TaxEvent` and `WithholdingRule` inputs, the engine
/// always produces identical `WithholdingResult` outputs. This is
/// guaranteed by:
/// - No internal mutable state
/// - Deterministic rule matching (sorted by rule_id)
/// - Fixed-precision decimal arithmetic via string parsing
pub struct WithholdingEngine {
    /// Rules indexed by jurisdiction, loaded from regpack data.
    rules: BTreeMap<String, Vec<WithholdingRule>>,
}

impl WithholdingEngine {
    /// Create a new engine with no rules.
    pub fn new() -> Self {
        Self {
            rules: BTreeMap::new(),
        }
    }

    /// Load withholding rules for a jurisdiction.
    ///
    /// Replaces any existing rules for the jurisdiction.
    pub fn load_rules(&mut self, jurisdiction_id: impl Into<String>, rules: Vec<WithholdingRule>) {
        self.rules.insert(jurisdiction_id.into(), rules);
    }

    /// Return all loaded rules for a jurisdiction.
    pub fn rules_for_jurisdiction(&self, jurisdiction_id: &str) -> &[WithholdingRule] {
        self.rules
            .get(jurisdiction_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Return the total number of rules loaded across all jurisdictions.
    pub fn rule_count(&self) -> usize {
        self.rules.values().map(|v| v.len()).sum()
    }

    /// Compute withholding for a tax event.
    ///
    /// Finds all applicable rules for the event's jurisdiction, event type,
    /// and filer status, then computes the withholding amount for each.
    /// Returns results sorted by rule_id for determinism.
    ///
    /// If no rules match, returns an empty vec (zero withholding).
    pub fn compute(&self, event: &TaxEvent) -> Vec<WithholdingResult> {
        let rules = match self.rules.get(&event.jurisdiction_id) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let gross = match parse_amount(&event.gross_amount) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let mut results: Vec<WithholdingResult> = rules
            .iter()
            .filter(|rule| self.rule_matches(rule, event, gross))
            .map(|rule| self.compute_single(event, rule, gross))
            .collect();

        // Sort by rule_id for deterministic output.
        results.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));
        results
    }

    /// Check whether a rule applies to a given event.
    fn rule_matches(&self, rule: &WithholdingRule, event: &TaxEvent, gross: i64) -> bool {
        // Event type must match.
        if !rule.applicable_event_types.contains(&event.event_type) {
            return false;
        }

        // Filer status must match.
        if !rule.applicable_filer_status.contains(&event.filer_status) {
            return false;
        }

        // Tax category must match.
        if rule.tax_category != event.tax_category {
            return false;
        }

        // Threshold check. An unparseable threshold_min silently becoming 0
        // would cause the rule to match ALL amounts — log the anomaly.
        let min = parse_amount(&rule.threshold_min).unwrap_or_else(|| {
            tracing::warn!(
                threshold_min = %rule.threshold_min,
                section = %rule.statutory_section,
                "unparseable threshold_min in withholding rule — defaulting to 0"
            );
            0
        });
        if gross < min {
            return false;
        }

        if let Some(ref max_str) = rule.threshold_max {
            if let Some(max) = parse_amount(max_str) {
                if gross >= max {
                    return false;
                }
            }
        }

        true
    }

    /// Compute withholding for a single matched rule.
    fn compute_single(
        &self,
        event: &TaxEvent,
        rule: &WithholdingRule,
        gross_cents: i64,
    ) -> WithholdingResult {
        let rate_bps = parse_rate_bps(&rule.rate_percent);

        // Withholding = gross * rate / 10000 (basis points).
        // Round down (truncate) — withholding should never exceed the rate.
        let withholding_cents = (gross_cents * rate_bps) / 10000;
        let net_cents = gross_cents - withholding_cents;

        WithholdingResult {
            event_id: event.event_id,
            entity_id: event.entity_id,
            rule_id: rule.rule_id.clone(),
            gross_amount: event.gross_amount.clone(),
            rate_percent: rule.rate_percent.clone(),
            withholding_amount: format_amount(withholding_cents),
            net_amount: format_amount(net_cents),
            currency: event.currency.clone(),
            tax_category: rule.tax_category,
            statutory_section: rule.statutory_section.clone(),
            is_final_tax: rule.is_final_tax,
            computed_at: Utc::now(),
        }
    }

    /// Create an engine pre-loaded with Pakistan's standard withholding rules
    /// based on the Income Tax Ordinance 2001 and current FBR SRO rates.
    pub fn with_pakistan_rules() -> Self {
        let mut engine = Self::new();
        engine.load_rules("PK", pakistan_standard_rules());
        engine
    }
}

impl Default for WithholdingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for WithholdingEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WithholdingEngine")
            .field("jurisdictions", &self.rules.keys().collect::<Vec<_>>())
            .field("total_rules", &self.rule_count())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Pakistan Standard Withholding Rules
// ---------------------------------------------------------------------------

/// Return Pakistan's standard withholding rules under the Income Tax
/// Ordinance 2001, as per FBR SRO rates effective FY 2025-2026.
///
/// These are the baseline rules. Jurisdiction-specific overrides and
/// SRO amendments are applied via regpack data at runtime.
pub fn pakistan_standard_rules() -> Vec<WithholdingRule> {
    vec![
        // Section 153(1)(a) — Goods, Filer
        WithholdingRule {
            rule_id: "PAK-ITO2001-S153-1A-GOODS-FILER".into(),
            applicable_event_types: vec![TaxEventType::PaymentForGoods],
            applicable_filer_status: vec![FilerStatus::Filer],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "4.5".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "ITO 2001 Section 153(1)(a)".into(),
            description: "WHT on payment for goods to filer".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: false,
        },
        // Section 153(1)(a) — Goods, Non-Filer
        WithholdingRule {
            rule_id: "PAK-ITO2001-S153-1A-GOODS-NONFILER".into(),
            applicable_event_types: vec![TaxEventType::PaymentForGoods],
            applicable_filer_status: vec![FilerStatus::NonFiler, FilerStatus::LateFiler],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "9.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "ITO 2001 Section 153(1)(a)".into(),
            description: "WHT on payment for goods to non-filer (double rate)".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: false,
        },
        // Section 153(1)(b) — Services, Filer
        WithholdingRule {
            rule_id: "PAK-ITO2001-S153-1B-SERVICES-FILER".into(),
            applicable_event_types: vec![TaxEventType::PaymentForServices],
            applicable_filer_status: vec![FilerStatus::Filer],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "8.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "ITO 2001 Section 153(1)(b)".into(),
            description: "WHT on payment for services to filer".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: false,
        },
        // Section 153(1)(b) — Services, Non-Filer
        WithholdingRule {
            rule_id: "PAK-ITO2001-S153-1B-SERVICES-NONFILER".into(),
            applicable_event_types: vec![TaxEventType::PaymentForServices],
            applicable_filer_status: vec![FilerStatus::NonFiler, FilerStatus::LateFiler],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "16.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "ITO 2001 Section 153(1)(b)".into(),
            description: "WHT on payment for services to non-filer (double rate)".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: false,
        },
        // Section 149 — Salary (simplified slab)
        WithholdingRule {
            rule_id: "PAK-ITO2001-S149-SALARY-FILER".into(),
            applicable_event_types: vec![TaxEventType::SalaryPayment],
            applicable_filer_status: vec![
                FilerStatus::Filer,
                FilerStatus::LateFiler,
                FilerStatus::NonFiler,
            ],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "5.0".into(),
            threshold_min: "5000000".into(),
            threshold_max: None,
            statutory_section: "ITO 2001 Section 149".into(),
            description: "WHT on salary above PKR 50,000/month (simplified)".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: false,
        },
        // Section 151 — Profit on Debt, Filer
        WithholdingRule {
            rule_id: "PAK-ITO2001-S151-PROFIT-FILER".into(),
            applicable_event_types: vec![TaxEventType::ProfitOnDebt],
            applicable_filer_status: vec![FilerStatus::Filer],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "15.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "ITO 2001 Section 151".into(),
            description: "WHT on profit on debt to filer".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: true,
        },
        // Section 151 — Profit on Debt, Non-Filer
        WithholdingRule {
            rule_id: "PAK-ITO2001-S151-PROFIT-NONFILER".into(),
            applicable_event_types: vec![TaxEventType::ProfitOnDebt],
            applicable_filer_status: vec![FilerStatus::NonFiler, FilerStatus::LateFiler],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "30.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "ITO 2001 Section 151".into(),
            description: "WHT on profit on debt to non-filer".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: true,
        },
        // Section 150 — Dividends, Filer
        WithholdingRule {
            rule_id: "PAK-ITO2001-S150-DIVIDEND-FILER".into(),
            applicable_event_types: vec![TaxEventType::DividendDistribution],
            applicable_filer_status: vec![FilerStatus::Filer],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "15.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "ITO 2001 Section 150".into(),
            description: "WHT on dividends to filer".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: true,
        },
        // Section 150 — Dividends, Non-Filer
        WithholdingRule {
            rule_id: "PAK-ITO2001-S150-DIVIDEND-NONFILER".into(),
            applicable_event_types: vec![TaxEventType::DividendDistribution],
            applicable_filer_status: vec![FilerStatus::NonFiler, FilerStatus::LateFiler],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "30.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "ITO 2001 Section 150".into(),
            description: "WHT on dividends to non-filer".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: true,
        },
        // Section 155 — Rent, Filer
        WithholdingRule {
            rule_id: "PAK-ITO2001-S155-RENT-FILER".into(),
            applicable_event_types: vec![TaxEventType::RentPayment],
            applicable_filer_status: vec![FilerStatus::Filer],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "15.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "ITO 2001 Section 155".into(),
            description: "WHT on rent payment to filer".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: false,
        },
        // Section 155 — Rent, Non-Filer
        WithholdingRule {
            rule_id: "PAK-ITO2001-S155-RENT-NONFILER".into(),
            applicable_event_types: vec![TaxEventType::RentPayment],
            applicable_filer_status: vec![FilerStatus::NonFiler, FilerStatus::LateFiler],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "30.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "ITO 2001 Section 155".into(),
            description: "WHT on rent payment to non-filer".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: false,
        },
        // Section 152 — Cross-border payment
        WithholdingRule {
            rule_id: "PAK-ITO2001-S152-CROSSBORDER".into(),
            applicable_event_types: vec![TaxEventType::CrossBorderPayment],
            applicable_filer_status: vec![
                FilerStatus::Filer,
                FilerStatus::NonFiler,
                FilerStatus::LateFiler,
            ],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "20.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "ITO 2001 Section 152".into(),
            description: "WHT on payments to non-residents".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: true,
        },
        // Sales Tax — Standard rate on supply of goods
        WithholdingRule {
            rule_id: "PAK-STA1990-STANDARD-GOODS".into(),
            applicable_event_types: vec![TaxEventType::SupplyOfGoods],
            applicable_filer_status: vec![
                FilerStatus::Filer,
                FilerStatus::NonFiler,
                FilerStatus::LateFiler,
            ],
            tax_category: TaxCategory::SalesTax,
            rate_percent: "18.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "Sales Tax Act 1990 Section 3".into(),
            description: "Standard sales tax rate on supply of goods".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: true,
        },
        // Sales Tax — Standard rate on supply of services
        WithholdingRule {
            rule_id: "PAK-STA1990-STANDARD-SERVICES".into(),
            applicable_event_types: vec![TaxEventType::SupplyOfServices],
            applicable_filer_status: vec![
                FilerStatus::Filer,
                FilerStatus::NonFiler,
                FilerStatus::LateFiler,
            ],
            tax_category: TaxCategory::SalesTax,
            rate_percent: "18.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "Sales Tax Act 1990 Section 3".into(),
            description: "Standard sales tax rate on supply of services".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: true,
        },
    ]
}

// ---------------------------------------------------------------------------
// Tax Reporting — FBR IRIS Integration Types
// ---------------------------------------------------------------------------

/// Status of a tax report submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    /// Report has been generated but not yet submitted.
    Generated,
    /// Report has been submitted to the tax authority.
    Submitted,
    /// Tax authority acknowledged receipt.
    Acknowledged,
    /// Tax authority rejected the report (requires correction).
    Rejected,
    /// Report was accepted and processed.
    Accepted,
}

impl ReportStatus {
    /// Return the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Generated => "generated",
            Self::Submitted => "submitted",
            Self::Acknowledged => "acknowledged",
            Self::Rejected => "rejected",
            Self::Accepted => "accepted",
        }
    }
}

impl std::fmt::Display for ReportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A tax report generated for submission to FBR IRIS (or equivalent
/// tax authority reporting system).
///
/// Each report covers a set of tax events and their computed withholdings
/// for a specific entity and tax period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxReport {
    /// Unique report identifier.
    pub report_id: Uuid,
    /// Entity this report covers.
    pub entity_id: Uuid,
    /// Entity NTN.
    pub ntn: Option<String>,
    /// Jurisdiction of the tax authority.
    pub jurisdiction_id: String,
    /// Tax period start (inclusive).
    pub period_start: String,
    /// Tax period end (inclusive).
    pub period_end: String,
    /// Tax year.
    pub tax_year: String,
    /// Report type (e.g., "monthly_withholding", "annual_return", "quarterly_advance").
    pub report_type: String,
    /// Total gross amount of all events in this report.
    pub total_gross: String,
    /// Total withholding amount.
    pub total_withholding: String,
    /// Currency.
    pub currency: String,
    /// Number of tax events included.
    pub event_count: usize,
    /// Line items — one per withholding rule applied.
    pub line_items: Vec<ReportLineItem>,
    /// Report status.
    pub status: ReportStatus,
    /// When the report was generated.
    pub generated_at: DateTime<Utc>,
    /// When the report was submitted (if submitted).
    pub submitted_at: Option<DateTime<Utc>>,
    /// Tax authority reference number (if acknowledged).
    pub authority_reference: Option<String>,
}

/// A single line item in a tax report.
///
/// Aggregates all withholdings under a single statutory section
/// and tax category for the reporting period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportLineItem {
    /// Statutory section reference.
    pub statutory_section: String,
    /// Tax category.
    pub tax_category: TaxCategory,
    /// Number of events aggregated in this line.
    pub event_count: usize,
    /// Total gross amount for this line.
    pub total_gross: String,
    /// Total withholding amount for this line.
    pub total_withholding: String,
    /// Applicable rate.
    pub rate_percent: String,
}

/// Parameters for tax report generation.
///
/// Groups the inputs to [`generate_report`] into a single struct to avoid
/// excessive function argument counts while remaining explicit about what
/// each report requires.
#[derive(Debug, Clone)]
pub struct ReportParams {
    /// Entity this report covers.
    pub entity_id: Uuid,
    /// Entity NTN.
    pub ntn: Option<String>,
    /// Jurisdiction of the tax authority.
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

/// Generate a tax report from a set of withholding results.
///
/// Aggregates results by statutory section and produces a single
/// report for submission. This is the final stage of the pipeline
/// before external transmission to FBR IRIS.
pub fn generate_report(params: &ReportParams, results: &[WithholdingResult]) -> TaxReport {
    let jurisdiction_id = params.jurisdiction_id.clone();
    let tax_year = params.tax_year.clone();
    let period_start = params.period_start.clone();
    let period_end = params.period_end.clone();
    let report_type = params.report_type.clone();

    // Aggregate by statutory section.
    let mut aggregated: BTreeMap<String, (TaxCategory, usize, i64, i64, String)> = BTreeMap::new();

    let mut total_gross_cents: i64 = 0;
    let mut total_wht_cents: i64 = 0;
    let mut currency = String::new();

    for r in results {
        if currency.is_empty() {
            currency.clone_from(&r.currency);
        }

        let gross = parse_amount(&r.gross_amount).unwrap_or_else(|| {
            tracing::warn!(
                gross_amount = %r.gross_amount,
                section = %r.statutory_section,
                "unparseable gross_amount in tax report — treating as 0; FBR submission may understate revenue"
            );
            0
        });
        let wht = parse_amount(&r.withholding_amount).unwrap_or_else(|| {
            tracing::warn!(
                withholding_amount = %r.withholding_amount,
                section = %r.statutory_section,
                "unparseable withholding_amount in tax report — treating as 0; FBR submission may understate withholding"
            );
            0
        });

        total_gross_cents = total_gross_cents.saturating_add(gross);
        total_wht_cents = total_wht_cents.saturating_add(wht);

        let entry = aggregated
            .entry(r.statutory_section.clone())
            .or_insert_with(|| (r.tax_category, 0, 0, 0, r.rate_percent.clone()));
        entry.1 = entry.1.saturating_add(1);
        entry.2 = entry.2.saturating_add(gross);
        entry.3 = entry.3.saturating_add(wht);
    }

    let line_items: Vec<ReportLineItem> = aggregated
        .into_iter()
        .map(
            |(section, (category, count, gross, wht, rate))| ReportLineItem {
                statutory_section: section,
                tax_category: category,
                event_count: count,
                total_gross: format_amount(gross),
                total_withholding: format_amount(wht),
                rate_percent: rate,
            },
        )
        .collect();

    TaxReport {
        report_id: Uuid::new_v4(),
        entity_id: params.entity_id,
        ntn: params.ntn.clone(),
        jurisdiction_id,
        period_start,
        period_end,
        tax_year,
        report_type,
        total_gross: format_amount(total_gross_cents),
        total_withholding: format_amount(total_wht_cents),
        currency,
        event_count: results.len(),
        line_items,
        status: ReportStatus::Generated,
        generated_at: Utc::now(),
        submitted_at: None,
        authority_reference: None,
    }
}

// ---------------------------------------------------------------------------
// Pipeline Orchestrator
// ---------------------------------------------------------------------------

/// The tax collection pipeline orchestrator.
///
/// Combines event generation, withholding computation, and report generation
/// into a single coherent pipeline. This is the top-level interface that
/// API routes call.
///
/// ## Pipeline Flow
///
/// ```text
/// Mass Fiscal Event
///   → classify_event()  → TaxEvent
///   → compute()         → Vec<WithholdingResult>
///   → generate_report() → TaxReport
/// ```
pub struct TaxPipeline {
    /// The withholding computation engine.
    pub engine: WithholdingEngine,
}

impl TaxPipeline {
    /// Create a new pipeline with the given withholding engine.
    pub fn new(engine: WithholdingEngine) -> Self {
        Self { engine }
    }

    /// Create a pipeline pre-loaded with Pakistan standard rules.
    pub fn pakistan() -> Self {
        Self {
            engine: WithholdingEngine::with_pakistan_rules(),
        }
    }

    /// Process a tax event through the full pipeline.
    ///
    /// Returns the withholding results. The caller is responsible for:
    /// 1. Persisting the tax event
    /// 2. Executing withholding via Mass fiscal API
    /// 3. Generating and submitting the tax report
    pub fn process_event(&self, event: &TaxEvent) -> Vec<WithholdingResult> {
        self.engine.compute(event)
    }
}

impl Default for TaxPipeline {
    fn default() -> Self {
        Self::new(WithholdingEngine::new())
    }
}

impl std::fmt::Debug for TaxPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaxPipeline")
            .field("engine", &self.engine)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Amount Parsing Utilities
// ---------------------------------------------------------------------------

/// Parse a string amount into cents (integer, 2 decimal places).
///
/// Handles:
/// - "10000" → 1000000 (whole number, assumed in minor units already if no dot)
/// - "10000.00" → 1000000
/// - "4.5" → 450 (when used as a pure number)
/// - "1234.56" → 123456
///
/// Returns `None` for unparseable strings.
pub fn parse_amount(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // If there's a decimal point, parse as fixed-point with 2 decimal places.
    if let Some(dot_pos) = s.find('.') {
        let integer_part = s[..dot_pos].parse::<i64>().ok()?;
        let frac_str = &s[dot_pos + 1..];

        // Pad or truncate to exactly 2 decimal places.
        let frac_cents = match frac_str.len() {
            0 => 0i64,
            1 => frac_str.parse::<i64>().ok()? * 10,
            2 => frac_str.parse::<i64>().ok()?,
            _ => frac_str[..2].parse::<i64>().ok()?,
        };

        let sign: i64 = if integer_part < 0 || s.starts_with('-') {
            -1
        } else {
            1
        };

        // Use checked arithmetic to reject overflow instead of silently
        // capping at i64::MAX. An amount that overflows i64 cents is not
        // representable and must return None rather than a wrong value.
        integer_part
            .abs()
            .checked_mul(100)
            .and_then(|v| v.checked_add(frac_cents))
            .map(|v| sign * v)
    } else {
        // No decimal point — treat as whole units, convert to cents.
        // Use checked_mul to reject overflow.
        s.parse::<i64>().ok().and_then(|v| v.checked_mul(100))
    }
}

/// Parse a rate percentage string into basis points.
///
/// "4.5" → 450 bps, "18.0" → 1800 bps, "15" → 1500 bps.
fn parse_rate_bps(rate_str: &str) -> i64 {
    let rate_str = rate_str.trim();
    if let Some(dot_pos) = rate_str.find('.') {
        let integer_part = rate_str[..dot_pos].parse::<i64>().unwrap_or(0);
        let frac_str = &rate_str[dot_pos + 1..];
        let frac = match frac_str.len() {
            0 => 0i64,
            1 => frac_str.parse::<i64>().unwrap_or(0) * 10,
            _ => frac_str[..2].parse::<i64>().unwrap_or(0),
        };
        integer_part.saturating_mul(100).saturating_add(frac)
    } else {
        rate_str.parse::<i64>().unwrap_or(0).saturating_mul(100)
    }
}

/// Format cents back into a string with 2 decimal places.
///
/// 1000000 → "10000.00", 450 → "4.50"
pub fn format_amount(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    format!("{}{}.{:02}", sign, abs / 100, abs % 100)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Amount parsing --

    #[test]
    fn parse_amount_whole_number() {
        assert_eq!(parse_amount("10000"), Some(1_000_000));
        assert_eq!(parse_amount("0"), Some(0));
        assert_eq!(parse_amount("1"), Some(100));
    }

    #[test]
    fn parse_amount_decimal() {
        assert_eq!(parse_amount("100.50"), Some(10050));
        assert_eq!(parse_amount("1234.56"), Some(123456));
        assert_eq!(parse_amount("0.01"), Some(1));
        assert_eq!(parse_amount("0.10"), Some(10));
    }

    #[test]
    fn parse_amount_empty_returns_none() {
        assert_eq!(parse_amount(""), None);
        assert_eq!(parse_amount("   "), None);
    }

    #[test]
    fn parse_amount_invalid_returns_none() {
        assert_eq!(parse_amount("abc"), None);
    }

    #[test]
    fn format_amount_roundtrip() {
        assert_eq!(format_amount(1_000_000), "10000.00");
        assert_eq!(format_amount(450), "4.50");
        assert_eq!(format_amount(0), "0.00");
        assert_eq!(format_amount(1), "0.01");
        assert_eq!(format_amount(99), "0.99");
    }

    #[test]
    fn parse_rate_bps_decimal() {
        assert_eq!(parse_rate_bps("4.5"), 450);
        assert_eq!(parse_rate_bps("18.0"), 1800);
        assert_eq!(parse_rate_bps("15.0"), 1500);
        assert_eq!(parse_rate_bps("8.0"), 800);
        assert_eq!(parse_rate_bps("0.5"), 50);
    }

    #[test]
    fn parse_rate_bps_whole() {
        assert_eq!(parse_rate_bps("15"), 1500);
        assert_eq!(parse_rate_bps("20"), 2000);
    }

    // -- TaxCategory --

    #[test]
    fn tax_category_count() {
        assert_eq!(TaxCategory::all().len(), 7);
    }

    #[test]
    fn tax_category_serde_roundtrip() {
        for cat in TaxCategory::all() {
            let json = serde_json::to_string(cat).unwrap();
            let parsed: TaxCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(*cat, parsed);
        }
    }

    #[test]
    fn tax_category_display() {
        assert_eq!(TaxCategory::IncomeTax.to_string(), "income_tax");
        assert_eq!(TaxCategory::SalesTax.to_string(), "sales_tax");
    }

    // -- TaxEventType --

    #[test]
    fn tax_event_type_count() {
        assert_eq!(TaxEventType::all().len(), 16);
    }

    #[test]
    fn tax_event_type_serde_roundtrip() {
        for evt in TaxEventType::all() {
            let json = serde_json::to_string(evt).unwrap();
            let parsed: TaxEventType = serde_json::from_str(&json).unwrap();
            assert_eq!(*evt, parsed);
        }
    }

    #[test]
    fn tax_event_type_default_category() {
        assert_eq!(
            TaxEventType::PaymentForGoods.default_category(),
            TaxCategory::IncomeTax
        );
        assert_eq!(
            TaxEventType::SupplyOfGoods.default_category(),
            TaxCategory::SalesTax
        );
        assert_eq!(
            TaxEventType::ImportOfGoods.default_category(),
            TaxCategory::CustomsDuty
        );
    }

    // -- FilerStatus --

    #[test]
    fn filer_status_display() {
        assert_eq!(FilerStatus::Filer.to_string(), "filer");
        assert_eq!(FilerStatus::NonFiler.to_string(), "non_filer");
        assert_eq!(FilerStatus::LateFiler.to_string(), "late_filer");
    }

    // -- TaxEvent --

    #[test]
    fn tax_event_builder() {
        let entity_id = Uuid::new_v4();
        let event = TaxEvent::new(
            entity_id,
            TaxEventType::PaymentForGoods,
            "PK",
            "100000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer)
        .with_statutory_section("ITO 2001 Section 153(1)(a)");

        assert_eq!(event.entity_id, entity_id);
        assert_eq!(event.event_type, TaxEventType::PaymentForGoods);
        assert_eq!(event.tax_category, TaxCategory::IncomeTax);
        assert_eq!(event.ntn.as_deref(), Some("1234567"));
        assert_eq!(event.filer_status, FilerStatus::Filer);
        assert_eq!(event.jurisdiction_id, "PK");
        assert_eq!(event.gross_amount, "100000");
        assert_eq!(event.currency, "PKR");
    }

    #[test]
    fn tax_event_all_builders() {
        let entity_id = Uuid::new_v4();
        let tx_id = Uuid::new_v4();
        let pay_id = Uuid::new_v4();
        let counter_id = Uuid::new_v4();

        let event = TaxEvent::new(
            entity_id,
            TaxEventType::DividendDistribution,
            "PK",
            "500000",
            "PKR",
            "2025-2026",
        )
        .with_mass_transaction(tx_id)
        .with_mass_payment(pay_id)
        .with_counterparty(counter_id)
        .with_tax_category(TaxCategory::IncomeTax)
        .with_metadata(serde_json::json!({"note": "test"}));

        assert_eq!(event.mass_transaction_id, Some(tx_id));
        assert_eq!(event.mass_payment_id, Some(pay_id));
        assert_eq!(event.counterparty_entity_id, Some(counter_id));
        assert_eq!(event.metadata["note"], "test");
    }

    // -- WithholdingEngine --

    #[test]
    fn engine_pakistan_rules_loaded() {
        let engine = WithholdingEngine::with_pakistan_rules();
        assert!(engine.rule_count() > 0);
        assert!(!engine.rules_for_jurisdiction("PK").is_empty());
        assert!(engine.rules_for_jurisdiction("AE").is_empty());
    }

    #[test]
    fn engine_compute_goods_filer() {
        let engine = WithholdingEngine::with_pakistan_rules();

        let event = TaxEvent::new(
            Uuid::new_v4(),
            TaxEventType::PaymentForGoods,
            "PK",
            "100000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer);

        let results = engine.compute(&event);
        assert_eq!(results.len(), 1);

        let r = &results[0];
        assert_eq!(r.rate_percent, "4.5");
        // 100000 * 100 cents = 10000000 cents, * 450 bps / 10000 = 450000 cents = 4500.00
        assert_eq!(r.withholding_amount, "4500.00");
        assert_eq!(r.net_amount, "95500.00");
        assert_eq!(r.statutory_section, "ITO 2001 Section 153(1)(a)");
    }

    #[test]
    fn engine_compute_goods_nonfiler_double_rate() {
        let engine = WithholdingEngine::with_pakistan_rules();

        let event = TaxEvent::new(
            Uuid::new_v4(),
            TaxEventType::PaymentForGoods,
            "PK",
            "100000",
            "PKR",
            "2025-2026",
        );
        // Default is NonFiler

        let results = engine.compute(&event);
        assert_eq!(results.len(), 1);

        let r = &results[0];
        assert_eq!(r.rate_percent, "9.0");
        assert_eq!(r.withholding_amount, "9000.00");
        assert_eq!(r.net_amount, "91000.00");
    }

    #[test]
    fn engine_compute_services_filer() {
        let engine = WithholdingEngine::with_pakistan_rules();

        let event = TaxEvent::new(
            Uuid::new_v4(),
            TaxEventType::PaymentForServices,
            "PK",
            "50000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer);

        let results = engine.compute(&event);
        assert_eq!(results.len(), 1);

        let r = &results[0];
        assert_eq!(r.rate_percent, "8.0");
        assert_eq!(r.withholding_amount, "4000.00");
    }

    #[test]
    fn engine_compute_no_matching_jurisdiction() {
        let engine = WithholdingEngine::with_pakistan_rules();

        let event = TaxEvent::new(
            Uuid::new_v4(),
            TaxEventType::PaymentForGoods,
            "AE",
            "100000",
            "AED",
            "2025",
        );

        let results = engine.compute(&event);
        assert!(results.is_empty());
    }

    #[test]
    fn engine_compute_dividend_filer() {
        let engine = WithholdingEngine::with_pakistan_rules();

        let event = TaxEvent::new(
            Uuid::new_v4(),
            TaxEventType::DividendDistribution,
            "PK",
            "200000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer);

        let results = engine.compute(&event);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].rate_percent, "15.0");
        assert!(results[0].is_final_tax);
    }

    #[test]
    fn engine_compute_crossborder() {
        let engine = WithholdingEngine::with_pakistan_rules();

        let event = TaxEvent::new(
            Uuid::new_v4(),
            TaxEventType::CrossBorderPayment,
            "PK",
            "1000000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer);

        let results = engine.compute(&event);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].rate_percent, "20.0");
        assert_eq!(results[0].withholding_amount, "200000.00");
    }

    #[test]
    fn engine_compute_deterministic() {
        let engine = WithholdingEngine::with_pakistan_rules();

        let event = TaxEvent::new(
            Uuid::new_v4(),
            TaxEventType::PaymentForGoods,
            "PK",
            "100000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer);

        // Run 5 times, verify identical results.
        let first = engine.compute(&event);
        for _ in 0..5 {
            let result = engine.compute(&event);
            assert_eq!(result.len(), first.len());
            for (a, b) in result.iter().zip(first.iter()) {
                assert_eq!(a.rule_id, b.rule_id);
                assert_eq!(a.withholding_amount, b.withholding_amount);
                assert_eq!(a.net_amount, b.net_amount);
            }
        }
    }

    #[test]
    fn engine_load_custom_rules() {
        let mut engine = WithholdingEngine::new();
        assert_eq!(engine.rule_count(), 0);

        engine.load_rules(
            "AE",
            vec![WithholdingRule {
                rule_id: "AE-VAT-5".into(),
                applicable_event_types: vec![TaxEventType::SupplyOfGoods],
                applicable_filer_status: vec![
                    FilerStatus::Filer,
                    FilerStatus::NonFiler,
                    FilerStatus::LateFiler,
                ],
                tax_category: TaxCategory::SalesTax,
                rate_percent: "5.0".into(),
                threshold_min: "0".into(),
                threshold_max: None,
                statutory_section: "UAE VAT Federal Decree-Law No. 8".into(),
                description: "UAE VAT at 5%".into(),
                effective_from: "2018-01-01".into(),
                effective_until: None,
                is_final_tax: true,
            }],
        );

        assert_eq!(engine.rule_count(), 1);

        let event = TaxEvent::new(
            Uuid::new_v4(),
            TaxEventType::SupplyOfGoods,
            "AE",
            "10000",
            "AED",
            "2025",
        )
        .with_tax_category(TaxCategory::SalesTax);

        let results = engine.compute(&event);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].rate_percent, "5.0");
        assert_eq!(results[0].withholding_amount, "500.00");
    }

    // -- Tax Report Generation --

    #[test]
    fn generate_report_from_results() {
        let entity_id = Uuid::new_v4();
        let engine = WithholdingEngine::with_pakistan_rules();

        // Two events.
        let event1 = TaxEvent::new(
            entity_id,
            TaxEventType::PaymentForGoods,
            "PK",
            "100000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer);

        let event2 = TaxEvent::new(
            entity_id,
            TaxEventType::PaymentForServices,
            "PK",
            "50000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer);

        let mut all_results = engine.compute(&event1);
        all_results.extend(engine.compute(&event2));

        let report = generate_report(
            &ReportParams {
                entity_id,
                ntn: Some("1234567".into()),
                jurisdiction_id: "PK".into(),
                tax_year: "2025-2026".into(),
                period_start: "2025-07-01".into(),
                period_end: "2025-07-31".into(),
                report_type: "monthly_withholding".into(),
            },
            &all_results,
        );

        assert_eq!(report.entity_id, entity_id);
        assert_eq!(report.ntn.as_deref(), Some("1234567"));
        assert_eq!(report.event_count, 2);
        assert_eq!(report.status, ReportStatus::Generated);
        assert_eq!(report.line_items.len(), 2);
        assert_eq!(report.currency, "PKR");
        // Total: 4500 + 4000 = 8500
        assert_eq!(report.total_withholding, "8500.00");
    }

    #[test]
    fn generate_report_empty_results() {
        let report = generate_report(
            &ReportParams {
                entity_id: Uuid::new_v4(),
                ntn: None,
                jurisdiction_id: "PK".into(),
                tax_year: "2025-2026".into(),
                period_start: "2025-07-01".into(),
                period_end: "2025-07-31".into(),
                report_type: "monthly_withholding".into(),
            },
            &[],
        );

        assert_eq!(report.event_count, 0);
        assert_eq!(report.total_gross, "0.00");
        assert_eq!(report.total_withholding, "0.00");
        assert!(report.line_items.is_empty());
    }

    // -- ReportStatus --

    #[test]
    fn report_status_display() {
        assert_eq!(ReportStatus::Generated.to_string(), "generated");
        assert_eq!(ReportStatus::Submitted.to_string(), "submitted");
        assert_eq!(ReportStatus::Acknowledged.to_string(), "acknowledged");
        assert_eq!(ReportStatus::Rejected.to_string(), "rejected");
        assert_eq!(ReportStatus::Accepted.to_string(), "accepted");
    }

    #[test]
    fn report_status_serde_roundtrip() {
        let statuses = [
            ReportStatus::Generated,
            ReportStatus::Submitted,
            ReportStatus::Acknowledged,
            ReportStatus::Rejected,
            ReportStatus::Accepted,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let parsed: ReportStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, parsed);
        }
    }

    // -- TaxPipeline --

    #[test]
    fn pipeline_pakistan_processes_event() {
        let pipeline = TaxPipeline::pakistan();

        let event = TaxEvent::new(
            Uuid::new_v4(),
            TaxEventType::PaymentForGoods,
            "PK",
            "100000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer);

        let results = pipeline.process_event(&event);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].withholding_amount, "4500.00");
    }

    #[test]
    fn pipeline_default_is_empty() {
        let pipeline = TaxPipeline::default();
        let event = TaxEvent::new(
            Uuid::new_v4(),
            TaxEventType::PaymentForGoods,
            "PK",
            "100000",
            "PKR",
            "2025-2026",
        );
        let results = pipeline.process_event(&event);
        assert!(results.is_empty());
    }

    #[test]
    fn pipeline_debug_format() {
        let pipeline = TaxPipeline::pakistan();
        let dbg = format!("{pipeline:?}");
        assert!(dbg.contains("TaxPipeline"));
        assert!(dbg.contains("WithholdingEngine"));
    }

    #[test]
    fn engine_debug_format() {
        let engine = WithholdingEngine::with_pakistan_rules();
        let dbg = format!("{engine:?}");
        assert!(dbg.contains("WithholdingEngine"));
        assert!(dbg.contains("PK"));
    }

    // -- Sales Tax --

    #[test]
    fn engine_compute_sales_tax_goods() {
        let engine = WithholdingEngine::with_pakistan_rules();

        let event = TaxEvent::new(
            Uuid::new_v4(),
            TaxEventType::SupplyOfGoods,
            "PK",
            "100000",
            "PKR",
            "2025-2026",
        );

        let results = engine.compute(&event);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].rate_percent, "18.0");
        assert_eq!(results[0].withholding_amount, "18000.00");
        assert_eq!(results[0].statutory_section, "Sales Tax Act 1990 Section 3");
    }
}
