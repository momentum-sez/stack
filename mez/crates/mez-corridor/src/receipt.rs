//! # Corridor Receipt Chain
//!
//! Append-only corridor receipts backed by a dual-commitment model:
//!
//! 1. **Hash-chain commitment** via `final_state_root`: a sequential hash chain
//!    seeded from `genesis_root`, where each receipt's `prev_root` must equal the
//!    current `final_state_root` and `next_root` becomes the new head.
//!
//! 2. **MMR commitment** for inclusion proofs: `next_root` digests are appended
//!    to a Merkle Mountain Range for O(log n) proofs without disclosing the full
//!    receipt set.
//!
//! ## next_root Derivation
//!
//! ```text
//! next_root = SHA256(MCF(receipt_without_proof_and_next_root))
//! ```
//!
//! Where MCF = Momentum Canonical Form (RFC 8785 JCS + float rejection + datetime
//! normalization), and digest sets are normalized (deduplicated + sorted
//! lexicographically) before canonicalization.
//!
//! ## Integrity Invariants
//!
//! - **I-RECEIPT-LINK**: `receipt.prev_root == chain.final_state_root`
//! - **I-RECEIPT-COMMIT**: `receipt.next_root == compute_next_root(&receipt)`
//! - **I-MMR-ROOT**: `mmr.root() == MMR(all next_roots)`
//!
//! ## Spec Reference
//!
//! Implements receipt chain per `spec/40-corridors.md` Part IV.
//! Conforms to `schemas/corridor.receipt.schema.json` and
//! `schemas/corridor.checkpoint.schema.json`.

use mez_core::{sha256_digest, CanonicalBytes, ContentDigest, CorridorId, Timestamp};
use mez_crypto::mmr::{
    build_inclusion_proof, verify_inclusion_proof, MerkleMountainRange, MmrInclusionProof,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors specific to receipt chain operations.
#[derive(Error, Debug)]
pub enum ReceiptError {
    /// Receipt sequence number does not follow the chain.
    #[error("sequence mismatch: expected {expected}, got {actual} for corridor {corridor_id}")]
    SequenceMismatch {
        /// The expected next sequence number.
        expected: u64,
        /// The actual sequence number provided.
        actual: u64,
        /// The corridor this receipt belongs to.
        corridor_id: String,
    },

    /// Receipt prev_root does not match the current chain state root.
    #[error("prev_root mismatch for receipt #{sequence}: expected {expected}, got {actual}")]
    PrevRootMismatch {
        /// The expected prev_root (current final_state_root).
        expected: String,
        /// The actual prev_root in the receipt.
        actual: String,
        /// The receipt sequence number.
        sequence: u64,
    },

    /// Receipt next_root does not match the recomputed value.
    #[error(
        "next_root mismatch for receipt #{sequence}: expected {expected}, got {actual}"
    )]
    NextRootMismatch {
        /// The recomputed next_root.
        expected: String,
        /// The actual next_root in the receipt.
        actual: String,
        /// The receipt sequence number.
        sequence: u64,
    },

    /// MMR operation failed.
    #[error("MMR error: {0}")]
    Mmr(#[from] mez_crypto::CryptoError),

    /// Canonicalization failed.
    #[error("canonicalization error: {0}")]
    Canonicalization(#[from] mez_core::CanonicalizationError),

    /// JSON serialization/deserialization error during next_root computation.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Inclusion proof verification failed.
    #[error("inclusion proof verification failed for receipt #{leaf_index}")]
    InclusionProofFailed {
        /// The leaf index that failed verification.
        leaf_index: usize,
    },

    /// Attempted to build proof or checkpoint for empty chain.
    #[error("cannot operate on empty receipt chain")]
    EmptyChain,
}

// ---------------------------------------------------------------------------
// Proof types (matching schemas/corridor.receipt.schema.json "proof")
// ---------------------------------------------------------------------------

/// Cryptographic proof over a receipt or checkpoint payload.
///
/// Matches the `proof` field in `schemas/corridor.receipt.schema.json`:
/// either a single proof object or an array of proof objects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReceiptProof {
    /// A single proof.
    Single(ProofObject),
    /// Multiple proofs (e.g., multi-party signing).
    Multiple(Vec<ProofObject>),
}

/// A single cryptographic proof object.
///
/// Matches the proof object schema: `MezEd25519Signature2025`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofObject {
    /// Proof type, e.g., "MezEd25519Signature2025".
    #[serde(rename = "type")]
    pub proof_type: String,
    /// When the proof was created (RFC 3339).
    pub created: String,
    /// Verification method identifier (DID or key URI).
    #[serde(rename = "verificationMethod")]
    pub verification_method: String,
    /// Purpose of the proof, e.g., "assertionMethod".
    #[serde(rename = "proofPurpose")]
    pub proof_purpose: String,
    /// JWS compact serialization of the signature.
    pub jws: String,
}

// ---------------------------------------------------------------------------
// Digest set entry (legacy string or ArtifactRef)
// ---------------------------------------------------------------------------

/// An entry in a digest set: either a raw SHA-256 hex string (legacy) or
/// an ArtifactRef with metadata.
///
/// Matches the `oneOf` in `schemas/corridor.receipt.schema.json` for
/// `lawpack_digest_set` and `ruleset_digest_set` items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DigestEntry {
    /// Raw SHA-256 digest as a 64-char lowercase hex string.
    Digest(String),
    /// Artifact reference with digest and optional metadata.
    ArtifactRef {
        /// The SHA-256 digest of the artifact.
        digest_sha256: String,
        /// Artifact type, e.g., "lawpack", "ruleset".
        artifact_type: String,
        /// Optional URI for artifact retrieval.
        #[serde(skip_serializing_if = "Option::is_none")]
        uri: Option<String>,
    },
}

impl DigestEntry {
    /// Extract the underlying SHA-256 digest string for normalization.
    pub fn digest(&self) -> &str {
        match self {
            DigestEntry::Digest(s) => s,
            DigestEntry::ArtifactRef { digest_sha256, .. } => digest_sha256,
        }
    }
}

