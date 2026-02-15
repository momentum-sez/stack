const {
  chapterHeading, h2,
  p,
  codeBlock, spacer
} = require("../lib/primitives");

module.exports = function build_chapter44() {
  return [
    chapterHeading("Chapter 44: Arbitration System"),

    // --- 44.1 Institution Registry ---
    h2("44.1 Institution Registry"),
    p("The arbitration system maintains a registry of recognized institutions. Recognized institutions: DIFC-LCIA Arbitration Centre, Singapore International Arbitration Centre (SIAC), AIFC International Arbitration Centre (IAC), International Chamber of Commerce (ICC) International Court of Arbitration, ADGM Arbitration Centre. Each institution has associated rules encoded as machine-readable specifications: filing procedures, tribunal formation rules, procedural timelines, fee schedules, and enforcement mechanisms."),

    // --- 44.2 Ruling Enforcement ---
    h2("44.2 Ruling Enforcement"),
    p("Arbitration rulings are issued as Verifiable Credentials signed by the tribunal. The ruling VC contains the dispute identifier, tribunal composition, decision summary, enforcement actions, and appeals deadline. Enforcement actions are executed automatically by the SEZ Stack: asset freezes are applied via corridor state updates, payment obligations are routed through the fiscal primitive, and compliance tensor entries are updated to reflect the ruling outcome."),
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
    spacer(),
  ];
};
