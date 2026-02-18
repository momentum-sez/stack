const {
  chapterHeading, h2,
  p, p_runs, bold,
  table
} = require("../lib/primitives");

module.exports = function build_chapter48() {
  return [
    chapterHeading("Chapter 48: Zero-Knowledge Proof Circuits"),

    p("This chapter specifies the circuit architecture for the SEZ Stack\u2019s zero-knowledge proof layer. The msez-zkp crate currently defines four circuit modules (compliance, identity, migration, settlement) that establish the interface contracts and witness structures. Concrete constraint implementations will be delivered in Phase 4 when the ZK layer activates. The circuit taxonomy below specifies the target architecture that these modules will implement."),

    // --- 48.1 Implemented Circuit Modules ---
    h2("48.1 Implemented Circuit Modules"),
    p("The four circuit modules in msez-zkp/src/circuits/ define the public interface for each proof type. They specify the public inputs, private witness structure, and verification semantics without yet implementing the constraint systems."),

    table(
      ["Module", "File", "Public Inputs", "Private Witness", "Verification Semantics"],
      [
        ["Compliance", "circuits/compliance.rs", "Entity commitment, jurisdiction code, domain set, evaluation timestamp", "Compliance state vector, evidence attestations", "All compliance predicates for the specified (jurisdiction, domain) pair evaluate to true"],
        ["Identity", "circuits/identity.rs", "Identity commitment, KYC tier, jurisdiction code", "Personal information, KYC documents, identity proofs", "The entity meets the claimed KYC tier without revealing identity details"],
        ["Migration", "circuits/migration.rs", "Source jurisdiction, target jurisdiction, asset commitment", "Migration terms, compliance evaluations, settlement details", "The asset migration satisfies both jurisdictions\u2019 requirements"],
        ["Settlement", "circuits/settlement.rs", "Corridor ID, settlement commitment, netting result", "Transaction details, compliance attestations, tax computations", "The settlement computation is correct and all compliance requirements are met"],
      ],
      [1400, 1800, 2400, 2000, 1760]
    ),

    // --- 48.2 Target Circuit Taxonomy ---
    h2("48.2 Target Circuit Taxonomy (Phase 4)"),
    p("The following circuit taxonomy specifies the full set of circuits planned for Phase 4 activation. Constraint counts are estimates based on the circuit design; actual counts will be determined during implementation and audit."),

    table(
      ["Circuit", "Est. Constraints", "Purpose", "Backend"],
      [
        ["\u03c0comp (Compliance)", "~18,000", "Compliance predicate satisfaction without revealing evidence", "Groth16 or Plonk"],
        ["\u03c0kyc (KYC)", "~8,000", "KYC tier verification without identity disclosure", "Groth16 or Plonk"],
        ["\u03c0tax (Tax)", "~12,000", "Tax withholding correctness proof", "Groth16 or Plonk"],
        ["\u03c0sanc (Sanctions)", "~6,000", "Non-membership in sanctions set via Merkle exclusion", "Groth16 or Plonk"],
        ["\u03c0corr (Corridor)", "~22,000", "Corridor state transition validity", "Groth16 or Plonk"],
        ["\u03c0net (Netting)", "~20,000", "Multilateral netting correctness", "Groth16 or Plonk"],
        ["\u03c0settle (Settlement)", "~15,000", "Cross-border settlement computation", "Groth16 or Plonk"],
        ["\u03c0migrate (Migration)", "~14,000", "Cross-jurisdictional asset migration validity", "Groth16 or Plonk"],
      ],
      [2000, 1600, 3200, 2560]
    ),

    // --- 48.3 Circuit Composition ---
    h2("48.3 Circuit Composition (Planned)"),
    p("The circuit architecture is designed for composition: combining sub-circuits from different top-level circuits into a single unified proof. This reduces verifier cost, eliminates redundant constraint evaluations, and ensures atomicity. Common composition patterns include:"),

    table(
      ["Composition", "Circuits Combined", "Use Case"],
      [
        ["Corridor + Compliance", "\u03c0corr + \u03c0comp", "Corridor state transition gated on compliance attestation"],
        ["Netting + Tax", "\u03c0net + \u03c0tax", "Multilateral netting with verified withholding tax deductions"],
        ["Settlement + Compliance + Tax", "\u03c0settle + \u03c0comp + \u03c0tax", "End-to-end settlement with inline compliance and tax proof"],
        ["Migration + Compliance", "\u03c0migrate + \u03c0comp", "Cross-jurisdictional migration with dual-jurisdiction compliance"],
      ],
      [2400, 2400, 4560]
    ),

    p_runs([
      bold("Implementation Note. "),
      "The circuit composition framework, including shared sub-circuit deduplication and recursive proof aggregation, is specified but not yet implemented. The msez-zkp crate\u2019s ProofSystem trait will be extended with a compose() method when Phase 4 delivers the concrete constraint systems. Until then, the four circuit modules serve as interface contracts that downstream code can compile against."
    ]),
  ];
};