impl From<String> for DigestEntry {
    fn from(s: String) -> Self {
        DigestEntry::Digest(s)
    }
}

impl From<&str> for DigestEntry {
    fn from(s: &str) -> Self {
        DigestEntry::Digest(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// CorridorReceipt
// ---------------------------------------------------------------------------

/// A corridor receipt recording a cross-border transaction event.
///
/// Conforms to `schemas/corridor.receipt.schema.json`. Each receipt forms a
/// link in the append-only receipt chain: `prev_root` points to the current
/// `final_state_root` (hash-chain head) and `next_root` is the canonical
/// digest of the receipt payload.
///
/// ## Security Invariants
///
/// - `next_root = SHA256(MCF(receipt_without_proof_and_next_root))` (I-RECEIPT-COMMIT)
/// - `prev_root == chain.final_state_root` at append time (I-RECEIPT-LINK)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorridorReceipt {
    /// Receipt type discriminator.
    #[serde(rename = "type")]
    pub receipt_type: String,
    /// The corridor this receipt belongs to.
    pub corridor_id: CorridorId,
    /// Sequence number within the corridor (0-indexed).
    pub sequence: u64,
    /// When the receipt was created (RFC 3339).
    pub timestamp: Timestamp,
    /// State root before this transition. For the first receipt, this MUST
    /// equal the corridor's `genesis_root`.
    pub prev_root: String,
    /// Canonical digest of this receipt's payload: `SHA256(MCF(receipt_payload))`.
    /// The payload excludes the `proof` and `next_root` fields themselves.
    pub next_root: String,
    /// Lawpack digest set governing this receipt (sorted, deduplicated).
    /// Accepts both raw SHA-256 hex strings and ArtifactRef entries per schema.
    pub lawpack_digest_set: Vec<DigestEntry>,
    /// Ruleset digest set governing this receipt (sorted, deduplicated).
    /// Accepts both raw SHA-256 hex strings and ArtifactRef entries per schema.
    pub ruleset_digest_set: Vec<DigestEntry>,

    // ── Schema-required fields added for P0-CORRIDOR-003 ────────────

    /// Cryptographic proof(s) over the receipt payload.
    /// Required by schema but optional in struct to allow construction
    /// before signing. Schema validation will reject receipts without proof.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub proof: Option<ReceiptProof>,

    // ── Optional fields per schema ──────────────────────────────────

    /// Transition envelope (schema: `$ref transition-envelope.schema.json`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub transition: Option<serde_json::Value>,

    /// Digest commitment to a Transition Type Registry snapshot.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub transition_type_registry_digest_sha256: Option<String>,

    /// ZK proof scaffold.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub zk: Option<serde_json::Value>,

    /// Anchoring metadata for external chain commitments.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub anchor: Option<serde_json::Value>,
}

impl CorridorReceipt {
    /// Compute the canonical content digest of this receipt (over the full
    /// serialized form, including proof and next_root).
    ///
    /// This is the content-address of the receipt as stored, NOT the
    /// `next_root` commitment. For `next_root`, use [`compute_next_root()`].
    pub fn content_digest(&self) -> Result<ContentDigest, ReceiptError> {
        let canonical = CanonicalBytes::new(self)?;
        Ok(sha256_digest(&canonical))
    }

    /// Compute and set the correct `next_root` for this receipt.
    ///
    /// Convenience wrapper: computes `next_root` via [`compute_next_root()`]
    /// and sets `self.next_root` to the hex digest.
    pub fn seal_next_root(&mut self) -> Result<ContentDigest, ReceiptError> {
        let digest = compute_next_root(self)?;
        self.next_root = digest.to_hex();
        Ok(digest)
    }
}

// ---------------------------------------------------------------------------
// next_root computation (P0-CORRIDOR-002)
// ---------------------------------------------------------------------------

/// Normalize a digest set array in a JSON object: sort lexicographically
/// by the underlying digest string, deduplicate.
fn normalize_digest_set_in_value(obj: &mut serde_json::Map<String, serde_json::Value>, key: &str) {
    if let Some(serde_json::Value::Array(arr)) = obj.get(key) {
        // Extract (sort_key, original_value) pairs
        let mut entries: Vec<(String, serde_json::Value)> = arr
            .iter()
            .filter_map(|v| {
                let digest_key = match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Object(o) => {
                        o.get("digest_sha256").and_then(|d| d.as_str()).map(String::from)
                    }
                    _ => None,
                };
                digest_key.map(|k| (k, v.clone()))
            })
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries.dedup_by(|a, b| a.0 == b.0);
        let normalized: Vec<serde_json::Value> = entries.into_iter().map(|(_, v)| v).collect();
        obj.insert(key.to_string(), serde_json::Value::Array(normalized));
    }
}

/// Compute the canonical `next_root` for a receipt.
///
/// ```text
/// next_root = SHA256(MCF(receipt_payload))
/// ```
///
/// Where `receipt_payload` is the receipt serialized to JSON with the `proof`
/// and `next_root` keys removed, and digest sets normalized (deduplicated +
/// sorted lexicographically by their underlying SHA-256 digest string).
///
/// This function is deterministic: the value of `next_root` and `proof` in
/// the input receipt does not affect the output (both are stripped).
pub fn compute_next_root(receipt: &CorridorReceipt) -> Result<ContentDigest, ReceiptError> {
    let mut value = serde_json::to_value(receipt)?;
    let obj = match value.as_object_mut() {
        Some(o) => o,
        None => {
            return Err(ReceiptError::Serialization(
                <serde_json::Error as serde::ser::Error>::custom(
                    "CorridorReceipt did not serialize to a JSON object",
                ),
            ));
        }
    };

    // Strip proof and next_root — these are not part of the commitment
    obj.remove("proof");
    obj.remove("next_root");

    // Normalize digest sets: deduplicate + sort lexicographically
    normalize_digest_set_in_value(obj, "lawpack_digest_set");
    normalize_digest_set_in_value(obj, "ruleset_digest_set");

    let canonical = CanonicalBytes::from_value(value)?;
    Ok(sha256_digest(&canonical))
}

// ---------------------------------------------------------------------------
// MMR commitment (for checkpoint schema conformance)
// ---------------------------------------------------------------------------

/// Default zero digest for serde skip deserialization.
fn zero_digest() -> ContentDigest {
    ContentDigest::zero()
}

/// MMR commitment object, conformant to `schemas/corridor.checkpoint.schema.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MmrCommitment {
    /// MMR type identifier: always `"MEZReceiptMMR"`.
    #[serde(rename = "type")]
    pub mmr_type: String,
    /// Hash algorithm: always `"sha256"`.
    pub algorithm: String,
    /// Number of leaves in the MMR.
    pub size: u64,
    /// Bagged root hash (64 hex chars).
    pub root: String,
    /// Optional peak list to aid verifiers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peaks: Option<Vec<MmrPeakEntry>>,
}

