const {
  chapterHeading, h2, h3,
  p, p_runs, bold, definition,
  codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter09() {
  return [
    chapterHeading("Chapter 9: Receipt Chain Architecture"),
    p("Receipt chains provide the cryptographic backbone for corridor state management. Every cross-border transaction event is recorded as a receipt in an append-only chain, producing an auditable, tamper-evident log that any party can independently verify."),

    // --- 9.1 Dual-Commitment Model ---
    h2("9.1 Dual-Commitment Model"),
    p("The corridor receipt chain uses a dual-commitment model that provides two independent integrity guarantees:"),

    p_runs([
      bold("Hash-chain commitment (final_state_root). "),
      "A sequential hash chain seeded from the corridor\u2019s genesis_root. Each receipt\u2019s prev_root must equal the current final_state_root, and each receipt\u2019s next_root becomes the new chain head. This provides linear ordering and tamper detection: modifying any receipt breaks the chain linkage."
    ]),

    p_runs([
      bold("MMR commitment (inclusion proofs). "),
      "Each receipt\u2019s next_root digest is appended to a Merkle Mountain Range. This provides O(log n) inclusion proofs without disclosing the full receipt set, enabling efficient verification by external auditors and corridor counterparties."
    ]),

    definition(
      "Definition 9.1 (Receipt next_root Derivation).",
      "next_root = SHA-256(MCF(receipt_without_proof_and_next_root)), where MCF is Momentum Canonical Form (RFC 8785 JCS + float rejection + datetime normalization). Digest sets within the receipt are normalized (deduplicated + sorted lexicographically) before canonicalization."
    ),

    // --- 9.2 Receipt Structure ---
    h2("9.2 Receipt Structure"),
    p("The CorridorReceipt struct conforms to schemas/corridor.receipt.schema.json. It captures the corridor identifier, sequence number, hash-chain linkage, governing regulatory digests, cryptographic proof, and optional transition metadata."),

    ...codeBlock(
`/// A corridor receipt (msez-corridor::receipt)
/// Conforms to schemas/corridor.receipt.schema.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorReceipt {
    pub receipt_type: String,           // Receipt type discriminator
    pub corridor_id: CorridorId,        // The corridor this receipt belongs to
    pub sequence: u64,                  // Sequence number (0-indexed)
    pub timestamp: Timestamp,           // RFC 3339 creation time
    pub prev_root: String,              // State root before this transition
    pub next_root: String,              // SHA256(MCF(payload_sans_proof_and_next_root))
    pub lawpack_digest_set: Vec<String>,  // Governing lawpack digests (sorted, deduped)
    pub ruleset_digest_set: Vec<String>,  // Governing ruleset digests (sorted, deduped)
    pub proof: Option<ReceiptProof>,    // Ed25519 proof(s) over the payload
    pub transition: Option<Value>,      // Transition envelope (optional)
    pub transition_type_registry_digest_sha256: Option<String>,
    pub zk: Option<Value>,             // ZK proof scaffold (optional)
}`
    ),

    p("Digest set entries support two forms via the DigestEntry enum: raw SHA-256 hex strings (legacy) and ArtifactRef objects carrying digest, artifact type, and optional retrieval URI. Both forms are normalized to their underlying digest string for canonicalization."),

    // --- 9.2.1 Proof Object ---
    h3("9.2.1 Proof Object"),
    p("The proof field carries one or more Ed25519 signature objects conforming to the MsezEd25519Signature2025 proof type. Multi-party signing is supported via the ReceiptProof::Multiple variant."),

    ...codeBlock(
`/// Proof: single object or array (msez-corridor::receipt)
#[serde(untagged)]
pub enum ReceiptProof {
    Single(ProofObject),
    Multiple(Vec<ProofObject>),
}

/// A single cryptographic proof object.
pub struct ProofObject {
    pub proof_type: String,            // "MsezEd25519Signature2025"
    pub created: String,               // RFC 3339 timestamp
    pub verification_method: String,   // DID or key URI
    pub proof_purpose: String,         // "assertionMethod"
    pub jws: String,                   // JWS compact serialization
}`
    ),

    // --- 9.3 Integrity Invariants ---
    h2("9.3 Integrity Invariants"),
    p("Three invariants must hold at all times. Violation of any invariant indicates tampering or implementation error."),
    table(
      ["Invariant", "ID", "Enforcement"],
      [
        ["receipt.prev_root == chain.final_state_root", "I-RECEIPT-LINK", "Runtime check in append(); reject on mismatch"],
        ["receipt.next_root == compute_next_root(&receipt)", "I-RECEIPT-COMMIT", "Recompute and compare in append(); reject on mismatch"],
        ["mmr.root() == MMR(all next_roots)", "I-MMR-ROOT", "MMR maintained by ReceiptChain; verified on checkpoint"],
      ],
      [4000, 1800, 3560]
    ),

    // --- 9.4 Receipt Chain Operations ---
    h2("9.4 Receipt Chain Operations"),
    p("The ReceiptChain struct manages the dual-commitment state and exposes core operations."),

    ...codeBlock(
`/// Append-only receipt chain with dual commitment (msez-corridor::receipt)
pub struct ReceiptChain {
    corridor_id: CorridorId,
    genesis_root: String,          // Seed for the hash chain
    final_state_root: String,      // Current hash-chain head
    sequence: u64,                 // Next expected sequence number
    mmr: MerkleMountainRange,      // MMR over next_root digests
    receipts: Vec<CorridorReceipt>,
}`
    ),

    table(
      ["Operation", "Description", "Preconditions"],
      [
        ["append(receipt)", "Verify next_root and prev_root, advance final_state_root, append next_root to MMR", "Sequence matches; prev_root == final_state_root; next_root recomputation matches"],
        ["verify_inclusion(index)", "Build MMR inclusion proof for the receipt at the given index", "Chain is non-empty; index within bounds"],
        ["checkpoint()", "Create a Checkpoint summarizing the current chain state with all schema-required fields", "Chain is non-empty"],
        ["root()", "Return the current MMR root for external verification", "Chain is non-empty"],
        ["len()", "Return the number of receipts in the chain", "None"],
      ],
      [2400, 4400, 2560]
    ),

    // --- 9.5 next_root Computation ---
    h2("9.5 next_root Computation"),
    p("The compute_next_root function strips the proof and next_root fields from the receipt, normalizes digest sets (deduplicate + sort lexicographically), and computes the canonical digest of the resulting payload."),

    ...codeBlock(
`/// Compute next_root for a receipt (msez-corridor::receipt)
///
/// 1. Serialize receipt to serde_json::Value
/// 2. Remove "proof" and "next_root" fields
/// 3. Normalize digest sets: deduplicate, sort lexicographically
/// 4. Return SHA256(MCF(stripped_payload))
pub fn compute_next_root(receipt: &CorridorReceipt)
    -> Result<String, ReceiptError>;`
    ),

    p("The normalization step ensures that two implementations producing the same digest sets in different orders will compute identical next_root values. This is critical for cross-party verification: both sides of a corridor must agree on the receipt\u2019s canonical identity regardless of the order in which they assembled the digest sets."),

    // --- 9.6 Checkpoints ---
    h2("9.6 Checkpoints"),
    p("Checkpoints are periodic summaries of receipt chain state conforming to schemas/corridor.checkpoint.schema.json. They contain the genesis_root, final_state_root, receipt_count, digest sets, the full MMR commitment (type, algorithm, size, root, peaks), and a cryptographic proof."),

    ...codeBlock(
`/// Schema-conformant checkpoint (msez-corridor::receipt)
pub struct Checkpoint {
    pub corridor_id: CorridorId,
    pub height: u64,                       // Number of receipts covered
    pub genesis_root: String,              // Corridor genesis seed
    pub final_state_root: String,          // Hash-chain head at checkpoint time
    pub receipt_count: u64,                // Total receipts in corridor
    pub lawpack_digest_set: Vec<String>,   // Governing lawpack digests
    pub ruleset_digest_set: Vec<String>,   // Governing ruleset digests
    pub mmr: MmrCommitment,               // Full MMR state (type, algorithm, size, root, peaks)
    pub timestamp: Timestamp,
    pub checkpoint_digest: String,
    pub proof: Option<ReceiptProof>,       // Required for production
}`
    ),

    // --- 9.7 Fork Detection ---
    h2("9.7 Fork Detection"),
    p("Fork detection occurs when two receipts reference the same prev_root with different transitions. The receipt chain treats forks as exceptional but recoverable events. Resolution follows the evidence-driven protocol specified in Chapter 26 (Watcher Architecture): competing branches are evaluated using cryptographically-bound watcher attestations, timestamp ordering with clock skew tolerance, and deterministic lexicographic tiebreaking. Non-canonical receipts are preserved in an evidence package for audit purposes."),

    // --- 9.8 Verification Process ---
    h2("9.8 Verification Process"),
    p("A verifier checking a receipt chain performs the following steps in order."),

    p_runs([
      bold("Step 1 \u2014 Hash-Chain Linkage. "),
      "For each receipt r[i], verify that r[i].prev_root equals the chain\u2019s final_state_root after processing r[i-1]. For the genesis receipt (sequence 0), prev_root must equal the corridor\u2019s genesis_root."
    ]),
    p_runs([
      bold("Step 2 \u2014 next_root Recomputation. "),
      "For each receipt r[i], recompute next_root by stripping proof and next_root fields, normalizing digest sets, and computing SHA-256(MCF(payload)). Verify the recomputed value matches r[i].next_root."
    ]),
    p_runs([
      bold("Step 3 \u2014 Sequence Monotonicity. "),
      "Verify that r[i].sequence equals r[i-1].sequence + 1 for every consecutive pair. Gaps or duplicates indicate tampering."
    ]),
    p_runs([
      bold("Step 4 \u2014 Proof Validation. "),
      "Verify that each receipt carries a valid proof (Ed25519 signature over the payload). Receipts without proofs are rejected in production mode."
    ]),
    p_runs([
      bold("Step 5 \u2014 MMR Consistency. "),
      "Reconstruct the MMR from all next_root digests and verify the resulting root matches the checkpoint\u2019s MMR root commitment."
    ]),
  ];
};
