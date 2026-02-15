const {
  chapterHeading, h2,
  p,
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
    p("Recursive composition leverages Halo2-style accumulation to avoid the need for a trusted setup at intermediate layers. Each STARK proof attests not only to the correctness of its own computation but also to the validity of all proofs it aggregates. The final Groth16 wrapping step is the only component requiring a structured reference string (SRS), and its circuit is fixed and auditable."),

    // --- 14.3 Client-Side Proving ---
    h2("14.3 Client-Side Proving"),
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
  ];
};
