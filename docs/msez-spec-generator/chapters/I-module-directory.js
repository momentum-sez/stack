const { chapterHeading, codeBlock, spacer, h2, p, table } = require("../lib/primitives");

module.exports = function build_appendixI() {
  return [
    chapterHeading("Appendix I: Module Directory Structure"),
    ...codeBlock(
      "crates/msez-modules/src/\n" +
      "\u251C\u2500\u2500 compliance/          # Tensor, manifold, ZK circuits\n" +
      "\u251C\u2500\u2500 corridors/           # State sync, bridge, multilateral\n" +
      "\u251C\u2500\u2500 governance/          # Constitutional, voting, delegation\n" +
      "\u251C\u2500\u2500 financial/           # Accounts, payments, custody, FX\n" +
      "\u251C\u2500\u2500 regulatory/          # KYC, AML, sanctions, reporting\n" +
      "\u251C\u2500\u2500 licensing/           # Applications, monitoring, portability\n" +
      "\u251C\u2500\u2500 legal/               # Contracts, disputes, arbitration\n" +
      "\u251C\u2500\u2500 operational/         # HR, procurement, facilities\n" +
      "\u251C\u2500\u2500 corporate/           # Formation, cap table, dissolution (v0.4.44)\n" +
      "\u251C\u2500\u2500 identity/            # DID, KYC tiers, credentials (v0.4.44)\n" +
      "\u251C\u2500\u2500 tax/                 # Regimes, fees, incentives (v0.4.44)\n" +
      "\u251C\u2500\u2500 capital_markets/     # Securities, trading, CSD (v0.4.44)\n" +
      "\u2514\u2500\u2500 trade/               # LCs, documents, SCF (v0.4.44)"
    ),
    spacer(),

    h2("I.1 Line Count Estimates by Module Family"),
    p("The following table provides approximate line counts for each module family and its constituent crates across the workspace. Estimates are based on the v0.4.44 codebase as of February 2026."),
    spacer(),

    table(
      ["Module Family", "Primary Crate(s)", "Est. Lines", "Key Files"],
      [
        ["compliance/", "msez-tensor, msez-compliance, msez-zkp", "~6,200", "tensor_v2.rs, manifold.rs, compliance_domain.rs, proof_system.rs"],
        ["corridors/", "msez-corridor, msez-state", "~5,800", "corridor.rs, receipt_chain.rs, fork_resolution.rs, netting.rs, fsm.rs"],
        ["governance/", "msez-governance (in msez-modules)", "~2,400", "constitutional.rs, voting.rs, delegation.rs, quorum.rs"],
        ["financial/", "msez-mass-client (gateway to treasury-info)", "~3,100", "treasury_client.rs, accounts.rs, payments.rs, fx.rs"],
        ["regulatory/", "msez-pack (regpack), msez-agentic", "~4,500", "regpack.rs, sanctions.rs, aml_rules.rs, filing_calendar.rs, triggers.rs"],
        ["licensing/", "msez-pack (licensepack)", "~2,300", "licensepack.rs, registry_client.rs, portability.rs"],
        ["legal/", "msez-arbitration", "~2,600", "dispute.rs, evidence.rs, ruling.rs, escrow.rs"],
        ["operational/", "msez-modules/operational", "~1,200", "hr.rs, procurement.rs, facilities.rs"],
        ["corporate/", "msez-mass-client (gateway to org-info)", "~2,800", "organization_client.rs, formation.rs, beneficial_ownership.rs"],
        ["identity/", "msez-mass-client (aggregation facade), msez-vc", "~3,400", "identity_client.rs, kyc_tiers.rs, did.rs, verifiable_credentials.rs"],
        ["tax/", "msez-pack (lawpack), msez-agentic", "~3,200", "lawpack.rs, composition.rs, tax_events.rs, withholding.rs"],
        ["capital_markets/", "msez-modules/capital_markets", "~1,800", "securities.rs, trading.rs, csd.rs"],
        ["trade/", "msez-corridor, msez-modules/trade", "~2,100", "letter_of_credit.rs, trade_docs.rs, scf.rs"],
      ],
      [1400, 2800, 900, 4260]
    ),
    spacer(),

    h2("I.2 Core Infrastructure Crates"),
    p("In addition to the domain module families above, the following infrastructure crates provide the foundational services:"),
    spacer(),

    table(
      ["Crate", "Est. Lines", "Scope"],
      [
        ["msez-core", "~2,800", "Canonical digest, ComplianceDomain (20 variants), identifier newtypes, error hierarchy, timestamps, CanonicalBytes"],
        ["msez-crypto", "~1,900", "Ed25519 signing/verification with Zeroize, MMR append/verify, CAS put/get, key management"],
        ["msez-vc", "~2,200", "W3C Verifiable Credentials data model, Ed25519 proofs, BBS+ selective disclosure, revocation lists"],
        ["msez-pack", "~4,800", "Pack Trilogy: lawpacks (Akoma Ntoso), regpacks (sanctions, calendars), licensepacks (live registries), composition engine"],
        ["msez-schema", "~1,400", "116 JSON Schemas, schema loader, validation engine, schema versioning"],
        ["msez-mass-client", "~2,600", "Typed HTTP client for all five Mass primitives, auth token management, retry/backoff logic"],
        ["msez-api", "~5,200", "Axum HTTP server, route composition, middleware (auth, rate-limit, logging), database pool, mass proxy routes"],
        ["msez-cli", "~1,800", "Clap-derived CLI, command dispatch, output formatting, config file loading"],
      ],
      [1800, 1000, 6560]
    ),
    spacer(),

    p("Total workspace estimate: ~62,000 lines of Rust (excluding tests and generated code). Test modules add approximately 18,000 additional lines."),
    spacer(),
  ];
};
