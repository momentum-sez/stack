const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table,
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

    // --- 15.2.1 Compliance Integration ---
    h3("15.2.1 Compliance Integration"),
    p("Privacy and compliance coexist through ZK proofs that demonstrate regulatory predicates without revealing underlying data. A transaction can prove that both parties passed KYC/KYB verification, that the transfer amount is below a jurisdictional reporting threshold, that neither party appears on a sanctions list, and that applicable withholding tax has been computed correctly -- all without disclosing the identities, the amount, or the tax computation to any observer. Compliance Viewing Keys (cvk) enable authorized regulators to decrypt transaction details when presented with a valid legal instrument, providing a controlled escape hatch that satisfies regulatory requirements without compromising the privacy of uninvolved parties."),

    // --- 15.3 Note Encryption ---
    h2("15.3 Note Encryption"),
    p("Output records (notes) produced by each transaction are encrypted so that only the intended recipient can decrypt and spend them. The encryption scheme uses a hybrid construction combining Diffie-Hellman key agreement with symmetric authenticated encryption, ensuring that note contents remain confidential even if the underlying blockchain state is fully public."),
    p_runs([bold("Encryption Procedure."), " When a transaction produces an output note for a recipient, the sender generates an ephemeral key pair (esk, epk) and computes a shared secret via Diffie-Hellman between esk and the recipient's public transmission key (pk_d). The shared secret is fed through a KDF (BLAKE2s with personalization) to derive a symmetric encryption key. The note plaintext -- containing the asset type, value, recipient diversifier, and a random commitment trapdoor -- is encrypted using ChaCha20-Poly1305 with the derived key. The ciphertext and the ephemeral public key epk are published alongside the note commitment on-chain."]),
    p_runs([bold("Decryption and Trial Decryption."), " The recipient scans new on-chain ciphertexts by computing the shared secret from each epk and their incoming viewing key (ivk). If the decryption succeeds (Poly1305 MAC verifies), the recipient recovers the note plaintext and can reconstruct the full note commitment to verify it matches the on-chain commitment. This trial decryption is computationally lightweight (a single DH operation plus one symmetric decryption per candidate note) and can be delegated to a Detection Key holder for bandwidth-efficient scanning without revealing note contents."]),
    p_runs([bold("Forward Secrecy."), " Because each transaction uses a fresh ephemeral key pair, compromise of a recipient's long-term keys does not retroactively reveal the contents of previously received notes unless the attacker also obtains the on-chain ciphertexts and the recipient's viewing keys. The ephemeral keys are discarded after encryption and never stored by the sender. This provides forward secrecy at the note level: each note's encryption is independent of every other note's encryption."]),
    ...codeBlock(
      "/// Encrypted output note as stored on-chain.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct EncryptedNote {\n" +
      "    pub epk: EphemeralPublicKey,        // Sender's ephemeral public key\n" +
      "    pub commitment: NoteCommitment,      // Pedersen commitment to note contents\n" +
      "    pub ciphertext: Vec<u8>,             // ChaCha20-Poly1305 encrypted plaintext\n" +
      "    pub enc_ciphertext_len: u32,         // 580 bytes for standard notes\n" +
      "}"
    ),
    spacer(),

    // --- 15.3.1 Metadata Protection ---
    h3("15.3.1 Metadata Protection"),
    p("Even when transaction contents are encrypted, metadata leakage -- timing patterns, transaction graph structure, output counts, and network-level information -- can enable sophisticated traffic analysis attacks that deanonymize participants. The MASS L1 privacy architecture incorporates multiple layers of metadata protection to resist these attacks."),
    p_runs([bold("Uniform Transaction Size."), " All transactions on the MASS L1 are padded to a uniform size regardless of their actual content. A simple single-input, single-output transfer is indistinguishable in byte length from a complex multi-asset corridor settlement. This eliminates transaction-size-based classification attacks that could otherwise distinguish transaction types and narrow the anonymity set."]),
    p_runs([bold("Decoy Inputs and Outputs."), " Each transaction includes a configurable number of decoy inputs (references to existing commitments that are not actually consumed) and decoy outputs (encrypted notes with zero value that are indistinguishable from real outputs). The ZK proof ensures that the decoys do not affect the transaction's semantic correctness while making it impossible for an observer to determine which inputs are real and which are decoys. The default decoy count is 7 inputs and 2 outputs, providing an anonymity set of 8x for the sender and 3x for the receiver per transaction."]),
    p_runs([bold("Network-Layer Protection."), " Transaction submission uses a mixnet relay protocol where transactions are encrypted in layers and routed through a series of relay nodes before reaching a block producer. Each relay strips one encryption layer and introduces random delay (50-500ms), breaking the correlation between the submitting IP address and the transaction's appearance in a block. Validators accept transactions only from the mixnet output, never directly from end-user clients."]),
    p_runs([bold("Timing Decorrelation."), " Block producers aggregate transactions into fixed-size batches at regular intervals (200ms) rather than publishing transactions as they arrive. This constant-rate batching eliminates timing-based correlation attacks where an observer monitors transaction arrival times to link transactions to specific users or events."]),

    // --- 15.4 Privacy Guarantees by Transaction Type ---
    h2("15.4 Privacy Guarantees by Transaction Type"),
    table(
      ["Transaction Type", "Sender Hidden", "Receiver Hidden", "Amount Hidden", "Asset Type Hidden", "Compliance Proven"],
      [
        ["Private Transfer", "Yes", "Yes", "Yes", "Yes", "ZK predicate"],
        ["Entity Formation", "No (public registry)", "N/A", "N/A", "N/A", "ZK predicate"],
        ["Corridor Settlement", "Yes", "Yes", "Yes (net only)", "Yes", "ZK predicate"],
        ["Tax Withholding", "Yes", "Yes", "Yes", "Yes", "ZK predicate + CVK escrow"],
        ["Regulatory Disclosure", "Disclosed to CVK holder", "Disclosed to CVK holder", "Disclosed to CVK holder", "Disclosed to CVK holder", "Full audit via CVK"],
        ["Cross-Harbor Orbit", "Yes (origin Harbor)", "Yes (dest Harbor)", "Yes", "Yes", "ZK predicate (both jurisdictions)"],
      ],
      [1600, 1400, 1500, 1400, 1600, 1860]
    ),
    spacer(),
  ];
};
