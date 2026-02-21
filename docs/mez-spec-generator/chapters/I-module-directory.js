const { chapterHeading, codeBlock, h2, p, table } = require("../lib/primitives");

module.exports = function build_appendixI() {
  return [
    chapterHeading("Appendix I: Module Directory Structure"),

    p("The EZ Stack organizes 323 module descriptors across 16 families in the modules/ directory. These YAML/JSON descriptors define the configuration vocabulary for zone deployments. The Rust crates in mez/crates/ provide the implementation logic that interprets these descriptors at runtime."),

    ...codeBlock(
      "modules/\n" +
      "\u251c\u2500\u2500 compliance/          # Compliance evaluation modules\n" +
      "\u251c\u2500\u2500 corridors/           # Corridor configuration modules\n" +
      "\u251c\u2500\u2500 governance/          # Constitutional and governance modules\n" +
      "\u251c\u2500\u2500 financial/           # Banking and payment modules\n" +
      "\u251c\u2500\u2500 regulatory/          # KYC, AML, sanctions modules\n" +
      "\u251c\u2500\u2500 licensing/           # Business authorization modules\n" +
      "\u251c\u2500\u2500 legal/               # Contracts and dispute modules\n" +
      "\u251c\u2500\u2500 operational/         # Administrative modules\n" +
      "\u251c\u2500\u2500 corporate/           # Corporate service modules (v0.4.44)\n" +
      "\u251c\u2500\u2500 identity/            # Identity and credentialing modules (v0.4.44)\n" +
      "\u251c\u2500\u2500 tax/                 # Tax and revenue modules (v0.4.44)\n" +
      "\u251c\u2500\u2500 capital_markets/     # Securities infrastructure modules (v0.4.44)\n" +
      "\u251c\u2500\u2500 trade/               # Trade and commerce modules (v0.4.44)\n" +
      "\u251c\u2500\u2500 settlement/          # Settlement layer modules\n" +
      "\u251c\u2500\u2500 migration/           # Cross-jurisdictional migration modules\n" +
      "\u2514\u2500\u2500 watcher/             # Attestation economy modules"
    ),

    h2("I.1 Module Family to Crate Mapping"),
    p("Each module family maps to one or more Rust crates that provide the runtime implementation. The following table shows the mapping and approximate production line counts (non-test source files only)."),

    table(
      ["Module Family", "Primary Crate(s)", "Est. Lines", "Key Source Files"],
      [
        ["compliance/", "mez-tensor, mez-compliance, mez-zkp", "~7,000", "evaluation.rs, manifold.rs, tensor.rs, commitment.rs, policy.rs"],
        ["corridors/", "mez-corridor, mez-state", "~10,200", "receipt.rs, fork.rs, netting.rs, bridge.rs, payment_rail.rs, corridor.rs"],
        ["governance/", "mez-state (corridor FSM states)", "~1,000", "corridor.rs, entity.rs, watcher.rs"],
        ["financial/", "mez-mass-client (treasury-info)", "~1,200", "fiscal.rs, types.rs"],
        ["regulatory/", "mez-pack (regpack), mez-agentic", "~6,500", "regpack.rs, evaluation.rs, audit.rs, scheduler.rs"],
        ["licensing/", "mez-pack (licensepack)", "~3,500", "licensepack/ (6 submodules), composition.rs"],
        ["legal/", "mez-arbitration", "~5,200", "dispute.rs, evidence.rs, enforcement.rs, escrow.rs"],
        ["corporate/", "mez-mass-client (entities)", "~1,000", "entities.rs, types.rs"],
        ["identity/", "mez-mass-client (identity, NADRA), mez-vc", "~3,500", "identity.rs, nadra.rs, credential.rs, registry.rs"],
        ["tax/", "mez-agentic (tax pipeline), mez-pack (lawpack)", "~4,500", "tax.rs, lawpack.rs, parser.rs"],
        ["capital_markets/", "mez-api (settlement routes)", "~1,000", "settlement.rs"],
        ["trade/", "mez-corridor (SWIFT, payment rails)", "~1,200", "swift.rs, payment_rail.rs"],
        ["settlement/", "mez-zkp (circuits), mez-corridor (anchor)", "~2,000", "circuits/, anchor.rs"],
        ["migration/", "mez-corridor (migration), mez-state", "~2,500", "migration.rs (corridor + state)"],
        ["watcher/", "mez-state (watcher economy)", "~1,500", "watcher.rs"],
      ],
      [1600, 2800, 900, 4060]
    ),

    h2("I.2 Infrastructure Crates"),
    p("The following crates provide cross-cutting infrastructure used by all module families:"),

    table(
      ["Crate", "Lines", "Scope"],
      [
        ["mez-core", "~3,300", "MCF canonical digest, ComplianceDomain (20 variants), sovereignty enforcement, identifier newtypes, timestamps"],
        ["mez-crypto", "~3,300", "Ed25519 signing/verification with Zeroize, SHA-256, MMR, CAS, Poseidon2 (stub), BBS+ (stub)"],
        ["mez-schema", "~1,800", "JSON Schema Draft 2020-12 validation, 116 schemas, $ref resolution, codegen policy"],
        ["mez-api", "~17,100", "Axum HTTP server, 10 route groups, orchestration, Postgres persistence, auth + rate-limit middleware"],
        ["mez-cli", "~4,800", "Clap-derived CLI: validate, lock, corridor, artifact, vc (signing)"],
      ],
      [1800, 1000, 6560]
    ),

    p("Total workspace: 164,000 lines of Rust across 322 source files in 17 crates, with 4,683 tests (including the mez-integration-tests crate with 113 test files)."),
  ];
};
