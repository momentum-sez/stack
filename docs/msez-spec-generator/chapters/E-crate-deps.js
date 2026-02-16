const { chapterHeading, codeBlock, h2, p, table } = require("../lib/primitives");

module.exports = function build_appendixE() {
  return [
    chapterHeading("Appendix E: Rust Crate Dependency Graph"),
    ...codeBlock(
      "msez-cli\n" +
      "  \u251C\u2500\u2500 msez-govos\n" +
      "  \u2502   \u251C\u2500\u2500 msez-mass-bridge\n" +
      "  \u2502   \u2502   \u251C\u2500\u2500 msez-pack\n" +
      "  \u2502   \u2502   \u251C\u2500\u2500 msez-tensor\n" +
      "  \u2502   \u2502   \u2514\u2500\u2500 msez-core\n" +
      "  \u2502   \u251C\u2500\u2500 msez-modules\n" +
      "  \u2502   \u2514\u2500\u2500 msez-corridor\n" +
      "  \u251C\u2500\u2500 msez-vm\n" +
      "  \u2502   \u251C\u2500\u2500 msez-tensor\n" +
      "  \u2502   \u2514\u2500\u2500 msez-core\n" +
      "  \u251C\u2500\u2500 msez-migration\n" +
      "  \u2502   \u251C\u2500\u2500 msez-corridor\n" +
      "  \u2502   \u251C\u2500\u2500 msez-watcher\n" +
      "  \u2502   \u2514\u2500\u2500 msez-tensor\n" +
      "  \u251C\u2500\u2500 msez-watcher\n" +
      "  \u2502   \u2514\u2500\u2500 msez-core\n" +
      "  \u2514\u2500\u2500 msez-governance\n" +
      "      \u2514\u2500\u2500 msez-core\n" +
      "\n" +
      "Shared dependencies: serde, tokio, chrono, ed25519-dalek, arkworks, halo2"
    ),

    h2("E.1 Dependency Invariants"),
    p("The following six invariants must hold at all times. Violating any invariant is a blocking code review failure and must be resolved before merge."),

    table(
      ["ID", "Invariant", "Rationale", "Enforcement"],
      [
        [
          "INV-1",
          "msez-core has zero internal dependencies. It depends only on serde, serde_json, thiserror, chrono, uuid, sha2.",
          "msez-core is the foundation layer defining canonical digests, ComplianceDomain, identifier newtypes, error hierarchy, and timestamps. Any internal dependency would create cycle risk and pollute the foundation with domain logic.",
          "cargo deny check + CI gate: reject any Cargo.toml change adding an msez-* dependency to msez-core."
        ],
        [
          "INV-2",
          "msez-mass-client has zero or at most one internal dependency. If it depends on msez-core, it is ONLY for identifier newtypes.",
          "msez-mass-client is the sole gateway to live Mass APIs. It must remain a thin, typed HTTP client. Importing SEZ domain logic (tensors, packs, corridors) would conflate orchestration with transport.",
          "CI lint: msez-mass-client may only import msez-core. Any other msez-* import is rejected."
        ],
        [
          "INV-3",
          "No cycles in the dependency graph.",
          "Cycles make independent compilation, testing, and change-propagation reasoning impossible. Rust forbids crate cycles at compile time, but workspace feature flags can mask transitive cycles.",
          "cargo deny check + topological sort verification in CI. The crate graph must be a DAG at all times."
        ],
        [
          "INV-4",
          "msez-api is the sole composition point. No other crate depends on msez-api.",
          "msez-api is the Axum HTTP server composing all other crates into a running service. If another crate depended on msez-api it would invert dependency direction and couple domain logic to HTTP transport.",
          "CI gate: reject any Cargo.toml adding msez-api as a dependency. Only msez-cli and integration tests may reference msez-api."
        ],
        [
          "INV-5",
          "All SHA-256 computation flows through CanonicalBytes::new(). No crate may compute SHA-256 digests through any other path.",
          "Canonical digests underpin the receipt chain, MMR, CAS, and VC integrity. Multiple digest paths produce inconsistent hashes for the same data, breaking verification across the stack.",
          "grep/clippy lint: reject direct use of sha2::Sha256 or Digest::new() outside msez-core::CanonicalBytes."
        ],
        [
          "INV-6",
          "ComplianceDomain is defined exactly once in msez-core. No other crate may define its own domain enum.",
          "ComplianceDomain (20 variants) is the shared vocabulary for tensor, pack trilogy, agentic triggers, and arbitration. Duplicate definitions cause silent domain mismatches.",
          "grep lint: reject any enum containing 'ComplianceDomain' outside msez-core/src/."
        ],
      ],
      [600, 2800, 3200, 2760]
    ),

    h2("E.2 Invariant Verification"),
    p("All six invariants are verified automatically on every pull request via the following CI checks:"),
    ...codeBlock(
      "# CI invariant checks (run in GitHub Actions)\n" +
      "cargo deny check                              # INV-1, INV-2, INV-3\n" +
      "cargo metadata --format-version 1 \\           # INV-4\n" +
      "  | jq '.packages[] | select(.dependencies[]\n" +
      "    | .name == \"msez-api\") | .name'\n" +
      "  | grep -v msez-cli && exit 1 || true\n" +
      "rg 'sha2::|Sha256::new|Digest::new' \\         # INV-5\n" +
      "  --glob '!msez-core/**' --glob '!**/test*'\n" +
      "  && exit 1 || true\n" +
      "rg 'enum ComplianceDomain' \\                   # INV-6\n" +
      "  --glob '!msez-core/**'\n" +
      "  && exit 1 || true"
    ),
  ];
};
