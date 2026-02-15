const { chapterHeading, table, spacer, h2, p } = require("../lib/primitives");

module.exports = function build_appendixJ() {
  return [
    chapterHeading("Appendix J: Conformance Levels"),
    table(
      ["Level", "Category", "Requirements"],
      [
        ["1", "Schema Conformance", "JSON Schema validation, Akoma Ntoso, W3C VC data model"],
        ["2", "Behavioral Conformance", "Module dependency resolution, deterministic outputs"],
        ["3", "Cryptographic Conformance", "Signature verification, ZK soundness, correct hashes"],
        ["4", "Corridor Integrity", "Definition VC binding, agreement binding, fork detection"],
        ["5", "Migration Integrity", "State machine transitions, compensation execution"],
      ],
      [800, 2600, 5960]
    ),
    spacer(),

    h2("J.1 Level 1: Schema Conformance \u2014 Test Criteria"),
    p("Schema conformance ensures all data structures exchanged within and between crates match their canonical JSON Schema definitions."),
    spacer(),
    table(
      ["Test ID", "Criterion", "Pass Condition"],
      [
        ["L1-01", "All 116 JSON Schemas load without error", "msez-schema loads every schema from the registry; zero parse failures"],
        ["L1-02", "Round-trip serialization for every domain type", "For each type T: deserialize(serialize(t)) == t for all test vectors"],
        ["L1-03", "Akoma Ntoso lawpack documents validate", "Every lawpack XML document validates against the Akoma Ntoso 3.0 schema"],
        ["L1-04", "W3C VC data model compliance", "All issued VCs validate against the W3C Verifiable Credentials Data Model v2.0 JSON Schema"],
        ["L1-05", "Reject malformed inputs", "Each schema rejects at least 5 known-bad inputs (missing required fields, wrong types, extra fields when additionalProperties is false)"],
        ["L1-06", "Schema version compatibility", "Schemas with version bumps maintain backward compatibility for all fields marked as stable"],
      ],
      [800, 3600, 4960]
    ),
    spacer(),

    h2("J.2 Level 2: Behavioral Conformance \u2014 Test Criteria"),
    p("Behavioral conformance ensures deterministic outputs and correct module composition under all valid input combinations."),
    spacer(),
    table(
      ["Test ID", "Criterion", "Pass Condition"],
      [
        ["L2-01", "Module dependency resolution is acyclic", "Topological sort of module graph completes without cycle detection for all jurisdiction configurations"],
        ["L2-02", "Deterministic compliance evaluation", "Same entity + jurisdiction + pack versions produces identical tensor output across 100 repeated evaluations"],
        ["L2-03", "Pack composition produces stable output", "Composing lawpack + regpack + licensepack yields the same composite for identical inputs regardless of composition order"],
        ["L2-04", "Agentic trigger idempotency", "Firing the same trigger twice with the same state produces identical side effects (no double-counting)"],
        ["L2-05", "State machine transition completeness", "Every valid (state, event) pair has a defined transition; no implicit drops or silent failures"],
        ["L2-06", "Error propagation preserves context", "Every error returned by a public API includes: error code, human message, and causal chain back to the originating crate"],
      ],
      [800, 3600, 4960]
    ),
    spacer(),

    h2("J.3 Level 3: Cryptographic Conformance \u2014 Test Criteria"),
    p("Cryptographic conformance ensures all signature, hash, and proof operations are correct, constant-time where required, and interoperable with external verifiers."),
    spacer(),
    table(
      ["Test ID", "Criterion", "Pass Condition"],
      [
        ["L3-01", "Ed25519 signature round-trip", "Sign(key, message) produces a signature that Verify(pubkey, message, signature) accepts for all test vectors from RFC 8032"],
        ["L3-02", "CanonicalBytes digest stability", "SHA-256 digests for all cross-language test vectors match hardcoded expected values (formerly verified against Python; now Rust-only)"],
        ["L3-03", "MMR append and verify", "Appending N leaves to an MMR and verifying each inclusion proof succeeds; verifying a proof against a modified leaf fails"],
        ["L3-04", "CAS content-addressability", "CAS.put(data) returns digest D; CAS.get(D) returns data; CAS.get(D') for any D' != D returns None"],
        ["L3-05", "Key zeroization on drop", "After a SigningKey is dropped, the memory region formerly holding the key material contains only zeros (verified via Zeroize trait)"],
        ["L3-06", "Constant-time token comparison", "Bearer token comparison uses subtle::ConstantTimeEq; timing analysis across 10,000 iterations shows no correlation between match length and comparison time"],
        ["L3-07", "BBS+ selective disclosure", "A VC with N claims can produce a proof revealing any subset of claims; the verifier accepts the proof without learning the hidden claims"],
      ],
      [800, 3200, 5360]
    ),
    spacer(),

    h2("J.4 Level 4: Corridor Integrity \u2014 Test Criteria"),
    p("Corridor integrity ensures the trade corridor lifecycle, receipt chain, and cross-border state synchronization are tamper-evident and fork-resistant."),
    spacer(),
    table(
      ["Test ID", "Criterion", "Pass Condition"],
      [
        ["L4-01", "Corridor definition VC binding", "A corridor can only be created when a valid definition VC is provided; the corridor ID is derived from the VC digest"],
        ["L4-02", "Agreement VC binding", "Corridor activation requires a valid bilateral or multilateral agreement VC signed by all counterparty zone authorities"],
        ["L4-03", "Receipt chain append-only", "Attempting to modify or delete any receipt in the chain causes verification failure for all subsequent receipts"],
        ["L4-04", "Fork detection", "When two parties independently append conflicting receipts, the fork is detected within one sync cycle and flagged for resolution"],
        ["L4-05", "Fork resolution", "After fork detection, the resolution protocol selects the canonical branch and produces a merge receipt signed by both parties"],
        ["L4-06", "Netting reconciliation", "After N bilateral transactions, a netting cycle computes the correct net obligation and produces a settlement receipt that balances to zero"],
        ["L4-07", "Cross-corridor isolation", "Operations on corridor A have zero observable effect on the state of corridor B, even when both corridors share a jurisdiction endpoint"],
      ],
      [800, 3200, 5360]
    ),
    spacer(),

    h2("J.5 Level 5: Migration Integrity \u2014 Test Criteria"),
    p("Migration integrity ensures the 8-phase migration saga executes correctly, with compensation for partial failures and watcher attestation for state consistency."),
    spacer(),
    table(
      ["Test ID", "Criterion", "Pass Condition"],
      [
        ["L5-01", "State machine phase ordering", "The 8 migration phases execute in strict order: Prepare, Validate, Snapshot, Transfer, Verify, Activate, Cleanup, Complete"],
        ["L5-02", "Compensation on failure", "If phase N fails, phases N-1 through 1 execute their compensation handlers in reverse order; final state is equivalent to pre-migration"],
        ["L5-03", "Watcher attestation required", "Each phase transition requires at least one watcher attestation before proceeding; transitions without attestation are rejected"],
        ["L5-04", "Watcher bond slashing", "A watcher that submits a false attestation (detected by quorum disagreement) has their bond slashed by the configured penalty amount"],
        ["L5-05", "Snapshot completeness", "The snapshot phase captures the full corridor state, tensor snapshot, and credential registry; the restored state passes a byte-level comparison"],
        ["L5-06", "Idempotent retry", "Re-executing a phase that has already completed returns success without duplicating side effects"],
        ["L5-07", "Migration audit trail", "Every phase transition emits a signed audit event; the complete trail is verifiable as a receipt chain from Prepare to Complete"],
      ],
      [800, 3200, 5360]
    ),
    spacer(),
  ];
};
