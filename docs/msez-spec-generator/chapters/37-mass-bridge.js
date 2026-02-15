const {
  partHeading, chapterHeading, h2,
  p, p_runs, bold,
  codeBlock, table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter37() {
  return [
    ...partHeading("PART XIII: MASS API INTEGRATION LAYER"),

    chapterHeading("Chapter 37: The msez-mass-bridge Crate"),

    p("The msez-mass-bridge crate formalizes the interface between Mass APIs (System A) and the MSEZ Stack (System B). This is the architectural boundary that enforces the two-system separation. Mass APIs call into the bridge for jurisdictional context. The MSEZ Stack provides context through trait implementations."),

    // --- 37.1 JurisdictionalContext Trait ---
    h2("37.1 JurisdictionalContext Trait"),

    ...codeBlock(
`/// Trait that Mass APIs call to get jurisdictional context.
/// Implemented by the MSEZ Stack for each deployed jurisdiction.
pub trait JurisdictionalContext: Send + Sync {
    /// Returns permitted entity types for this jurisdiction.
    fn permitted_entity_types(&self) -> Vec<EntityType>;

    /// Validates a formation application against jurisdictional rules.
    fn validate_formation(
        &self,
        app: &FormationApplication,
    ) -> Result<(), ComplianceViolation>;

    /// Returns the fee schedule for a given operation.
    fn fee_schedule(&self, operation: Operation) -> FeeSchedule;

    /// Evaluates compliance tensor for a proposed action.
    fn evaluate_compliance(
        &self,
        asset: &AssetId,
        jurisdiction: &JurisdictionId,
        domains: &[ComplianceDomain],
    ) -> ComplianceTensorSlice;

    /// Returns current Pack Trilogy state for this jurisdiction.
    fn pack_state(&self) -> PackTrilogyState;

    /// Returns securities issuance rules for this jurisdiction.
    fn securities_rules(&self, security_type: SecurityType) -> SecuritiesRules;

    /// Returns KYC tier requirements for this jurisdiction.
    fn kyc_requirements(&self, tier: KycTier) -> KycRequirements;

    /// Returns governance rules (quorum, voting, delegation).
    fn governance_rules(&self) -> GovernanceRules;

    /// Returns tax rules applicable to a transaction.
    fn tax_rules(
        &self,
        transaction_type: TransactionType,
        parties: &TransactionParties,
    ) -> TaxRules;
}`
    ),

    spacer(),

    // --- 37.2 Mass Primitive Mapping ---
    h2("37.2 Mass Primitive Mapping"),
    p("Each of the five Mass primitives calls specific bridge methods:"),

    table(
      ["Mass Primitive", "API Endpoint", "Bridge Methods Called"],
      [
        ["Entities", "organization-info.api.mass.inc", "permitted_entity_types(), validate_formation(), fee_schedule(), pack_state()"],
        ["Ownership", "investment-info (Heroku seed)", "securities_rules(), evaluate_compliance()"],
        ["Fiscal", "treasury-info.api.mass.inc", "tax_rules(), fee_schedule(), evaluate_compliance()"],
        ["Identity", "Distributed across org + consent", "kyc_requirements(), evaluate_compliance()"],
        ["Consent", "consent.api.mass.inc", "governance_rules(), evaluate_compliance()"],
      ],
      [1600, 3000, 4760]
    ),

    spacer(),

    // --- 37.3 The Organs ---
    h2("37.3 The Organs"),
    p("The Organs are regulated interface implementations that make Mass deployable in licensed environments:"),

    table(
      ["Organ", "Function", "Regulatory Status"],
      [
        ["Center of Mass", "Banking infrastructure: accounts, payments, custody, FX, on/off-ramps", "FinCEN MSB, state MTLs, UAE Central Bank API, Northern Trust custody"],
        ["Torque", "Licensing infrastructure: license applications, compliance monitoring, renewals", "ADGM FSP, Dubai DFZC integration"],
        ["Inertia", "Corporate services: entity formation, secretarial, registered agent", "CSP licenses, SECP authorized agent"],
      ],
      [1800, 4200, 3360]
    ),

    spacer(),

    p("Each Organ implements a subset of Mass API functionality within a specific regulatory regime. The Organ does not change Mass API behavior; it adds the regulatory licenses and operational compliance required for lawful operation. The MSEZ Stack provides the jurisdictional context that each Organ requires through the JurisdictionalContext trait."),
  ];
};
