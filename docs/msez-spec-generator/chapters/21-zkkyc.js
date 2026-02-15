const {
  chapterHeading, h2,
  p, p_runs, bold,
  codeBlock, table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter21() {
  return [
    chapterHeading("Chapter 21: zkKYC and Privacy-Preserving Compliance"),

    // --- 21.1 zkKYC Architecture ---
    h2("21.1 zkKYC Architecture"),
    p("zkKYC attestations bind verified identity to cryptographic keys without continuous identity disclosure. A KYC provider verifies a natural person or entity through standard documentary processes, then issues a BBS+-signed credential containing identity attributes. The credential holder can subsequently prove KYC tier satisfaction to any verifier without revealing the underlying identity data. This separation between verification (one-time, identity-disclosing) and presentation (repeated, privacy-preserving) is the foundation of privacy-compliant commerce in the SEZ Stack."),

    // --- 21.2 KYC Tiers ---
    h2("21.2 KYC Tiers"),
    table(
      ["Tier", "Verification Level", "Transaction Limits", "Required Evidence"],
      [
        ["Tier 0", "Anonymous (wallet only)", "Receive only, no transfer", "None"],
        ["Tier 1", "Basic identity", "Low-value transactions (<$1,000/day)", "Government ID + selfie match"],
        ["Tier 2", "Enhanced identity", "Standard transactions (<$50,000/day)", "Tier 1 + proof of address + source of funds declaration"],
        ["Tier 3", "Full due diligence", "High-value transactions (unlimited)", "Tier 2 + enhanced due diligence + ongoing monitoring"],
        ["Tier 4", "Institutional", "Institutional volumes", "Entity KYB + UBO verification + board resolution"],
      ],
      [1200, 2200, 2800, 3160]
    ),
    spacer(),

    // --- 21.3 ZK Proof Structure ---
    h2("21.3 ZK Proof Structure"),
    ...codeBlock(
      "/// zkKYC: prove KYC tier without revealing identity.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct ZkKycProof {\n" +
      "    pub proof: ZkProof,\n" +
      "    pub public_inputs: ZkKycPublicInputs,\n" +
      "}\n" +
      "\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct ZkKycPublicInputs {\n" +
      "    pub credential_commitment: Digest,\n" +
      "    pub kyc_tier: KycTier,\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    pub expiry: chrono::DateTime<chrono::Utc>,\n" +
      "    pub nullifier: Digest,\n" +
      "}"
    ),
    spacer(),
    p("The ZkKycProof proves three statements simultaneously: the prover holds a valid BBS+-signed credential issued by a recognized KYC provider, the credential attests to at least the claimed KYC tier, and the credential has not expired. The proof reveals only the KYC tier, jurisdiction, and expiry date. The credential commitment binds the proof to a specific credential without revealing its contents. The nullifier prevents double-presentation of revoked credentials."),

    // --- 21.4 Private Proofs of Innocence ---
    h2("21.4 Private Proofs of Innocence (PPOI)"),
    p("PPOI extends zkKYC to sanctions screening. The \u03C0sanctions circuit (approximately 6,000 constraints) proves non-membership in a sanctions set without revealing the entity being checked. The prover demonstrates that their identity hash does not appear in the Merkle tree of sanctioned entities, using a non-membership proof against the sanctions list root published in the regpack."),
    p("PPOI enables privacy-preserving corridor transactions: a sender in the PAK\u2194UAE corridor proves they are not on OFAC, EU, or UN sanctions lists without revealing their identity to the corridor counterparty. The corridor watcher verifies the proof against the current sanctions list root and issues an attestation that clears the transaction for settlement."),

    // --- 21.5 Credential Lifecycle ---
    h2("21.5 Credential Lifecycle"),
    p("KYC credentials follow a four-stage lifecycle: issuance (KYC provider verifies identity and issues BBS+-signed credential), active (credential valid for presentations with ZK proofs), renewal (periodic re-verification before expiry, typically annually for Tier 3+), and revocation (credential nullifier added to spent set, preventing further use). Revocation triggers are: identity fraud detection, sanctions list match after issuance, credential holder request, or KYC provider termination. The agentic trigger system monitors credential expiry dates and initiates renewal workflows at 90, 60, and 30 days before expiry."),
  ];
};
