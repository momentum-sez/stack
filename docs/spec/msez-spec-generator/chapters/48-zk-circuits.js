const {
  chapterHeading, h2,
  p,
  table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter48() {
  return [
    chapterHeading("Chapter 48: Zero-Knowledge Proof Circuits"),

    // --- 48.1 Circuit Taxonomy ---
    h2("48.1 Circuit Taxonomy"),
    table(
      ["Circuit", "Constraints", "Purpose", "Proof System"],
      [
        ["\u03C0priv (Privacy)", "~34,000", "Private transfer validity", "Plonky3 STARK"],
        ["\u03C0comp (Compliance)", "~18,000", "Compliance predicate satisfaction", "Plonky3 STARK"],
        ["\u03C0kyc (KYC)", "~8,000", "KYC tier verification without identity disclosure", "Plonky3 STARK"],
        ["\u03C0tax (Tax)", "~12,000", "Tax withholding correctness", "Plonky3 STARK"],
        ["\u03C0sanc (Sanctions)", "~6,000", "Non-membership in sanctions set", "Plonky3 STARK"],
        ["\u03C0own (Ownership)", "~15,000", "Cap table operation validity", "Plonky3 STARK"],
        ["\u03C0vest (Vesting)", "~10,000", "Vesting schedule compliance", "Plonky3 STARK"],
        ["\u03C0corr (Corridor)", "~22,000", "Corridor state transition validity", "Plonky3 STARK"],
        ["\u03C0net (Netting)", "~20,000", "Multilateral netting correctness", "Plonky3 STARK"],
        ["\u03C0arb (Arbitration)", "~14,000", "Ruling enforcement validity", "Plonky3 STARK"],
        ["\u03C0agg (Aggregation)", "~50,000", "Recursive proof aggregation", "Plonky3 STARK + Groth16"],
        ["\u03C0bridge (Bridge)", "~30,000", "Cross-chain state proof", "Plonky3 STARK + Groth16"],
      ],
      [1800, 1600, 3200, 2760]
    ),
    spacer(),

    // --- 48.2 Privacy Circuit ---
    h2("48.2 Privacy Circuit (\u03C0priv)"),
    p("\u03C0priv proves private transfer validity without revealing sender, recipient, amount, or asset type. Constraint breakdown:"),
    table(
      ["Sub-circuit", "Constraints", "Function"],
      [
        ["Input record opening", "~4,000", "Verify Pedersen commitment opening for input"],
        ["Output record commitment", "~4,000", "Compute Pedersen commitment for output"],
        ["Balance proof", "~2,000", "Verify sum(inputs) = sum(outputs) + fee"],
        ["Nullifier derivation", "~3,000", "Compute and verify nullifier from spending key"],
        ["Merkle membership", "~8,000", "Verify input record exists in global state tree"],
        ["Signature verification", "~6,000", "Verify Ed25519 spending authorization"],
        ["Range proofs", "~4,000", "Verify all amounts are non-negative"],
        ["Compliance hook", "~3,000", "Optional compliance predicate evaluation"],
      ],
      [2400, 1800, 5160]
    ),
    spacer(),
  ];
};
