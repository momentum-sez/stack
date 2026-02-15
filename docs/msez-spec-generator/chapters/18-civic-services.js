const {
  chapterHeading, h2,
  p, p_runs, bold,
  table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter18() {
  return [
    chapterHeading("Chapter 18: Civic Services Integration"),

    // --- 18.1 Identity Services ---
    h2("18.1 Identity Services"),
    p("Zone identity services provide residents and businesses with verifiable credentials that enable participation in economic activities across the zone and its connected corridors. Four credential categories serve the full spectrum of identity requirements:"),
    table(
      ["Credential Type", "Issued By", "Use Cases"],
      [
        ["Resident Credential", "Zone authority", "Zone residency status, rights, obligations, access to civic services"],
        ["Business Credential", "Zone authority + SECP/equivalent", "Entity registration, good standing, authorized activities, annual compliance status"],
        ["Professional Credential", "Licensing authority", "Qualifications for regulated professions (legal, medical, financial advisory)"],
        ["Employment Credential", "Employer entity + zone", "Work permit status, employer verification, social security enrollment"],
      ],
      [2400, 2800, 4160]
    ),
    spacer(),
    p("All credentials support selective disclosure via BBS+ signatures: a holder presents the minimum set of attributes required for a specific interaction. A business credential holder proving authorized activity to a corridor counterparty reveals the activity code and good-standing status without exposing formation date, director list, or financial statements. Credential revocation propagates through the nullifier system; a revoked credential's nullifier enters the spent set, preventing further presentation."),

    // --- 18.2 Property Services ---
    h2("18.2 Property Services"),
    p("Property rights are represented as Smart Assets with zone-specific lawpack bindings. The property services subsystem provides three capabilities:"),
    p_runs([bold("Title Registry."), " The authoritative record of property ownership is maintained as an append-only receipt chain. Each title transfer produces a receipt containing the property identifier, transferor, transferee, consideration amount, and applicable taxes. The receipt chain provides a complete provenance record from original registration through every subsequent transfer. Title searches resolve by replaying the receipt chain for a given property identifier."]),
    p_runs([bold("Transfer Services."), " Property transfers require compliance verification across multiple domains: REAL_ESTATE (transfer restrictions, foreign ownership limits), TAX (stamp duty, capital gains tax, withholding), AML_CFT (source of funds verification), and CORPORATE (if the transferee is an entity, beneficial ownership verification). The compliance tensor evaluation produces a combined determination that gates the transfer."]),
    p_runs([bold("Encumbrance Management."), " Liens, mortgages, and other property interests are recorded as encumbrance receipts appended to the property's receipt chain. Each encumbrance specifies the creditor, principal amount, interest terms, maturity date, and priority rank. Transfer attempts on encumbered property require creditor consent or satisfaction of the encumbrance as a precondition, enforced by the state machine."]),

    // --- 18.3 Dispute Resolution Services ---
    h2("18.3 Dispute Resolution Services"),
    p("The civic services layer integrates with the arbitration module to provide structured dispute resolution across three tiers:"),
    table(
      ["Tier", "Procedure", "Value Threshold", "Resolution Target"],
      [
        ["Small Claims", "Expedited single-arbitrator process", "<$50,000 equivalent", "30 days"],
        ["Commercial Arbitration", "Three-arbitrator panel, institutional rules", "$50,000\u2013$10M", "90\u2013180 days"],
        ["Complex Commercial", "Full tribunal, expert witnesses, document discovery", ">$10M", "180\u2013360 days"],
      ],
      [2000, 3200, 2200, 1960]
    ),
    spacer(),
    p("Arbitration institutions supported include DIFC-LCIA (Dubai), SIAC (Singapore), AIFC-IAC (Kazakhstan), ICC (Paris), and LCIA (London). The arbitration module manages evidence packages as content-addressed artifacts, ruling issuance as Verifiable Credentials, and enforcement through the corridor system. Rulings from one zone are enforceable in connected zones through corridor-level recognition agreements, with enforcement actions triggered by the agentic framework upon ruling finality."),

    // --- 18.4 Public Service Delivery ---
    h2("18.4 Public Service Delivery"),
    p("For sovereign GovOS deployments (e.g., Pakistan), civic services extend to public service delivery. The GovOS Console provides citizen-facing portals for tax filing (direct integration with FBR IRIS), business registration (SECP e-filing), utility connections, license applications, and permit processing. Each service interaction generates a receipt in the relevant entity's chain and a compliance tensor update if the interaction affects regulatory status."),
  ];
};
