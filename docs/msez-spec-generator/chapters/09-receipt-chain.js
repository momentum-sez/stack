const {
  chapterHeading, h2, h3,
  p, p_runs, bold, definition, theorem,
  codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter09() {
  return [
    chapterHeading("Chapter 9: Receipt Chain Architecture"),
    p("Receipt chains provide the cryptographic backbone for Smart Asset state management."),

    // --- 9.1 Receipt Structure ---
    h2("9.1 Receipt Structure"),
    ...codeBlock(
      "/// A single state transition receipt in the chain.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct Receipt {\n" +
      "    pub asset_id: Digest,\n" +
      "    pub sequence: u64,\n" +
      "    pub prev_digest: Digest,\n" +
      "    pub transition: TransitionPayload,\n" +
      "    pub state_commitment: Digest,\n" +
      "    pub tensor_commitment: Digest,\n" +
      "    pub timestamp: DateTime<Utc>,\n" +
      "    pub signatures: Vec<Signature>,\n" +
      "    pub watcher_attestations: Vec<WatcherAttestation>,\n" +
      "}"
    ),

    // --- 9.2 MMR Checkpoints ---
    h2("9.2 MMR Checkpoints"),
    definition("Definition 9.1 (MMR Checkpoint).", "An MMR checkpoint contains: asset_id and checkpoint_seq for identity; receipt_range indicating the covered receipts; mmr_root and state_commitment for cryptographic binding; watcher_attestations for validation; and optional l1_anchor for settlement layer integration."),
    p("MMR checkpoints serve as periodic summaries of receipt chain state. They enable efficient verification without replaying the entire chain, support pruning of historical receipts, and provide natural integration points with the agentic framework. Watchers attest to checkpoint validity, and their attestations are accumulated into the checkpoint record. The agentic trigger system monitors checkpoint creation to initiate settlement layer anchoring when configured."),

    // --- 9.2.1 Fork Resolution ---
    h3("9.2.1 Fork Resolution"),
    p("Fork detection occurs when two receipts reference the same prev_digest with different transitions. The receipt chain architecture treats forks as exceptional but recoverable events. Resolution follows a deterministic protocol: the fork is detected by watchers or participants, competing branches are evaluated against compliance predicates and timestamp ordering, and the canonical branch is selected through a combination of watcher consensus and corridor-level arbitration. Non-canonical receipts are preserved in an evidence package for audit purposes."),
    theorem("Theorem 9.1 (Object Survivability).", "A Smart Asset with a valid receipt chain maintains full operational capability without connectivity to any external system, including the MASS L1 settlement layer. Proof: The receipt chain provides total ordering, the Compliance Tensor carries compliance state, and the state machine specification enables deterministic execution. No external oracle is required for continued operation."),

    // --- 9.3 Receipt Verification Process ---
    h2("9.3 Receipt Verification Process"),
    p("A verifier checking a receipt chain performs the following steps in order. First, the verifier obtains the chain head digest and the set of receipts to verify, either the full chain or a suffix anchored to a trusted MMR checkpoint."),
    p_runs([
      bold("Step 1 — Structural Integrity. "),
      "For each receipt r[i] in the chain, verify that r[i].prev_digest equals the SHA-256 digest of the canonical serialization of r[i-1]. For the genesis receipt (sequence 0), prev_digest must equal the well-known zero digest. This confirms the chain is linked and no receipt has been inserted, removed, or reordered."
    ]),
    p_runs([
      bold("Step 2 — Sequence Monotonicity. "),
      "Verify that r[i].sequence equals r[i-1].sequence + 1 for every consecutive pair. Gaps or duplicates indicate tampering or data loss and must cause verification to fail immediately."
    ]),
    p_runs([
      bold("Step 3 — Signature Validation. "),
      "For each receipt, verify every entry in the signatures vector against the authorized signers for the asset. At least one signature from the asset owner and one from a corridor participant must be present. Ed25519 signature verification uses the public keys bound to the asset at the corresponding sequence number."
    ]),
    p_runs([
      bold("Step 4 — Watcher Attestation Quorum. "),
      "Verify that each receipt carries watcher attestations meeting the quorum threshold defined by the corridor configuration. Each attestation must include the watcher's bonded identity, a signature over the receipt digest, and a timestamp within the acceptable clock skew window. Attestations from slashed or unbonded watchers are rejected."
    ]),
    p_runs([
      bold("Step 5 — State Commitment Consistency. "),
      "Recompute the state_commitment by applying the transition payload to the state at sequence i-1 and hashing the resulting state. Verify the recomputed commitment matches r[i].state_commitment. This confirms that the declared state transition was applied correctly."
    ]),
    p_runs([
      bold("Step 6 — Tensor Commitment Validation. "),
      "Verify that r[i].tensor_commitment matches the SHA-256 digest of the compliance tensor state after evaluating the transition against all applicable compliance domains. This binds every state transition to a specific compliance snapshot, ensuring no transition occurred outside the compliance envelope."
    ]),

    // --- 9.3.1 Receipt Chain Operations ---
    h3("9.3.1 Receipt Chain Operations"),
    p("The receipt chain supports five core operations. Each operation has defined preconditions, effects, and failure modes."),
    table(
      ["Operation", "Description", "Preconditions", "Output"],
      [
        [
          "Append",
          "Adds a new receipt to the chain head. Computes prev_digest from the current head, increments sequence, collects signatures, and writes the receipt to the chain store.",
          "Valid transition payload; owner signature; tensor evaluation passes; no pending fork",
          "New chain head receipt with updated sequence and digests"
        ],
        [
          "Verify",
          "Validates a contiguous range of receipts against the six-step verification process (Section 9.4). Can verify the full chain or a suffix from a trusted checkpoint.",
          "Chain segment to verify; trusted anchor (genesis zero digest or MMR checkpoint root)",
          "Boolean validity result plus first failing receipt index and error category on failure"
        ],
        [
          "Checkpoint",
          "Creates an MMR checkpoint summarizing a range of receipts. Computes the MMR root over the receipt range, collects watcher attestations, and optionally anchors to the settlement layer.",
          "At least one new receipt since last checkpoint; watcher quorum available",
          "MMR checkpoint record with root, attestations, and optional L1 anchor transaction ID"
        ],
        [
          "Prune",
          "Removes historical receipts that are fully covered by a verified MMR checkpoint. Retains the checkpoint and all receipts after it. Pruned data is archived to cold storage before deletion.",
          "Valid MMR checkpoint covering the prune range; cold storage archive confirmed",
          "Reduced chain storage; archive reference for pruned receipts"
        ],
        [
          "Fork-Detect",
          "Identifies forks by scanning for multiple receipts sharing the same prev_digest. Reports competing branches with their respective watcher attestation counts and compliance evaluation results.",
          "Access to the full unpruned chain segment or watcher gossip network",
          "Fork report listing branch heads, attestation counts, compliance status, and recommended canonical branch"
        ],
      ],
      [1400, 2800, 2800, 2360]
    ),
  ];
};
