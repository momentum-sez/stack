const {
  chapterHeading, h2,
  p, p_runs, bold
} = require("../lib/primitives");

module.exports = function build_chapter16() {
  return [
    chapterHeading("Chapter 16: L1 Anchoring Protocol"),

    // --- 16.1 Anchor Types ---
    h2("16.1 Anchor Types"),
    p_runs([bold("Asset Checkpoint Anchor."), " Periodically commits a cryptographic summary of asset state to the settlement layer. Fields include: asset_id (canonical digest of the asset), checkpoint_seq (monotonically increasing sequence number), mmr_root (Merkle Mountain Range root covering all receipts since last checkpoint), state_commitment (hash of current asset state), tensor_snapshot (compliance tensor evaluation at checkpoint time), and watcher_signatures (threshold attestations from the watcher set validating checkpoint correctness)."]),
    p_runs([bold("Corridor State Anchor."), " Commits the bilateral or multilateral corridor state at netting boundaries. Fields include: corridor_id (canonical corridor identifier), epoch (corridor epoch number), net_positions (cryptographic commitment to net position vector across all participants), receipt_chain_roots (MMR roots for each participant's receipt chain), compliance_attestation (ZK proof that all corridor activity satisfies jurisdictional predicates), and settlement_instructions (encrypted settlement directives for the treasury layer)."]),

    // --- 16.2 Anchor Targets ---
    h2("16.2 Anchor Targets"),
    p("The anchoring protocol supports multiple anchor targets to avoid dependence on any single external chain. Each anchor target is defined by a chain_type (e.g., Ethereum, Solana, Bitcoin, or a sovereign L1), a contract_address (the on-chain registry that accepts anchor submissions), and an anchor_method (the verification strategy: direct state proof, optimistic with fraud window, or ZK-verified). Anchor submissions are batched and aggregated so that a single on-chain transaction can anchor thousands of asset checkpoints and corridor states, amortizing gas costs across all participants."),

    // --- 16.3 L1-Optional Design ---
    h2("16.3 L1-Optional Design"),
    p("The MASS architecture is L1-optional by design. In the Pre-L1 phase (current), all settlement guarantees are provided by the receipt chain architecture, watcher attestations, and corridor-level netting. Assets operate with full functionality \u2014 formation, transfer, compliance evaluation, and dispute resolution \u2014 without any blockchain dependency. The SEZ Stack, Mass API primitives, and credential system provide the complete operational substrate."),
    p("In the With-L1 phase, the settlement layer adds cryptographic finality anchoring, cross-chain bridge support, and ZK-proven global state roots. The transition is additive: existing assets and corridors gain stronger settlement guarantees without any change to their operational behavior or data model. This design ensures that sovereign deployments (e.g., Pakistan GovOS) can operate immediately with full capability while the L1 settlement infrastructure matures in parallel."),

    // --- 16.4 Anchor Batching and Amortization ---
    h2("16.4 Anchor Batching and Amortization"),
    p("Anchor submissions are batched so that a single on-chain transaction can anchor thousands of asset checkpoints and corridor states. The batching engine aggregates pending anchors into a Merkle tree, submits the root with a Groth16 proof of correct aggregation, and distributes inclusion proofs to individual anchor requestors. Gas costs are amortized across all participants in the batch. Batching intervals are configurable per deployment: high-frequency corridors (PAK\u2194UAE) may batch every 5 minutes, while lower-frequency deployments batch hourly."),
  ];
};
