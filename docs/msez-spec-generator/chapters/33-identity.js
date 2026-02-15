const {
  chapterHeading, h2,
  p, pageBreak, table
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

    // --- 33.3 Progressive KYC Tiers ---
    h2("33.3 Progressive KYC Tiers"),
    p("Progressive KYC allows users to start with minimal identity verification and incrementally increase their verification level as needed. Each tier enables additional capabilities and higher transaction limits. Tier advancement is non-destructive: all prior credentials remain valid while new attestations are added. The system supports jurisdiction-specific KYC requirements through the compliance tensor."),

    // --- 33.4 Credentials Module ---
    h2("33.4 Credentials Module"),
    p("The Credentials Module issues, verifies, and manages W3C Verifiable Credentials for all identity attestations. Credentials are issued using Ed25519 proofs with BBS+ selective disclosure support. Credential types include: KYC Tier Attestation, Beneficial Ownership Certificate, Formation Certificate, Compliance Attestation, and Professional Qualification. All credentials are anchored to the receipt chain and corridor state."),

    // --- 33.5 Binding Module ---
    h2("33.5 Binding Module"),
    p("The Binding Module creates verifiable links between decentralized identifiers and external identity systems. Bindings include: CNIC binding (Pakistan national ID via NADRA integration), NTN binding (Pakistan tax number via FBR), passport binding (ICAO 9303 MRZ verification), and corporate registry binding (SECP, DIFC, ADGM). Each binding produces a Verifiable Credential that can be selectively disclosed without revealing the underlying identity document."),
  ];
};
