const {
  chapterHeading, h2,
  p, definition, theorem,
  codeBlock, spacer
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
    spacer(),

    // --- 9.2 MMR Checkpoints ---
    h2("9.2 MMR Checkpoints"),
    definition("Definition 9.1 (MMR Checkpoint).", "An MMR checkpoint contains: asset_id and checkpoint_seq for identity; receipt_range indicating the covered receipts; mmr_root and state_commitment for cryptographic binding; watcher_attestations for validation; and optional l1_anchor for settlement layer integration."),
    p("MMR checkpoints serve as periodic summaries of receipt chain state. They enable efficient verification without replaying the entire chain, support pruning of historical receipts, and provide natural integration points with the agentic framework. Watchers attest to checkpoint validity, and their attestations are accumulated into the checkpoint record. The agentic trigger system monitors checkpoint creation to initiate settlement layer anchoring when configured."),

    // --- 9.3 Fork Resolution ---
    h2("9.3 Fork Resolution"),
    p("Fork detection occurs when two receipts reference the same prev_digest with different transitions. The receipt chain architecture treats forks as exceptional but recoverable events. Resolution follows a deterministic protocol: the fork is detected by watchers or participants, competing branches are evaluated against compliance predicates and timestamp ordering, and the canonical branch is selected through a combination of watcher consensus and corridor-level arbitration. Non-canonical receipts are preserved in an evidence package for audit purposes."),
    theorem("Theorem 9.1 (Object Survivability).", "A Smart Asset with a valid receipt chain maintains full operational capability without connectivity to any external system, including the MASS L1 settlement layer. Proof: The receipt chain provides total ordering, the Compliance Tensor carries compliance state, and the state machine specification enables deterministic execution. No external oracle is required for continued operation."),
  ];
};
