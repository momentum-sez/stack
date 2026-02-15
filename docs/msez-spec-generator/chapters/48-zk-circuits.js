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
    p("\u03C0priv proves private transfer validity without revealing sender, recipient, amount, or asset type. The circuit decomposes into eight labeled sub-circuits (C1 through C8), each handling a distinct verification concern. Constraint breakdown:"),
    table(
      ["Label", "Sub-circuit", "Constraints", "Function"],
      [
        ["C1", "Input record opening", "~4,000", "Verify Pedersen commitment opening for input record"],
        ["C2", "Output record commitment", "~4,000", "Compute Pedersen commitment for output record"],
        ["C3", "Balance proof", "~2,000", "Verify sum(inputs) = sum(outputs) + fee"],
        ["C4", "Nullifier derivation", "~3,000", "Compute and verify nullifier from spending key"],
        ["C5", "Merkle membership", "~8,000", "Verify input record exists in global state tree"],
        ["C6", "Signature verification", "~6,000", "Verify Ed25519 spending authorization"],
        ["C7", "Range proofs", "~4,000", "Verify all amounts are non-negative and within field bounds"],
        ["C8", "Compliance hook", "~3,000", "Optional compliance predicate evaluation gate"],
      ],
      [800, 2000, 1400, 5160]
    ),
    p("The C1-C8 labeling convention provides a stable reference for audit, composition, and selective circuit inclusion. When composing with other circuits (see Section 48.4), sub-circuits are referenced by label to enable precise constraint accounting and to identify which verification steps are shared across composed proofs."),
    spacer(),

    // --- 48.3 Compliance Circuit ---
    h2("48.3 Compliance Circuit (\u03C0comp)"),
    p("\u03C0comp proves that a given entity satisfies all compliance predicates for a specific (jurisdiction, domain) pair without revealing the underlying compliance evidence. This circuit is the ZK analogue of the compliance tensor evaluation performed by msez-tensor: it takes a private compliance state vector and proves that every required predicate evaluates to true. The circuit decomposes into seven labeled sub-circuits (CC1 through CC7):"),
    table(
      ["Label", "Sub-circuit", "Constraints", "Function"],
      [
        ["CC1", "Entity binding", "~1,500", "Verify entity identifier commitment matches the claimed entity without revealing entity ID"],
        ["CC2", "Jurisdiction selector", "~1,000", "Verify the jurisdiction code is in the set of valid jurisdictions and select the correct predicate vector"],
        ["CC3", "Domain predicate evaluation", "~4,500", "Evaluate all compliance predicates for the selected domain against the private compliance state"],
        ["CC4", "Temporal validity", "~2,000", "Verify all evidence timestamps fall within their validity windows and are not expired"],
        ["CC5", "Sanctions non-membership", "~3,000", "Prove the entity does not appear in any applicable sanctions list using set non-membership via Merkle exclusion proof"],
        ["CC6", "Attestation aggregation", "~3,500", "Aggregate watcher attestations and verify quorum threshold is met for each predicate"],
        ["CC7", "Output commitment", "~2,500", "Produce a Pedersen commitment to the compliance result that can be consumed by downstream circuits"],
      ],
      [800, 2200, 1400, 4960]
    ),
    p("Total constraint count for \u03C0comp is approximately 18,000, consistent with the taxonomy in Section 48.1. CC3 (domain predicate evaluation) is the most variable sub-circuit: its constraint count scales with the number of predicates in the compliance domain, ranging from approximately 2,000 constraints for simple domains to 7,000 for domains with complex multi-condition predicates such as tax compliance. CC5 reuses the same Merkle exclusion proof construction as \u03C0sanc but operates on a subset of the sanctions set scoped to the relevant jurisdiction."),
    spacer(),

    // --- 48.4 Circuit Composition ---
    h2("48.4 Circuit Composition"),
    p("Complex operations in the SEZ Stack require proving multiple properties simultaneously. Rather than generating independent proofs for each property and verifying them separately, the circuit architecture supports composition: combining sub-circuits from different top-level circuits into a single unified proof. This reduces verifier cost, eliminates redundant constraint evaluations, and ensures atomicity (all properties hold for the same witness, not merely for witnesses that happen to share public inputs)."),
    p("Composition operates at three levels. First, intra-circuit sharing: when two top-level circuits share identical sub-circuits, the shared sub-circuit is instantiated once and its output wires are routed to both consumers. For example, a composed \u03C0priv+\u03C0comp proof shares the compliance hook (C8) with the output commitment (CC7), eliminating approximately 3,000 redundant constraints. Second, sequential composition: the output commitment of one circuit feeds as a public input to the next. A corridor state transition (\u03C0corr) that depends on a compliance attestation (\u03C0comp) composes sequentially: \u03C0comp produces a commitment, and \u03C0corr consumes it as a verified input, ensuring the corridor transition is predicated on a valid compliance evaluation. Third, recursive aggregation via \u03C0agg: when many proofs must be verified together (such as all compliance proofs for entities in a netting batch), \u03C0agg recursively verifies each proof inside a STARK and produces a single succinct proof. The final Groth16 wrapper converts the STARK proof to a constant-size proof suitable for on-chain verification."),
    p("The following table summarizes common composition patterns used in production operations:"),
    table(
      ["Composition", "Circuits Combined", "Shared Sub-circuits", "Total Constraints", "Use Case"],
      [
        ["Transfer + Compliance", "\u03C0priv + \u03C0comp", "C8 \u2194 CC7 (compliance hook / output commitment)", "~49,000", "Private transfer with inline compliance proof for jurisdiction-aware settlement"],
        ["Corridor + Compliance", "\u03C0corr + \u03C0comp", "CC7 output feeds \u03C0corr public input", "~40,000", "Corridor state transition gated on compliance attestation validity"],
        ["Netting + Tax", "\u03C0net + \u03C0tax", "Balance sub-circuits shared", "~30,000", "Multilateral netting with verified withholding tax deductions"],
        ["Full Settlement", "\u03C0priv + \u03C0comp + \u03C0tax + \u03C0corr", "Multiple shared sub-circuits via sequential composition", "~82,000", "End-to-end settlement: private transfer, compliance, tax, and corridor update in one proof"],
        ["Batch Aggregation", "N \u00D7 \u03C0comp \u2192 \u03C0agg", "Recursive STARK verification per proof", "~50,000 + N \u00D7 ~800", "Batch compliance verification for all entities in a netting round"],
      ],
      [1600, 1600, 2160, 1400, 2600]
    ),
    p("Constraint counts for composed circuits are not simply additive because shared sub-circuits are deduplicated. The savings depend on the specific overlap. For the Transfer + Compliance composition, deduplication saves approximately 3,000 constraints compared to independent proofs (34,000 + 18,000 = 52,000 independent vs. ~49,000 composed). For Full Settlement, the savings are more substantial at approximately 4,000 constraints due to multiple shared commitment and balance sub-circuits across four composed proofs."),
    spacer(),
  ];
};