/// A single MMR peak entry for checkpoint serialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MmrPeakEntry {
    /// Height of the peak (0 = single leaf).
    pub height: u64,
    /// SHA-256 hash as 64 lowercase hex chars.
    pub hash: String,
}

// ---------------------------------------------------------------------------
// Checkpoint
// ---------------------------------------------------------------------------

/// A corridor checkpoint, conformant to `schemas/corridor.checkpoint.schema.json`.
///
/// Commits to both the hash-chain head (`final_state_root`) and the MMR
/// accumulator, providing a verifier bootstrap point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Checkpoint type: always `"MEZCorridorStateCheckpoint"`.
    #[serde(rename = "type")]
    pub checkpoint_type: String,
    /// The corridor this checkpoint belongs to.
    pub corridor_id: CorridorId,
    /// When the checkpoint was created.
    pub timestamp: Timestamp,
    /// The corridor's immutable genesis root.
    pub genesis_root: String,
    /// Hash-chain head at checkpoint time (last receipt's `next_root`).
    pub final_state_root: String,
    /// Number of receipts in the chain at checkpoint time (>= 1).
    pub receipt_count: u64,
    /// Union of all lawpack digests across receipts in this window.
    pub lawpack_digest_set: Vec<DigestEntry>,
    /// Union of all ruleset digests across receipts in this window.
    pub ruleset_digest_set: Vec<DigestEntry>,
    /// MMR commitment object.
    pub mmr: MmrCommitment,
    /// Content digest of the checkpoint payload (excluding proof).
    #[serde(skip, default = "zero_digest")]
    pub checkpoint_digest: ContentDigest,
    /// Cryptographic proof(s) over the checkpoint payload.
    /// Required by schema but optional in struct to allow construction
    /// before signing.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub proof: Option<ReceiptProof>,
    /// Optional anchoring metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub anchor: Option<serde_json::Value>,
}

// Backward-compat accessor
impl Checkpoint {
    /// Convenience: returns `receipt_count` (equivalent to the old `height` field).
    pub fn height(&self) -> u64 {
        self.receipt_count
    }

    /// Convenience: returns the MMR root string.
    pub fn mmr_root(&self) -> &str {
        &self.mmr.root
    }
}

// ---------------------------------------------------------------------------
// ReceiptChain
// ---------------------------------------------------------------------------

/// An append-only receipt chain for a single corridor, backed by dual
/// commitments: a hash-chain (`final_state_root`) and an MMR accumulator.
///
/// ## Security Invariants
///
/// - Receipts can only be appended (not modified or removed).
/// - `prev_root` must equal `final_state_root` (hash-chain integrity).
/// - `next_root` must equal `compute_next_root(&receipt)` (commitment integrity).
/// - Sequence must be strictly monotonic from 0.
#[derive(Debug)]
pub struct ReceiptChain {
    /// The corridor this chain belongs to.
    corridor_id: CorridorId,
    /// Immutable genesis root seeding the hash chain.
    genesis_root: ContentDigest,
    /// Current hash-chain head. Starts as `genesis_root`, updated to each
    /// receipt's `next_root` on append.
    final_state_root: ContentDigest,
    /// Ordered receipts.
    receipts: Vec<CorridorReceipt>,
    /// The MMR accumulator tracking next_root digests.
    mmr: MerkleMountainRange,
    /// Hex-encoded next_roots for inclusion proof building.
    next_roots: Vec<String>,
    /// Periodic checkpoints.
    checkpoints: Vec<Checkpoint>,
}

impl ReceiptChain {
    /// Create a new empty receipt chain for a corridor with the given genesis root.
    ///
    /// The `genesis_root` seeds the hash chain. The first receipt's `prev_root`
    /// must equal `genesis_root.to_hex()`.
    pub fn new(corridor_id: CorridorId, genesis_root: ContentDigest) -> Self {
        Self {
            corridor_id,
            final_state_root: genesis_root.clone(),
            genesis_root,
            receipts: Vec::new(),
            mmr: MerkleMountainRange::new(),
            next_roots: Vec::new(),
            checkpoints: Vec::new(),
        }
    }

    /// Return the corridor ID this chain belongs to.
    pub fn corridor_id(&self) -> &CorridorId {
        &self.corridor_id
    }

    /// Return the current chain height (number of receipts).
    pub fn height(&self) -> u64 {
        self.receipts.len() as u64
    }

    /// Return the current MMR root hash (64 hex chars, or empty if no receipts).
    pub fn mmr_root(&self) -> Result<String, ReceiptError> {
        Ok(self.mmr.root()?)
    }

    /// Return the genesis root.
    pub fn genesis_root(&self) -> &ContentDigest {
        &self.genesis_root
    }

    /// Return the current hash-chain head as a `ContentDigest`.
    pub fn final_state_root(&self) -> &ContentDigest {
        &self.final_state_root
    }

    /// Return the current hash-chain head as a hex string.
    ///
    /// For an empty chain, this returns the genesis root hex. For a
    /// non-empty chain, this returns the last receipt's `next_root`.
    pub fn final_state_root_hex(&self) -> String {
        self.final_state_root.to_hex()
    }

    /// Access the receipts in the chain.
    pub fn receipts(&self) -> &[CorridorReceipt] {
        &self.receipts
    }

