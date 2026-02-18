const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter14() {
  return [
    chapterHeading("Chapter 14: Proving System"),

    p("The SEZ Stack\u2019s proving system is implemented in the msez-zkp crate, which defines a sealed ProofSystem trait with pluggable backends. The architecture separates proof system selection from business logic: any component that requires a ZK proof depends on the trait, not a concrete backend. This chapter specifies the implemented backends, the production policy enforcement mechanism, and the planned Mass L1 proving pipeline."),

    // --- 14.1 ProofSystem Trait ---
    h2("14.1 ProofSystem Trait"),
    p("The sealed ProofSystem trait defines the interface that all proof backends must implement. It is sealed (not implementable outside msez-zkp) to prevent untrusted third-party backends from being injected into the verification pipeline."),

    ...codeBlock(
      "/// Sealed trait for zero-knowledge proof backends (msez-zkp)\n" +
      "pub trait ProofSystem: Send + Sync {\n" +
      "    /// Generate a proof for the given circuit and witness.\n" +
      "    fn prove(&self, circuit: &Circuit, witness: &Witness) -> Result<Proof>;\n" +
      "\n" +
      "    /// Verify a proof against a circuit and public inputs.\n" +
      "    fn verify(&self, circuit: &Circuit, proof: &Proof, public_inputs: &[u8]) -> Result<bool>;\n" +
      "\n" +
      "    /// Return the backend identifier for policy enforcement.\n" +
      "    fn backend_id(&self) -> &str;\n" +
      "}"
    ),

    // --- 14.2 Implemented Backends ---
    h2("14.2 Implemented Backends"),
    table(
      ["Backend", "Module", "Status", "Trust Model", "Primary Use Case"],
      [
        ["Mock (SHA-256 deterministic)", "msez-zkp::mock", "Testing only", "None (not cryptographic)", "Development, CI, property testing. Proofs are SHA-256 hashes of inputs \u2014 deterministic but not zero-knowledge"],
        ["Groth16", "msez-zkp::groth16", "Implemented", "Trusted setup (per-circuit SRS)", "Compact proofs for on-chain verification. Constant proof size (288 bytes). Pairing-based verification"],
        ["Plonk", "msez-zkp::plonk", "Implemented", "Universal setup (updateable SRS)", "General-purpose ZK proofs. Larger proofs than Groth16 but no per-circuit trusted setup"],
      ],
      [2400, 1800, 1200, 2000, 1960]
    ),

    // --- 14.3 Production Policy Enforcement ---
    h2("14.3 Production Policy Enforcement"),
    p("The msez-zkp::policy module prevents the catastrophic scenario where mock proofs are accepted in a production deployment. The policy is configured via a content-addressed policy artifact that specifies which backends are permitted for each deployment environment."),

    p_runs([
      bold("Compile-time enforcement. "),
      "CI gates verify that release builds do not enable the mock feature flag. cargo metadata checks confirm that msez-zkp is compiled without mock features in release profiles."
    ]),
    p_runs([
      bold("Runtime enforcement. "),
      "The policy module checks the backend_id() of every proof against the deployment\u2019s policy artifact. Mock proofs submitted to a production-configured deployment are rejected at the verification boundary, regardless of their mathematical validity."
    ]),

    // --- 14.4 Circuit Modules ---
    h2("14.4 Circuit Modules"),
    p("The msez-zkp crate defines five circuit modules covering the governance operations that require zero-knowledge proofs. These modules define the circuit interfaces and witness structures; the concrete constraint systems will be implemented when the ZK layer activates in Phase 4."),

    table(
      ["Circuit", "Module", "Purpose"],
      [
        ["Compliance", "circuits/compliance.rs", "Prove compliance evaluation correctness without revealing entity data or tensor state"],
        ["Identity", "circuits/identity.rs", "Prove identity claims (KYC tier, nationality, accreditation) without revealing personal information"],
        ["Migration", "circuits/migration.rs", "Prove valid asset migration between jurisdictions without revealing migration terms"],
        ["Settlement", "circuits/settlement.rs", "Prove correct settlement computation for corridor transactions without revealing transaction details"],
      ],
      [1800, 2400, 5160]
    ),

    // --- 14.5 Planned: Mass L1 Proving Pipeline ---
    h2("14.5 Planned: Mass L1 Proving Pipeline"),
    p("The Mass Protocol\u2019s L1 settlement layer specifies a multi-layer proof aggregation pipeline that progressively reduces verification cost. This pipeline is not yet implemented in the SEZ Stack; it describes the target architecture that will be integrated when the Mass L1 is deployed."),

    table(
      ["Layer", "Input", "Output", "Aggregation Factor"],
      [
        ["Transaction", "Single transaction", "Transaction proof (STARK)", "1:1"],
        ["Block (Layer 1)", "1K\u201310K transaction proofs", "Block proof (STARK)", "1,000\u201310,000:1"],
        ["Epoch (Layer 2)", "10\u2013100 block proofs", "Epoch proof (STARK)", "10\u2013100:1"],
        ["Final Wrapping", "Single epoch proof", "Groth16 proof (288 bytes)", "1:1"],
      ],
      [1800, 2800, 2800, 1960]
    ),

    p("The final Groth16 wrapping step produces a constant-size proof suitable for on-chain verification on EVM-compatible chains. Verification cost is approximately 200K gas on Ethereum, independent of the complexity of the underlying computation. This proof portability enables jurisdictional regulators to independently verify compliance attestations without running Mass L1 infrastructure."),
  ];
};
