const { chapterHeading, codeBlock, h2, p, table } = require("../lib/primitives");

module.exports = function build_appendixE() {
  return [
    chapterHeading("Appendix E: Rust Crate Dependency Graph"),

    p("The 16-crate workspace forms a directed acyclic graph rooted at msez-core (zero internal dependencies) and culminating at msez-api (the composition point). The following tree shows the primary dependency paths; transitive dependencies are elided for clarity."),

    ...codeBlock(
      "msez-api (Axum HTTP server \u2014 composition point)\n" +
      "  \u251c\u2500\u2500 msez-corridor       (receipt chains, fork resolution, netting, payment rails)\n" +
      "  \u2502   \u251c\u2500\u2500 msez-core       (canonical digest, ComplianceDomain, timestamps)\n" +
      "  \u2502   \u2514\u2500\u2500 msez-crypto     (Ed25519, MMR, CAS, SHA-256)\n" +
      "  \u251c\u2500\u2500 msez-tensor        (compliance tensor, manifold, path optimization)\n" +
      "  \u2502   \u2514\u2500\u2500 msez-core\n" +
      "  \u251c\u2500\u2500 msez-pack          (lawpacks, regpacks, licensepacks, composition)\n" +
      "  \u2502   \u251c\u2500\u2500 msez-core\n" +
      "  \u2502   \u2514\u2500\u2500 msez-schema     (JSON Schema Draft 2020-12 validation)\n" +
      "  \u251c\u2500\u2500 msez-state         (corridor FSM, migration saga, watcher economy)\n" +
      "  \u2502   \u2514\u2500\u2500 msez-core\n" +
      "  \u251c\u2500\u2500 msez-agentic       (triggers, policy, tax pipeline, audit)\n" +
      "  \u2502   \u2514\u2500\u2500 msez-core\n" +
      "  \u251c\u2500\u2500 msez-arbitration   (disputes, evidence, escrow, enforcement)\n" +
      "  \u2502   \u2514\u2500\u2500 msez-core\n" +
      "  \u251c\u2500\u2500 msez-vc            (W3C Verifiable Credentials, Ed25519 proofs)\n" +
      "  \u2502   \u251c\u2500\u2500 msez-core\n" +
      "  \u2502   \u2514\u2500\u2500 msez-crypto\n" +
      "  \u251c\u2500\u2500 msez-zkp           (proof systems, circuits, production policy)\n" +
      "  \u2502   \u2514\u2500\u2500 msez-core\n" +
      "  \u251c\u2500\u2500 msez-compliance    (evaluator composition)\n" +
      "  \u2502   \u2514\u2500\u2500 msez-core\n" +
      "  \u2514\u2500\u2500 msez-mass-client   (typed HTTP client for 5 Mass primitives)\n" +
      "      \u2514\u2500\u2500 msez-core\n" +
      "\n" +
      "msez-cli (command-line interface)\n" +
      "  \u251c\u2500\u2500 msez-core\n" +
      "  \u251c\u2500\u2500 msez-crypto\n" +
      "  \u251c\u2500\u2500 msez-schema\n" +
      "  \u2514\u2500\u2500 msez-vc\n" +
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
          "msez-core has zero internal dependencies. It depends only on serde, serde_json, thiserror, chrono, uuid, sha2.",
          "msez-core is the foundation layer defining canonical digests, ComplianceDomain, identifier newtypes, error hierarchy, and timestamps. Any internal dependency would create cycle risk.",
          "cargo deny check + CI gate: reject any Cargo.toml change adding an msez-* dependency to msez-core."
        ],
        [
          "INV-2",
          "msez-mass-client depends only on msez-core for identifier newtypes.",
          "msez-mass-client is the sole gateway to live Mass APIs. It must remain a thin, typed HTTP client. Importing SEZ domain logic would conflate orchestration with transport.",
          "CI lint: msez-mass-client may only import msez-core. Any other msez-* import is rejected."
        ],
        [
          "INV-3",
          "No cycles in the dependency graph.",
          "Cycles make independent compilation, testing, and change-propagation reasoning impossible. Rust forbids crate cycles at compile time.",
          "cargo deny check + topological sort verification in CI."
        ],
        [
          "INV-4",
          "msez-api is the sole composition point. No other crate depends on msez-api.",
          "msez-api composes all other crates into a running service. If another crate depended on msez-api it would invert dependency direction.",
          "CI gate: reject any Cargo.toml adding msez-api as a dependency."
        ],
        [
          "INV-5",
          "All SHA-256 computation flows through CanonicalBytes / sha256_digest(). No crate may compute SHA-256 through any other path.",
          "Canonical digests underpin the receipt chain, MMR, CAS, and VC integrity. Multiple paths produce inconsistent hashes.",
          "CI lint: reject direct use of sha2::Sha256 outside msez-core and msez-crypto."
        ],
        [
          "INV-6",
          "ComplianceDomain is defined exactly once in msez-core.",
          "ComplianceDomain (20 variants) is the shared vocabulary for tensor, pack trilogy, agentic triggers, and arbitration.",
          "CI lint: reject any enum containing ComplianceDomain outside msez-core."
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
      "    | .name == \"msez-api\") | .name'\n" +
      "  | grep -v msez-cli && exit 1 || true\n" +
      "rg 'sha2::|Sha256::new|Digest::new' \\         # INV-5\n" +
      "  --glob '!msez-core/**' --glob '!msez-crypto/**'\n" +
      "  --glob '!**/test*'\n" +
      "  && exit 1 || true\n" +
      "rg 'enum ComplianceDomain' \\                   # INV-6\n" +
      "  --glob '!msez-core/**'\n" +
      "  && exit 1 || true"
    ),
  ];
};
