const {
  chapterHeading,
  p,
  codeBlock
} = require("../lib/primitives");

module.exports = function build_chapter21() {
  return [
    chapterHeading("Chapter 21: zkKYC and Privacy-Preserving Compliance"),

    p("zkKYC attestations bind verified identity to cryptographic keys without continuous identity disclosure. Private Proofs of Innocence (PPOI) demonstrate non-membership in sanctions lists without revealing identity. These are implemented as ZK circuits in the proof hierarchy."),

    ...codeBlock(
      "/// zkKYC: prove KYC tier without revealing identity.\n" +
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
      "}"
    ),
  ];
};
