const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table,
  spacer
} = require("../lib/primitives");

module.exports = function build_chapter14() {
  return [
    chapterHeading("Chapter 14: Proving System"),

    // --- 14.1 Plonky3 Architecture ---
    h2("14.1 Plonky3 Architecture"),
    p("The MASS L1 proving system is built on Plonky3, configured for optimal performance across server, desktop, and mobile proving environments. The core configuration uses the BabyBear field (p = 2^31 - 1) with Extension Degree 4, providing 128-bit security with efficient arithmetic on 32-bit hardware. The hash function is Poseidon2 (width 16, alpha 7), chosen for its STARK-friendly algebraic structure and competitive performance in recursive proof composition. FRI parameters use a folding factor of 8 with 21 queries, balancing proof size against verification cost."),
    ...codeBlock(
      "/// Plonky3 configuration for the MASS L1 proving system.\n" +
      "#[derive(Debug, Clone)]\n" +
      "pub struct Plonky3Config {\n" +
      "    pub field: Field,          // BabyBear (p = 2^31 - 1)\n" +
      "    pub extension_degree: u8,  // 4\n" +
      "    pub hash: HashConfig,      // Poseidon2 (width=16, alpha=7)\n" +
      "    pub fri_folding: u8,       // 8\n" +
      "    pub fri_queries: u8,       // 21\n" +
      "    pub security_bits: u16,    // 128\n" +
      "}\n" +
      "\n" +
      "impl Plonky3Config {\n" +
      "    pub fn production() -> Self {\n" +
      "        Self {\n" +
      "            field: Field::BabyBear,\n" +
      "            extension_degree: 4,\n" +
      "            hash: HashConfig::poseidon2(16, 7),\n" +
      "            fri_folding: 8,\n" +
      "            fri_queries: 21,\n" +
      "            security_bits: 128,\n" +
      "        }\n" +
      "    }\n" +
      "}"
    ),
    spacer(),

    // --- 14.2 Proof Aggregation ---
    h2("14.2 Proof Aggregation"),
    p("Transaction proofs are aggregated through a four-layer pipeline that progressively reduces the verification burden from millions of individual transaction proofs to a single succinct proof per epoch. Each layer applies recursive STARK composition, with a final wrapping step that produces a Groth16 proof for constant-size on-chain verification."),
    table(
      ["Layer", "Input", "Output", "Aggregation Factor"],
      [
        ["Transaction", "Single transaction", "Transaction proof (STARK)", "1:1"],
        ["Block (Layer 1)", "1K-10K transaction proofs", "Block proof (STARK)", "1000-10000:1"],
        ["Epoch (Layer 2)", "10-100 block proofs", "Epoch proof (STARK)", "10-100:1"],
        ["Final Wrapping", "Single epoch proof", "Groth16 proof (288 bytes)", "1:1"],
      ],
      [1800, 2800, 2800, 1960]
    ),
    spacer(),

    // --- 14.2.1 Recursive Composition ---
    h3("14.2.1 Recursive Composition"),
    p("Recursive composition uses Halo2-style accumulation to avoid the need for a trusted setup at intermediate layers. Each STARK proof attests not only to the correctness of its own computation but also to the validity of all proofs it aggregates. The final Groth16 wrapping step is the only component requiring a structured reference string (SRS), and its circuit is fixed and auditable."),
    p_runs([bold("Accumulation Scheme."), " Rather than fully verifying each inner proof within the recursive circuit (which would impose prohibitive constraint costs), the system uses an accumulation scheme inspired by Halo2's polynomial commitment accumulation. Each recursive step takes an inner proof and an accumulator as input, performs a cheap partial verification that checks structural validity, and outputs an updated accumulator that defers the expensive pairing or FRI check to the final step. This reduces the per-recursion overhead from millions of constraints to approximately 50,000 constraints, enabling deep recursion trees (up to 20 levels) without exponential blowup in prover time."]),
    p_runs([bold("Split Accumulation."), " The accumulation is split into two independent streams: the polynomial commitment accumulator (which defers FRI evaluation) and the permutation accumulator (which defers PLONK-style copy constraint checks). These two streams are carried independently through the recursion tree and merged only at the final wrapping step. This split enables parallelization of the recursive proving process, as subtrees can be proven independently and their accumulators merged in a binary-tree reduction pattern."]),
    p_runs([bold("Final Decider."), " At the root of the recursion tree, the final decider circuit consumes the accumulated state and performs the deferred checks: FRI query verification for the polynomial commitments and copy constraint verification for the permutation argument. The decider output is then wrapped in a Groth16 proof (288 bytes) suitable for on-chain verification on any EVM-compatible chain or the MASS L1 root chain. The Groth16 SRS is generated via a multi-party computation ceremony with at least 64 participants, and the resulting parameters are published and auditable."]),

    // --- 14.2.2 Client-Side Proving ---
    h3("14.2.2 Client-Side Proving"),
    table(
      ["Device Category", "Prove Time", "Notes"],
      [
        ["Modern Smartphone (2023+)", "<10s", "GPU acceleration available"],
        ["Older Smartphone", "<60s", "CPU-only"],
        ["Desktop (GPU)", "<2s", "CUDA/Metal acceleration"],
        ["Server (Multi-GPU)", "<100ms", "Production prover"],
      ],
      [3200, 2000, 4160]
    ),
    spacer(),

    // --- 14.3 Proof Portability ---
    h2("14.3 Proof Portability"),
    p("Proofs generated by the MASS L1 proving system are designed for cross-platform verification. A proof produced by a server-class prover must be verifiable on a mobile device, an embedded system, a browser, or a third-party blockchain's smart contract -- without requiring the verifier to trust the prover's hardware or software environment. Proof portability is essential for the multi-chain anchoring strategy (Chapter 16) and for enabling jurisdictional regulators to independently verify compliance attestations without running MASS L1 infrastructure."),
    p_runs([bold("Verification Targets."), " The final Groth16 wrapper proof is the portable artifact. Its verification requires only elliptic curve pairing operations (BN254), which are available as precompiled contracts on Ethereum (EIP-196/197), as native instructions on Solana (alt_bn128), and as library functions on virtually every platform. Verification cost is constant regardless of the complexity of the underlying computation: approximately 200K gas on Ethereum, under 1ms on mobile, and under 100 microseconds on server hardware."]),
    p_runs([bold("Platform-Specific Verifiers."), " The MASS L1 ships reference verifier implementations for five target platforms: (1) Solidity smart contract for EVM chains, (2) Rust library for native applications and the MASS L1 itself, (3) WebAssembly module for browser-based verification, (4) Swift/Kotlin wrappers for mobile applications, and (5) a standalone CLI verifier for offline audit scenarios. All five implementations are generated from a single canonical circuit description and produce identical accept/reject decisions for any given proof."]),
    table(
      ["Verification Target", "Format", "Cost / Latency", "Use Case"],
      [
        ["Ethereum (EVM)", "Solidity contract", "~200K gas", "On-chain anchor verification"],
        ["Solana", "BPF program", "~50K compute units", "High-throughput anchor verification"],
        ["Browser (WASM)", "wasm module", "<50ms", "Client-side compliance checks"],
        ["Mobile (iOS/Android)", "Native library", "<1ms", "Wallet proof verification"],
        ["CLI (offline)", "Rust binary", "<100\u00B5s", "Regulatory audit verification"],
      ],
      [2000, 2000, 2200, 3160]
    ),
    spacer(),
  ];
};
