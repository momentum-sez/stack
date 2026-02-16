const { chapterHeading, h2, h3, p, p_runs, bold, table, codeBlock, spacer } = require("../lib/primitives");

module.exports = function build_appendixB() {
  return [
    chapterHeading("Appendix B: Test Coverage Summary"),

    // --- Test Methodology ---
    h2("B.1 Test Methodology"),
    p(
      "The SEZ Stack employs a four-tier testing methodology designed to verify correctness " +
      "at every level of abstraction, from individual function behavior through cross-service " +
      "integration. All tests run in the standard Rust test harness via cargo test --workspace, " +
      "with no external test frameworks or Python dependencies."
    ),
    spacer(),

    h3("B.1.1 Unit Tests"),
    p(
      "Unit tests verify individual functions, type constructors, and error paths in isolation. " +
      "Each crate contains a #[cfg(test)] module co-located with the implementation source. " +
      "Unit tests cover: canonical digest computation (CanonicalBytes), compliance domain enumeration " +
      "and serialization round-trips, identifier newtype validation, error hierarchy construction, " +
      "and timestamp handling. Unwrap calls are permitted in test code but forbidden in production paths."
    ),

    h3("B.1.2 Integration Tests"),
    p(
      "Integration tests exercise cross-crate interactions and end-to-end workflows. Located " +
      "in each crate's tests/ directory, they verify: corridor lifecycle state transitions " +
      "(creation through settlement), compliance tensor evaluation across multiple jurisdictions, " +
      "pack trilogy composition (lawpack + regpack + licensepack), receipt chain append-and-verify " +
      "cycles, and the Axum HTTP handler stack including middleware (auth, rate limiting, error mapping). " +
      "Integration tests load JSON fixtures from tests/fixtures/ for deterministic scenario replay."
    ),

    h3("B.1.3 Property-Based Tests"),
    p(
      "Property-based tests use randomized inputs to verify algebraic invariants that must hold " +
      "for all possible values. Key properties tested include: receipt chain append is associative " +
      "and produces monotonically increasing MMR roots; compliance tensor evaluation is idempotent " +
      "(evaluating the same entity against the same jurisdiction twice yields identical scores); " +
      "canonical serialization is deterministic (identical structs always produce identical SHA-256 digests); " +
      "and corridor state machine transitions are total (every state has a defined response to every event, " +
      "even if that response is rejection)."
    ),

    h3("B.1.4 Contract Tests"),
    p(
      "Contract tests validate the msez-mass-client against the live Mass API Swagger/OpenAPI " +
      "specifications. They verify that request serialization matches the expected schema, that " +
      "response deserialization handles all documented response shapes, and that error codes map " +
      "correctly to the SEZ Stack error hierarchy. Contract tests use wiremock-based HTTP fixtures " +
      "for offline execution and are validated against live endpoints in the CI staging environment. " +
      "P0-008 has been resolved with 2,015 lines of wiremock contract tests across 6 test files."
    ),
    spacer(),

    // --- Example Test Structure ---
    h2("B.2 Example Test Structure"),
    p(
      "The following example illustrates the standard pattern for a Rust unit test within the " +
      "SEZ Stack. Tests follow a consistent arrange-act-assert structure with descriptive names " +
      "that encode the scenario under test."
    ),
    ...codeBlock(
`#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compliance_tensor_evaluates_all_20_domains() {
        // Arrange: construct entity and jurisdiction context
        let entity = test_entity("ACME-PAK-001");
        let jurisdiction = Jurisdiction::PAK;
        let tensor = ComplianceTensorV2::new();

        // Act: evaluate compliance across all domains
        let result = tensor.evaluate(&entity, &jurisdiction);

        // Assert: all 20 ComplianceDomain variants are present
        assert_eq!(result.scores.len(), 20);
        for domain in ComplianceDomain::all_variants() {
            assert!(
                result.scores.contains_key(&domain),
                "Missing domain: {:?}", domain
            );
            let score = &result.scores[&domain];
            assert!(score.value >= 0.0 && score.value <= 1.0);
        }
    }

    #[test]
    fn receipt_chain_append_produces_valid_mmr_root() {
        // Arrange: create chain with genesis receipt
        let mut chain = ReceiptChain::genesis(test_receipt());

        // Act: append a new receipt
        let receipt = test_receipt_with_payload("transfer-001");
        chain.append(receipt.clone()).expect("append should succeed");

        // Assert: MMR root incorporates new receipt
        assert_eq!(chain.len(), 2);
        assert_ne!(chain.root(), ReceiptChain::genesis(test_receipt()).root());
        assert!(chain.verify_inclusion(&receipt).is_ok());
    }
}`
    ),
    spacer(),

    // --- Coverage Summary Table ---
    h2("B.3 Coverage by Category"),
    table(
      ["Crate / Category", "Test Count", "Source"],
      [
        ["msez-core (foundation types, digest, domains)", "161", "msez-core/src/ #[cfg(test)]"],
        ["msez-crypto (Ed25519, MMR, CAS)", "173", "msez-crypto/src/ #[cfg(test)]"],
        ["msez-vc (verifiable credentials)", "74", "msez-vc/src/ #[cfg(test)]"],
        ["msez-tensor (compliance tensor V2)", "109", "msez-tensor/src/ #[cfg(test)]"],
        ["msez-pack (pack trilogy, composition, licensepacks)", "314", "msez-pack/src/ #[cfg(test)]"],
        ["msez-corridor (corridor lifecycle, receipt chain)", "109", "msez-corridor/src/ #[cfg(test)]"],
        ["msez-state (FSM, migration saga)", "186", "msez-state/src/ #[cfg(test)]"],
        ["msez-agentic (triggers, tax pipeline)", "143", "msez-agentic/src/ #[cfg(test)]"],
        ["msez-arbitration (disputes, institutions)", "160", "msez-arbitration/src/ #[cfg(test)]"],
        ["msez-schema (JSON schema validation)", "72", "msez-schema/src/ #[cfg(test)]"],
        ["msez-zkp (ZK circuits, proofs)", "96", "msez-zkp/src/ #[cfg(test)]"],
        ["msez-compliance (orchestration)", "10", "msez-compliance/src/ #[cfg(test)]"],
        ["msez-mass-client (Mass API contract tests)", "27", "msez-mass-client/src/ #[cfg(test)]"],
        ["msez-api (HTTP handlers, routes, middleware)", "200", "msez-api/src/ #[cfg(test)]"],
        ["msez-cli (CLI commands)", "176", "msez-cli/src/ #[cfg(test)]"],
        ["msez-integration-tests (cross-crate E2E)", "1,313", "msez-integration-tests/tests/"],
        ["Total", "3,323+", "cargo test --workspace"],
      ],
      [4800, 1200, 3360]
    ),
    spacer(),
  ];
};