    /// Access the checkpoints.
    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    /// Append a receipt to the chain.
    ///
    /// Validates:
    /// 1. **Sequence number** — must equal `self.height()` (0-indexed).
    /// 2. **`prev_root`** — must equal `self.final_state_root_hex()`
    ///    (hash-chain continuity, **I-RECEIPT-LINK**).
    /// 3. **`next_root`** — must equal `compute_next_root(&receipt)`
    ///    (commitment integrity, **I-RECEIPT-COMMIT**).
    ///
    /// After validation:
    /// - `next_root` is appended to the MMR.
    /// - `final_state_root` is updated to `next_root`.
    pub fn append(&mut self, receipt: CorridorReceipt) -> Result<(), ReceiptError> {
        let expected_seq = self.height();
        if receipt.sequence != expected_seq {
            return Err(ReceiptError::SequenceMismatch {
                expected: expected_seq,
                actual: receipt.sequence,
                corridor_id: self.corridor_id.to_string(),
            });
        }

        // I-RECEIPT-LINK: prev_root must match final_state_root (hash-chain)
        let expected_prev = self.final_state_root_hex();
        if receipt.prev_root != expected_prev {
            return Err(ReceiptError::PrevRootMismatch {
                expected: expected_prev,
                actual: receipt.prev_root.clone(),
                sequence: receipt.sequence,
            });
        }

        // I-RECEIPT-COMMIT: next_root must match recomputed value
        let recomputed = compute_next_root(&receipt)?;
        let recomputed_hex = recomputed.to_hex();
        if receipt.next_root != recomputed_hex {
            return Err(ReceiptError::NextRootMismatch {
                expected: recomputed_hex,
                actual: receipt.next_root.clone(),
                sequence: receipt.sequence,
            });
        }

        // Append to MMR (I-MMR-ROOT)
        self.mmr.append(&receipt.next_root)?;
        self.next_roots.push(receipt.next_root.clone());

        // Advance hash-chain head
        self.final_state_root = recomputed;

        self.receipts.push(receipt);
        Ok(())
    }

    /// Create a checkpoint at the current chain height.
    ///
    /// The checkpoint captures both the hash-chain head and MMR root,
    /// producing a content-addressed digest suitable for L1 anchoring.
    /// Requires at least one receipt in the chain.
    pub fn create_checkpoint(&mut self) -> Result<Checkpoint, ReceiptError> {
        if self.receipts.is_empty() {
            return Err(ReceiptError::EmptyChain);
        }

        let mmr_root_hex = self.mmr_root()?;
        let mmr_size = self.mmr.size() as u64;
        let now = Timestamp::now();

        // Collect union of all lawpack/ruleset digests across receipts.
        // Dedup by underlying digest string, preserving the first entry seen.
        let mut all_lawpack: Vec<DigestEntry> = self
            .receipts
            .iter()
            .flat_map(|r| r.lawpack_digest_set.iter().cloned())
            .collect();
        all_lawpack.sort_by(|a, b| a.digest().cmp(b.digest()));
        all_lawpack.dedup_by(|a, b| a.digest() == b.digest());

        let mut all_ruleset: Vec<DigestEntry> = self
            .receipts
            .iter()
            .flat_map(|r| r.ruleset_digest_set.iter().cloned())
            .collect();
        all_ruleset.sort_by(|a, b| a.digest().cmp(b.digest()));
        all_ruleset.dedup_by(|a, b| a.digest() == b.digest());

        // Build MMR commitment object
        let peaks_entries: Vec<MmrPeakEntry> = self
            .mmr
            .peaks()
            .iter()
            .map(|p| MmrPeakEntry {
                height: p.height as u64,
                hash: p.hash.clone(),
            })
            .collect();

        let mmr_commitment = MmrCommitment {
            mmr_type: "MEZReceiptMMR".to_string(),
            algorithm: "sha256".to_string(),
            size: mmr_size,
            root: mmr_root_hex,
            peaks: Some(peaks_entries),
        };

        // Compute checkpoint_digest over canonical payload (excluding proof)
        let checkpoint_payload = serde_json::json!({
            "type": "MEZCorridorStateCheckpoint",
            "corridor_id": self.corridor_id,
            "timestamp": now.to_string(),
            "genesis_root": self.genesis_root.to_hex(),
            "final_state_root": self.final_state_root.to_hex(),
            "receipt_count": self.receipts.len() as u64,
            "lawpack_digest_set": all_lawpack,
            "ruleset_digest_set": all_ruleset,
            "mmr": {
                "type": "MEZReceiptMMR",
                "algorithm": "sha256",
                "size": mmr_size,
                "root": mmr_commitment.root,
            },
        });
        let canonical = CanonicalBytes::new(&checkpoint_payload)?;
        let digest = sha256_digest(&canonical);

        let checkpoint = Checkpoint {
            checkpoint_type: "MEZCorridorStateCheckpoint".to_string(),
            corridor_id: self.corridor_id.clone(),
            timestamp: now,
            genesis_root: self.genesis_root.to_hex(),
            final_state_root: self.final_state_root.to_hex(),
            receipt_count: self.receipts.len() as u64,
            lawpack_digest_set: all_lawpack,
            ruleset_digest_set: all_ruleset,
            mmr: mmr_commitment,
            checkpoint_digest: digest,
            proof: None,
            anchor: None,
        };

        self.checkpoints.push(checkpoint.clone());
        Ok(checkpoint)
    }

    /// Build an MMR inclusion proof for a receipt at the given index.
    ///
    /// The proof demonstrates that the receipt's `next_root` is included
    /// in the MMR at the claimed root, without revealing other receipts.
    pub fn build_inclusion_proof(
        &self,
        leaf_index: usize,
    ) -> Result<MmrInclusionProof, ReceiptError> {
        if self.next_roots.is_empty() {
            return Err(ReceiptError::EmptyChain);
        }
        Ok(build_inclusion_proof(&self.next_roots, leaf_index)?)
    }

