const {
  chapterHeading, h2, h3,
  p, p_runs, bold, pageBreak, table,
  spacer, codeBlock
} = require("../lib/primitives");

module.exports = function build_chapter33() {
  return [
    pageBreak(),
    chapterHeading("Chapter 33: Identity and Credentialing Module Family"),

    // --- 33.1 Module Overview ---
    h2("33.1 Module Overview"),
    p("The Identity and Credentialing module family implements progressive identity verification, credential issuance, and cross-jurisdictional identity binding. Identity is currently split across consent-info and organization-info Mass APIs, with msez-mass-client's IdentityClient providing an aggregation facade."),
    table(
      ["Tier", "Access Level", "Requirements", "Limits"],
      [
        ["Tier 0", "Anonymous browsing", "None", "Read-only public data"],
        ["Tier 1", "Basic operations", "Email + phone verification", "$1,000/month transaction limit"],
        ["Tier 2", "Standard operations", "Government ID + liveness check", "$50,000/month transaction limit"],
        ["Tier 3", "Full operations", "Full KYC/KYB + address verification", "Jurisdiction-specific limits"],
        ["Tier 4", "Institutional", "Enhanced due diligence + ongoing monitoring", "No preset limits"],
      ],
      [1200, 2200, 3200, 2760]
    ),

    // --- 33.2 Core Identity Module ---
    h2("33.2 Core Identity Module"),
    p("The Core Identity Module manages decentralized identifiers (DIDs), key bindings, and identity lifecycle. It creates and manages DIDs that serve as the root identity anchor across all jurisdictions. Key rotation, recovery mechanisms, and multi-device support are built into the identity lifecycle."),
    h3("33.2.1 DID Methods"),
    p("The identity system supports four DID methods, each serving a distinct operational context. Method selection is determined by the jurisdiction's requirements, the entity type, and the deployment environment. All methods produce DIDs that can anchor Verifiable Credentials issued by msez-vc."),
    table(
      ["DID Method", "Format", "Description", "Use Case"],
      [
        ["did:msez", "did:msez:<zone>:<id>", "Native Mass/SEZ method anchored to the corridor receipt chain. Provides the strongest binding to jurisdictional context and compliance state. Resolution requires access to the SEZ Stack.", "Primary method for all entities formed through the SEZ Stack. Required for corridor participation and compliance attestation."],
        ["did:web", "did:web:<domain>:<path>", "Web-based DID resolved via HTTPS to a DID Document hosted at the specified domain. Uses existing PKI infrastructure and DNS for discoverability.", "Institutional identities, government agencies, and regulated entities that require web-discoverable identity documents."],
        ["did:key", "did:key:z<multibase>", "Self-certifying DID derived directly from a public key with no external resolution required. Compact and ephemeral, suitable for offline verification.", "Ephemeral sessions, offline credential verification, device-level key identifiers, and test environments."],
        ["did:ion", "did:ion:<long-form>", "Decentralized DID anchored to a Layer 1 blockchain via the ION network. Provides maximum censorship resistance and long-term persistence independent of any single operator.", "Long-lived identities requiring maximum decentralization, cross-platform portability, and independence from SEZ Stack availability."],
      ],
      [1400, 1800, 3560, 2600]
    ),
    spacer(),

    // --- 33.3 Progressive KYC Tiers ---
    h2("33.3 Progressive KYC Tiers"),
    p("Progressive KYC allows users to start with minimal identity verification and incrementally increase their verification level as needed. Each tier enables additional capabilities and higher transaction limits. Tier advancement is non-destructive: all prior credentials remain valid while new attestations are added. The system supports jurisdiction-specific KYC requirements through the compliance tensor."),
    h3("33.3.1 Per-Tier Capabilities"),
    p("Each KYC tier grants a specific set of operational capabilities within the SEZ Stack. The compliance tensor evaluates tier requirements per jurisdiction, meaning the same tier may grant different capabilities depending on the applicable regulatory framework."),
    table(
      ["Tier", "Permitted Operations", "Credential Types Issued", "Compliance Scope"],
      [
        ["Tier 0", "Browse public zone data, view published compliance attestations, access open documentation", "None", "No compliance evaluation required"],
        ["Tier 1", "Create draft entities, submit formation applications, participate in sandbox corridors, receive test credentials", "Email Verification VC, Phone Verification VC", "Basic identity domain only"],
        ["Tier 2", "Form entities, execute domestic transactions, hold securities, sign governance resolutions, file regulatory returns", "KYC Attestation VC, Government ID Binding VC, Liveness Proof VC", "Full 20-domain tensor evaluation for single jurisdiction"],
        ["Tier 3", "Cross-border corridor participation, multi-jurisdictional entity formation, beneficial ownership declarations, tax treaty claims", "Full KYC/KYB VC, Address Verification VC, Source of Funds VC, Professional Qualification VC", "Full 20-domain tensor evaluation across all corridor jurisdictions"],
        ["Tier 4", "Institutional treasury operations, fund administration, regulatory reporting on behalf of portfolio entities, act as registered agent", "Enhanced Due Diligence VC, Institutional Accreditation VC, Ongoing Monitoring VC", "Continuous compliance monitoring with agentic triggers"],
      ],
      [1000, 3160, 2800, 2400]
    ),
    spacer(),

    // --- 33.4 Credentials Module ---
    h2("33.4 Credentials Module"),
    p("The Credentials Module issues, verifies, and manages W3C Verifiable Credentials for all identity attestations. Credentials are issued using Ed25519 proofs with BBS+ selective disclosure support. Credential types include: KYC Tier Attestation, Beneficial Ownership Certificate, Formation Certificate, Compliance Attestation, and Professional Qualification. All credentials are anchored to the receipt chain and corridor state."),

    // --- 33.5 Binding Module ---
    h2("33.5 Binding Module"),
    p("The Binding Module creates verifiable links between decentralized identifiers and external identity systems. Bindings include: CNIC binding (Pakistan national ID via NADRA integration), NTN binding (Pakistan tax number via FBR), passport binding (ICAO 9303 MRZ verification), and corporate registry binding (SECP, DIFC, ADGM). Each binding produces a Verifiable Credential that can be selectively disclosed without revealing the underlying identity document."),
    spacer(),

    // --- 33.6 Identity Recovery Module ---
    h2("33.6 Identity Recovery Module"),
    p("The Identity Recovery Module provides mechanisms to restore access to a decentralized identity when the primary key material is lost, compromised, or inaccessible. Recovery is a critical requirement for sovereign identity systems where no central authority can issue a password reset. The module implements multiple recovery strategies that can be combined based on jurisdiction requirements and entity risk profile."),
    h3("33.6.1 Recovery Strategies"),
    p_runs([bold("Social Recovery."), " A threshold (m-of-n) scheme where designated recovery contacts each hold a recovery share. The identity holder pre-selects trusted contacts who receive encrypted shares of a recovery key. To recover, the holder must obtain approval from at least m of n contacts, who each submit their share through consent.api.mass.inc. The module enforces minimum thresholds (e.g., 3-of-5) and prohibits single-party recovery to prevent social engineering attacks."]),
    p_runs([bold("Institutional Recovery."), " For Tier 3 and Tier 4 identities, a regulated institution (e.g., a registered agent or licensed custodian) can serve as a recovery authority. The institution verifies the identity holder through enhanced due diligence procedures and submits a recovery attestation. This method requires the institution to hold a valid Institutional Accreditation VC and maintain an active license in the relevant jurisdiction."]),
    p_runs([bold("Government ID Re-verification."), " The identity holder re-presents their government-issued identity document (CNIC, passport, or national ID) through the same verification pipeline used during initial KYC. If the re-verification matches the original binding record, a new key pair is generated and the DID Document is updated. This method is available only for identities that have an active Government ID Binding VC."]),
    p_runs([bold("Time-locked Recovery."), " A pre-configured recovery key is sealed with a time-lock that becomes active only after a specified inactivity period (e.g., 180 days of no authenticated operations). The time-lock can be extended by performing any authenticated action, preventing premature activation. This serves as a last-resort mechanism and issues an alert to all bound contact channels before activation."]),
    h3("33.6.2 Recovery Safeguards"),
    p("All recovery operations are subject to a mandatory cooling-off period during which the original key holder can contest the recovery. Recovery events generate an immutable audit trail anchored to the corridor receipt chain via msez-corridor, and a Recovery Event VC is issued to all parties involved. The compliance tensor evaluates the Identity domain for the applicable jurisdiction to ensure recovery procedures meet regulatory requirements for identity continuity."),
    ...codeBlock(
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct RecoveryConfig {\n" +
      "    pub strategy: RecoveryStrategy,\n" +
      "    pub cooling_off_period: Duration,\n" +
      "    pub notification_channels: Vec<NotificationChannel>,\n" +
      "    pub requires_compliance_check: bool,\n" +
      "}\n" +
      "\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub enum RecoveryStrategy {\n" +
      "    Social { threshold: u32, total_shares: u32 },\n" +
      "    Institutional { authority_did: Did, license_vc: VcId },\n" +
      "    GovernmentReVerification { binding_type: BindingType },\n" +
      "    TimeLocked { inactivity_days: u32, sealed_key_ref: KeyRef },\n" +
      "}"
    ),
    spacer(),
  ];
};
