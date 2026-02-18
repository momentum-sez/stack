const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  definition, codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter18() {
  return [
    chapterHeading("Chapter 18: Civic Services Integration"),

    // --- 18.1 Identity Services ---
    h2("18.1 Identity Services"),
    p("Zone identity services provide residents and businesses with verifiable credentials: resident credentials (zone residency status, rights, obligations), business credentials (entity registration, good standing, authorized activities), professional credentials (qualifications, licensing for regulated professions). All credentials support selective disclosure via BBS+."),

    h3("18.1.1 Credential Types"),
    p("Each credential type maps to a specific lifecycle managed by the zone authority. Credentials are issued as W3C Verifiable Credentials with Ed25519 proofs and optional BBS+ selective disclosure signatures for privacy-preserving verification."),
    table(
      ["Credential Type", "Issuer", "Validity Period", "Selective Disclosure Fields"],
      [
        ["Resident Credential", "Zone Authority", "1 year, renewable", "Name, residency status, rights tier, obligations"],
        ["Business Credential", "Zone Registrar", "Matches entity lifecycle", "Entity name, registration number, good standing, authorized activities"],
        ["Professional Credential", "Licensing Body", "Per profession regulation", "Holder name, qualification, license number, scope of practice"],
        ["Tax Residency Certificate", "Zone Fiscal Authority", "Fiscal year", "Entity/individual ID, tax jurisdiction, residency start date"],
        ["Good Standing Attestation", "Zone Compliance Office", "90 days", "Entity ID, compliance status, last audit date"],
      ],
      [1800, 1600, 1600, 4360]
    ),

    h3("18.1.2 Credential Lifecycle"),
    p("Credentials follow a four-phase lifecycle: Issuance (identity verification, KYC/KYB completion, credential minting), Active (credential in use, subject to periodic re-verification), Suspended (temporarily halted pending review or investigation), and Revoked (permanently invalidated with revocation reason recorded). Re-verification triggers include: expiration of validity period, material change in circumstances (e.g., change of beneficial ownership), regulatory directive, or adverse finding in compliance monitoring."),

    // --- 18.2 Property Services ---
    h2("18.2 Property Services"),
    p("Property rights are represented as Smart Assets with zone-specific lawpack bindings. Title registry maintains the authoritative record of property ownership using append-only receipt chains. Transfer services facilitate property transactions with compliance verification. Encumbrance management tracks liens, mortgages, and other property interests."),

    h3("18.2.1 Title Registry"),
    p("The title registry is the authoritative record of property ownership within the zone. Each property title is represented as a Smart Asset whose receipt chain captures the complete ownership history. The genesis record establishes the initial title grant from the zone authority. Subsequent transfers append receipts that record the grantor, grantee, consideration, and compliance attestations. The registry supports fractional ownership through share-class mechanics inherited from the ownership primitive."),
    definition("Definition 18.1 (Property Title).", "A PropertyTitle is a Smart Asset whose manifest declares property-specific metadata (parcel identifier, boundaries, permitted use, zoning classification) and whose state machine enforces transfer preconditions including compliance verification, encumbrance clearance, and consent workflows."),
    ...codeBlock(
`/// A property title represented as a Smart Asset with zone-specific bindings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyTitle {
    /// Unique identifier for this title, derived from genesis record digest.
    pub title_id: TitleId,
    /// The Smart Asset backing this property title.
    pub asset: SmartAsset,
    /// Zone-specific parcel identifier (e.g., DIFC plot number, AIFC land ID).
    pub parcel_id: ParcelId,
    /// Legal description of property boundaries.
    pub boundaries: BoundaryDescription,
    /// Current zoning classification from the zone lawpack.
    pub zoning: ZoningClassification,
    /// Permitted use categories (commercial, residential, mixed, industrial).
    pub permitted_use: Vec<UseCategory>,
    /// Current owner entity, resolved through the ownership primitive.
    pub owner: EntityId,
    /// Active encumbrances (liens, mortgages, easements, restrictive covenants).
    pub encumbrances: Vec<Encumbrance>,
    /// Valuation history for tax assessment purposes.
    pub valuations: Vec<Valuation>,
    /// Receipt chain root for the full ownership history.
    pub chain_root: CanonicalDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encumbrance {
    pub encumbrance_id: EncumbranceId,
    pub kind: EncumbranceKind,
    pub beneficiary: EntityId,
    pub registered_date: chrono::DateTime<chrono::Utc>,
    pub expiry: Option<chrono::DateTime<chrono::Utc>>,
    pub amount: Option<Amount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncumbranceKind {
    Mortgage,
    Lien,
    Easement,
    RestrictiveCovenant,
    RegulatoryHold,
}`
    ),

    h3("18.2.2 Transfer Process"),
    p("Property transfers follow a multi-step process that composes compliance verification with ownership mutation. The transfer pipeline evaluates the compliance tensor for the destination jurisdiction, checks the lawpack for transfer restrictions (foreign ownership limits, zoning compatibility), verifies encumbrance clearance, obtains consent from required parties (existing lien holders, co-owners), and finally executes the ownership transfer through the Mass ownership primitive. A transfer receipt is appended to the property title's receipt chain, and a Transfer VC is issued to both parties."),
    table(
      ["Step", "System", "Action", "Failure Handling"],
      [
        ["1", "mez-tensor", "Evaluate compliance for buyer + jurisdiction", "Block transfer, issue compliance gap report"],
        ["2", "mez-pack (lawpack)", "Check transfer restrictions (foreign ownership, zoning)", "Block transfer, cite specific legal provision"],
        ["3", "mez-pack (regpack)", "Verify no sanctions hits on buyer or seller", "Block transfer, escalate to compliance officer"],
        ["4", "Title Registry", "Confirm all encumbrances cleared or subordinated", "Block transfer, list outstanding encumbrances"],
        ["5", "mez-mass-client (consent)", "Obtain required consents (lien holders, co-owners)", "Block transfer until consents obtained"],
        ["6", "mez-mass-client (ownership)", "Execute ownership mutation in Mass", "Rollback, no receipt appended"],
        ["7", "Receipt Chain", "Append transfer receipt to title's chain", "Retry with idempotency key"],
        ["8", "mez-vc", "Issue Transfer VC to buyer and seller", "Queue for async issuance"],
      ],
      [600, 2200, 3760, 2800]
    ),

    // --- 18.3 Dispute Resolution Services ---
    h2("18.3 Dispute Resolution Services"),
    p("Small claims procedures handle low-value disputes through expedited processes. Commercial arbitration handles business disputes through international arbitration institutions (DIFC-LCIA, SIAC, AIFC-IAC, ICC). Appellate procedures enable review of initial determinations."),

    h3("18.3.1 Supported Institutions"),
    p("The dispute resolution system integrates with recognized international arbitration institutions. Each institution's procedural rules are encoded as machine-readable specifications within the zone's lawpack, enabling automated procedural compliance and deadline tracking."),
    table(
      ["Institution", "Abbreviation", "Jurisdiction", "Specialization"],
      [
        ["DIFC-LCIA Arbitration Centre", "DIFC-LCIA", "Dubai (DIFC)", "Commercial, financial, construction disputes"],
        ["Singapore International Arbitration Centre", "SIAC", "Singapore", "Cross-border commercial, investment treaty disputes"],
        ["AIFC International Arbitration Centre", "AIFC-IAC", "Astana (AIFC)", "Commercial, investment, financial services disputes"],
        ["ICC International Court of Arbitration", "ICC", "Paris (global)", "Complex commercial, multi-party, multi-contract disputes"],
        ["ADGM Arbitration Centre", "ADGM", "Abu Dhabi (ADGM)", "Commercial, financial, technology disputes"],
        ["London Court of International Arbitration", "LCIA", "London", "International commercial, investment disputes"],
      ],
      [2800, 1200, 2000, 3360]
    ),

    h3("18.3.2 Dispute Case Lifecycle"),
    p("Every dispute case progresses through a structured lifecycle. Filing initiates the case with an evidence package and claim summary. Tribunal formation selects arbitrators per the institution's rules. Proceedings follow the encoded procedural timeline. A ruling is issued as a Verifiable Credential signed by the tribunal. Enforcement is executed automatically through the EZ Stack's corridor state, fiscal primitive, and compliance tensor updates."),
    definition("Definition 18.2 (Dispute Case).", "A DisputeCase encapsulates the full lifecycle of a dispute from filing through enforcement, including party identities, evidence packages, procedural history, tribunal composition, ruling, and enforcement status."),
    ...codeBlock(
`/// A dispute case tracking the full lifecycle from filing to enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeCase {
    /// Unique dispute identifier.
    pub dispute_id: DisputeId,
    /// The institution handling this dispute.
    pub institution: InstitutionId,
    /// Current phase of the dispute lifecycle.
    pub phase: DisputePhase,
    /// The filing party (claimant).
    pub claimant: EntityId,
    /// The responding party (respondent).
    pub respondent: EntityId,
    /// Claim category and monetary value (if applicable).
    pub claim: ClaimSummary,
    /// Evidence packages submitted by both parties.
    pub evidence: Vec<EvidencePackage>,
    /// Tribunal composition once formed.
    pub tribunal: Option<TribunalComposition>,
    /// Procedural timeline with deadlines.
    pub timeline: Vec<ProceduralEvent>,
    /// The ruling, once issued.
    pub ruling: Option<ArbitrationRuling>,
    /// Enforcement status for each enforcement action.
    pub enforcement_status: Vec<EnforcementStatus>,
    /// Filing date.
    pub filed_at: chrono::DateTime<chrono::Utc>,
    /// Last updated timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisputePhase {
    /// Case filed, awaiting respondent acknowledgment.
    Filed,
    /// Respondent acknowledged, tribunal formation in progress.
    TribunalFormation,
    /// Tribunal seated, proceedings underway.
    Proceedings,
    /// Deliberation phase, no new submissions accepted.
    Deliberation,
    /// Ruling issued, enforcement pending.
    RulingIssued,
    /// All enforcement actions executed.
    Enforced,
    /// Case closed (settled, withdrawn, or fully enforced).
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimSummary {
    pub category: ClaimCategory,
    pub description: String,
    pub monetary_value: Option<Amount>,
    pub currency: Option<CurrencyCode>,
    pub relief_sought: Vec<String>,
}`
    ),

    h3("18.3.3 Small Claims Procedures"),
    p("Small claims disputes (below the zone-configured monetary threshold) follow an expedited single-arbitrator process. The filing party submits a claim with supporting evidence. The respondent has a fixed response window (typically 14 days). A sole arbitrator is appointed from the institution's expedited panel. Document-only proceedings are the default unless either party requests a hearing. Rulings are issued within 30 days of tribunal formation. The expedited process reduces costs and time while maintaining the same enforcement mechanisms as full commercial arbitration."),

    h3("18.3.4 Appellate Procedures"),
    p("Appellate review is available for rulings that meet grounds for appeal as defined in the institution's rules: procedural irregularity, tribunal exceeded its jurisdiction, or ruling conflicts with zone public policy. Appeals are filed within the institution-specified deadline (typically 30 days from ruling). An appellate tribunal of three arbitrators reviews the original record. The appellate tribunal may affirm, modify, or remand the original ruling. During appeal, enforcement of the original ruling may be stayed upon posting of security."),
  ];
};