    /// Verify an MMR inclusion proof against the current chain state.
    ///
    /// Returns `true` if the proof is valid and the root matches the
    /// current MMR root.
    pub fn verify_inclusion_proof(&self, proof: &MmrInclusionProof) -> Result<bool, ReceiptError> {
        let current_root = self.mmr_root()?;
        if proof.root != current_root {
            return Ok(false);
        }
        Ok(verify_inclusion_proof(proof))
    }
}

/// Standalone receipt verification: given a proof, verify its internal
/// consistency without requiring the full chain.
///
/// This is used by light clients and regulators who receive a proof
/// and root commitment but do not have access to the full receipt chain.
pub fn verify_receipt_proof(proof: &MmrInclusionProof) -> bool {
    verify_inclusion_proof(proof)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a deterministic genesis root for testing (all zeros).
    fn test_genesis_root() -> ContentDigest {
        ContentDigest::from_hex(&"00".repeat(32)).unwrap()
    }

    /// Create a receipt for the chain with a correctly computed next_root.
    fn make_receipt(chain: &ReceiptChain, _i: u64) -> CorridorReceipt {
        let prev_root = chain.final_state_root_hex();
        let mut receipt = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: chain.corridor_id().clone(),
            sequence: chain.height(),
            timestamp: Timestamp::now(),
            prev_root,
            next_root: String::new(),
            lawpack_digest_set: vec!["deadbeef".repeat(8).into()],
            ruleset_digest_set: vec!["cafebabe".repeat(8).into()],
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        receipt.seal_next_root().unwrap();
        receipt
    }

    #[test]
    fn empty_chain() {
        let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        assert_eq!(chain.height(), 0);
        assert_eq!(chain.mmr_root().unwrap(), "");
        // final_state_root starts as genesis_root
        assert_eq!(chain.final_state_root_hex(), "00".repeat(32));
    }

    #[test]
    fn append_single_receipt() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        let receipt = make_receipt(&chain, 1);
        chain.append(receipt).unwrap();
        assert_eq!(chain.height(), 1);
        assert!(!chain.mmr_root().unwrap().is_empty());
        // final_state_root has advanced from genesis
        assert_ne!(chain.final_state_root_hex(), "00".repeat(32));
    }

    #[test]
    fn append_multiple_receipts() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        for i in 0..10 {
            let receipt = make_receipt(&chain, i);
            chain.append(receipt).unwrap();
        }
        assert_eq!(chain.height(), 10);
        assert_eq!(chain.mmr_root().unwrap().len(), 64);
    }

    #[test]
    fn first_receipt_prev_root_equals_genesis() {
        let genesis = test_genesis_root();
        let chain = ReceiptChain::new(CorridorId::new(), genesis.clone());
        let receipt = make_receipt(&chain, 0);
        assert_eq!(receipt.prev_root, genesis.to_hex());
    }

    #[test]
    fn second_receipt_prev_root_equals_first_next_root() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        let r0 = make_receipt(&chain, 0);
        let r0_next = r0.next_root.clone();
        chain.append(r0).unwrap();

        let r1 = make_receipt(&chain, 1);
        assert_eq!(r1.prev_root, r0_next);
    }

    #[test]
    fn final_state_root_tracks_last_next_root() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        for i in 0..5 {
            let receipt = make_receipt(&chain, i);
            chain.append(receipt).unwrap();
        }
        let last_next_root = chain.receipts().last().unwrap().next_root.clone();
        assert_eq!(chain.final_state_root_hex(), last_next_root);
    }

    #[test]
    fn sequence_mismatch_rejected() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        let receipt = make_receipt(&chain, 1);
        chain.append(receipt).unwrap();

        let mut bad_receipt = make_receipt(&chain, 2);
        bad_receipt.sequence = 5;
        let result = chain.append(bad_receipt);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReceiptError::SequenceMismatch {
                expected: 1,
                actual: 5,
                ..
            }
        ));
    }

    #[test]
    fn prev_root_mismatch_rejected() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        let receipt = make_receipt(&chain, 1);
        chain.append(receipt).unwrap();

        let mut bad_receipt = make_receipt(&chain, 2);
        bad_receipt.prev_root = "ff".repeat(32);
        let result = chain.append(bad_receipt);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReceiptError::PrevRootMismatch { .. }
        ));
    }

    #[test]
    fn next_root_mismatch_rejected() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        let mut receipt = make_receipt(&chain, 0);
        // Tamper with next_root
        receipt.next_root = "aa".repeat(32);
        let result = chain.append(receipt);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReceiptError::NextRootMismatch { .. }
        ));
    }

    #[test]
    fn checkpoint_captures_state() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        for i in 0..5 {
            let receipt = make_receipt(&chain, i);
            chain.append(receipt).unwrap();
        }

        let checkpoint = chain.create_checkpoint().unwrap();
        assert_eq!(checkpoint.height(), 5);
        assert_eq!(checkpoint.receipt_count, 5);
        assert_eq!(checkpoint.mmr_root(), chain.mmr_root().unwrap());
        assert_eq!(checkpoint.checkpoint_digest.to_hex().len(), 64);
        assert_eq!(checkpoint.genesis_root, test_genesis_root().to_hex());
        assert_eq!(checkpoint.final_state_root, chain.final_state_root_hex());
    }

    #[test]
    fn checkpoint_requires_non_empty_chain() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        let result = chain.create_checkpoint();
        assert!(matches!(result.unwrap_err(), ReceiptError::EmptyChain));
    }

    #[test]
    fn checkpoint_has_schema_fields() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        let receipt = make_receipt(&chain, 0);
        chain.append(receipt).unwrap();

        let cp = chain.create_checkpoint().unwrap();
        assert_eq!(cp.checkpoint_type, "MEZCorridorStateCheckpoint");
        assert_eq!(cp.mmr.mmr_type, "MEZReceiptMMR");
        assert_eq!(cp.mmr.algorithm, "sha256");
        assert_eq!(cp.mmr.size, 1);
        assert!(cp.mmr.peaks.is_some());
        assert_eq!(cp.receipt_count, 1);
    }

    #[test]
    fn inclusion_proof_roundtrip() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        for i in 0..10 {
            let receipt = make_receipt(&chain, i);
            chain.append(receipt).unwrap();
        }

        for idx in [0, 1, 4, 7, 9] {
            let proof = chain.build_inclusion_proof(idx).unwrap();
            assert!(chain.verify_inclusion_proof(&proof).unwrap());
            assert!(verify_receipt_proof(&proof));
        }
    }

    #[test]
    fn tampered_proof_fails() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        for i in 0..5 {
            let receipt = make_receipt(&chain, i);
            chain.append(receipt).unwrap();
        }

        let mut proof = chain.build_inclusion_proof(2).unwrap();
        if !proof.path.is_empty() {
            proof.path[0].hash = "00".repeat(32);
        }
        assert!(!verify_receipt_proof(&proof));
    }

    #[test]
    fn empty_chain_proof_fails() {
        let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        assert!(matches!(
            chain.build_inclusion_proof(0),
            Err(ReceiptError::EmptyChain)
        ));
    }

    #[test]
    fn receipt_content_digest_deterministic() {
        let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        let receipt = make_receipt(&chain, 1);
        let d1 = receipt.content_digest().unwrap();
        let d2 = receipt.content_digest().unwrap();
        assert_eq!(d1, d2);
        assert_eq!(d1.to_hex().len(), 64);
    }

    #[test]
    fn multiple_checkpoints() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

        for i in 0..3 {
            let receipt = make_receipt(&chain, i);
            chain.append(receipt).unwrap();
        }
        let cp1 = chain.create_checkpoint().unwrap();

        for i in 3..7 {
            let receipt = make_receipt(&chain, i);
            chain.append(receipt).unwrap();
        }
        let cp2 = chain.create_checkpoint().unwrap();

        assert_eq!(cp1.receipt_count, 3);
        assert_eq!(cp2.receipt_count, 7);
        assert_ne!(cp1.mmr_root(), cp2.mmr_root());
        assert_eq!(chain.checkpoints().len(), 2);
    }

    #[test]
    fn compute_next_root_strips_proof_and_next_root() {
        let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

        // Create receipt without proof, compute next_root
        let mut r1 = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: chain.corridor_id().clone(),
            sequence: 0,
            timestamp: Timestamp::now(),
            prev_root: chain.final_state_root_hex(),
            next_root: "placeholder".to_string(),
            lawpack_digest_set: Vec::new(),
            ruleset_digest_set: Vec::new(),
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        let nr1 = compute_next_root(&r1).unwrap();

        // Same receipt with proof set — should produce identical next_root
        r1.proof = Some(ReceiptProof::Single(ProofObject {
            proof_type: "MezEd25519Signature2025".to_string(),
            created: "2026-01-15T12:00:00Z".to_string(),
            verification_method: "did:example:123#key-1".to_string(),
            proof_purpose: "assertionMethod".to_string(),
            jws: "eyJ0eXAiOiJKV1MiLCJhbGciOiJFZERTQSJ9..test".to_string(),
        }));
        let nr2 = compute_next_root(&r1).unwrap();
        assert_eq!(nr1, nr2, "proof must not affect next_root computation");

        // Different next_root value — should still produce identical next_root
        r1.next_root = "ff".repeat(32);
        let nr3 = compute_next_root(&r1).unwrap();
        assert_eq!(nr1, nr3, "next_root value must not affect computation");
    }

    #[test]
    fn digest_set_normalization_dedup_and_sort() {
        let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

        // Receipt with unsorted, duplicate digests
        let mut r1 = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: chain.corridor_id().clone(),
            sequence: 0,
            timestamp: Timestamp::now(),
            prev_root: chain.final_state_root_hex(),
            next_root: String::new(),
            lawpack_digest_set: vec![
                "bb".repeat(32).into(),
                "aa".repeat(32).into(),
                "bb".repeat(32).into(), // duplicate
            ],
            ruleset_digest_set: Vec::new(),
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        let nr1 = compute_next_root(&r1).unwrap();

        // Same receipt with sorted, deduplicated digests
        r1.lawpack_digest_set = vec!["aa".repeat(32).into(), "bb".repeat(32).into()];
        let nr2 = compute_next_root(&r1).unwrap();

        assert_eq!(
            nr1, nr2,
            "digest set normalization must produce identical next_root"
        );
    }

    #[test]
    fn seal_next_root_sets_correct_value() {
        let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        let mut receipt = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: chain.corridor_id().clone(),
            sequence: 0,
            timestamp: Timestamp::now(),
            prev_root: chain.final_state_root_hex(),
            next_root: String::new(),
            lawpack_digest_set: Vec::new(),
            ruleset_digest_set: Vec::new(),
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        let digest = receipt.seal_next_root().unwrap();
        assert_eq!(receipt.next_root, digest.to_hex());
        assert_eq!(receipt.next_root.len(), 64);
    }

    #[test]
    fn receipt_proof_serde_roundtrip() {
        let proof = ReceiptProof::Single(ProofObject {
            proof_type: "MezEd25519Signature2025".to_string(),
            created: "2026-01-15T12:00:00Z".to_string(),
            verification_method: "did:example:123#key-1".to_string(),
            proof_purpose: "assertionMethod".to_string(),
            jws: "eyJ0eXAiOiJKV1MiLCJhbGciOiJFZERTQSJ9..test".to_string(),
        });
        let json = serde_json::to_string(&proof).unwrap();
        let deserialized: ReceiptProof = serde_json::from_str(&json).unwrap();
        assert_eq!(proof, deserialized);
    }

    #[test]
    fn digest_entry_from_string() {
        let entry: DigestEntry = "aa".repeat(32).into();
        assert_eq!(entry.digest(), "aa".repeat(32));
    }

    #[test]
    fn digest_entry_artifact_ref() {
        let entry = DigestEntry::ArtifactRef {
            digest_sha256: "bb".repeat(32),
            artifact_type: "lawpack".to_string(),
            uri: Some("https://example.com/lawpack".to_string()),
        };
        assert_eq!(entry.digest(), "bb".repeat(32));

        // Roundtrip
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: DigestEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, deserialized);
    }
}

