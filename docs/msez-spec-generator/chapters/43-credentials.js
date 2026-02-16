const {
  chapterHeading, h2, h3,
  p, p_runs, bold, code,
  definition,
  codeBlock, table, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter43() {
  return [
    chapterHeading("Chapter 43: Verifiable Credentials"),

    // --- 43.1 Credential Types ---
    h2("43.1 Credential Types"),
    p("The SEZ Stack issues Verifiable Credentials (VCs) conforming to the W3C VC Data Model. Each credential binds a cryptographic proof to a set of claims about a subject, enabling third-party verification without contacting the issuer. The following credential types are defined for sovereign economic zone operations."),
    table(
      ["Credential Type", "Issuer", "Verifier", "Selective Disclosure"],
      [
        ["KYC Attestation", "Verification Provider", "Service Provider", "Tier level without identity details"],
        ["License Credential", "Licensing Authority", "Counterparties", "Active status without full license details"],
        ["Compliance Certificate", "Compliance Watcher", "Regulators", "Domain-specific state"],
        ["Corridor Authorization", "Corridor Administrator", "Jurisdiction Nodes", "Permitted operations subset"],
        ["Entity Registration", "Corporate Registry", "Third Parties", "Entity type and status"],
        ["Tax Compliance", "Tax Authority", "Financial Institutions", "Good standing without financials"],
      ],
      [2200, 2000, 2000, 3160]
    ),

    // --- 43.2 Credential Lifecycle ---
    h2("43.2 Credential Lifecycle"),
    p("A Verifiable Credential progresses through four lifecycle phases: issuance, presentation, verification, and revocation. Each phase enforces strict security invariants to prevent forgery, replay, and unauthorized disclosure."),

    h3("43.2.1 Issuance"),
    p("Issuance begins when an authorized issuer constructs a credential envelope containing the JSON-LD context, credential type array, issuer DID, issuance date, optional expiration date, and a credential subject carrying the domain-specific claims. The credential body is then canonicalized using JCS (JSON Canonicalization Scheme) via CanonicalBytes, producing a deterministic byte sequence. The issuer signs this canonical form with their Ed25519 private key and attaches the resulting Proof object. The credential is now tamper-evident: any modification to any field invalidates the signature."),
    p("The core data structure is the VerifiableCredential struct, which enforces the W3C envelope at the type level. Serde rename attributes map between Rust snake_case field names and the W3C-specified JSON field names (@context, type, issuanceDate, expirationDate, credentialSubject)."),
    ...codeBlock(
`/// A W3C Verifiable Credential with SEZ Stack extensions.
///
/// The envelope structure is rigid, while credential_subject is
/// intentionally extensible per the W3C specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifiableCredential {
    /// JSON-LD context URIs.
    #[serde(rename = "@context")]
    pub context: ContextValue,

    /// Credential identifier (URN or DID).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Credential type(s). MUST include "VerifiableCredential".
    #[serde(rename = "type")]
    pub credential_type: CredentialTypeValue,

    /// DID of the credential issuer.
    pub issuer: String,

    /// When the credential was issued (UTC).
    #[serde(rename = "issuanceDate")]
    pub issuance_date: DateTime<Utc>,

    /// Optional expiration date (UTC).
    #[serde(rename = "expirationDate")]
    pub expiration_date: Option<DateTime<Utc>>,

    /// The credential subject -- domain-specific claims.
    #[serde(rename = "credentialSubject")]
    pub credential_subject: serde_json::Value,

    /// Cryptographic proofs attached to this credential.
    #[serde(default, skip_serializing_if = "ProofValue::is_empty")]
    pub proof: ProofValue,
}`
    ),

    h3("43.2.2 Presentation"),
    p("Presentation is the act of a credential holder sharing a credential (or a derived subset of its claims) with a verifier. In the Ed25519 flow, the holder presents the full credential including its proof. The verifier can independently verify the signature without contacting the issuer. In the BBS+ selective disclosure flow (Section 43.4), the holder derives a proof that reveals only selected claims, preventing unnecessary exposure of private data. For example, a KYC credential holder can prove their verification tier without revealing their CNIC number or date of birth."),

    h3("43.2.3 Verification"),
    p("Verification reconstructs the canonical signing input from the credential body (with the proof field removed), resolves the issuer's public key from their DID, and checks the Ed25519 signature. The verify method returns a ProofResult for each attached proof, enabling multi-party verification where a credential carries signatures from multiple authorities. The verify_all method enforces that every proof passes, returning an error on any failure."),
    p("The signing input computation is a critical security invariant. It serializes the credential to a serde_json::Value, removes the proof field, and passes the result through CanonicalBytes::from_value(). This guarantees that both issuer and verifier compute identical byte sequences regardless of JSON field ordering or whitespace. Raw serde_json::to_vec() is never used in the signing or verification path."),

    h3("43.2.4 Revocation"),
    p("Revocation is handled through credential status lists (Section 43.5). When a credential is revoked, its index in the issuer's status list is flipped. Verifiers check the status list as part of their verification workflow. Expired credentials (where the current time exceeds expirationDate) are rejected without consulting the status list."),

    table(
      ["Phase", "Actor", "Security Invariant"],
      [
        ["Issuance", "Issuer", "Signs CanonicalBytes of body (proof excluded); never raw serialization"],
        ["Presentation", "Holder", "Full credential or BBS+ derived proof with selective claim disclosure"],
        ["Verification", "Verifier", "Recomputes CanonicalBytes, resolves DID to public key, checks Ed25519"],
        ["Revocation", "Issuer", "Flips bit in status list; verifiers check list on every verification"],
      ],
      [1800, 1400, 6160]
    ),

    pageBreak(),

    // --- 43.3 Ed25519 Proof Generation ---
    h2("43.3 Ed25519 Proof Generation"),
    p("The Ed25519 proof generation process is the foundation of credential integrity in the SEZ Stack. The process enforces type-level security: the SigningKey::sign() method only accepts &CanonicalBytes, making it impossible to sign non-canonicalized data without an explicit unsafe bypass."),

    definition("Definition 43.1 (Signing Input).", "The signing input for a Verifiable Credential is the JCS-canonicalized byte sequence of the credential body with the proof field removed. Formally: signing_input(vc) = JCS(vc \\ {proof})."),

    p("The signing process proceeds in four steps:"),
    p("Step 1: Serialize the credential to a serde_json::Value and remove the proof field. This produces the unsigned credential body as a JSON value."),
    p("Step 2: Pass the JSON value through CanonicalBytes::from_value(), which applies JCS canonicalization. JCS sorts object keys lexicographically, normalizes whitespace, and rejects floating-point numbers (which have non-deterministic serialization). If the credential subject contains a float, canonicalization fails with a CanonicalizationError."),
    p("Step 3: Sign the canonical bytes with the Ed25519 signing key, producing a 64-byte signature."),
    p("Step 4: Construct a Proof object containing the proof type, creation timestamp, verification method (the DID URL of the signing key), proof purpose (assertionMethod), and the hex-encoded signature value. Attach this proof to the credential."),

    ...codeBlock(
`/// Sign this credential with an Ed25519 key pair.
///
/// Security: signing input is computed via CanonicalBytes,
/// not raw serialization. SigningKey::sign() only accepts
/// &CanonicalBytes, enforcing this at the type level.
pub fn sign_ed25519(
    &mut self,
    signing_key: &SigningKey,
    verification_method: String,
    proof_type: ProofType,
    created: Option<Timestamp>,
) -> Result<(), VcError> {
    let canonical = self.signing_input()?;
    let signature = signing_key.sign(&canonical);

    let proof = Proof {
        proof_type,
        created: *created.unwrap_or_else(Timestamp::now).as_datetime(),
        verification_method,
        proof_purpose: ProofPurpose::AssertionMethod,
        proof_value: signature.to_hex(),
    };

    self.proof.push(proof);
    Ok(())
}`
    ),

    p_runs([bold("Multi-party signing."), " A credential can carry multiple proofs from different signers. Each call to sign_ed25519 appends a new Proof to the proof array. The signing input is computed identically each time (proof field excluded), so each signer signs the same canonical body. During verification, the resolver function maps each proof's verification method to the corresponding public key. All proofs must pass for verify_all to succeed."]),

    p_runs([bold("Proof types."), " The SEZ Stack supports two Ed25519 proof type identifiers: ", code("Ed25519Signature2020"), " (W3C standard) and ", code("MsezEd25519Signature2025"), " (SEZ Stack-specific, for internal interoperability). Both use identical Ed25519 signing mechanics; the type string is purely a namespace identifier. The ", code("BbsBlsSignature2020"), " type is reserved for Phase 2 selective disclosure (Section 43.4)."]),

    // --- 43.4 BBS+ Selective Disclosure ---
    h2("43.4 BBS+ Selective Disclosure"),
    p("BBS+ signatures enable a credential holder to derive a zero-knowledge proof that reveals only a chosen subset of the credential's claims. This is critical for jurisdictional compliance: a regulator may need to verify an entity's tax compliance status without accessing its full financial records, or a counterparty may need to confirm a license is active without seeing the license's full terms."),

    definition("Definition 43.2 (Selective Disclosure).", "Given a credential C with claims {c_1, c_2, ..., c_n} and a BBS+ signature sigma over C, the holder can produce a derived proof pi that reveals only the subset {c_i, c_j, ...} while proving that these claims were part of a credential signed by the issuer, without revealing the remaining claims or the original signature."),

    h3("43.4.1 How BBS+ Selective Disclosure Works"),
    p("Unlike Ed25519 (which signs a single opaque message), BBS+ signs an ordered list of messages -- one per claim in the credential subject. At issuance time, the issuer's BBS+ private key produces a signature over all n claim values. At presentation time, the holder selects which claims to reveal and computes a zero-knowledge proof of knowledge of the remaining claims. The verifier sees only the revealed claims plus a proof that they belong to a validly signed credential, without learning anything about the hidden claims."),

    p("The protocol operates in three phases:"),

    p_runs([bold("Phase 1 -- Issuance."), " The issuer decomposes the credential subject into an ordered list of (key, value) pairs. Each pair is encoded as a message m_i. The issuer computes a BBS+ signature sigma = BBS_Sign(sk, [m_1, m_2, ..., m_n]) over all messages using their BBS+ private key sk. The signature and the full credential are delivered to the holder."]),

    p_runs([bold("Phase 2 -- Selective Disclosure."), " The holder chooses a disclosure set D (the indices of claims to reveal). For the unrevealed claims, the holder computes blinding factors and constructs a zero-knowledge proof pi = BBS_ProofGen(pk, sigma, [m_1, ..., m_n], D) that proves knowledge of the hidden messages without revealing them."]),

    p_runs([bold("Phase 3 -- Verification."), " The verifier receives the revealed claims and the derived proof pi. The verifier runs BBS_ProofVerify(pk, pi, revealed_messages, D) using the issuer's public key pk. If the proof is valid, the verifier is assured that the revealed claims are part of a credential signed by the issuer."]),

    h3("43.4.2 Example: KYC Selective Disclosure"),
    p("Consider a KYC Attestation credential with the following claims in its credential subject:"),
    table(
      ["Claim Index", "Claim Key", "Claim Value", "Disclosed?"],
      [
        ["0", "subject_did", "did:key:z6MkHolder...", "Yes"],
        ["1", "verification_tier", "enhanced", "Yes"],
        ["2", "cnic_number", "42101-XXXXXXX-X", "No"],
        ["3", "date_of_birth", "1990-05-15", "No"],
        ["4", "full_name", "Ahmad Khan", "No"],
        ["5", "verified_at", "2026-01-15T12:00:00Z", "Yes"],
      ],
      [1600, 2200, 2800, 2760]
    ),

    p("The holder reveals only claims 0, 1, and 5 (subject DID, verification tier, and verification date). The verifier learns that the holder has enhanced KYC verification performed on a specific date, but learns nothing about the holder's CNIC number, date of birth, or full name. The BBS+ proof mathematically guarantees these hidden claims exist and were signed by the issuer."),

    p_runs([bold("Implementation status."), " BBS+ selective disclosure is Phase 2 functionality, gated behind the ", code("bbs-plus"), " feature flag in ", code("msez-crypto"), ". The ", code("ProofType::BbsBlsSignature2020"), " variant is defined in the proof type enum. The credential structure's extensible ", code("credential_subject"), " field (", code("serde_json::Value"), ") supports the claim decomposition required by BBS+ without structural changes to the VerifiableCredential type."]),

    pageBreak(),

    // --- 43.5 Credential Revocation ---
    h2("43.5 Credential Revocation"),
    p("Credential revocation uses a status list approach inspired by the W3C Status List 2021 specification. This mechanism enables efficient, privacy-preserving revocation checking without requiring the verifier to contact the issuer for each verification."),

    h3("43.5.1 Status List Structure"),
    p("Each issuer maintains a status list: a bitfield where each bit position corresponds to a credential index. A bit value of 0 means the credential at that index is active; a bit value of 1 means revoked. The status list is itself published as a Verifiable Credential, signed by the issuer, ensuring that revocation status is tamper-evident."),

    ...codeBlock(
`/// A credential status entry embedded in the VC.
/// Points to the issuer's status list and the credential's
/// index within that list.
pub struct CredentialStatus {
    /// Status list credential URL.
    pub status_list_credential: String,
    /// Index of this credential in the status list bitfield.
    pub status_list_index: u64,
    /// Purpose: "revocation" or "suspension".
    pub status_purpose: String,
}

/// The issuer's status list, published as a signed VC.
/// The encoded_list is a GZIP-compressed, base64-encoded
/// bitfield where each bit represents one credential.
pub struct StatusList {
    /// Base64-encoded, GZIP-compressed bitfield.
    pub encoded_list: String,
    /// Total number of entries in the list.
    pub length: u64,
    /// Purpose this list tracks (revocation or suspension).
    pub status_purpose: String,
}`
    ),

    h3("43.5.2 Revocation Workflow"),
    p("When an issuer needs to revoke a credential (due to key compromise, entity dissolution, compliance failure, or regulatory order), the workflow proceeds as follows:"),
    p("Step 1: The issuer looks up the credential's status_list_index from their issuance records."),
    p("Step 2: The issuer decodes and decompresses the current status list bitfield, flips the bit at the credential's index from 0 to 1, re-compresses, and re-encodes."),
    p("Step 3: The issuer re-signs the status list VC with a fresh proof and publishes the updated status list at the same URL."),
    p("Step 4: Verifiers fetch the status list (with appropriate caching) and check the bit at the credential's index. If the bit is 1, verification fails with a revocation error."),

    p_runs([bold("Privacy properties."), " The status list approach preserves holder privacy. A verifier checking a specific credential learns only the status of that one index. The verifier does not learn which other credentials have been revoked. The compressed bitfield representation means the list size is efficient even for issuers with millions of active credentials."]),

    p_runs([bold("Suspension vs. revocation."), " The status_purpose field distinguishes between permanent revocation and temporary suspension. A suspended credential can be reactivated by flipping its bit back to 0. A revoked credential's bit is never flipped back. Issuers may maintain separate status lists for revocation and suspension to enforce this distinction."]),

    table(
      ["Event", "Trigger", "Effect on Status List", "Reversible"],
      [
        ["Revocation", "Key compromise, dissolution, compliance failure", "Bit flipped to 1 permanently", "No"],
        ["Suspension", "Regulatory investigation, pending renewal", "Bit flipped to 1 temporarily", "Yes"],
        ["Reactivation", "Investigation cleared, renewal completed", "Bit flipped back to 0", "N/A"],
        ["Expiration", "expirationDate exceeded", "No status list change (time-based)", "No"],
      ],
      [1800, 3000, 2800, 1760]
    ),
  ];
};
