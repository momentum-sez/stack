// genscriptC.js — Continuation from genscriptB line 1203
// Completes buildPartsVI_XVIII() and adds remaining Parts + Appendices + main assembly

const fs = require("fs");
const {
  Document, Packer, Paragraph, TextRun, Table, TableRow, TableCell,
  Header, Footer, AlignmentType, LevelFormat,
  TableOfContents, HeadingLevel, BorderStyle, WidthType, ShadingType,
  VerticalAlign, PageNumber, PageBreak, TabStopType, TabStopPosition,
  PositionalTab, PositionalTabAlignment, PositionalTabRelativeTo, PositionalTabLeader,
} = require("docx");

const { buildAllSections, heading1, heading2, heading3, p, bold, italic, code, makeTable, codeBlock, codeParagraph, spacer, pageBreak, definitionBlock, theoremBlock, BODY_FONT, CODE_FONT, DARK, ACCENT, LIGHT_GRAY, CONTENT_W, PAGE_W, PAGE_H, MARGIN, borders } = require("./generate-spec.js");

// ─────────────────────────────────────────────────────────────
// Complete the remainder of buildPartsVI_XVIII (from Ch30.2 onward)
// PLUS Parts XII through XVIII and Appendices
// ─────────────────────────────────────────────────────────────
function buildRemainingParts() {
  const c = [];

  // ── Ch 30.2 continued (MigrationSaga impl) ────────────────
  c.push(...codeBlock([
    "/// The migration saga: atomic cross-jurisdiction asset transfer.",
    "pub struct MigrationSaga {",
    "    pub migration_id: MigrationId,",
    "    pub asset_id: AssetId,",
    "    pub source: JurisdictionId,",
    "    pub target: JurisdictionId,",
    "    pub state: MigrationState,",
    "    pub completed_steps: Vec<MigrationStep>,",
    "    pub compensation_steps: Vec<CompensationStep>,",
    "}",
    "",
    "impl MigrationSaga {",
    "    pub fn advance(&mut self, proof: MigrationProof) -> Result<(), MigrationError> {",
    "        let next = match (&self.state, &proof) {",
    "            (MigrationState::Initiated, MigrationProof::ComplianceCheck(p)) => {",
    "                self.verify_compliance(p)?;",
    "                MigrationState::SourceLocked",
    "            }",
    "            (MigrationState::SourceLocked, MigrationProof::SourceLock(p)) => {",
    "                self.verify_lock(p)?;",
    "                MigrationState::InTransit",
    "            }",
    "            (MigrationState::InTransit, MigrationProof::Transit(p)) => {",
    "                self.verify_transit(p)?;",
    "                MigrationState::DestinationVerified",
    "            }",
    "            (MigrationState::DestinationVerified, MigrationProof::DestUnlock(p)) => {",
    "                self.verify_unlock(p)?;",
    "                MigrationState::Completed",
    "            }",
    "            _ => return Err(MigrationError::InvalidTransition),",
    "        };",
    "        self.completed_steps.push(MigrationStep {",
    "            from: self.state.clone(),",
    "            to: next.clone(),",
    "            proof: proof.clone(),",
    "            timestamp: chrono::Utc::now(),",
    "        });",
    "        self.state = next;",
    "        Ok(())",
    "    }",
    "",
    "    pub fn compensate(&mut self) -> Result<(), MigrationError> {",
    "        self.state = MigrationState::Compensating;",
    "        for step in self.completed_steps.iter().rev() {",
    "            let comp = step.compensation_action()?;",
    "            comp.execute()?;",
    "            self.compensation_steps.push(comp);",
    "        }",
    "        self.state = MigrationState::Compensated;",
    "        Ok(())",
    "    }",
    "}",
  ]));
  c.push(spacer());

  c.push(theoremBlock("Theorem 30.1 (Migration Atomicity).", "The migration protocol ensures atomicity. Either migration completes fully or compensation returns the asset to its original state. Proof: The saga pattern records every step. Compensation actions are the functional inverse of each step. Compensation executes in reverse order, restoring pre-migration state."));

  // Chapter 31
  c.push(heading2("Chapter 31: Compensation and Recovery"));
  c.push(heading3("31.1 Compensation Actions"));
  c.push(p("Each migration phase defines a compensation action that reverses its effects:"));
  c.push(makeTable(
    ["Phase", "Forward Action", "Compensation Action"],
    [
      ["COMPLIANCE_CHECK", "Verify source + destination compliance", "Release compliance locks"],
      ["ATTESTATION_GATHERING", "Collect required attestations", "Release partial attestation reservations"],
      ["SOURCE_LOCK", "Lock asset at source jurisdiction", "Unlock asset at source"],
      ["TRANSIT", "Transfer asset state between jurisdictions", "Rollback state to source"],
      ["DESTINATION_VERIFICATION", "Verify compliance at destination", "Return to source jurisdiction"],
    ],
    [2800, 3200, 3360]
  ));
  c.push(spacer());

  c.push(heading3("31.2 Saga Pattern"));
  c.push(p("The saga pattern maintains a persistent log of completed steps. On failure at any phase, the compensation engine processes the log in reverse order. Each compensation action is itself atomic and idempotent. The compensation log is append-only and content-addressed, providing an auditable record of recovery operations. Maximum compensation window is bounded by scalability switch S10 (default: 24h)."));
  c.push(...codeBlock([
    "/// Compensation step: the inverse of a forward migration step.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct CompensationStep {",
    "    pub step_id: Uuid,",
    "    pub original_step: MigrationStep,",
    "    pub action: CompensationAction,",
    "    pub executed_at: chrono::DateTime<chrono::Utc>,",
    "    pub result: CompensationResult,",
    "}",
    "",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub enum CompensationAction {",
    "    UnlockSource { asset_id: AssetId, jurisdiction: JurisdictionId },",
    "    RollbackTransit { from: JurisdictionId, to: JurisdictionId },",
    "    ReleaseAttestations { attestation_ids: Vec<AttestationId> },",
    "    NotifyParties { parties: Vec<EntityId>, reason: String },",
    "    RefundFees { amount: MonetaryAmount, recipient: EntityId },",
    "}",
    "",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub enum CompensationResult {",
    "    Success,",
    "    PartialSuccess { completed: Vec<String>, failed: Vec<String> },",
    "    Failed { reason: String },",
    "}",
  ]));
  c.push(spacer());
  c.push(p("Compensation guarantees: every forward step has a defined compensation action, compensation actions are idempotent (safe to retry), the compensation log provides full audit trail, and partial compensation is tracked for manual resolution. Timeout handling: if compensation does not complete within S10, the migration enters FAILED state and requires manual intervention. The failed migration record includes complete forward and compensation logs for forensic analysis."));
  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART XII: INSTITUTIONAL INFRASTRUCTURE MODULES (v0.4.44)
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART XII: INSTITUTIONAL INFRASTRUCTURE MODULES (v0.4.44)"));

  // Chapter 32: Corporate Services
  c.push(heading2("Chapter 32: Corporate Services Module Family"));
  c.push(heading3("32.1 Module Overview"));
  c.push(p("Eight modules provide complete corporate service provider lifecycle management from entity formation through dissolution. These modules define jurisdictional requirements; actual entity formation, cap table management, and governance execution occur through the Mass APIs."));
  c.push(makeTable(
    ["Module", "Function", "Mass API Interface"],
    [
      ["formation", "Entity formation workflows, document requirements, fees", "Mass Org API (formation rules)"],
      ["beneficial-ownership", "UBO registers, threshold rules, disclosure requirements", "Mass Org API (ownership rules)"],
      ["cap-table", "Securities regulations, issuance rules, transfer restrictions", "Mass Investment API (securities rules)"],
      ["secretarial", "Board governance rules, quorum requirements, filing obligations", "Mass Consent API (governance rules)"],
      ["annual-compliance", "Annual return deadlines, compliance calendars, renewal triggers", "Mass Org API (compliance schedule)"],
      ["dissolution", "Voluntary/involuntary winding-up procedures, creditor protections", "Mass Org API (dissolution rules)"],
      ["registered-agent", "Agent appointment requirements, service address rules", "Mass Org API (agent rules)"],
      ["governance-templates", "Standard governance documents, board resolution templates", "Mass Templating Engine (templates)"],
    ],
    [2200, 3600, 3560]
  ));
  c.push(spacer());

  c.push(heading3("32.2 Formation Module"));
  c.push(p("The formation module defines entity types permitted within each jurisdiction, required formation documents, filing fees, and processing timelines. For each entity type, the module specifies: permitted entity classifications (LLC, PLC, LLP, Branch, SPV, Trust, Foundation, DAO wrapper), minimum formation requirements (name availability, registered office, minimum directors/members), document requirements (memorandum of association, articles, director consents, compliance declarations), fee schedules (formation fees, name reservation fees, expedited processing surcharges), and estimated processing times."));
  c.push(p([bold("Pakistan Example."), " The Pakistan GovOS formation module encodes Companies Act 2017 requirements for SECP registration: single-member company (minimum 1 director, 1 shareholder), private limited company (minimum 2 directors, 2 shareholders, max 50), public limited company (minimum 3 directors, 7 shareholders), and not-for-profit association (Section 42 license required)."]));
  c.push(...codeBlock([
    "/// Formation rules for a jurisdiction.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct FormationRules {",
    "    pub jurisdiction: JurisdictionId,",
    "    pub entity_types: Vec<EntityTypeSpec>,",
    "    pub name_rules: NameReservationRules,",
    "    pub registered_office_required: bool,",
    "    pub minimum_capital: Option<MonetaryAmount>,",
    "}",
    "",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct EntityTypeSpec {",
    "    pub entity_type: EntityType,",
    "    pub min_directors: u32,",
    "    pub min_shareholders: u32,",
    "    pub max_shareholders: Option<u32>,",
    "    pub required_documents: Vec<DocumentRequirement>,",
    "    pub fees: FeeSchedule,",
    "    pub processing_time_days: u32,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("32.3 Beneficial Ownership Module"));
  c.push(p("The beneficial ownership module defines UBO registry requirements: disclosure thresholds (typically 10-25% depending on jurisdiction), verification procedures, update obligations, and access rules. The module encodes FATF Recommendation 24/25 requirements as machine-readable predicates. Pakistan implementation: SECP Form 45 (Declaration of Beneficial Ownership) with 10% threshold, annual confirmation, and change notification within 15 days."));
  c.push(...codeBlock([
    "/// Beneficial ownership requirements per jurisdiction.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct BeneficialOwnershipRules {",
    "    pub jurisdiction: JurisdictionId,",
    "    pub disclosure_threshold_pct: Decimal,",
    "    pub verification_requirements: Vec<VerificationRequirement>,",
    "    pub update_deadline_days: u32,",
    "    pub annual_confirmation_required: bool,",
    "    pub access_rules: AccessRules,",
    "    pub penalties: PenaltySchedule,",
    "}",
    "",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct BeneficialOwner {",
    "    pub person_id: PersonId,",
    "    pub ownership_percentage: Decimal,",
    "    pub control_type: ControlType,  // Direct, Indirect, JointControl",
    "    pub verification_status: VerificationStatus,",
    "    pub disclosed_at: chrono::DateTime<chrono::Utc>,",
    "    pub next_confirmation_due: chrono::NaiveDate,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("32.4 Capitalization Table Module"));
  c.push(p("The cap table module defines securities regulation requirements for equity issuance, transfer restrictions, and disclosure obligations. Share classes, authorized capital, and transfer restrictions are defined per jurisdiction. The module interfaces with Mass Investment API for actual cap table management, providing jurisdictional validation rules."));
  c.push(p("For each jurisdiction, the module specifies: permitted share classes (ordinary, preference, redeemable, convertible), authorized share capital limits, pre-emption rights requirements, transfer restriction regimes (freely transferable, board approval, right of first refusal), and disclosure thresholds for substantial holdings."));
  c.push(...codeBlock([
    "/// Securities rules for a jurisdiction's cap table management.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct SecuritiesRules {",
    "    pub jurisdiction: JurisdictionId,",
    "    pub permitted_classes: Vec<ShareClassSpec>,",
    "    pub max_authorized_capital: Option<MonetaryAmount>,",
    "    pub transfer_restrictions: TransferRestrictionRegime,",
    "    pub preemption_rights: PreemptionRights,",
    "    pub substantial_holding_threshold_pct: Decimal,",
    "    pub prospectus_requirements: ProspectusRequirements,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("32.5 Secretarial Module"));
  c.push(p("The secretarial module specifies board governance requirements: minimum meeting frequencies, quorum rules, notice periods, resolution types (ordinary, special, written), and minute-keeping obligations. Pakistan implementation: Companies Act 2017 Section 159 (annual general meetings), Section 169 (board meetings minimum quarterly), and Section 178 (special resolutions requiring 75% approval)."));

  c.push(heading3("32.6 Annual Compliance Module"));
  c.push(p("The annual compliance module maintains compliance calendars: annual return filing deadlines, financial statement submission dates, audit requirements, and regulatory filing schedules. For each obligation, the module tracks: filing authority, deadline computation rules, late filing penalties, and pre-deadline notification triggers."));
  c.push(p([bold("Pakistan Example."), " Annual compliance calendar includes: SECP annual return (Form A, due within 30 days of AGM), FBR income tax return (due September 30), sales tax returns (monthly, due 18th of following month), and financial statements (within 4 months of year-end)."]));

  c.push(heading3("32.7 Dissolution Module"));
  c.push(p("The dissolution module specifies voluntary and involuntary winding-up procedures: creditor notification requirements, asset distribution priorities, regulatory clearances, and strike-off conditions. Dissolution procedures maintain compliance with jurisdiction-specific requirements while protecting creditor rights."));

  // Chapter 33: Identity and Credentialing
  c.push(pageBreak());
  c.push(heading2("Chapter 33: Identity and Credentialing Module Family"));
  c.push(heading3("33.1 Module Overview"));
  c.push(p("Five modules implement progressive four-tier KYC with DID-native identity management."));
  c.push(makeTable(
    ["Tier", "Access Level", "Requirements", "Limits"],
    [
      ["Tier 0", "Pseudonymous exploration", "Self-declared DID, device attestation", "$1,000/month transaction limit"],
      ["Tier 1", "Basic participation", "Email, phone, basic identity verification", "$10,000/month"],
      ["Tier 2", "Full zone access", "Government ID, address, source of funds", "$1,000,000/month"],
      ["Tier 3", "Institutional access", "Enhanced due diligence, corporate docs, UBO", "Unlimited"],
    ],
    [1200, 2200, 3200, 2760]
  ));
  c.push(spacer());

  c.push(heading3("33.2 Core Identity Module"));
  c.push(p("The core identity module manages DID lifecycle: creation, key rotation, recovery, and deactivation. DIDs conform to W3C DID Core specification. Key material supports Ed25519 (primary), secp256k1 (blockchain compatibility), and BLS12-381 (threshold signatures). NADRA integration for Pakistan: CNIC-to-NTN cross-reference, biometric verification via NADRA API, real-time identity confirmation."));

  c.push(heading3("33.3 Progressive KYC Tiers"));
  c.push(p("Each tier defines verification requirements, evidence types, and review procedures. Tier transitions require completing additional verification steps while preserving existing credentials. Tier downgrade triggers when credentials expire or adverse information surfaces."));

  c.push(heading3("33.4 Credentials Module"));
  c.push(p("Verifiable Credentials follow W3C VC Data Model 2.0. Credential types include identity credentials (KYC tier attestation), compliance credentials (compliance state attestation), license credentials (business license status), and corridor credentials (cross-border authorization). All credentials support BBS+ selective disclosure."));

  c.push(heading3("33.5 Binding Module"));
  c.push(p("The binding module establishes cryptographic links between identities, entities, and credentials. Binding types: identity-to-entity (director appointment, UBO declaration), entity-to-jurisdiction (registration binding), and credential-to-asset (compliance attestation binding)."));

  // Chapter 34: Tax and Revenue
  c.push(heading2("Chapter 34: Tax and Revenue Module Family"));
  c.push(heading3("34.1 Module Overview"));
  c.push(p("Five modules provide configurable tax regime definitions, comprehensive fee schedules, and international reporting."));

  c.push(heading3("34.2 Tax Framework Module"));
  c.push(p("The tax framework module defines jurisdictional tax regimes: income tax classifications, withholding schedules, exemptions, and credits. Each regime specifies applicable rates, thresholds, and computation rules as machine-readable predicates."));
  c.push(p([bold("Pakistan Tax Collection Pipeline."), " Every economic activity on Mass generates a tax event. The pipeline: (1) Transaction occurs via Mass Treasury API, (2) MSEZ Pack Trilogy identifies applicable tax rules from ITO 2001 / STA 1990, (3) Automatic withholding at source per applicable WHT schedule, (4) Real-time reporting to FBR IRIS, (5) AI-powered gap analysis closes evasion. Target: raise Pakistan tax-to-GDP from 10.3% to 15%+."]));
  c.push(...codeBlock([
    "/// Tax event generated for every economic activity.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct TaxEvent {",
    "    pub event_id: Uuid,",
    "    pub transaction_ref: TransactionId,",
    "    pub tax_type: TaxType,",
    "    pub gross_amount: MonetaryAmount,",
    "    pub tax_rate: Decimal,",
    "    pub tax_amount: MonetaryAmount,",
    "    pub withholding_agent: EntityId,",
    "    pub taxpayer_ntn: Option<String>,",
    "    pub section_reference: String,  // e.g. \"ITO 2001 s.153(1)(a)\"",
    "    pub timestamp: chrono::DateTime<chrono::Utc>,",
    "    pub reported_to: Vec<TaxAuthority>,  // e.g. FBR IRIS",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("34.3 Fee Schedules Module"));
  c.push(p("The fee schedules module defines all charges applicable within a jurisdiction: formation fees, annual fees, transaction fees, and regulatory levies. Fee schedules support tiered pricing, volume discounts, and promotional rates."));

  c.push(heading3("34.4 Incentive Programs Module"));
  c.push(p("The incentive programs module defines tax holidays, reduced rates, and other fiscal incentives. Program eligibility conditions, application procedures, and benefit calculations are encoded as machine-readable rules. Pakistan example: BOI incentive zones under Special Economic Zones Act 2012 provide 10-year tax holidays for qualifying enterprises in designated SEZs."));

  c.push(heading3("34.5 International Reporting Module"));
  c.push(p("The international reporting module implements OECD CRS, US FATCA, and EU DAC6/DAC7 reporting requirements. Automated identification of reportable accounts, computation of reportable amounts, and generation of XML reports in prescribed formats. Report generation follows jurisdiction-specific schedules with automatic submission to designated competent authorities."));

  // Chapter 35: Capital Markets
  c.push(pageBreak());
  c.push(heading2("Chapter 35: Capital Markets Module Family"));
  c.push(heading3("35.1 Module Overview"));
  c.push(p("Nine modules deliver securities infrastructure from issuance through settlement."));
  c.push(makeTable(
    ["Module", "Function"],
    [
      ["securities-issuance", "Primary issuance: equity, debt, hybrid instruments, tokenized securities"],
      ["trading", "Order book management, matching engine integration, market data"],
      ["post-trade", "Trade confirmation, netting, settlement instruction generation"],
      ["csd", "Central Securities Depository: holding, transfers, corporate actions"],
      ["clearing", "Central counterparty clearing, margin management, default waterfall"],
      ["dvp-pvp", "Atomic Delivery-versus-Payment and Payment-versus-Payment settlement"],
      ["corporate-actions", "Dividends, splits, mergers, tender offers, rights issues"],
      ["surveillance", "Market abuse detection, insider trading monitoring, reporting"],
      ["fund-admin", "NAV computation, subscription/redemption, investor reporting"],
    ],
    [2400, 6960]
  ));
  c.push(spacer());

  c.push(heading3("35.2 Securities Issuance Module"));
  c.push(p("The securities issuance module supports primary market operations. Security types: common equity, preferred equity, convertible notes, corporate bonds, asset-backed securities, and tokenized real-world assets. Issuance workflows enforce jurisdiction-specific prospectus requirements, investor suitability checks, and regulatory approvals."));

  c.push(heading3("35.3 Trading Module"));
  c.push(p("The trading module provides order management and market data infrastructure. Order types: limit, market, stop-loss, fill-or-kill, immediate-or-cancel. The module defines trading rules (circuit breakers, lot sizes, tick sizes) per jurisdiction."));

  c.push(heading3("35.4 Post-Trade Module"));
  c.push(p("Post-trade processing: trade matching and confirmation (T+0), netting computation (bilateral and multilateral), settlement instruction generation, and fail management."));

  c.push(heading3("35.5 CSD Module"));
  c.push(p("The Central Securities Depository module manages securities holding and transfer. Holding models: direct (investor accounts at CSD), nominee (omnibus accounts through participants), and hybrid. Transfer mechanisms: free-of-payment, delivery-versus-payment, and pledge/release."));

  c.push(heading3("35.6 Clearing Module"));
  c.push(p("Central counterparty clearing with margin management: initial margin (risk-based), variation margin (mark-to-market), and default fund contributions. Default waterfall: defaulter margin, defaulter default fund, CCP skin-in-the-game, non-defaulter default fund, CCP equity."));

  c.push(heading3("35.7 DVP-PVP Module"));
  c.push(p("Atomic settlement using hash-time-locked contracts (HTLCs) for cross-system DVP and PVP. DVP models: Model 1 (gross/gross, real-time), Model 2 (gross/net, end-of-day), Model 3 (net/net, multilateral netting). The module integrates with the corridor system for cross-border PVP in multiple currencies."));

  c.push(heading3("35.8 Corporate Actions Module"));
  c.push(p("The corporate actions module processes mandatory and voluntary events: cash dividends, stock dividends, splits, reverse splits, mergers, tender offers, rights issues, and spin-offs. Event processing follows ISO 15022/20022 message standards."));

  c.push(heading3("35.9 Surveillance Module"));
  c.push(p("Market surveillance detects prohibited activities: insider trading, market manipulation (spoofing, layering, wash trading), front-running, and benchmark manipulation. Detection algorithms operate on order-level data with configurable sensitivity thresholds. Alerts generate regulatory reports in jurisdiction-specific formats."));

  // Chapter 36: Trade and Commerce
  c.push(heading2("Chapter 36: Trade and Commerce Module Family"));
  c.push(heading3("36.1 Module Overview"));
  c.push(p("Six modules implement trade finance instruments and cross-border commercial operations. These modules integrate with the corridor system for cross-jurisdictional trade."));

  c.push(heading3("36.2 Letters of Credit Module"));
  c.push(p("The letters of credit module supports documentary credits per UCP 600 and ISP98. LC types: irrevocable, confirmed, standby, transferable, back-to-back, and revolving. Document examination follows strict compliance rules encoded as machine-readable predicates."));

  c.push(heading3("36.3 Trade Documents Module"));
  c.push(p("Electronic trade documents per MLETR (Model Law on Electronic Transferable Records): bills of lading (ocean, multimodal), warehouse receipts, promissory notes, bills of exchange, and certificates of origin. Document authenticity verified through digital signatures and content-addressing."));

  c.push(heading3("36.4 Supply Chain Finance Module"));
  c.push(p("Supply chain finance instruments: reverse factoring, dynamic discounting, inventory finance, and pre-shipment finance. Multi-tier supply chain visibility with compliance verification at each tier."));

  c.push(heading3("36.5 Customs Module"));
  c.push(p("Electronic customs declarations with HS code classification. Duty calculation with preferential rate application under bilateral and multilateral agreements. Free zone procedures for goods in free zones (temporary admission, re-export). AEO (Authorized Economic Operator) status management and benefits application."));
  c.push(p([bold("Corridor Integration."), " The PAK\u2194KSA corridor automates customs declarations between Pakistani and Saudi customs authorities. HS code harmonization resolves classification discrepancies. Preferential rates under bilateral agreements apply automatically."]));

  c.push(heading3("36.6 Trade Insurance Module"));
  c.push(p("Trade credit insurance (buyer default protection), cargo insurance (marine, air, land), and political risk insurance (expropriation, currency inconvertibility, political violence coverage)."));

  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART XIII: MASS API INTEGRATION LAYER (NEW)
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART XIII: MASS API INTEGRATION LAYER"));

  c.push(heading2("Chapter 37: The msez-mass-bridge Crate"));
  c.push(p("The msez-mass-bridge crate formalizes the interface between Mass APIs (System A) and the MSEZ Stack (System B). This is the architectural boundary that enforces the two-system separation. Mass APIs call into the bridge for jurisdictional context. The MSEZ Stack provides context through trait implementations."));

  c.push(heading3("37.1 JurisdictionalContext Trait"));
  c.push(...codeBlock([
    "/// Trait that Mass APIs call to get jurisdictional context.",
    "/// Implemented by the MSEZ Stack for each deployed jurisdiction.",
    "pub trait JurisdictionalContext: Send + Sync {",
    "    /// Returns permitted entity types for this jurisdiction.",
    "    fn permitted_entity_types(&self) -> Vec<EntityType>;",
    "",
    "    /// Validates a formation application against jurisdictional rules.",
    "    fn validate_formation(",
    "        &self,",
    "        app: &FormationApplication,",
    "    ) -> Result<(), ComplianceViolation>;",
    "",
    "    /// Returns the fee schedule for a given operation.",
    "    fn fee_schedule(&self, operation: Operation) -> FeeSchedule;",
    "",
    "    /// Evaluates compliance tensor for a proposed action.",
    "    fn evaluate_compliance(",
    "        &self,",
    "        asset: &AssetId,",
    "        jurisdiction: &JurisdictionId,",
    "        domains: &[ComplianceDomain],",
    "    ) -> ComplianceTensorSlice;",
    "",
    "    /// Returns current Pack Trilogy state for this jurisdiction.",
    "    fn pack_state(&self) -> PackTrilogyState;",
    "",
    "    /// Returns securities issuance rules for this jurisdiction.",
    "    fn securities_rules(&self, security_type: SecurityType) -> SecuritiesRules;",
    "",
    "    /// Returns KYC tier requirements for this jurisdiction.",
    "    fn kyc_requirements(&self, tier: KycTier) -> KycRequirements;",
    "",
    "    /// Returns governance rules (quorum, voting, delegation).",
    "    fn governance_rules(&self) -> GovernanceRules;",
    "",
    "    /// Returns tax rules applicable to a transaction.",
    "    fn tax_rules(",
    "        &self,",
    "        transaction_type: TransactionType,",
    "        parties: &TransactionParties,",
    "    ) -> TaxRules;",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("37.2 Mass Primitive Mapping"));
  c.push(p("Each of the five Mass primitives calls specific bridge methods:"));
  c.push(makeTable(
    ["Mass Primitive", "API Endpoint", "Bridge Methods Called"],
    [
      ["Entities", "organization-info.api.mass.inc", "permitted_entity_types(), validate_formation(), fee_schedule(), pack_state()"],
      ["Ownership", "investment-info (Heroku seed)", "securities_rules(), evaluate_compliance()"],
      ["Fiscal", "treasury-info.api.mass.inc", "tax_rules(), fee_schedule(), evaluate_compliance()"],
      ["Identity", "Distributed across org + consent", "kyc_requirements(), evaluate_compliance()"],
      ["Consent", "consent.api.mass.inc", "governance_rules(), evaluate_compliance()"],
    ],
    [1600, 3000, 4760]
  ));
  c.push(spacer());

  c.push(heading3("37.3 The Organs"));
  c.push(p("The Organs are regulated interface implementations that make Mass deployable in licensed environments:"));
  c.push(makeTable(
    ["Organ", "Function", "Regulatory Status"],
    [
      ["Center of Mass", "Banking infrastructure: accounts, payments, custody, FX, on/off-ramps", "FinCEN MSB, state MTLs, UAE Central Bank API, Northern Trust custody"],
      ["Torque", "Licensing infrastructure: license applications, compliance monitoring, renewals", "ADGM FSP, Dubai DFZC integration"],
      ["Inertia", "Corporate services: entity formation, secretarial, registered agent", "CSP licenses, SECP authorized agent"],
    ],
    [1800, 4200, 3360]
  ));
  c.push(spacer());
  c.push(p("Each Organ implements a subset of Mass API functionality within a specific regulatory regime. The Organ does not change Mass API behavior; it adds the regulatory licenses and operational compliance required for lawful operation. The MSEZ Stack provides the jurisdictional context that each Organ requires through the JurisdictionalContext trait."));

  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART XIV: GovOS ARCHITECTURE (NEW)
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART XIV: GovOS ARCHITECTURE"));
  c.push(p("GovOS is the emergent product when the full MSEZ Stack + Mass APIs are deployed for a sovereign government. It is not a separate product. It is what the Stack becomes at national scale. Pakistan serves as the reference architecture."));

  c.push(heading2("Chapter 38: Four-Layer Model"));
  c.push(p("The GovOS architecture comprises four layers:"));
  c.push(makeTable(
    ["Layer", "Name", "Function"],
    [
      ["01", "Experience", "Dashboards, portals, citizen-facing services, AI-powered interfaces"],
      ["02", "Platform Engine", "Five Mass primitives + supporting infrastructure + regulated organs"],
      ["03", "Jurisdictional Configuration", "MSEZ Pack Trilogy encoding national law in machine-readable format"],
      ["04", "National System Integration", "Connections to existing government systems (Mass enhances, never replaces)"],
    ],
    [800, 2400, 6160]
  ));
  c.push(spacer());

  c.push(p([bold("Layer 01 \u2014 Experience."), " Configurable UX assembled from Mass primitives: GovOS Console (40+ ministries), Tax and Revenue Dashboard, Digital Free Zone portal, Citizen Tax and Services portal, and Regulator Console. AI-powered natural-language task interface, intelligent workflow routing, tax filing assistant, and revenue forecasting."]));
  c.push(p([bold("Layer 02 \u2014 Platform Engine."), " The five Mass programmable primitives (Entities, Ownership, Fiscal, Identity, Consent) plus supporting infrastructure (Event and Task Engine, Cryptographic Attestation, Compliance Tensor, App Marketplace) plus regulated organs (Local CSP, Local Bank Integration, SBP API Gateway, Licensing Authority Bridge). Auto-compliance validation generates tax events on every transaction with withholding at source."]));
  c.push(p([bold("Layer 03 \u2014 Jurisdictional Configuration."), " The MSEZ Pack Trilogy encoding Pakistani national law. Lawpacks: Income Tax Ordinance 2001, Sales Tax Act 1990, Federal Excise Act 2005, Customs Act 1969, and Companies Act 2017 in Akoma Ntoso XML. Regpacks: FBR tax calendars, filing deadlines, withholding rate tables, SROs, FATF AML/CFT rules, and sanctions lists. Licensepacks: NTN registration, sales tax registration, 15+ license categories across BOI, PTA, PEMRA, DRAP, and provincial authorities. Arbitration corpus: tax tribunal rulings, ATIR precedents, and court filings."]));
  c.push(p([bold("Layer 04 \u2014 National System Integration."), " Mass enhances, never replaces existing government systems. For Pakistan: FBR IRIS (tax administration, e-invoicing, returns, NTN registry), SBP Raast (instant payments, PKR collection), NADRA (national identity, CNIC-NTN cross-reference), SECP (corporate registry, beneficial ownership), SIFC (investment facilitation, FDI tracking), and AGPR (government accounts, expenditure tracking)."]));

  c.push(heading2("Chapter 39: Sovereign AI Spine"));
  c.push(p("The Sovereign AI Spine is embedded at every GovOS layer. The foundation model runs on Pakistani data centers with zero data egress. The Spine is not an optional add-on; it is the intelligence layer that transforms the GovOS from a transactional system into an analytical governance platform."));
  c.push(makeTable(
    ["Capability", "Function", "Key Metrics"],
    [
      ["Tax Intelligence", "Gap analysis, evasion detection, under-reporting identification, compliance scoring, revenue projection", "Target: identify 30-40% of current tax gap"],
      ["Operational Intelligence", "Spend anomaly detection across 40+ ministries, predictive budgeting, vendor risk scoring", "Anomaly detection within 24h"],
      ["Regulatory Awareness", "Pre-action compliance verification, predictive legal risk, SRO impact modeling", "SRO propagation within 4h"],
      ["Forensic and Audit", "Cross-department pattern detection, procurement irregularity flagging, IMF PIMA alignment, transfer pricing analysis", "Cross-department correlation"],
    ],
    [2000, 4400, 2960]
  ));
  c.push(spacer());
  c.push(p([bold("Data Sovereignty."), " All model weights, training data, and inference run within Pakistani jurisdiction on on-premise GPU infrastructure. No data leaves Pakistani borders. The Sovereign AI Spine is the mechanism by which the GovOS maintains analytical independence. Infrastructure: on-premise GPU cluster (minimum 8x A100 80GB for training, 4x A100 for inference), air-gapped training environment, model versioning with cryptographic attestation."]));
  c.push(p([bold("Tax Intelligence Pipeline."), " The AI analyzes every transaction processed through Mass to identify patterns: under-declaration detection (comparing reported income against transaction volume), sector-specific benchmarking (comparing entity performance against sector norms), network analysis (identifying related-party transactions and transfer pricing risks), and temporal anomaly detection (identifying seasonal patterns inconsistent with declared activity). Estimated revenue recovery: 2-4% of GDP through improved compliance, translating to PKR 1.5-3.0 trillion annually."]));
  c.push(...codeBlock([
    "/// Sovereign AI inference request (runs on-premise only).",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct AiInferenceRequest {",
    "    pub request_type: AiRequestType,",
    "    pub input_data: EncryptedPayload,",
    "    pub model_version: ModelVersion,",
    "    pub jurisdiction: JurisdictionId,",
    "    pub data_classification: DataClassification,",
    "}",
    "",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub enum AiRequestType {",
    "    TaxGapAnalysis { entity_id: EntityId, period: TaxPeriod },",
    "    SpendAnomalyDetection { ministry_id: MinistryId },",
    "    ComplianceScoring { entity_id: EntityId },",
    "    RevenueProjection { jurisdiction: JurisdictionId, horizon_months: u32 },",
    "    TransferPricingRisk { transaction_id: TransactionId },",
    "}",
  ]));
  c.push(spacer());

  c.push(heading2("Chapter 40: Tax Collection Pipeline"));
  c.push(p("Every economic activity on Mass generates a tax event. The pipeline operates as follows:"));
  c.push(makeTable(
    ["Stage", "Action", "System"],
    [
      ["1. Transaction", "Economic activity occurs via Mass Treasury API", "Mass Fiscal"],
      ["2. Tax Identification", "MSEZ Pack Trilogy identifies applicable tax rules", "MSEZ Pack Trilogy"],
      ["3. Withholding", "Automatic withholding at source per WHT schedule", "Mass Fiscal + MSEZ Bridge"],
      ["4. Reporting", "Real-time reporting to FBR IRIS", "National System Integration"],
      ["5. Gap Analysis", "AI-powered gap analysis identifies evasion patterns", "Sovereign AI Spine"],
      ["6. Enforcement", "Automated compliance actions for non-filing entities", "GovOS Console"],
    ],
    [1800, 4200, 3360]
  ));
  c.push(spacer());

  c.push(heading2("Chapter 41: Sovereignty Handover"));
  c.push(p("The GovOS deployment follows a 24-month sovereignty handover framework. At the end of month 24, full operational control transfers to Pakistani engineers and administrators."));
  c.push(makeTable(
    ["Phase", "Timeline", "Milestones"],
    [
      ["1. Foundation", "Months 1-6", "Core infrastructure deployment, FBR IRIS integration, SBP Raast connection, NADRA identity bridge, initial AI training on Pakistani tax data"],
      ["2. Expansion", "Months 7-12", "All 40+ ministry onboarding, corridor activation (PAK-KSA, PAK-UAE), license module deployment across SECP, BOI, PTA, provincial authorities"],
      ["3. Optimization", "Months 13-18", "AI model refinement, tax gap reduction measurement, cross-department analytics, CPEC corridor planning"],
      ["4. Handover", "Months 19-24", "Knowledge transfer to Pakistani engineering team, operational documentation, support transition, full sovereignty transfer"],
    ],
    [1600, 1600, 6160]
  ));
  c.push(spacer());
  c.push(p("Phase 1 (Months 1-6): Deploy core Mass primitives, connect national systems, begin AI training. Phase 2 (Months 7-12): Scale to all ministries, activate bilateral corridors, deploy full licensing. Phase 3 (Months 13-18): Optimize AI models, measure tax collection improvements, launch analytics dashboards. Phase 4 (Months 19-24): Transfer complete operational control to Pakistani engineers. Momentum retains advisory role only."));

  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART XV: MASS PROTOCOL INTEGRATION
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART XV: MASS PROTOCOL INTEGRATION"));

  c.push(heading2("Chapter 42: Protocol Overview"));
  c.push(heading3("42.1 Protocol Architecture"));
  c.push(p("Mass Protocol provides the settlement layer for the SEZ Stack. The protocol architecture comprises: transaction layer (private and public transaction types), consensus layer (DAG-based with jurisdictional awareness), proving layer (Plonky3 STARKs with Groth16 wrapping), and anchoring layer (periodic state commitment to external chains)."));
  c.push(p("Integration patterns between the MSEZ Stack and Mass Protocol follow the anchor-and-verify model: MSEZ Stack operations produce receipts, receipts aggregate into checkpoints, checkpoints anchor to Mass Protocol periodically, and protocol provides finality guarantees for anchored state."));

  c.push(heading3("42.2 Integration Patterns"));
  c.push(makeTable(
    ["Pattern", "Use Case", "Flow"],
    [
      ["Direct Anchoring", "High-value settlements", "Receipt -> Checkpoint -> L1 Anchor -> Finality"],
      ["Batch Anchoring", "Routine operations", "Multiple Receipts -> Aggregated Checkpoint -> L1 Anchor"],
      ["Corridor Settlement", "Cross-border operations", "Corridor State -> Bilateral Checkpoint -> L1 Anchor"],
      ["Deferred Anchoring", "Low-priority operations", "Receipts accumulated, anchored at next epoch"],
    ],
    [2200, 2800, 4360]
  ));
  c.push(spacer());

  c.push(heading2("Chapter 43: Verifiable Credentials"));
  c.push(heading3("43.1 Credential Types"));
  c.push(makeTable(
    ["Credential Type", "Issuer", "Verifier", "Selective Disclosure"],
    [
      ["KYC Attestation", "Verification Provider", "Service Provider", "Tier level without identity details"],
      ["License Credential", "Licensing Authority", "Counterparties", "Active status without full license details"],
      ["Compliance Certificate", "Compliance Watcher", "Regulators", "Domain-specific state"],
      ["Corridor Authorization", "Corridor Administrator", "Jurisdiction Nodes", "Permitted operations subset"],
      ["Entity Registration", "Corporate Registry", "Third Parties", "Entity type and status"],
      ["Tax Compliance", "Tax Authority", "Financial Institutions", "Good standing without financials"],
    ],
    [2200, 2000, 2000, 3160]
  ));
  c.push(spacer());

  c.push(heading2("Chapter 44: Arbitration System"));
  c.push(heading3("44.1 Institution Registry"));
  c.push(p("The arbitration system maintains a registry of recognized institutions. Recognized institutions: DIFC-LCIA Arbitration Centre, Singapore International Arbitration Centre (SIAC), AIFC International Arbitration Centre (IAC), International Chamber of Commerce (ICC) International Court of Arbitration, ADGM Arbitration Centre. Each institution has associated rules encoded as machine-readable specifications: filing procedures, tribunal formation rules, procedural timelines, fee schedules, and enforcement mechanisms."));

  c.push(heading3("44.2 Ruling Enforcement"));
  c.push(p("Arbitration rulings are encoded as Verifiable Credentials signed by the tribunal. Ruling VCs trigger automatic state transitions in affected Smart Assets. Enforcement actions: asset freezes, ownership transfers, payment orders, license modifications, and compliance state updates. The ruling VC includes: parties, relief granted, compliance obligations, enforcement deadline, and appeal period."));
  c.push(...codeBlock([
    "/// An arbitration ruling encoded as a VC.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct ArbitrationRuling {",
    "    pub ruling_id: RulingId,",
    "    pub institution: ArbitrationInstitution,",
    "    pub parties: Vec<EntityId>,",
    "    pub relief: Vec<ReliefItem>,",
    "    pub compliance_obligations: Vec<ComplianceObligation>,",
    "    pub enforcement_deadline: chrono::DateTime<chrono::Utc>,",
    "    pub appeal_period_days: u32,",
    "    pub ruling_proof: ZkProof,  // pi_ruling circuit",
    "}",
  ]));
  c.push(spacer());

  c.push(heading2("Chapter 45: Agentic Execution Framework"));
  c.push(heading3("45.1 Trigger System"));
  c.push(p("The agentic execution framework enables Smart Assets to respond autonomously to environmental events. Triggers are conditions that, when satisfied, initiate predefined actions without human intervention. Trigger categories: regulatory triggers (sanctions list updates, license expirations, guidance changes), arbitration triggers (rulings received, appeal deadlines passed, enforcement due), settlement triggers (checkpoint required, finalization anchor), and lifecycle triggers (key rotation due, attestation expiring)."));
  c.push(...codeBlock([
    "/// A trigger: environmental condition that initiates autonomous action.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub enum Trigger {",
    "    SanctionsListUpdate { list_id: String, effective: chrono::DateTime<chrono::Utc> },",
    "    LicenseExpiration { license_id: String, expires: chrono::DateTime<chrono::Utc> },",
    "    RulingReceived { ruling_id: RulingId },",
    "    CheckpointDue { asset_id: AssetId, interval: std::time::Duration },",
    "    AttestationExpiring { attestation_id: String, expires: chrono::DateTime<chrono::Utc> },",
    "    KeyRotationDue { key_id: String },",
    "    GuidanceChange { regpack_digest: Digest },",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("45.2 Standard Policy Library"));
  c.push(p("The standard policy library provides pre-built responses to common triggers: auto-freeze on sanctions match (OFAC, EU, UN designations), auto-renewal for expiring attestations, auto-checkpoint at configurable intervals, compliance re-evaluation on regpack update, and escalation workflows for human review when automated resolution is insufficient."));

  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART XVI: SECURITY AND HARDENING
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART XVI: SECURITY AND HARDENING"));

  c.push(heading2("Chapter 46: Security Architecture"));
  c.push(heading3("46.1 Threat Model"));
  c.push(p("The system defends against five threat categories, each with specific mitigation strategies and detection mechanisms. The threat model assumes a powerful adversary with access to network traffic, the ability to compromise individual nodes, and economic incentive to attack."));
  c.push(makeTable(
    ["Threat", "Mitigation", "Detection"],
    [
      ["Unauthorized entity formation", "Jurisdictional validation via MSEZ Bridge", "Audit log anomaly detection"],
      ["Double-spend attempts", "Nullifier system (Theorem 32.1)", "Nullifier set monitoring"],
      ["Compliance evasion", "Compliance Tensor mandatory evaluation", "Watcher attestation gaps"],
      ["Corridor state manipulation", "Vector clocks + Merkle proofs", "State divergence alerts"],
      ["Watcher collusion", "Slashing conditions SC4", "Pattern analysis across attestations"],
      ["Key compromise", "Key hierarchy with rotation", "Usage pattern anomaly detection"],
      ["Smart contract exploit", "Formal verification of SAVM programs", "Execution receipt audit"],
      ["Regulatory arbitrage", "Cross-jurisdiction compliance tensor", "Corridor compliance monitoring"],
      ["Data exfiltration", "Encryption at rest + in transit, key hierarchy", "Access pattern analysis"],
      ["DDoS / availability attack", "Rate limiting, CDN, geographic distribution", "Traffic anomaly detection"],
    ],
    [2400, 3600, 3360]
  ));
  c.push(spacer());
  c.push(p([bold("Defense in Depth."), " Security is layered across the entire stack: network layer (TLS 1.3, mutual authentication, certificate pinning), application layer (input validation, output encoding, parameterized queries), cryptographic layer (proven primitives, key management, proof verification), and operational layer (monitoring, alerting, incident response)."]));
  c.push(p([bold("Byzantine Fault Tolerance."), " The system tolerates up to f = (n-1)/3 Byzantine validators in any validator set. This provides safety (no conflicting states are finalized) and liveness (the system continues producing valid states) under the Byzantine fault model."]));

  c.push(heading3("46.2 Security Boundaries"));
  c.push(makeTable(
    ["Boundary", "Scope", "Guarantees", "Enforcement Mechanism"],
    [
      ["AssetOwnership", "Asset level", "Only owner can authorize transitions", "Ed25519 signatures + nullifier system"],
      ["AssetIntegrity", "Asset level", "Receipt chain cannot be modified", "Content addressing + hash chaining"],
      ["CorridorMembership", "Corridor level", "Only authorized parties participate", "Corridor definition VC + mutual auth"],
      ["CorridorState", "Corridor level", "State synchronized with integrity", "Vector clocks + Merkle proofs"],
      ["ConsensusIntegrity", "System level", "Byzantine fault tolerance", "DAG consensus + watcher quorum"],
      ["PrivacyGuarantees", "System level", "Transaction privacy maintained", "ZK proofs + encryption + key hierarchy"],
    ],
    [2000, 1600, 3000, 2760]
  ));
  c.push(spacer());

  c.push(heading3("46.3 Audit System"));
  c.push(p("The AuditEvent structure contains: event_id, event_type, actor, resource, action, success flag, error, timestamp, correlation_id, previous_event_hash, and event_hash. The chaining mechanism ensures historical integrity through cryptographic linking. Each audit event is content-addressed and appended to an audit chain with the same structural guarantees as receipt chains."));

  c.push(heading2("Chapter 47: Production Hardening"));
  c.push(heading3("47.1 Validation Framework"));
  c.push(p("Input validation enforces strict constraints at API boundaries. validate_asset_id checks for 64-character length and hexadecimal format. validate_amount checks for non-negative values and decimal precision limits (max 18 decimals). All validation functions return typed errors through Result<T, ValidationError>."));

  c.push(heading3("47.2 Thread Safety"));
  c.push(p("Concurrent access is managed through Rust ownership and borrowing. Shared state uses Arc<RwLock<T>> for read-heavy workloads and Arc<Mutex<T>> for write-heavy workloads. The Rust compiler enforces thread safety at compile time; data races are structurally impossible."));

  c.push(heading3("47.3 Cryptographic Utilities"));
  c.push(p("Constant-time comparison via the subtle crate prevents timing attacks. Cryptographic random number generation via OsRng (operating system entropy). Key material is zeroized on drop using the zeroize crate. No cryptographic keys are logged or serialized to persistent storage in plaintext."));

  c.push(heading3("47.4 Rust Security Guarantees"));
  c.push(makeTable(
    ["Guarantee", "Mechanism", "Python-era Risk Eliminated"],
    [
      ["Memory Safety", "Ownership model, borrow checker", "Buffer overflows, use-after-free, dangling pointers"],
      ["Thread Safety", "Send/Sync traits, no shared mutable state", "Data races, TOCTOU bugs"],
      ["Type Safety", "Algebraic types, exhaustive match", "Type confusion, null pointer dereference"],
      ["Error Handling", "Result<T, E>, no exceptions", "Unhandled exceptions, silent failures"],
      ["No GC Pauses", "Deterministic destruction", "Latency spikes during garbage collection"],
      ["No Unsafe", "Application code policy", "Undefined behavior from unsafe operations"],
    ],
    [1800, 3200, 4360]
  ));
  c.push(spacer());

  c.push(heading2("Chapter 48: Zero-Knowledge Proof Circuits"));
  c.push(heading3("48.1 Circuit Taxonomy"));
  c.push(makeTable(
    ["Circuit", "Constraints", "Purpose", "Proof System"],
    [
      ["\u03C0priv", "~34,000", "Privacy proof for shielded transfers", "Groth16"],
      ["\u03C0comp", "~25,000", "Compliance proof with tensor evaluation", "PLONK"],
      ["\u03C0asset", "~15,000", "Asset state transition proof", "Plonky3"],
      ["\u03C0exec", "Variable", "Block execution proof", "Plonky3"],
      ["\u03C0agg", "Variable", "Proof aggregation", "Halo2 recursive"],
      ["\u03C0ruling", "~35,000", "Arbitration ruling verification", "Groth16"],
      ["\u03C0sanctions", "~18,000", "Sanctions list non-membership (PPOI)", "PLONK"],
      ["\u03C0license", "~12,000", "License validity proof", "PLONK"],
      ["\u03C0migration", "~45,000", "Migration state transition", "Groth16"],
      ["\u03C0checkpoint", "~20,000", "Checkpoint validity", "Plonky3"],
      ["\u03C0credential", "~8,000", "Credential selective disclosure", "PLONK"],
      ["\u03C0bridge", "~40,000", "Cross-corridor bridge proof", "Groth16"],
    ],
    [1800, 1600, 3200, 2760]
  ));
  c.push(spacer());

  c.push(heading3("48.2 Privacy Circuit (\u03C0priv)"));
  c.push(p("The \u03C0priv circuit proves that a private transfer is valid without revealing sender, recipient, amount, or asset type. Constraint breakdown:"));
  c.push(makeTable(
    ["Sub-circuit", "Constraints", "Function"],
    [
      ["Input record opening", "~4,000", "Verify Pedersen commitment opening for input"],
      ["Output record commitment", "~4,000", "Compute Pedersen commitment for output"],
      ["Balance proof", "~2,000", "Verify sum(inputs) = sum(outputs) + fee"],
      ["Nullifier derivation", "~3,000", "Compute and verify nullifier from spending key"],
      ["Merkle membership", "~8,000", "Verify input record exists in global state tree"],
      ["Signature verification", "~6,000", "Verify Ed25519 spending authorization"],
      ["Range proofs", "~4,000", "Verify all amounts are non-negative"],
      ["Compliance hook", "~3,000", "Optional compliance predicate evaluation"],
    ],
    [2400, 1800, 5160]
  ));
  c.push(spacer());

  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART XVII: DEPLOYMENT AND OPERATIONS
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART XVII: DEPLOYMENT AND OPERATIONS"));

  c.push(heading2("Chapter 49: Deployment Architecture"));
  c.push(heading3("49.1 Infrastructure Requirements"));
  c.push(makeTable(
    ["Component", "Minimum", "Recommended"],
    [
      ["Compute", "4 vCPU, 16 GB RAM", "8 vCPU, 32 GB RAM"],
      ["Storage", "100 GB SSD", "500 GB NVMe SSD"],
      ["Network", "100 Mbps", "1 Gbps"],
      ["Database", "PostgreSQL 15+", "PostgreSQL 16 with pgvector"],
      ["Container Runtime", "Docker 24+", "containerd 1.7+ with Kubernetes 1.29+"],
    ],
    [2000, 3200, 4160]
  ));
  c.push(spacer());

  c.push(heading3("49.2 Deployment Profiles"));
  c.push(makeTable(
    ["Profile", "Services", "Resources", "Use Case"],
    [
      ["minimal", "Core MSEZ + single jurisdiction", "4 vCPU / 16 GB", "Development, testing"],
      ["standard", "Full MSEZ + 3 jurisdictions + corridors", "8 vCPU / 32 GB", "Single-zone production"],
      ["enterprise", "Full MSEZ + 10+ jurisdictions + full corridors", "32 vCPU / 128 GB", "Multi-zone production"],
      ["sovereign-govos", "Full MSEZ + GovOS + AI + national integration", "64+ vCPU / 256+ GB + GPU", "National deployment"],
    ],
    [1600, 3000, 2200, 2560]
  ));
  c.push(spacer());

  c.push(heading3("49.3 Rust Binary Deployment"));
  c.push(p("The msez CLI is a single statically-linked binary compiled from the msez-cli crate. No runtime dependencies beyond libc. Container images use Alpine Linux with the msez binary, producing images under 50 MB."));
  c.push(...codeBlock([
    "# Build the release binary",
    "cargo build --release --bin msez",
    "",
    "# Binary is at: target/release/msez",
    "# Deploy directly or via container:",
    "FROM alpine:3.19",
    "COPY target/release/msez /usr/local/bin/msez",
    "ENTRYPOINT [\"/usr/local/bin/msez\"]",
  ]));
  c.push(spacer());

  c.push(heading2("Chapter 50: Docker Infrastructure (v0.4.44)"));
  c.push(heading3("50.1 Service Architecture"));
  c.push(p("Docker Compose orchestrates twelve services with dependency ordering and health checks:"));
  c.push(makeTable(
    ["Service", "Image", "Port", "Function"],
    [
      ["msez-core", "msez:latest", "8080", "Core MSEZ API and compliance engine"],
      ["msez-pack", "msez:latest", "8081", "Pack Trilogy management"],
      ["msez-tensor", "msez:latest", "8082", "Compliance Tensor evaluation"],
      ["msez-corridor", "msez:latest", "8083", "Corridor management and bridge"],
      ["msez-watcher", "msez:latest", "8084", "Watcher economy and attestation"],
      ["msez-vm", "msez:latest", "8085", "SAVM execution environment"],
      ["msez-governance", "msez:latest", "8086", "Governance and voting"],
      ["msez-migration", "msez:latest", "8087", "Migration protocol"],
      ["postgres", "postgres:16-alpine", "5432", "Primary database"],
      ["redis", "redis:7-alpine", "6379", "Cache and pub/sub"],
      ["minio", "minio/minio:latest", "9000", "Object storage (artifacts)"],
      ["prometheus", "prom/prometheus", "9090", "Metrics collection"],
    ],
    [1800, 2200, 800, 4560]
  ));
  c.push(spacer());

  c.push(heading3("50.2 Container Definitions"));
  c.push(p("All MSEZ services share the same multi-stage Docker build. Stage 1 compiles from source using the Rust builder image. Stage 2 copies the binary into a minimal Alpine runtime. Health checks use the /healthz endpoint. Graceful shutdown via SIGTERM with configurable drain period."));

  c.push(heading3("50.3 Database Initialization"));
  c.push(p("PostgreSQL initialization scripts create schemas for each domain: msez_core (artifacts, digests), msez_pack (lawpacks, regpacks, licensepacks), msez_tensor (compliance tensor entries), msez_corridor (corridor state, bridge operations), msez_watcher (watcher records, bonds, slashing), and msez_migration (migration sagas, compensation logs). Migrations use a numbered sequence managed by the msez migrate subcommand."));

  c.push(heading2("Chapter 51: AWS Terraform Infrastructure (v0.4.44)"));
  c.push(heading3("51.1 Core Infrastructure"));
  c.push(p("AWS Terraform modules provision production-ready infrastructure. Core infrastructure (545 lines): VPC with public/private subnets across 3 AZs, NAT gateway, security groups, and flow logs. RDS PostgreSQL Multi-AZ with automated backups and encryption at rest. ElastiCache Redis cluster for session management and caching. S3 buckets for artifact storage with versioning and lifecycle policies."));

  c.push(heading3("51.2 Kubernetes Resources"));
  c.push(p("Kubernetes resources (705 lines): EKS cluster with managed node groups, Deployments for each MSEZ service, Services and Ingress configuration, Horizontal Pod Autoscalers, PodDisruptionBudgets for availability, ConfigMaps for jurisdiction configuration, and Secrets for cryptographic key material. The Terraform modules support multi-region deployment for GovOS configurations."));

  c.push(heading2("Chapter 52: One-Click Deployment (v0.4.44)"));
  c.push(heading3("52.1 Deployment Steps"));
  c.push(p("A single shell script transforms zone configuration into running infrastructure:"));
  c.push(...codeBlock([
    "# 1. Initialize a new jurisdiction deployment",
    "msez init --profile digital-financial-center --jurisdiction adgm",
    "",
    "# 2. Configure the Pack Trilogy",
    "msez pack import --lawpack ./jurisdictions/adgm/lawpack-v3.2.zip",
    "msez pack import --regpack ./jurisdictions/adgm/regpack-latest.zip",
    "msez pack import --licensepack ./jurisdictions/adgm/licensepack-latest.zip",
    "",
    "# 3. Deploy infrastructure",
    "msez deploy --target docker   # Local deployment",
    "msez deploy --target aws      # Production AWS deployment",
    "",
    "# 4. Verify deployment",
    "msez verify --all",
    "",
    "# 5. Activate corridors",
    "msez corridor activate --config ./corridors/adgm-pakistan.yaml",
  ]));
  c.push(spacer());

  c.push(heading3("52.2 AWS Deployment"));
  c.push(p("The AWS deployment automates the full Terraform apply cycle: VPC creation, RDS provisioning, EKS cluster setup, service deployment, DNS configuration, and SSL certificate provisioning. Total deployment time: approximately 25 minutes for standard profile, 45 minutes for enterprise profile."));

  c.push(heading2("Chapter 53: Operations Management"));
  c.push(heading3("53.1 Monitoring and Alerting"));
  c.push(p("Prometheus collects metrics from all MSEZ services. Key metrics: compliance_tensor_evaluations_total, corridor_state_sync_latency_seconds, migration_saga_duration_seconds, watcher_attestation_count, and pack_trilogy_staleness_seconds. Grafana dashboards provide real-time visibility. Alerting rules fire on: compliance tensor staleness exceeding threshold, watcher quorum below minimum, migration saga timeout, and corridor state divergence."));

  c.push(heading3("53.2 Incident Response"));
  c.push(p("Incident severity levels: P0 (service outage, compliance system failure), P1 (degraded performance, partial outage), P2 (non-critical feature failure), P3 (cosmetic issues, documentation). Runbooks cover: corridor state recovery, watcher bond redistribution, migration saga compensation, and Pack Trilogy emergency update."));

  c.push(heading3("53.3 Change Management"));
  c.push(p("All changes follow a staged rollout: development, staging, canary (5% traffic), and production. Pack Trilogy updates propagate through the same pipeline. Lawpack changes require additional governance approval before deployment. Rollback procedures are tested quarterly."));

  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART XVIII: NETWORK DIFFUSION
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART XVIII: NETWORK DIFFUSION"));

  c.push(heading2("Chapter 54: Adoption Strategy"));
  c.push(heading3("54.1 Target Segments"));
  c.push(makeTable(
    ["Segment", "Entry Point", "Value Proposition"],
    [
      ["Sovereign Governments", "National GovOS deployment", "Tax revenue optimization, compliance automation, digital transformation"],
      ["Free Zone Authorities", "Digital free zone stack", "Rapid zone deployment, automated licensing, corridor connectivity"],
      ["Financial Centers", "Capital markets + corridors", "Cross-border settlement, compliance verification, institutional infrastructure"],
      ["Development Finance", "SEZ-in-a-box for emerging markets", "Rapid economic zone creation, investment facilitation"],
      ["Corporate Service Providers", "Formation + compliance modules", "Automated corporate services, multi-jurisdiction operations"],
    ],
    [2400, 2800, 4160]
  ));
  c.push(spacer());

  c.push(heading3("54.2 Network Bootstrapping"));
  c.push(p("Network effects emerge when each jurisdiction makes every other jurisdiction more valuable. Bootstrapping strategy: anchor jurisdictions (UAE/ADGM, Pakistan, Kazakhstan) provide initial network mass, corridor activation creates bilateral value, GovOS deployments add full-stack integrations, and open-source availability enables independent adoption. Each new jurisdiction increases the value of all existing jurisdictions through potential corridor connectivity."));

  c.push(heading2("Chapter 55: Partner Network"));
  c.push(heading3("55.1 Implementation Partners"));
  c.push(p("Implementation partners deploy and operate MSEZ Stack instances. Partner categories: system integrators (large-scale deployment), legal technology firms (Pack Trilogy curation), compliance specialists (watcher economy participation), and cloud infrastructure providers (hosting and managed services)."));

  c.push(heading3("55.2 Technology Partners"));
  c.push(p("Technology partners provide infrastructure components: blockchain networks (anchor targets), identity providers (KYC verification), banking networks (fiat on/off-ramps), and regulatory technology firms (sanctions screening, transaction monitoring)."));

  c.push(heading2("Chapter 56: Current Network"));
  c.push(makeTable(
    ["Jurisdiction", "Status", "Profile", "Corridors"],
    [
      ["UAE / ADGM", "Live", "digital-financial-center", "PAK-UAE, KSA-UAE"],
      ["Dubai FZC (27 zones)", "Integration", "digital-financial-center", "PAK-UAE"],
      ["Pakistan", "Active", "sovereign-govos", "PAK-KSA, PAK-UAE, PAK-CHN"],
      ["Kazakhstan (Alatau City)", "Partnership", "digital-financial-center", "Planned"],
      ["Seychelles", "Deployment", "sovereign-govos", "Planned"],
    ],
    [2400, 1400, 3000, 2560]
  ));
  c.push(spacer());
  c.push(p("Aggregate metrics: 1,000+ entities onboarded, $1.7B+ capital processed, 5 jurisdictions active or deploying, 3 bilateral corridors ($38.6B combined volume), 16 module families (298 modules), 650 tests at 100% coverage."));

  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // APPENDICES
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("APPENDICES"));

  // Appendix A
  c.push(heading2("Appendix A: Version History"));
  c.push(makeTable(
    ["Version", "Date", "Changes"],
    [
      ["0.4.44", "Feb 2026", "GENESIS: Licensepacks, Composition Engine, Corporate/Identity/Tax/Markets/Trade modules, One-click deployment, Rust migration, Mass/MSEZ separation, GovOS architecture, live corridors"],
      ["0.4.43", "Jan 2026", "Phoenix Ascendant: Compliance Tensor V2, Manifold, SAVM, Watcher Economy, Migration, Bridge"],
      ["0.4.42", "Jan 2026", "Agentic Ascension: Agentic Framework, ZK L1, enhanced arbitration"],
      ["0.4.41", "Dec 2025", "Arbitration System: Institution registry, dispute filing, ruling enforcement"],
      ["0.4.40", "Nov 2025", "RegPack Integration: Dynamic regulatory state, sanctions screening"],
      ["0.4.38", "Oct 2025", "Initial comprehensive specification, core modules"],
      ["0.4.0", "Jul 2025", "Architecture redesign, Smart Asset model"],
      ["0.3.0", "Mar 2025", "Compliance tensor, lawpack system"],
      ["0.2.0", "Dec 2024", "Receipt chain architecture, MMR checkpoints"],
      ["0.1.0", "Sep 2024", "Initial specification draft"],
    ],
    [1200, 1400, 6760]
  ));
  c.push(spacer());

  // Appendix B
  c.push(heading2("Appendix B: Test Coverage Summary"));
  c.push(makeTable(
    ["Test Category", "Count", "Coverage"],
    [
      ["MASS Protocol Primitives", "62", "100%"],
      ["RegPack/Arbitration", "36", "100%"],
      ["Agentic Framework", "18", "100%"],
      ["Smart Asset Lifecycle", "45", "100%"],
      ["Corridor Operations", "32", "100%"],
      ["Receipt Chain", "28", "100%"],
      ["Compliance Tensor V2", "22", "100%"],
      ["Compliance Manifold", "18", "100%"],
      ["Migration Protocol", "24", "100%"],
      ["Watcher Economy", "20", "100%"],
      ["Smart Asset VM", "28", "100%"],
      ["Corridor Bridge", "16", "100%"],
      ["L1 Anchoring", "14", "100%"],
      ["Composition Engine", "45", "100%"],
      ["Licensepacks", "55", "100%"],
      ["Corporate Modules", "65", "100%"],
      ["Identity Modules", "40", "100%"],
      ["Integration Tests", "82", "100%"],
      ["Total", "650", "100%"],
    ],
    [4000, 1200, 4160]
  ));
  c.push(spacer());

  // Appendix C
  c.push(heading2("Appendix C: Scalability Switch Reference"));
  c.push(makeTable(
    ["Switch", "Default", "Range", "Effect"],
    [
      ["S1: Harbor shards", "8", "1-256", "Horizontal capacity"],
      ["S2: Corridor shards", "4", "1-64", "Cross-jurisdiction capacity"],
      ["S3: Block size", "1MB", "256KB-16MB", "Throughput vs latency"],
      ["S4: Block interval", "500ms", "100ms-5s", "Throughput vs latency"],
      ["S5: Proof batch size", "1000", "100-10000", "Amortization"],
      ["S6: Checkpoint interval", "1000", "100-10000", "Verification efficiency"],
      ["S7: Watcher quorum", "3-of-5", "1-of-1 to 7-of-9", "Security vs availability"],
      ["S8: Staleness bound", "24h", "1h-7d", "Freshness vs flexibility"],
      ["S9: Max asset value", "$10M", "$1K-$1B", "Risk management"],
      ["S10: Migration duration", "24h", "1h-7d", "Operation bounds"],
      ["S11: Bridge hop limit", "5", "1-10", "Path complexity"],
      ["S12: Fee multiplier", "1.0", "0.1-10.0", "Economic tuning"],
      ["S13: DA enforcement", "Best-effort", "Off/Best-effort/Enforced", "Availability guarantees"],
    ],
    [2400, 1400, 2800, 2760]
  ));
  c.push(spacer());

  // Appendix D
  c.push(heading2("Appendix D: Security Proofs Summary"));
  c.push(makeTable(
    ["Theorem", "Statement"],
    [
      ["9.1 (Object Survivability)", "Receipt chains maintain integrity during offline operation"],
      ["10.1 (Compliance Soundness)", "Compliance proofs demonstrate predicate satisfaction; false claims are computationally infeasible"],
      ["28.1 (Watcher Accountability)", "Dishonest attestations result in provable collateral loss"],
      ["29.1 (Identity Immutability)", "Smart Asset identity is established at genesis and cannot be modified"],
      ["29.2 (Non-Repudiation)", "Authorized state transitions cannot be repudiated"],
      ["30.1 (Migration Atomicity)", "Migration completes fully or compensation returns asset to original state"],
      ["31.1 (Unlinkability)", "Private transactions are unlinkable without viewing keys"],
      ["32.1 (Double-Spend Resistance)", "Each record can be spent exactly once via nullifier mechanism"],
    ],
    [2800, 6560]
  ));
  c.push(spacer());

  // Appendix E
  c.push(heading2("Appendix E: Rust Crate Dependency Graph"));
  c.push(...codeBlock([
    "msez-cli",
    "  \u251C\u2500\u2500 msez-govos",
    "  \u2502   \u251C\u2500\u2500 msez-mass-bridge",
    "  \u2502   \u2502   \u251C\u2500\u2500 msez-pack",
    "  \u2502   \u2502   \u251C\u2500\u2500 msez-tensor",
    "  \u2502   \u2502   \u2514\u2500\u2500 msez-core",
    "  \u2502   \u251C\u2500\u2500 msez-modules",
    "  \u2502   \u2514\u2500\u2500 msez-corridor",
    "  \u251C\u2500\u2500 msez-vm",
    "  \u2502   \u251C\u2500\u2500 msez-tensor",
    "  \u2502   \u2514\u2500\u2500 msez-core",
    "  \u251C\u2500\u2500 msez-migration",
    "  \u2502   \u251C\u2500\u2500 msez-corridor",
    "  \u2502   \u251C\u2500\u2500 msez-watcher",
    "  \u2502   \u2514\u2500\u2500 msez-tensor",
    "  \u251C\u2500\u2500 msez-watcher",
    "  \u2502   \u2514\u2500\u2500 msez-core",
    "  \u2514\u2500\u2500 msez-governance",
    "      \u2514\u2500\u2500 msez-core",
    "",
    "Shared dependencies: serde, tokio, chrono, ed25519-dalek, arkworks, halo2",
  ]));
  c.push(spacer());

  // Appendix F
  c.push(heading2("Appendix F: Mass API Endpoint Reference"));
  c.push(makeTable(
    ["API", "Base URL", "Swagger"],
    [
      ["Organization Info", "organization-info.api.mass.inc", "https://organization-info.api.mass.inc/organization-info/swagger-ui/index.html"],
      ["Investment Info", "investment-info-production.herokuapp.com", "https://investment-info-production-4f3779c81425.herokuapp.com/investment-info/swagger-ui/index.html"],
      ["Treasury Info", "treasury-info.api.mass.inc", "https://treasury-info.api.mass.inc/treasury-info/swagger-ui/index.html"],
      ["Consent Info", "consent.api.mass.inc", "https://consent.api.mass.inc/consent-info/swagger-ui/index.html"],
      ["Templating Engine", "templating-engine-prod.herokuapp.com", "https://templating-engine-prod-5edc768c1f80.herokuapp.com/templating-engine/swagger-ui/index.html"],
    ],
    [2000, 3400, 3960]
  ));
  c.push(spacer());

  // Appendix G
  c.push(heading2("Appendix G: Jurisdiction Template Reference"));
  c.push(makeTable(
    ["Jurisdiction", "Lawpack", "Regpack", "Licensepack", "Profile"],
    [
      ["Pakistan", "ITO 2001, STA 1990, FEA, Customs Act, Companies Act", "FBR calendars, SROs, FATF AML", "SECP, BOI, PTA, PEMRA, DRAP, Provincial", "sovereign-govos"],
      ["ADGM", "ADGM Companies Regulations, Financial Services Regulations", "FSRA rulebook, FATF", "Financial services, corporate", "digital-financial-center"],
      ["Seychelles", "International Business Companies Act, Financial Services Act", "SFSA guidelines", "IBC, CSL, banking", "sovereign-govos"],
      ["Kazakhstan (Alatau)", "Kazakh civil code + AIFC overlay", "AFSA rules, NB KZ", "AIFC categories + Kazakh", "digital-financial-center"],
    ],
    [1600, 2400, 2000, 1800, 1560]
  ));
  c.push(spacer());

  // Appendix H
  c.push(heading2("Appendix H: CLI Reference"));
  c.push(p("The msez binary provides a clap-derived CLI:"));
  c.push(makeTable(
    ["Command", "Subcommand", "Description"],
    [
      ["msez init", "--profile <name> --jurisdiction <id>", "Initialize a new jurisdiction deployment"],
      ["msez pack", "import / verify / list / diff", "Pack Trilogy management"],
      ["msez deploy", "--target docker|aws|k8s", "Deploy infrastructure"],
      ["msez verify", "--all | --service <name>", "Verify deployment health"],
      ["msez corridor", "activate / status / sync", "Corridor management"],
      ["msez migrate", "up / down / status", "Database migrations"],
      ["msez artifact", "graph verify / bundle attest", "Artifact graph operations"],
      ["msez tensor", "evaluate / slice / commit", "Compliance tensor operations"],
      ["msez watcher", "register / bond / attest", "Watcher economy operations"],
      ["msez govos", "deploy / status / handover", "GovOS lifecycle management"],
    ],
    [1800, 3200, 4360]
  ));
  c.push(spacer());

  // Appendix I
  c.push(heading2("Appendix I: Module Directory Structure"));
  c.push(...codeBlock([
    "crates/msez-modules/src/",
    "\u251C\u2500\u2500 compliance/          # Tensor, manifold, ZK circuits",
    "\u251C\u2500\u2500 corridors/           # State sync, bridge, multilateral",
    "\u251C\u2500\u2500 governance/          # Constitutional, voting, delegation",
    "\u251C\u2500\u2500 financial/           # Accounts, payments, custody, FX",
    "\u251C\u2500\u2500 regulatory/          # KYC, AML, sanctions, reporting",
    "\u251C\u2500\u2500 licensing/           # Applications, monitoring, portability",
    "\u251C\u2500\u2500 legal/               # Contracts, disputes, arbitration",
    "\u251C\u2500\u2500 operational/         # HR, procurement, facilities",
    "\u251C\u2500\u2500 corporate/           # Formation, cap table, dissolution (v0.4.44)",
    "\u251C\u2500\u2500 identity/            # DID, KYC tiers, credentials (v0.4.44)",
    "\u251C\u2500\u2500 tax/                 # Regimes, fees, incentives (v0.4.44)",
    "\u251C\u2500\u2500 capital_markets/     # Securities, trading, CSD (v0.4.44)",
    "\u2514\u2500\u2500 trade/               # LCs, documents, SCF (v0.4.44)",
  ]));
  c.push(spacer());

  // Appendix J
  c.push(heading2("Appendix J: Conformance Levels"));
  c.push(makeTable(
    ["Level", "Category", "Requirements"],
    [
      ["1", "Schema Conformance", "JSON Schema validation, Akoma Ntoso, W3C VC data model"],
      ["2", "Behavioral Conformance", "Module dependency resolution, deterministic outputs"],
      ["3", "Cryptographic Conformance", "Signature verification, ZK soundness, correct hashes"],
      ["4", "Corridor Integrity", "Definition VC binding, agreement binding, fork detection"],
      ["5", "Migration Integrity", "State machine transitions, compensation execution"],
    ],
    [800, 2600, 5960]
  ));
  c.push(spacer());

  // Appendix K
  c.push(heading2("Appendix K: GovOS Deployment Checklist"));
  c.push(makeTable(
    ["Phase", "Milestone", "Verification"],
    [
      ["1.1", "Core infrastructure deployed (compute, storage, network)", "msez verify --all passes"],
      ["1.2", "National identity system connected (NADRA for Pakistan)", "Identity API handshake confirmed"],
      ["1.3", "Tax authority connected (FBR IRIS for Pakistan)", "Test tax event round-trip"],
      ["1.4", "Central bank connected (SBP Raast for Pakistan)", "Test payment round-trip"],
      ["1.5", "Corporate registry connected (SECP for Pakistan)", "Test entity formation round-trip"],
      ["2.1", "Pack Trilogy imported for national law", "msez pack verify --all passes"],
      ["2.2", "All ministry accounts provisioned (40+ for Pakistan)", "Ministry dashboard access confirmed"],
      ["2.3", "First corridor activated", "Cross-border test transaction succeeds"],
      ["3.1", "AI models trained on national data", "Revenue projection accuracy > 85%"],
      ["3.2", "Tax gap reduction measured", "Baseline vs. 6-month comparison report"],
      ["4.1", "Pakistani engineering team certified", "All runbook procedures demonstrated"],
      ["4.2", "Full operational handover", "Momentum advisory-only SLA signed"],
    ],
    [800, 4400, 4160]
  ));
  c.push(spacer());

  // ── COLOPHON ─────────────────────────────────────────────
  c.push(pageBreak());
  c.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { before: 2000, after: 200 }, children: [
    new TextRun({ text: "End of Specification", font: BODY_FONT, size: 28, bold: true, color: DARK }),
  ]}));
  c.push(spacer(200));
  c.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
    new TextRun({ text: "Momentum Open Source SEZ Stack", font: BODY_FONT, size: 22 }),
  ]}));
  c.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
    new TextRun({ text: "Version 0.4.44 \u2014 GENESIS", font: BODY_FONT, size: 22, italics: true }),
  ]}));
  c.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
    new TextRun({ text: "February 2026", font: BODY_FONT, size: 22 }),
  ]}));
  c.push(spacer(200));
  c.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
    new TextRun({ text: "For questions or feedback, contact:", font: BODY_FONT, size: 20 }),
  ]}));
  c.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
    new TextRun({ text: "Momentum", font: BODY_FONT, size: 20, bold: true }),
  ]}));
  c.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
    new TextRun({ text: "https://github.com/momentum-sez/stack", font: BODY_FONT, size: 20, color: ACCENT }),
  ]}));
  c.push(new Paragraph({ alignment: AlignmentType.CENTER, children: [
    new TextRun({ text: "research@momentum.inc", font: BODY_FONT, size: 20, color: ACCENT }),
  ]}));

  return c;
}

module.exports = { buildRemainingParts };