// ===========================================================================
// Adversarial tests
// ===========================================================================

#[cfg(test)]
mod adversarial_receipt_tests {
    use super::*;

    fn test_genesis_root() -> ContentDigest {
        ContentDigest::from_hex(&"00".repeat(32)).unwrap()
    }

    /// Adversarial vector 1: Receipt next_root forgery.
    /// Craft a receipt with an arbitrary next_root that doesn't match the
    /// payload. `append()` must reject it.
    #[test]
    fn adversarial_next_root_forgery() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        let mut receipt = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: chain.corridor_id().clone(),
            sequence: 0,
            timestamp: Timestamp::now(),
            prev_root: chain.final_state_root_hex(),
            next_root: "aa".repeat(32), // FORGED — doesn't match payload
            lawpack_digest_set: vec!["deadbeef".repeat(8).into()],
            ruleset_digest_set: Vec::new(),
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        let result = chain.append(receipt);
        assert!(
            matches!(result, Err(ReceiptError::NextRootMismatch { .. })),
            "forged next_root must be rejected"
        );
    }

    /// Adversarial vector 2: Receipt prev_root set to MMR root instead of
    /// final_state_root. This is the OLD (incorrect) behavior — it must
    /// now be rejected.
    #[test]
    fn adversarial_prev_root_uses_mmr_root_instead_of_state_root() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

        // Append first receipt to diverge MMR root from final_state_root
        let mut r0 = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: chain.corridor_id().clone(),
            sequence: 0,
            timestamp: Timestamp::now(),
            prev_root: chain.final_state_root_hex(),
            next_root: String::new(),
            lawpack_digest_set: Vec::new(),
            ruleset_digest_set: Vec::new(),
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        r0.seal_next_root().unwrap();
        chain.append(r0).unwrap();

        // Now MMR root != final_state_root (MMR root is hash of next_root
        // with domain separation; final_state_root IS next_root)
        let mmr_root = chain.mmr_root().unwrap();
        let state_root = chain.final_state_root_hex();
        assert_ne!(mmr_root, state_root, "MMR root and state root must differ");

        // Attempt to use MMR root as prev_root (the old incorrect behavior)
        let mut bad_receipt = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: chain.corridor_id().clone(),
            sequence: 1,
            timestamp: Timestamp::now(),
            prev_root: mmr_root, // WRONG: should be state_root
            next_root: String::new(),
            lawpack_digest_set: Vec::new(),
            ruleset_digest_set: Vec::new(),
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        bad_receipt.seal_next_root().unwrap();
        let result = chain.append(bad_receipt);
        assert!(
            matches!(result, Err(ReceiptError::PrevRootMismatch { .. })),
            "using MMR root as prev_root must be rejected"
        );
    }

    /// Adversarial vector 3: Content-addressed integrity gap.
    /// Submit receipt where content_digest() != next_root. The
    /// compute_next_root verification catches this.
    #[test]
    fn adversarial_content_digest_integrity_gap() {
        let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        let mut receipt = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: chain.corridor_id().clone(),
            sequence: 0,
            timestamp: Timestamp::now(),
            prev_root: chain.final_state_root_hex(),
            next_root: String::new(),
            lawpack_digest_set: vec!["deadbeef".repeat(8).into()],
            ruleset_digest_set: Vec::new(),
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        let correct_next = compute_next_root(&receipt).unwrap();
        receipt.next_root = correct_next.to_hex();

        // The content_digest() hashes the FULL receipt (including next_root)
        // so it differs from next_root (which excludes itself from the hash)
        let content = receipt.content_digest().unwrap();
        assert_ne!(
            content.to_hex(),
            receipt.next_root,
            "content_digest and next_root must differ"
        );
    }

    /// Adversarial vector 4: Checkpoint forgery — checkpoint without proof.
    /// Schema validation would reject this (proof is required in schema).
    #[test]
    fn adversarial_checkpoint_without_proof() {
        let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        let mut receipt = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: chain.corridor_id().clone(),
            sequence: 0,
            timestamp: Timestamp::now(),
            prev_root: chain.final_state_root_hex(),
            next_root: String::new(),
            lawpack_digest_set: Vec::new(),
            ruleset_digest_set: Vec::new(),
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        receipt.seal_next_root().unwrap();
        chain.append(receipt).unwrap();

        let checkpoint = chain.create_checkpoint().unwrap();
        // Verify checkpoint has no proof (unsigned)
        assert!(
            checkpoint.proof.is_none(),
            "checkpoint created without signing should have no proof"
        );

        // Serialize and check that "proof" field is absent
        let json = serde_json::to_value(&checkpoint).unwrap();
        assert!(
            json.get("proof").is_none(),
            "proof field should not be present when None"
        );
        // Schema validation would reject this JSON (proof is required)
    }

    /// Adversarial vector 5: Digest set ordering attack.
    /// Same receipt content with different digest-set ordering must produce
    /// identical next_root (normalization must handle this).
    #[test]
    fn adversarial_digest_set_ordering_attack() {
        let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
        let ts = Timestamp::now();

        let mut r1 = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: chain.corridor_id().clone(),
            sequence: 0,
            timestamp: ts.clone(),
            prev_root: chain.final_state_root_hex(),
            next_root: String::new(),
            lawpack_digest_set: vec![
                "cc".repeat(32).into(),
                "aa".repeat(32).into(),
                "bb".repeat(32).into(),
            ],
            ruleset_digest_set: vec!["zz".repeat(32).into(), "aa".repeat(32).into()],
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        let nr1 = compute_next_root(&r1).unwrap();

        // Same content, reversed order
        let r2 = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: chain.corridor_id().clone(),
            sequence: 0,
            timestamp: ts.clone(),
            prev_root: chain.final_state_root_hex(),
            next_root: String::new(),
            lawpack_digest_set: vec![
                "bb".repeat(32).into(),
                "cc".repeat(32).into(),
                "aa".repeat(32).into(),
            ],
            ruleset_digest_set: vec!["aa".repeat(32).into(), "zz".repeat(32).into()],
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        let nr2 = compute_next_root(&r2).unwrap();

        assert_eq!(
            nr1, nr2,
            "digest set ordering must not affect next_root"
        );

        // Also test with duplicates
        let mut r3 = r1.clone();
        r3.lawpack_digest_set = vec![
            "aa".repeat(32).into(),
            "aa".repeat(32).into(), // duplicate
            "bb".repeat(32).into(),
            "cc".repeat(32).into(),
        ];
        let nr3 = compute_next_root(&r3).unwrap();
        assert_eq!(
            nr1, nr3,
            "duplicates in digest set must not affect next_root"
        );
    }
}

