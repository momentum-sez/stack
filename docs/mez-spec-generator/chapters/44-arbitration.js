const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter44() {
  return [
    chapterHeading("Chapter 44: Arbitration System"),

    // --- 44.1 Institution Registry ---
    h2("44.1 Institution Registry"),
    p("The arbitration system maintains a registry of recognized institutions. Recognized institutions: DIFC-LCIA Arbitration Centre, Singapore International Arbitration Centre (SIAC), AIFC International Arbitration Centre (IAC), International Chamber of Commerce (ICC) International Court of Arbitration, ADGM Arbitration Centre. Each institution has associated rules encoded as machine-readable specifications: filing procedures, tribunal formation rules, procedural timelines, fee schedules, and enforcement mechanisms."),

    // --- 44.2 Ruling Enforcement ---
    h2("44.2 Ruling Enforcement"),
    p("Arbitration rulings are issued as Verifiable Credentials signed by the tribunal. The ruling VC contains the dispute identifier, tribunal composition, decision summary, enforcement actions, and appeals deadline. Enforcement actions are executed automatically by the EZ Stack: asset freezes are applied via corridor state updates, payment obligations are routed through the fiscal primitive, and compliance tensor entries are updated to reflect the ruling outcome."),
    ...codeBlock(
`/// An arbitration ruling issued as a Verifiable Credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrationRuling {
    pub dispute_id: DisputeId,
    pub tribunal: TribunalComposition,
    pub institution: InstitutionId,
    pub ruling_date: chrono::DateTime<chrono::Utc>,
    pub decision: DecisionSummary,
    pub enforcement_actions: Vec<EnforcementAction>,
    pub appeals_deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub ruling_vc: Option<VerifiableCredential>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnforcementAction {
    AssetFreeze { asset_id: AssetId, duration: Duration },
    PaymentObligation { from: EntityId, to: EntityId, amount: Amount },
    ComplianceUpdate { domain: ComplianceDomain, new_state: ComplianceState },
    LicenseRevocation { license_id: LicenseId },
    CorridorSuspension { corridor_id: CorridorId },
}`
    ),

    // --- 44.2.1 Dispute Filing Process ---
    h3("44.2.1 Dispute Filing Process"),
    p("Dispute filing follows a deterministic six-step process that ensures jurisdictional correctness, evidence integrity, and institutional routing. Each step produces auditable state transitions recorded in the corridor state machine."),
    table(
      ["Step", "Action", "System Behavior"],
      [
        ["1. Initiation", "Claimant submits DisputeRequest via API", "Validate corridor membership, verify claimant is a party to the corridor, assign DisputeId, set state to Filed"],
        ["2. Jurisdiction Resolution", "System determines governing law", "Evaluate corridor's bilateral agreement, resolve arbitration clause, select institution from registry based on seat and governing law"],
        ["3. Counterparty Notification", "Respondent receives dispute notice", "Issue notification VC to respondent entity via Mass consent primitive, start 30-day response window, freeze disputed corridor operations if requested"],
        ["4. Tribunal Formation", "Institution appoints arbitrators", "Record tribunal composition (sole arbitrator or three-member panel), verify arbitrator credentials against institution rules, issue tribunal appointment VCs"],
        ["5. Proceedings", "Evidence exchange and hearings", "Manage evidence submission deadlines, verify evidence package integrity via SHA-256 digests, track procedural timeline against institution rules"],
        ["6. Resolution", "Tribunal issues ruling", "Mint ArbitrationRuling VC, execute enforcement actions, update corridor state, close dispute or enter appeals phase"],
      ],
      [1200, 2400, 5760]
    ),
    p_runs([bold("State Machine."), " The dispute lifecycle is modeled as a finite state machine with states: Filed, JurisdictionResolved, CounterpartyNotified, TribunalFormed, InProceedings, RulingIssued, EnforcementExecuted, Appealed, and Closed. Transitions are guarded by temporal constraints (response windows, filing deadlines) and authorization checks (only tribunal members can advance from InProceedings to RulingIssued)."]),

    // --- 44.2.2 Evidence Packages ---
    h3("44.2.2 Evidence Packages"),
    p("Evidence packages are cryptographically sealed collections of documents, transaction records, and attestations submitted by parties during arbitration proceedings. Each package is content-addressed and tamper-evident."),
    ...codeBlock(
`/// A sealed evidence package submitted during arbitration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidencePackage {
    pub package_id: EvidencePackageId,
    pub dispute_id: DisputeId,
    pub submitted_by: EntityId,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
    pub items: Vec<EvidenceItem>,
    pub package_digest: CanonicalBytes,
    pub submitter_signature: Ed25519Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub item_id: EvidenceItemId,
    pub category: EvidenceCategory,
    pub description: String,
    pub content_digest: CanonicalBytes,
    pub content_ref: ContentAddressedRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceCategory {
    TransactionRecord,
    CorridorReceipt,
    ComplianceAttestation,
    ContractDocument,
    CorrespondenceLog,
    ExpertReport,
    WatcherAttestation,
}`
    ),
    p_runs([bold("Verification."), " When an evidence package is submitted, the system verifies: (1) the submitter is a recognized party to the dispute, (2) the package digest matches the SHA-256 hash computed over all evidence items via CanonicalBytes, (3) the Ed25519 signature is valid for the submitter's public key, and (4) all content-addressed references resolve to valid objects in the CAS. Evidence packages are immutable once submitted; amendments require a new package referencing the original."]),

    // --- 44.2.3 Escrow Management ---
    h3("44.2.3 Escrow Management"),
    p("When a dispute involves monetary claims, the arbitration system can place disputed amounts in escrow. Escrow is managed through the fiscal primitive (treasury-info) via mez-mass-client, with the EZ Stack providing the jurisdictional and compliance overlay."),
    table(
      ["Escrow State", "Trigger", "Effect"],
      [
        ["Requested", "Claimant files dispute with monetary claim", "Escrow hold request sent to treasury-info via mez-mass-client, amount and currency recorded in dispute state"],
        ["Active", "Tribunal confirms escrow requirement", "Funds locked in designated escrow account, corridor payment flows adjusted to exclude escrowed amount from netting"],
        ["Partial Release", "Tribunal orders interim release", "Specified portion released to designated party, remaining balance stays in escrow, release recorded as enforcement action"],
        ["Full Release", "Final ruling issued", "Entire escrowed amount distributed per ruling, PaymentObligation enforcement actions executed, escrow account closed"],
        ["Expired", "Dispute closed without escrow resolution", "Funds returned to original holder after appeals deadline passes, automatic release triggered by temporal guard"],
      ],
      [1600, 2400, 5360]
    ),
    p_runs([bold("Cross-Border Escrow."), " For disputes spanning multiple jurisdictions (e.g., a PAK-UAE corridor dispute), escrow accounts are created in the jurisdiction specified by the governing law clause. Currency conversion, if required, uses the SBP or central bank rate locked at the time of escrow activation. The compliance tensor is evaluated to ensure escrow operations satisfy both jurisdictions' regulatory requirements, including sanctions screening and withholding tax implications on escrowed returns."]),
  ];
};
