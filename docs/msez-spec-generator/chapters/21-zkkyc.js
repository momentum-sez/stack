const {
  chapterHeading, h2,
  p, p_runs, bold,
  codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter21() {
  return [
    chapterHeading("Chapter 21: zkKYC and Privacy-Preserving Compliance"),

    // --- 21.1 zkKYC Model ---
    h2("21.1 zkKYC Model"),
    p("zkKYC attestations bind verified identity to cryptographic keys without continuous identity disclosure. Traditional KYC requires counterparties, regulators, and intermediaries to hold copies of sensitive personal data -- passports, national IDs, proof of address -- creating honeypots for data breaches and imposing ongoing liability on every party in the chain. zkKYC inverts this model: the identity holder retains custody of their documents, and produces zero-knowledge proofs that attest to specific properties of their verified identity without revealing the identity itself."),

    p("The zkKYC model operates on three principles. First, credential commitment: the identity holder commits to their verified KYC attestation using a binding commitment scheme (SHA-256 over canonical form), producing a digest that is publicly linkable to their DID but reveals nothing about the underlying data. Second, predicate proving: when a counterparty or regulator requires proof of a KYC property (tier level, jurisdiction, non-expiry), the holder generates a ZK proof over the committed credential that demonstrates the required predicate without opening the commitment. Third, issuer binding: every proof includes verification that the underlying attestation was signed by an approved KYC issuer, checked via Merkle inclusion against a public root of approved issuers maintained by the SEZ Stack operator."),

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

    p("The KycAttestationCircuit enforces the following constraints: (1) the attestation hash opens to a valid KYC document under the committed credential, (2) the issuer's signature over the attestation verifies against their public key, (3) the issuer's public key is included in the approved issuers Merkle tree with root equal to the public input, (4) the KYC level in the attestation is greater than or equal to the required minimum, and (5) the verification timestamp is before the attestation's expiry. The approximate constraint count is 4096, dominated by Ed25519 signature verification and Merkle path computation."),

    // --- 21.2 Private Proofs of Innocence (PPOI) ---
    h2("21.2 Private Proofs of Innocence (PPOI)"),
    p("Private Proofs of Innocence (PPOI) demonstrate non-membership in sanctions lists without revealing identity. Sanctions screening is a legal requirement for every cross-border transaction, yet traditional screening requires disclosing the screened party's identity to every intermediary in the payment chain. PPOI resolves this tension: the entity proves it does not appear on OFAC SDN, UN Security Council, EU Consolidated, or FATF-designated lists without revealing which entity is being screened."),

    p("The mechanism relies on a Merkle tree of hashed sanctioned-entity identifiers maintained as part of the regpack (see Chapter 6). The SEZ Stack operator publishes an updated sanctions Merkle root at regular intervals (configurable per jurisdiction, typically daily). To produce a PPOI, the entity hashes its identity using the same deterministic scheme used to construct the tree, then generates a Merkle non-membership proof demonstrating that its hash does not appear as a leaf. This proof is wrapped in a ZK circuit that additionally binds the entity hash to the prover's DID commitment, preventing proof transfer between entities."),

    ...codeBlock(
      "/// PPOI: prove non-membership in a sanctions list.\n" +
      "pub struct PpoiProof {\n" +
      "    pub proof: ZkProof,\n" +
      "    pub public_inputs: PpoiPublicInputs,\n" +
      "}\n" +
      "\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct PpoiPublicInputs {\n" +
      "    /// Root hash of the sanctions Merkle tree.\n" +
      "    pub sanctions_root: Digest,\n" +
      "    /// Timestamp of the sanctions list snapshot (UTC).\n" +
      "    pub list_timestamp: chrono::DateTime<chrono::Utc>,\n" +
      "    /// Commitment binding the proof to a specific DID.\n" +
      "    pub did_commitment: Digest,\n" +
      "    /// Which sanctions list was checked (OFAC, UN, EU, FATF).\n" +
      "    pub list_identifier: SanctionsListId,\n" +
      "}"
    ),

    p("The SanctionsClearanceCircuit enforces: (1) the entity hash is correctly derived from the prover's identity commitment and a domain separator, (2) the Merkle non-membership proof is valid against the public sanctions root, (3) the verification timestamp matches the published snapshot time, and (4) the DID commitment binds the proof to the prover. The approximate constraint count is 2048, dominated by Merkle path verification over a tree of depth up to 20 (supporting over one million sanctioned entries)."),

    // --- 21.3 Verification Flow ---
    h2("21.3 Verification Flow"),
    p("zkKYC verification follows a five-step protocol that separates credential issuance from proof generation and verification. This separation ensures that the KYC issuer never learns when or with whom the credential holder transacts, and the verifier never learns the holder's identity beyond the proven predicates."),

    p_runs([bold("Step 1 -- Credential Issuance. "), "The entity completes traditional KYC/KYB verification with an approved issuer (e.g., NADRA for Pakistan CNIC, or an accredited KYC provider for international passports). The issuer produces a signed attestation containing the entity's identity details, KYC tier, jurisdiction, and expiry date. This attestation is delivered exclusively to the entity and anchored as a W3C Verifiable Credential via msez-vc."]),
    p_runs([bold("Step 2 -- Commitment Registration. "), "The entity computes a commitment over the attestation using CanonicalBytes and registers the commitment digest with the SEZ Stack. The commitment is bound to the entity's DID. The attestation itself is never transmitted to the SEZ Stack."]),
    p_runs([bold("Step 3 -- Proof Generation. "), "When a transaction requires KYC verification (corridor entry, entity formation, capital transfer), the entity's client generates a ZK proof over the committed credential. The proof demonstrates the required predicates (tier >= minimum, jurisdiction match, non-expiry) without opening the commitment. For corridor transactions, a PPOI is generated in parallel against the current sanctions root."]),
    p_runs([bold("Step 4 -- On-Chain Verification. "), "The verifier (corridor counterparty, regulator endpoint, or the SEZ Stack API itself) checks the ZK proof against the public inputs. Verification is constant-time and requires no access to the underlying identity data. The proof is anchored to the receipt chain for audit trail purposes."]),
    p_runs([bold("Step 5 -- Selective Disclosure (Optional). "), "If a regulator presents a valid legal instrument (subpoena, court order, regulatory examination notice), the entity can produce a BBS+ selective disclosure proof from the original Verifiable Credential, revealing only the specific fields required by the instrument. This is mediated through the consent primitive via msez-mass-client."]),

    // --- 21.4 Disclosure Matrix ---
    h2("21.4 Disclosure Matrix"),
    p("The following table specifies what information is revealed versus hidden in each verification context. The design principle is minimum necessary disclosure: each context reveals only the predicates required by its legal or operational purpose."),

    table(
      ["Verification Context", "Revealed (Public Inputs)", "Hidden (Witness)"],
      [
        [
          "Corridor counterparty verification",
          "KYC tier >= required minimum, jurisdiction, credential non-expiry, PPOI valid",
          "Entity name, CNIC/passport number, address, date of birth, actual KYC tier (if above minimum), issuer identity"
        ],
        [
          "Regulatory reporting (automated)",
          "Compliance tensor coordinate, domain (AML/TAX/KYC), jurisdiction, time quantum, claimed compliance state",
          "Entity identity, transaction amounts, counterparty identities, underlying attestation details"
        ],
        [
          "Sanctions screening (PPOI)",
          "Sanctions list root, list snapshot timestamp, DID commitment, list identifier (OFAC/UN/EU/FATF)",
          "Entity hash, entity identity, Merkle non-membership path, all personal data"
        ],
        [
          "Entity formation",
          "KYC tier >= Tier 2, jurisdiction = target SEZ, credential non-expiry",
          "Beneficial owner identities, source of funds, residential addresses, CNIC numbers"
        ],
        [
          "Capital transfer (cross-border)",
          "KYC tier >= Tier 3, PPOI valid for both sender and receiver jurisdictions, balance >= transfer amount",
          "Actual balance, sender/receiver identities, transfer amount (if below reporting threshold)"
        ],
        [
          "Regulatory examination (with legal instrument)",
          "Selectively disclosed fields per legal instrument scope (via BBS+)",
          "All fields outside the scope of the legal instrument"
        ],
      ],
      [2200, 3400, 3760]
    ),

    p("This matrix is enforced at the circuit level: the ZK proof system makes it cryptographically impossible to extract hidden fields from the proof. Selective disclosure under regulatory examination uses BBS+ signatures on the original Verifiable Credential, not the ZK proof, ensuring that even under compelled disclosure the entity reveals only the specific fields demanded by the legal instrument and nothing more."),
  ];
};
