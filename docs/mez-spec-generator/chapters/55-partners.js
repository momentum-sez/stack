const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  table
} = require("../lib/primitives");

module.exports = function build_chapter55() {
  return [
    chapterHeading("Chapter 55: Partner Network"),

    // --- 55.1 Partner Categories ---
    h2("55.1 Partner Categories"),
    p("Partners are classified into three tiers based on their integration depth with the MEZ Stack and Mass APIs."),
    table(
      ["Category", "Role", "Integration Level", "Examples"],
      [
        ["Jurisdictional", "Government agencies and free zone authorities deploying sovereign infrastructure", "Full stack deployment; contribute lawpacks, regpacks, and licensepacks", "Pakistan PDA, UAE ADGM, Dubai FZC, Kazakhstan AIFC"],
        ["Operational", "Corporate service providers, banks, and financial institutions processing transactions", "Mass API integration; corridor participation; credential issuance", "Northern Trust (custody), SBP (payments), SECP (corporate registry)"],
        ["Integration", "Technology firms building on the MEZ Stack API", "API consumer; custom modules, compliance domains, credential types", "KYC providers, legal-tech firms, regtech platforms"],
      ],
      [1600, 3200, 2400, 2160]
    ),

    // --- 55.2 Technology Partners ---
    h2("55.2 Technology Partners"),
    p("Technology partner infrastructure is organized across five integration layers."),
    table(
      ["Layer", "Function", "Requirements", "Integration Point"],
      [
        ["Cloud Infrastructure", "Sovereign-compliant hosting with data residency guarantees", "Jurisdiction-local data centers, ISO 27001, SOC 2 Type II", "Terraform modules, Kubernetes operators"],
        ["Identity", "KYC/KYB verification, biometric auth, government ID integration", "NADRA (PAK), ICA (UAE), AIFC registry (KAZ) adapters", "mez-mass-client IdentityClient"],
        ["Payments", "Banking rails, SWIFT, real-time payment systems, FX", "SBP Raast (PAK), UAEPGS (UAE), SAMA SARIE (KSA)", "mez-mass-client FiscalClient"],
        ["Legal Technology", "Akoma Ntoso processing, regulatory change monitoring", "Legislative corpus parsing, SRO tracking, gazette monitoring", "mez-pack lawpack/regpack pipeline"],
        ["Security", "Penetration testing, cryptographic audit, compliance certification", "Annual pen test, quarterly vulnerability scan, ZK circuit audit", "Pre-deployment gate in CI/CD"],
      ],
      [1600, 2600, 2600, 2560]
    ),
  ];
};
