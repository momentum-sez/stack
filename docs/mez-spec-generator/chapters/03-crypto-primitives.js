const {
  partHeading, chapterHeading, h2, h3, p, p_runs, bold,
  definition, table, codeBlock, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter03() {
  return [
    // --- PART II: CRYPTOGRAPHIC PRIMITIVES ---
    ...partHeading("PART II: CRYPTOGRAPHIC PRIMITIVES"),

    chapterHeading("Chapter 3: Cryptographic Primitives"),

    p("The EZ Stack builds on a carefully selected set of cryptographic primitives chosen for their security properties, performance characteristics, and audit transparency. This chapter specifies each primitive, its implementation status, and its role within the Stack. Primitives are divided into two tiers: production primitives (fully implemented and tested) and Phase 4 primitives (type-level stubs behind feature flags, planned for the ZK activation phase)."),

    // --- 3.1 Hash Functions ---
    h2("3.1 Hash Functions"),

    p("The Stack uses SHA-256 as its sole production hash function. A second hash family, Poseidon2, is specified for future ZK-circuit-internal hashing and is present as a feature-gated stub."),

    p_runs([
      bold("SHA-256 (Production). "),
      "Used for canonical digests, content-addressed storage keys, Merkle Mountain Range nodes, receipt chain commitments, and all integrity verification. The implementation uses the sha2 crate (RustCrypto), which provides constant-time operations. All SHA-256 computation flows through the CanonicalBytes type to ensure consistent serialization before hashing. Direct use of sha2::Sha256 outside mez-core is prohibited by CI lint."
    ]),

    p_runs([
      bold("Poseidon2 (Phase 4 Stub). "),
      "Specified for ZK-circuit-internal hashing: nullifier derivation, commitment computation, and Merkle proof verification inside arithmetic circuits. The mez-crypto crate contains a poseidon module behind the poseidon2 feature flag. All public functions have correct type signatures but return Err(CryptoError::NotImplemented) at runtime. This allows downstream code to compile-check against Poseidon2 types without a concrete implementation. Phase 4 will provide a real implementation via an external crate."
    ]),

    // --- 3.2 The Canonical Digest: Momentum Canonical Form ---
    h2("3.2 The Canonical Digest: Momentum Canonical Form (MCF)"),

    p("The canonical digest is the fundamental identity mechanism for all data in the EZ Stack. Every entity, credential, compliance evaluation, corridor receipt, and pack version is identified by its canonical SHA-256 digest. The serialization pipeline that produces canonical bytes is called Momentum Canonical Form (MCF)."),

    definition(
      "Definition 3.1 (Momentum Canonical Form).",
      "MCF is based on RFC 8785 JSON Canonicalization Scheme (JCS) with two additional safety coercions: (1) Reject any JSON Number that is f64-only (non-integer, NaN, Inf); (2) Normalize RFC 3339 datetime strings to UTC, truncated to seconds, with Z suffix. The canonical digest is then SHA-256(MCF(payload))."
    ),

    p("These coercions are intentional deviations from pure RFC 8785 JCS. Float rejection prevents non-deterministic decimal representations across languages. Datetime normalization prevents equivalent timestamps (e.g., +00:00 vs Z, subsecond precision differences) from producing different digests. Any cross-language implementation (TypeScript, Python, Go) must replicate these exact coercions to produce matching digests."),

    ...codeBlock(
`/// CanonicalBytes — the sole construction path for digest input (mez-core)
///
/// The inner Vec<u8> is private. The only way to construct CanonicalBytes
/// is through ::new(), which applies the full MCF pipeline. This makes
/// the "wrong serialization path" class of defects structurally impossible.
pub struct CanonicalBytes(Vec<u8>);

impl CanonicalBytes {
    /// Apply MCF: serialize to serde_json::Value, reject floats,
    /// normalize datetimes, then serialize with RFC 8785 JCS rules
    /// (sorted keys, no whitespace, ES6 number serialization).
    pub fn new(value: &impl Serialize) -> Result<Self, CanonicalizationError>;
}

/// ContentDigest — SHA-256 of CanonicalBytes (mez-core)
pub struct ContentDigest(pub [u8; 32]);

/// Compute SHA-256(MCF(value)) in one step.
pub fn sha256_digest(value: &impl Serialize) -> Result<ContentDigest, CanonicalizationError>;`
    ),

    p("The serde_json::Map type uses BTreeMap internally, which iterates keys in lexicographic order. serde_json::to_vec preserves this order and produces compact JSON with no whitespace. A CI guard prevents the serde_json preserve_order feature from being enabled anywhere in the workspace, as it would switch the internal map to IndexMap and break digest determinism."),

    // --- 3.3 Digital Signatures ---
    h2("3.3 Digital Signatures: Ed25519"),

    p("Ed25519 is the sole production signature algorithm. It is used for Verifiable Credential proofs, corridor watcher attestations, artifact signing, and key-pair generation via the CLI. The implementation uses the ed25519-dalek crate with serde and zeroize features enabled."),

    ...codeBlock(
`/// Ed25519 key pair with automatic zeroization (mez-crypto)
pub struct SigningKey {
    inner: ed25519_dalek::SigningKey,  // Zeroize on drop
}

/// Ed25519 public key for verification (mez-crypto)
pub struct VerifyingKey {
    inner: ed25519_dalek::VerifyingKey,
}

/// Ed25519 signature (mez-crypto)
pub struct Ed25519Signature {
    inner: ed25519_dalek::Signature,
}`
    ),

    p("Key zeroization is enforced through the zeroize crate. SigningKey wraps ed25519-dalek's SigningKey, which implements ZeroizeOnDrop. When a SigningKey goes out of scope, the secret key material is overwritten with zeros before deallocation. This prevents key material from lingering in freed memory."),

    // --- 3.4 Merkle Mountain Range ---
    h2("3.4 Merkle Mountain Range (MMR)"),

    p("The Merkle Mountain Range is an append-only authenticated data structure used for corridor receipt chains. It enables O(log n) inclusion proofs without requiring the verifier to hold the complete receipt set. The MMR is the secondary commitment in the dual-commitment receipt chain model (the primary commitment is the hash-chain via prev_root/next_root linkage)."),

    definition(
      "Definition 3.2 (MMR Inclusion Proof).",
      "For an element at position i in an MMR with root R, the inclusion proof is a sequence of sibling hashes along the path from the leaf to the peak, plus the peak bagging proof. Verification recomputes the root from the leaf and sibling hashes and checks equality with R."
    ),

    ...codeBlock(
`/// Merkle Mountain Range (mez-crypto)
pub struct MerkleMountainRange {
    nodes: Vec<String>,     // hex-encoded SHA-256 digests
    leaf_count: usize,
}

impl MerkleMountainRange {
    pub fn new() -> Self;
    pub fn append(&mut self, leaf: &str);           // Append leaf digest
    pub fn root(&self) -> Option<String>;            // Current MMR root
    pub fn leaf_count(&self) -> usize;
}

/// Build an inclusion proof for a leaf at the given index.
pub fn build_inclusion_proof(mmr: &MerkleMountainRange, index: usize)
    -> Option<MmrInclusionProof>;

/// Verify an inclusion proof against a known root.
pub fn verify_inclusion_proof(proof: &MmrInclusionProof, root: &str) -> bool;`
    ),

    // --- 3.5 Content-Addressed Storage ---
    h2("3.5 Content-Addressed Storage (CAS)"),

    p("The CAS system stores artifacts (schemas, packs, bundles, lockfiles) keyed by their content digest. This provides built-in integrity verification: retrieving an artifact by its digest and re-hashing the content must produce the same digest. The CAS implementation in mez-crypto provides store, resolve, and verify operations."),

    // --- 3.6 Phase 4 Cryptographic Primitives ---
    h2("3.6 Phase 4 Cryptographic Primitives (Planned)"),

    p("The following primitives are specified in the architecture but not yet implemented. They exist as feature-gated stub modules that provide type signatures for compile-time checking. All stub functions return CryptoError::NotImplemented at runtime. Production builds must not enable these feature flags until real implementations are integrated."),

    table(
      ["Primitive", "Feature Flag", "Module", "Status", "Purpose"],
      [
        ["Poseidon2", "poseidon2", "mez-crypto::poseidon", "Stub", "ZK-friendly hashing for circuit-internal operations"],
        ["BBS+", "bbs-plus", "mez-crypto::bbs", "Stub", "Selective disclosure of credential attributes"],
      ],
      [1800, 1400, 2400, 800, 2960]
    ),

    p("When Phase 4 activates, additional primitives will be required: Pedersen commitments for value hiding in ZK circuits, KZG polynomial commitments for batch proof aggregation, and a nullifier system for double-spend prevention. These are not yet present in the codebase even as stubs."),

    h3("3.6.1 BBS+ Selective Disclosure (Planned)"),

    p("BBS+ signatures will enable selective disclosure of credential attributes. A credential signed with BBS+ can be presented with only a subset of its attributes revealed, while the verifier confirms the hidden attributes were signed by the issuer. For example, a KYC credential containing name, date of birth, nationality, address, and compliance tier could be presented to a corridor counterparty revealing only the compliance tier and nationality."),

    h3("3.6.2 Nullifier System (Planned)"),

    p("Nullifiers will prevent double-spending of credentials and double-evaluation of compliance attestations. Each credential will have a unique nullifier derived from a secret known only to the holder, computed as N = Poseidon2(sk, leaf_index). The nullifier is deterministic but unlinkable: knowing N does not reveal the secret or leaf index without breaking Poseidon2 preimage resistance."),

    // --- 3.7 Zero-Knowledge Proof Architecture ---
    h2("3.7 Zero-Knowledge Proof Architecture"),

    p("The mez-zkp crate defines a sealed ProofSystem trait with pluggable backends. The architecture supports multiple proof systems, selected per use case. Currently, the mock backend is the default. Groth16 and Plonk backends are available behind feature flags but have no activated circuits in Phase 1. The mock backend is guarded by a production policy module that rejects mock proofs when the deployment is configured for production."),

    table(
      ["Backend", "Status", "Use Case", "Trust Setup"],
      [
        ["Mock (SHA-256 deterministic)", "Testing only", "Development, CI, property testing", "None (not cryptographic)"],
        ["Groth16", "Feature-gated", "On-chain verification, compact proofs", "Trusted (per-circuit)"],
        ["Plonk", "Feature-gated", "General-purpose ZK proofs", "Universal (updateable)"],
      ],
      [2800, 1800, 3000, 1760]
    ),

    p("The crate defines five circuit modules covering the core governance operations that will require zero-knowledge proofs when the ZK layer activates."),

    table(
      ["Circuit Module", "File", "Purpose"],
      [
        ["Compliance", "circuits/compliance.rs", "Prove compliance evaluation correctness without revealing entity data"],
        ["Identity", "circuits/identity.rs", "Prove identity claims without revealing personal information"],
        ["Migration", "circuits/migration.rs", "Prove valid asset migration between jurisdictions"],
        ["Settlement", "circuits/settlement.rs", "Prove correct settlement computation for corridor transactions"],
      ],
      [2400, 2800, 4160]
    ),

    p_runs([
      bold("Production Policy Enforcement. "),
      "The mez-zkp::policy module enforces that production deployments cannot accept mock proofs. The policy is configured via a signed, content-addressed policy artifact. CI gates verify that release builds do not enable mock features. This prevents the catastrophic scenario where mock proofs are accepted as authoritative in a live deployment."
    ]),
  ];
};
