const { chapterHeading, codeBlock, h2, p, table } = require("../lib/primitives");

module.exports = function build_appendixE() {
  return [
    chapterHeading("Appendix E: Rust Crate Dependency Graph"),

    p("The 17-crate workspace forms a directed acyclic graph rooted at mez-core (zero internal dependencies) and culminating at mez-api (the composition point). The following tree shows the primary dependency paths; transitive dependencies are elided for clarity."),

    ...codeBlock(
      "mez-api (Axum HTTP server \u2014 composition point)\n" +
      "  \u251c\u2500\u2500 mez-corridor       (receipt chains, fork resolution, netting, payment rails)\n" +
      "  \u2502   \u251c\u2500\u2500 mez-core       (canonical digest, ComplianceDomain, timestamps)\n" +
      "  \u2502   \u2514\u2500\u2500 mez-crypto     (Ed25519, MMR, CAS, SHA-256)\n" +
      "  \u251c\u2500\u2500 mez-tensor        (compliance tensor, manifold, path optimization)\n" +
      "  \u2502   \u2514\u2500\u2500 mez-core\n" +
      "  \u251c\u2500\u2500 mez-pack          (lawpacks, regpacks, licensepacks, composition)\n" +
      "  \u2502   \u251c\u2500\u2500 mez-core\n" +
      "  \u2502   \u2514\u2500\u2500 mez-schema     (JSON Schema Draft 2020-12 validation)\n" +
      "  \u251c\u2500\u2500 mez-state         (corridor FSM, migration saga, watcher economy)\n" +
      "  \u2502   \u2514\u2500\u2500 mez-core\n" +
      "  \u251c\u2500\u2500 mez-agentic       (triggers, policy, tax pipeline, audit)\n" +
      "  \u2502   \u2514\u2500\u2500 mez-core\n" +
      "  \u251c\u2500\u2500 mez-arbitration   (disputes, evidence, escrow, enforcement)\n" +
      "  \u2502   \u2514\u2500\u2500 mez-core\n" +
      "  \u251c\u2500\u2500 mez-vc            (W3C Verifiable Credentials, Ed25519 proofs)\n" +
      "  \u2502   \u251c\u2500\u2500 mez-core\n" +
      "  \u2502   \u2514\u2500\u2500 mez-crypto\n" +
      "  \u251c\u2500\u2500 mez-zkp           (proof systems, circuits, production policy)\n" +
      "  \u2502   \u2514\u2500\u2500 mez-core\n" +
      "  \u251c\u2500\u2500 mez-compliance    (evaluator composition)\n" +
      "  \u2502   \u2514\u2500\u2500 mez-core\n" +
      "  \u2514\u2500\u2500 mez-mass-client   (typed HTTP client for 5 Mass primitives)\n" +
      "      \u2514\u2500\u2500 mez-core\n" +
      "\n" +
      "mez-cli (command-line interface)\n" +
      "  \u251c\u2500\u2500 mez-core\n" +
      "  \u251c\u2500\u2500 mez-crypto\n" +
      "  \u251c\u2500\u2500 mez-schema\n" +
      "  \u2514\u2500\u2500 mez-vc\n" +
      "\n" +
      "Shared: serde, serde_json, tokio, chrono, uuid, thiserror, ed25519-dalek, sha2"
    ),

    h2("E.1 Dependency Invariants"),
    p("The following six invariants must hold at all times. Violating any invariant is a blocking code review failure."),

    table(
      ["ID", "Invariant", "Rationale", "Enforcement"],
      [
        [
          "INV-1",
          "mez-core has zero internal dependencies. It depends only on serde, serde_json, thiserror, chrono, uuid, sha2.",
          "mez-core is the foundation layer defining canonical digests, ComplianceDomain, identifier newtypes, error hierarchy, and timestamps. Any internal dependency would create cycle risk.",
          "cargo deny check + CI gate: reject any Cargo.toml change adding an mez-* dependency to mez-core."
        ],
        [
          "INV-2",
          "mez-mass-client depends only on mez-core for identifier newtypes.",
          "mez-mass-client is the sole gateway to live Mass APIs. It must remain a thin, typed HTTP client. Importing EZ domain logic would conflate orchestration with transport.",
          "CI lint: mez-mass-client may only import mez-core. Any other mez-* import is rejected."
        ],
        [
          "INV-3",
          "No cycles in the dependency graph.",
          "Cycles make independent compilation, testing, and change-propagation reasoning impossible. Rust forbids crate cycles at compile time.",
          "cargo deny check + topological sort verification in CI."
        ],
        [
          "INV-4",
          "mez-api is the sole composition point. No other crate depends on mez-api.",
          "mez-api composes all other crates into a running service. If another crate depended on mez-api it would invert dependency direction.",
          "CI gate: reject any Cargo.toml adding mez-api as a dependency."
        ],
        [
          "INV-5",
          "All SHA-256 computation flows through CanonicalBytes / sha256_digest(). No crate may compute SHA-256 through any other path.",
          "Canonical digests underpin the receipt chain, MMR, CAS, and VC integrity. Multiple paths produce inconsistent hashes.",
          "CI lint: reject direct use of sha2::Sha256 outside mez-core and mez-crypto."
        ],
        [
          "INV-6",
          "ComplianceDomain is defined exactly once in mez-core.",
          "ComplianceDomain (20 variants) is the shared vocabulary for tensor, pack trilogy, agentic triggers, and arbitration.",
          "CI lint: reject any enum containing ComplianceDomain outside mez-core."
        ],
      ],
      [600, 2800, 3200, 2760]
    ),

    h2("E.2 Invariant Verification"),
    p("All six invariants are verified automatically on every pull request via CI checks."),
    ...codeBlock(
      "# CI invariant checks (GitHub Actions)\n" +
      "cargo deny check                              # INV-1, INV-2, INV-3\n" +
      "cargo metadata --format-version 1 \\           # INV-4\n" +
      "  | jq '.packages[] | select(.dependencies[]\n" +
      "    | .name == \"mez-api\") | .name'\n" +
      "  | grep -v mez-cli && exit 1 || true\n" +
      "rg 'sha2::|Sha256::new|Digest::new' \\         # INV-5\n" +
      "  --glob '!mez-core/**' --glob '!mez-crypto/**'\n" +
      "  --glob '!**/test*'\n" +
      "  && exit 1 || true\n" +
      "rg 'enum ComplianceDomain' \\                   # INV-6\n" +
      "  --glob '!mez-core/**'\n" +
      "  && exit 1 || true"
    ),
  ];
};
