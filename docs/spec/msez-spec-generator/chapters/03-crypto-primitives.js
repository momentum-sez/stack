const {
  partHeading, chapterHeading, h2, h3, p, p_runs, bold,
  definition, table, codeBlock, spacer, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter03() {
  return [
    // --- PART II: CRYPTOGRAPHIC PRIMITIVES ---
    ...partHeading("PART II: CRYPTOGRAPHIC PRIMITIVES"),

    chapterHeading("Chapter 3: Cryptographic Primitives"),

    p("The SEZ Stack builds on a carefully selected set of cryptographic primitives chosen for their security properties, performance characteristics, and suitability for zero-knowledge proof systems. This chapter specifies each primitive, its implementation, and its role within the Stack."),

    // --- 3.1 Hash Functions ---
    h2("3.1 Hash Functions"),

    p("The Stack uses two hash function families: SHA-256 for general-purpose hashing and Poseidon2 for ZK-friendly hashing. The choice of hash function is determined by context: SHA-256 for data integrity, content addressing, and Merkle trees in non-ZK contexts; Poseidon2 for all operations that may need to be proven in zero knowledge."),

    p_runs([
      bold("SHA-256. "),
      "Used for canonical digests, content-addressed storage keys, and Merkle Mountain Range nodes. The implementation uses the sha2 crate (RustCrypto), which provides constant-time operations and has been audited by multiple parties. All SHA-256 computation flows through CanonicalBytes::new() to ensure consistent serialization before hashing."
    ]),

    p_runs([
      bold("Poseidon2. "),
      "Used for all ZK-circuit-internal hashing: nullifier derivation, commitment computation, Merkle proof verification inside circuits. Poseidon2 is an algebraic hash function designed for efficient arithmetic circuit representation. The Stack uses the Plonky3 Poseidon2 implementation with the BN254 scalar field."
    ]),

    ...codeBlock(
`/// Poseidon2 hasher for ZK circuits (msez-zkp)
pub struct Poseidon2Hasher {
    params: Poseidon2Params<Fr>,
}

impl Poseidon2Hasher {
    pub fn hash_two(&self, left: Fr, right: Fr) -> Fr {
        poseidon2_hash(&self.params, &[left, right])
    }

    pub fn hash_nullifier(&self, secret: Fr, leaf_index: u64) -> Fr {
        let index_field = Fr::from(leaf_index);
        poseidon2_hash(&self.params, &[secret, index_field])
    }
}`
    ),
    spacer(),

    // --- 3.2 The Canonical Digest Bridge ---
    h2("3.2 The Canonical Digest Bridge"),

    p("The canonical digest is the fundamental identity mechanism for all data in the SEZ Stack. Every entity, credential, compliance evaluation, corridor state, and pack version is identified by its canonical SHA-256 digest. This section specifies the digest computation pipeline."),

    definition(
      "Definition 3.1 (Canonical Digest).",
      "For any data structure D implementing CanonicalBytes, the canonical digest is digest(D) = SHA-256(canonical_bytes(D)), where canonical_bytes produces a deterministic byte sequence independent of serialization format, field ordering, or platform."
    ),

    ...codeBlock(
`/// Digest type enumeration (msez-core)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DigestType {
    Sha256,
    Poseidon2,
    Keccak256, // Ethereum compatibility
}

/// A typed digest with algorithm tag (msez-core)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Digest {
    pub digest_type: DigestType,
    pub bytes: Vec<u8>,
}

/// The canonical digest: SHA-256 of canonical bytes (msez-core)
pub struct CanonicalDigest(pub [u8; 32]);

impl CanonicalDigest {
    pub fn new(data: &impl CanonicalBytes) -> Self {
        use sha2::{Sha256, Digest as _};
        let bytes = data.canonical_bytes();
        let hash = Sha256::digest(&bytes);
        Self(hash.into())
    }
}`
    ),
    spacer(),

    // --- 3.3 Commitment Schemes ---
    h2("3.3 Commitment Schemes"),

    p("The Stack uses two commitment schemes: Pedersen commitments for value hiding in ZK circuits, and KZG polynomial commitments for batch proof aggregation."),

    p_runs([
      bold("Pedersen Commitments. "),
      "Used to commit to values (account balances, compliance scores, entity attributes) within ZK circuits. A Pedersen commitment C = vG + rH hides value v with randomness r, where G and H are independent generators on the BN254 curve."
    ]),

    p_runs([
      bold("KZG Polynomial Commitments. "),
      "Used for batch verification of compliance evaluations. A single KZG commitment can attest to the correctness of an entire compliance tensor evaluation (20 domains across multiple jurisdictions), with individual domain scores verifiable through polynomial evaluation proofs."
    ]),

    ...codeBlock(
`/// Pedersen commitment (msez-zkp)
pub struct PedersenCommitment {
    pub commitment: G1Affine,
    pub blinding: Fr,  // Zeroize on drop
}

/// KZG polynomial commitment (msez-zkp)
pub struct KzgCommitment {
    pub commitment: G1Affine,
    pub degree: usize,
}`
    ),
    spacer(),

    // --- 3.4 Nullifier System ---
    h2("3.4 Nullifier System"),

    p("Nullifiers prevent double-spending of credentials and double-evaluation of compliance attestations. Each credential or attestation has a unique nullifier derived from a secret known only to the holder."),

    definition(
      "Definition 3.2 (Nullifier).",
      "For a credential with secret s and leaf index i in the credential Merkle tree, the nullifier is N = Poseidon2(s, i). The nullifier is deterministic (the same credential always produces the same nullifier) but unlinkable (knowing N does not reveal s or i without breaking Poseidon2 preimage resistance)."
    ),

    p("When a credential is presented for compliance verification, its nullifier is published. A nullifier set maintained by each jurisdiction tracks spent nullifiers. Presenting a credential whose nullifier is already in the set is rejected, preventing reuse of revoked or expired credentials."),

    // --- 3.5 BBS+ Signatures for Credentials ---
    h2("3.5 BBS+ Signatures for Credentials"),

    p("BBS+ signatures enable selective disclosure of credential attributes. A credential signed with BBS+ can be presented with only a subset of its attributes revealed, while the verifier can still confirm that the hidden attributes were signed by the issuer. This is critical for privacy-preserving compliance verification."),

    p("For example, a KYC credential may contain name, date of birth, nationality, address, and compliance tier. When presenting to a corridor counterparty, the holder can reveal only the compliance tier and nationality, proving they meet the corridor\u2019s requirements without exposing personal information."),

    ...codeBlock(
`/// BBS+ key pair for credential issuance (msez-vc)
pub struct BbsKeyPair {
    pub public_key: BbsPublicKey,
    pub secret_key: BbsSecretKey,  // Zeroize on drop
    pub message_count: usize,       // Number of attributes this key can sign
}

/// BBS+ signed credential (msez-vc)
pub struct BbsCredential {
    pub attributes: Vec<Fr>,         // Credential attributes as field elements
    pub signature: BbsSignature,     // BBS+ signature over all attributes
    pub issuer_pk: BbsPublicKey,     // Issuer's public key
    pub schema_digest: CanonicalDigest, // Schema identifying attribute semantics
}`
    ),
    spacer(),

    // --- 3.6 Zero-Knowledge Proof Systems ---
    h2("3.6 Zero-Knowledge Proof Systems"),

    p("The Stack supports multiple zero-knowledge proof systems, each chosen for specific use cases based on proof size, verification time, prover time, and trust assumptions."),

    table(
      ["System", "Use Case", "Proof Size", "Verification", "Trust Setup"],
      [
        ["Plonky3", "Settlement proofs, corridor state", "~45 KB", "~3 ms", "Transparent (FRI)"],
        ["Groth16", "On-chain verification (Ethereum bridge)", "128 bytes", "~1 ms", "Trusted (per-circuit)"],
        ["BBS+ Proofs", "Credential selective disclosure", "~400 bytes", "~2 ms", "None"],
        ["Bulletproofs", "Range proofs (compliance scores)", "~700 bytes", "~5 ms", "Transparent"],
        ["STARK", "Batch compliance evaluation", "~100 KB", "~10 ms", "Transparent"],
      ],
      [1800, 2800, 1400, 1560, 1800]
    ),
    spacer(),

    p("The Stack defines twelve circuit types that compose these proof systems for specific governance operations."),

    table(
      ["Circuit", "Proof System", "Purpose"],
      [
        ["ComplianceEvaluation", "Plonky3", "Prove compliance score for entity across 20 domains without revealing entity data"],
        ["CorridorStateTransition", "Plonky3", "Prove valid corridor state transition with receipt chain integrity"],
        ["CredentialPresentation", "BBS+", "Selectively disclose credential attributes to verifier"],
        ["NullifierDerivation", "Plonky3", "Prove nullifier corresponds to valid credential without revealing credential"],
        ["MerkleInclusion", "Plonky3", "Prove entity/credential membership in jurisdiction Merkle tree"],
        ["BalanceRange", "Bulletproofs", "Prove account balance in range without revealing exact amount"],
        ["TaxWithholding", "Plonky3", "Prove correct withholding tax computation without revealing transaction amount"],
        ["SanctionsScreening", "Plonky3", "Prove entity not on sanctions list without revealing entity identity"],
        ["OwnershipThreshold", "Plonky3", "Prove ownership percentage above/below threshold for UBO determination"],
        ["CorridorNetting", "Plonky3", "Prove correct bilateral netting computation across corridor transactions"],
        ["MigrationProof", "Plonky3", "Prove valid asset migration from source to destination jurisdiction"],
        ["WatcherAttestation", "Plonky3", "Prove watcher attestation validity and bond sufficiency"],
      ],
      [2600, 1600, 5160]
    ),
    spacer(),
  ];
};