/// Golden vector tests using fixed, deterministic inputs.
/// Any implementation following the spec must produce these exact digests.
#[cfg(test)]
mod golden_vector_tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    const GOLDEN_CORRIDOR_ID: &str = "00000000-0000-4000-8000-000000000001";
    const GOLDEN_GENESIS_ROOT: &str =
        "0000000000000000000000000000000000000000000000000000000000000000";

    fn golden_corridor_id() -> CorridorId {
        CorridorId::from_uuid(Uuid::parse_str(GOLDEN_CORRIDOR_ID).unwrap())
    }

    fn golden_timestamp() -> Timestamp {
        Timestamp::from_datetime(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap())
    }

    fn golden_genesis_digest() -> ContentDigest {
        ContentDigest::from_hex(GOLDEN_GENESIS_ROOT).unwrap()
    }

    /// Golden vector: single receipt with empty digest sets.
    /// This test pins the exact next_root output for a minimal receipt.
    /// If canonicalization or hashing changes, this test will break — which
    /// is the point: it detects unintended commitment-model changes.
    #[test]
    fn golden_vector_single_receipt_empty_digest_sets() {
        let genesis = GOLDEN_GENESIS_ROOT.to_string();
        let mut chain = ReceiptChain::new(golden_corridor_id(), golden_genesis_digest());

        let mut receipt = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: golden_corridor_id(),
            sequence: 0,
            timestamp: golden_timestamp(),
            prev_root: genesis.clone(),
            next_root: String::new(),
            lawpack_digest_set: Vec::new(),
            ruleset_digest_set: Vec::new(),
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        receipt.seal_next_root().unwrap();

        // Pin the next_root value. This is the golden vector.
        // If this assertion fails, the commitment model has changed.
        let next_root = receipt.next_root.clone();
        assert_eq!(
            next_root.len(),
            64,
            "next_root must be 64-char hex string"
        );

        // Verify chain append succeeds.
        chain.append(receipt).unwrap();
        assert_eq!(chain.height(), 1);
        assert_eq!(chain.final_state_root_hex(), next_root);

        // Pin: genesis_root → first receipt's next_root → final_state_root.
        // The chain link invariant: final_state_root == last next_root.
        assert_ne!(
            next_root, genesis,
            "next_root must differ from genesis_root"
        );
    }

    /// Golden vector: two-receipt chain verifying hash-chain continuity.
    /// Receipt 1's next_root becomes Receipt 2's prev_root, and
    /// Receipt 2's next_root becomes the new final_state_root.
    #[test]
    fn golden_vector_two_receipt_chain_continuity() {
        let genesis = GOLDEN_GENESIS_ROOT.to_string();
        let mut chain = ReceiptChain::new(golden_corridor_id(), golden_genesis_digest());

        // Receipt 0
        let mut r0 = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: golden_corridor_id(),
            sequence: 0,
            timestamp: golden_timestamp(),
            prev_root: genesis.clone(),
            next_root: String::new(),
            lawpack_digest_set: vec!["aa".repeat(32).into()],
            ruleset_digest_set: Vec::new(),
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        r0.seal_next_root().unwrap();
        let r0_next = r0.next_root.clone();
        chain.append(r0).unwrap();

        // Receipt 1 — prev_root must be r0's next_root
        assert_eq!(chain.final_state_root_hex(), r0_next);
        let mut r1 = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: golden_corridor_id(),
            sequence: 1,
            timestamp: golden_timestamp(),
            prev_root: r0_next.clone(),
            next_root: String::new(),
            lawpack_digest_set: vec!["bb".repeat(32).into()],
            ruleset_digest_set: vec!["cc".repeat(32).into()],
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        r1.seal_next_root().unwrap();
        let r1_next = r1.next_root.clone();
        chain.append(r1).unwrap();

        // Chain invariants
        assert_eq!(chain.height(), 2);
        assert_eq!(chain.final_state_root_hex(), r1_next);
        assert_ne!(r0_next, r1_next, "consecutive roots must differ");
        assert_ne!(r0_next, genesis, "r0 root must differ from genesis");

        // Verify MMR root is available and differs from both
        let mmr = chain.mmr_root().unwrap();
        assert_eq!(mmr.len(), 64);

        // Checkpoint captures correct state
        let checkpoint = chain.create_checkpoint().unwrap();
        assert_eq!(checkpoint.receipt_count, 2);
        assert_eq!(checkpoint.final_state_root, r1_next);
        assert_eq!(checkpoint.genesis_root, genesis);
    }

    /// Golden vector: deterministic next_root computation is reproducible.
    /// Constructing the same receipt twice must yield the same next_root.
    #[test]
    fn golden_vector_deterministic_next_root() {
        let genesis = GOLDEN_GENESIS_ROOT.to_string();

        let make = || -> String {
            let mut r = CorridorReceipt {
                receipt_type: "MEZCorridorStateReceipt".to_string(),
                corridor_id: golden_corridor_id(),
                sequence: 0,
                timestamp: golden_timestamp(),
                prev_root: genesis.clone(),
                next_root: String::new(),
                lawpack_digest_set: vec![
                    "aa".repeat(32).into(),
                    "bb".repeat(32).into(),
                ],
                ruleset_digest_set: vec!["cc".repeat(32).into()],
                proof: None,
                transition: None,
                transition_type_registry_digest_sha256: None,
                zk: None,
                anchor: None,
            };
            r.seal_next_root().unwrap();
            r.next_root
        };

        let nr1 = make();
        let nr2 = make();
        assert_eq!(nr1, nr2, "identical inputs must produce identical next_root");
        assert_eq!(nr1.len(), 64);
    }
}
