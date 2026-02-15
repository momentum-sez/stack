const {
  chapterHeading, h2,
  p,
  table,
  spacer
} = require("../lib/primitives");

module.exports = function build_chapter15() {
  return [
    chapterHeading("Chapter 15: Privacy Architecture"),

    // --- 15.1 Key Hierarchy ---
    h2("15.1 Key Hierarchy"),
    table(
      ["Key Type", "Capability", "Use Case"],
      [
        ["Spending Key (sk)", "Full account control", "Cold storage, high-value accounts"],
        ["Full Viewing Key (fvk)", "Decrypt all transactions", "Audit functions"],
        ["Incoming Viewing Key (ivk)", "Decrypt received only", "Accounting systems"],
        ["Detection Key (dk)", "Efficient scanning", "Lightweight clients"],
        ["Compliance Viewing Key (cvk)", "Selective disclosure", "Regulatory compliance"],
      ],
      [3000, 2800, 3560]
    ),
    spacer(),

    // --- 15.2 Transaction Privacy ---
    h2("15.2 Transaction Privacy"),
    p("Private transactions on the MASS L1 are untraceable by default. Each transaction consumes input records (nullified via ZK proof) and produces output records encrypted under recipient keys. Transaction amounts, sender identity, receiver identity, and asset type are all hidden behind zero-knowledge proofs. The only publicly visible data is the nullifier set (preventing double-spends) and the commitment set (enabling recipients to detect incoming transactions). This design draws from the Zcash Sapling model but extends it with compliance-aware selective disclosure and multi-asset support native to the protocol."),

    // --- 15.3 Compliance Integration ---
    h2("15.3 Compliance Integration"),
    p("Privacy and compliance coexist through ZK proofs that demonstrate regulatory predicates without revealing underlying data. A transaction can prove that both parties passed KYC/KYB verification, that the transfer amount is below a jurisdictional reporting threshold, that neither party appears on a sanctions list, and that applicable withholding tax has been computed correctly \u2014 all without disclosing the identities, the amount, or the tax computation to any observer."),
    p("Compliance Viewing Keys (cvk) enable authorized regulators to decrypt transaction details when presented with a valid legal instrument. The cvk hierarchy is jurisdiction-scoped: a Pakistani FBR officer's cvk decrypts only transactions involving Pakistani entities or PKR-denominated flows. A UAE Central Bank cvk decrypts only AED-denominated flows within ADGM-regulated entities. This scoping is enforced cryptographically, not by access control, ensuring that key compromise in one jurisdiction does not expose data from another."),

    // --- 15.4 Multi-Asset Privacy ---
    h2("15.4 Multi-Asset Privacy"),
    p("The privacy architecture supports multiple asset types within a single shielded pool. Asset type tags are encrypted alongside amounts in output records, preventing observers from distinguishing between PKR transfers, equity token movements, and trade finance instruments. Cross-asset operations (e.g., DVP settlement involving both securities and cash) execute atomically within the shielded pool, with the \u03C0priv circuit verifying balance conservation per asset type."),
  ];
};
