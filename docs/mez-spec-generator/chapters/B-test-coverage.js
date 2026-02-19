const { chapterHeading, h2, h3, p, p_runs, bold, table, codeBlock } = require("../lib/primitives");

module.exports = function build_appendixB() {
  return [
    chapterHeading("Appendix B: Test Coverage Summary"),

    // --- Test Methodology ---
    h2("B.1 Test Methodology"),
    p(
      "The EZ Stack employs a multi-tier testing methodology with 4,073 tests executed " +
      "via cargo test --workspace. Tests span seven campaigns developed during the production " +
      "hardening phase, covering unit, integration, property-based, and contract tests. All tests " +
      "run in the standard Rust test harness with no external test frameworks."
    ),

    h3("B.1.1 Unit Tests"),
    p(
      "Unit tests verify individual functions, type constructors, and error paths in isolation. " +
      "Each crate contains #[cfg(test)] modules co-located with the implementation source. " +
      "Unit tests cover: Momentum Canonical Form (MCF) digest computation, compliance domain " +
      "enumeration and serde round-trips, identifier newtype validation, error hierarchy " +
      "construction, timestamp handling, and sovereignty enforcement. Unwrap calls are " +
      "permitted in test code but forbidden in production paths."
    ),

    h3("B.1.2 Integration Tests"),
    p(
      "The mez-integration-tests crate (113 test files) exercises cross-crate interactions " +
      "and end-to-end workflows. Test suites cover: corridor lifecycle state transitions, " +
      "dual-commitment receipt chain append-and-verify cycles, compliance tensor evaluation " +
      "across multiple jurisdictions, pack trilogy composition (lawpack + regpack + licensepack), " +
      "the Axum HTTP handler stack including middleware (auth, rate limiting, error mapping), " +
      "and serde fidelity for all critical data structures."
    ),

    h3("B.1.3 Property-Based Tests"),
    p(
      "Property-based tests verify algebraic invariants that must hold for all possible values. " +
      "Key properties tested include: receipt chain append produces monotonically increasing MMR " +
      "roots; compute_next_root is deterministic regardless of digest set ordering; compliance " +
      "tensor evaluation is idempotent; canonical serialization is deterministic; corridor state " +
      "machine transitions are total; and saga compensation is idempotent."
    ),

    h3("B.1.4 Contract Tests"),
    p(
      "Contract tests in mez-mass-client validate serialization against the live Mass API " +
      "Swagger/OpenAPI specifications. They verify that request serialization matches the " +
      "expected schema, response deserialization handles all documented shapes, and error " +
      "codes map correctly to the EZ Stack error hierarchy. The NADRA adapter includes " +
      "additional contract tests for identity verification flows."
    ),

    h3("B.1.5 Test Campaigns"),
    p(
      "The production hardening phase executed seven structured test campaigns:"
    ),
    table(
      ["Campaign", "Focus", "Tests Added"],
      [
        ["Campaign 1", "Serde fidelity: round-trip serialization for all critical types", "Core serde tests"],
        ["Campaign 2", "Panic path coverage: 89 tests verifying graceful error handling", "89"],
        ["Campaign 3", "Cross-crate integration: 20 tests for inter-module workflows", "20"],
        ["Campaign 4", "Exhaustive transition matrix: 44 tests for corridor FSM completeness", "44"],
        ["Campaign 5+6", "Boundary conditions and determinism: 32 tests for edge cases", "32"],
        ["Campaign 7", "API contract exhaustive: 64 tests for HTTP endpoint correctness", "64"],
      ],
      [1800, 4400, 3160]
    ),

    // --- Example Test Structure ---
    h2("B.2 Example Test Structure"),
    p(
      "Tests follow a consistent arrange-act-assert pattern with descriptive names " +
      "encoding the scenario under test."
    ),
    ...codeBlock(
`#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_chain_append_enforces_prev_root_linkage() {
        // Arrange: create chain with genesis root
        let genesis_root = "abc123...".to_string();
        let mut chain = ReceiptChain::new(
            CorridorId::new("PAK-UAE"),
            genesis_root.clone(),
        );

        // Act: create and append a valid receipt
        let mut receipt = CorridorReceipt {
            corridor_id: CorridorId::new("PAK-UAE"),
            sequence: 0,
            prev_root: genesis_root,
            next_root: String::new(), // will be computed
            // ... other fields
        };
        receipt.next_root = compute_next_root(&receipt).unwrap();
        chain.append(receipt).expect("first append should succeed");

        // Assert: chain state advanced
        assert_eq!(chain.len(), 1);
        assert_ne!(chain.final_state_root(), &genesis_root);

        // Assert: wrong prev_root is rejected
        let mut bad_receipt = /* ... */;
        bad_receipt.prev_root = "wrong_root".to_string();
        assert!(matches!(
            chain.append(bad_receipt),
            Err(ReceiptError::PrevRootMismatch { .. })
        ));
    }
}`
    ),

    // --- Coverage Summary Table ---
    h2("B.3 Coverage by Crate"),
    p("Test counts are derived from #[test] and #[tokio::test] annotations across the workspace. Total: 4,073 tests."),
    table(
      ["Crate", "Approx. Tests", "Key Coverage Areas"],
      [
        ["mez-core", "~350", "MCF canonicalization, digest computation, domain enumeration, serde fidelity, sovereignty enforcement"],
        ["mez-crypto", "~300", "Ed25519 sign/verify, MMR append/inclusion/root, CAS store/resolve, SHA-256 centralization"],
        ["mez-vc", "~200", "VC issuance, Ed25519 proof validation, credential registry, asset_id binding"],
        ["mez-state", "~400", "Corridor FSM (exhaustive transition matrix), entity lifecycle, license lifecycle, migration saga, watcher bonds/slashing"],
        ["mez-tensor", "~250", "20-domain evaluation, manifold path optimization, tensor commitments, fail-closed empty slices, aggregate state"],
        ["mez-zkp", "~200", "Mock proof system, Groth16/Plonk backends, production policy enforcement, circuit module stubs"],
        ["mez-pack", "~500", "Lawpack parsing, regpack validation, licensepack submodules, pack composition, Akoma Ntoso integration"],
        ["mez-corridor", "~450", "Dual-commitment receipt chain, compute_next_root, fork resolution with signed attestations, netting, SWIFT adapter, payment rails"],
        ["mez-agentic", "~250", "Trigger taxonomy, policy evaluation, tax pipeline, audit trail, scheduling"],
        ["mez-arbitration", "~200", "Dispute lifecycle, evidence management, escrow, enforcement"],
        ["mez-mass-client", "~200", "Contract tests for 5 Mass APIs, NADRA adapter, retry logic, error mapping"],
        ["mez-api", "~400", "Route handlers, middleware, orchestration, database operations, auth"],
        ["mez-cli", "~150", "Corridor CLI, artifact operations, lockfile generation, signing"],
        ["mez-schema", "~100", "Schema validation (Draft 2020-12), $ref resolution, codegen policy"],
        ["mez-compliance", "~50", "Evaluator composition, jurisdiction-aware evaluation"],
        ["mez-integration-tests", "~350", "Cross-crate workflows, serde round-trips, API contract tests, determinism"],
      ],
      [2400, 1200, 5760]
    ),
  ];
};
