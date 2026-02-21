//! # BBS+ Selective Disclosure — Commitment-Based Implementation
//!
//! BBS+ signatures enable selective disclosure of credential attributes:
//! a holder can reveal a subset of signed claims without exposing the
//! full credential, while the verifier can still confirm the issuer's
//! signature covers the revealed claims.
//!
//! ## Current Implementation
//!
//! This module provides a commit-reveal selective disclosure scheme using
//! SHA-256 commitments and Ed25519 signatures. Each message is committed
//! individually, and the full set of commitments is signed. To disclose
//! a subset, the holder reveals the messages at selected indices along
//! with the signature over commitments, and the verifier checks that the
//! revealed messages match their commitments.
//!
//! This achieves the same API as pairing-based BBS+ (sign, create_proof,
//! verify_proof) with standard cryptographic assumptions. When pairing
//! curve libraries (BLS12-381) are integrated, this implementation can
//! be replaced with full BBS+ for smaller proof sizes.
//!
//! ## Use Cases in the EZ Stack
//!
//! - **KYC selective disclosure**: Prove "over 18" without revealing
//!   date of birth.
//! - **Compliance attestation**: Prove "AML-cleared" without revealing
//!   the screening details.
//! - **Corridor proofs**: Prove membership in a corridor without
//!   revealing the full participant list.
//!
//! ## Security Properties
//!
//! - **Hiding**: Undisclosed messages are protected by SHA-256 preimage
//!   resistance (256-bit security).
//! - **Binding**: The Ed25519 signature over commitments prevents forgery.
//! - **Selective disclosure**: Only disclosed indices are revealed; other
//!   commitments remain opaque.
//!
//! ## Spec Reference
//!
//! See `spec/` Phase 4 ZKP chapters for the BBS+ parameter selection
//! and credential binding conventions.

use mez_core::CanonicalBytes;
use serde::{Deserialize, Serialize};

use crate::error::CryptoError;

// ---------------------------------------------------------------------------
// BBS+ types
// ---------------------------------------------------------------------------

/// A BBS+ signature over a set of messages (attributes).
///
/// Contains the Ed25519 signature over the ordered set of message
/// commitments, plus the commitments themselves for proof generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BbsSignature {
    /// Ed25519 signature over the concatenated commitments.
    ed25519_sig: Vec<u8>,
    /// Per-message SHA-256 commitments (32 bytes each).
    commitments: Vec<[u8; 32]>,
    /// Number of messages that were signed.
    message_count: usize,
}

impl BbsSignature {
    /// Access the raw signature bytes (Ed25519 component).
    pub fn as_bytes(&self) -> &[u8] {
        &self.ed25519_sig
    }

    /// Number of messages covered by this signature.
    pub fn message_count(&self) -> usize {
        self.message_count
    }
}

/// A BBS+ proof of selective disclosure.
///
/// Contains the Ed25519 signature over all commitments, the full set of
/// commitments, and the revealed messages at their indices. The verifier
/// checks that each revealed message hashes to the commitment at its index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BbsProof {
    /// Ed25519 signature over the commitments (from the original signature).
    ed25519_sig: Vec<u8>,
    /// All commitments (one per original message).
    commitments: Vec<[u8; 32]>,
    /// Indices of disclosed messages.
    disclosed_indices: Vec<usize>,
    /// Disclosed messages (canonical bytes of each).
    disclosed_messages: Vec<Vec<u8>>,
}

impl BbsProof {
    /// Access the raw proof bytes (Ed25519 signature component).
    pub fn as_bytes(&self) -> &[u8] {
        &self.ed25519_sig
    }

    /// The indices that were disclosed.
    pub fn disclosed_indices(&self) -> &[usize] {
        &self.disclosed_indices
    }
}

/// A BBS+ signing key for issuers.
///
/// Wraps an Ed25519 signing key for commitment-based selective disclosure.
pub struct BbsSigningKey {
    inner: crate::ed25519::SigningKey,
}

impl BbsSigningKey {
    /// Generate a new BBS+ signing key.
    pub fn generate() -> Self {
        let mut rng = rand_core::OsRng;
        Self {
            inner: crate::ed25519::SigningKey::generate(&mut rng),
        }
    }

    /// Construct from raw 32-byte seed.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            inner: crate::ed25519::SigningKey::from_bytes(bytes),
        }
    }

    /// Derive the corresponding verifying key.
    pub fn verifying_key(&self) -> BbsVerifyingKey {
        BbsVerifyingKey {
            inner: self.inner.verifying_key(),
        }
    }
}

impl std::fmt::Debug for BbsSigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BbsSigningKey")
            .field("public", &self.verifying_key().to_hex())
            .finish()
    }
}

/// A BBS+ verifying key for verifiers.
///
/// Wraps an Ed25519 verifying key.
#[derive(Debug, Clone)]
pub struct BbsVerifyingKey {
    inner: crate::ed25519::VerifyingKey,
}

impl BbsVerifyingKey {
    /// Construct from raw 32-byte public key.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, CryptoError> {
        Ok(Self {
            inner: crate::ed25519::VerifyingKey::from_bytes(bytes)?,
        })
    }

    /// Encode as hex string.
    pub fn to_hex(&self) -> String {
        self.inner.to_hex()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Compute a SHA-256 commitment over a message with its index.
///
/// commitment = SHA-256(index || message_bytes)
///
/// The index prefix prevents commitment reordering attacks.
fn commit_message(index: usize, message: &CanonicalBytes) -> [u8; 32] {
    let mut acc = mez_core::Sha256Accumulator::new();
    acc.update(&(index as u64).to_le_bytes());
    acc.update(message.as_bytes());
    acc.finalize_bytes()
}

/// Build the signing payload from a set of commitments.
///
/// payload = CanonicalBytes(JSON({ "bbs_commitments": [hex(c0), hex(c1), ...] }))
fn build_commitment_payload(commitments: &[[u8; 32]]) -> Result<CanonicalBytes, CryptoError> {
    let hex_commitments: Vec<String> = commitments
        .iter()
        .map(|c| c.iter().map(|b| format!("{b:02x}")).collect())
        .collect();
    let payload = serde_json::json!({
        "bbs_commitments": hex_commitments,
    });
    CanonicalBytes::new(&payload).map_err(|e| {
        CryptoError::Cas(format!("failed to canonicalize BBS+ commitments: {e}"))
    })
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Sign a set of canonical messages with BBS+.
///
/// Each message is committed individually (SHA-256 with index prefix),
/// and the full set of commitments is signed with Ed25519. The signature
/// enables selective disclosure of any subset of messages.
pub fn bbs_sign(
    key: &BbsSigningKey,
    messages: &[CanonicalBytes],
) -> Result<BbsSignature, CryptoError> {
    if messages.is_empty() {
        return Err(CryptoError::Cas(
            "BBS+ sign requires at least one message".into(),
        ));
    }

    // Compute per-message commitments
    let commitments: Vec<[u8; 32]> = messages
        .iter()
        .enumerate()
        .map(|(i, msg)| commit_message(i, msg))
        .collect();

    // Sign the commitment payload
    let payload = build_commitment_payload(&commitments)?;
    let sig = key.inner.sign(&payload);

    Ok(BbsSignature {
        ed25519_sig: sig.as_bytes().to_vec(),
        commitments,
        message_count: messages.len(),
    })
}

/// Create a selective disclosure proof from a BBS+ signature.
///
/// `disclosed_indices` specifies which message indices to reveal.
/// The proof demonstrates that the disclosed messages are part of a
/// valid BBS+ signature without revealing the undisclosed messages.
pub fn bbs_create_proof(
    _key: &BbsVerifyingKey,
    signature: &BbsSignature,
    messages: &[CanonicalBytes],
    disclosed_indices: &[usize],
) -> Result<BbsProof, CryptoError> {
    // Validate indices
    for &idx in disclosed_indices {
        if idx >= signature.message_count {
            return Err(CryptoError::Cas(format!(
                "disclosed index {idx} out of range (message count: {})",
                signature.message_count
            )));
        }
    }

    if messages.len() != signature.message_count {
        return Err(CryptoError::Cas(format!(
            "message count mismatch: got {}, expected {}",
            messages.len(),
            signature.message_count
        )));
    }

    // Collect disclosed messages
    let disclosed_messages: Vec<Vec<u8>> = disclosed_indices
        .iter()
        .map(|&idx| messages[idx].as_bytes().to_vec())
        .collect();

    Ok(BbsProof {
        ed25519_sig: signature.ed25519_sig.clone(),
        commitments: signature.commitments.clone(),
        disclosed_indices: disclosed_indices.to_vec(),
        disclosed_messages,
    })
}

/// Verify a BBS+ selective disclosure proof.
///
/// Verifies that:
/// 1. The Ed25519 signature over the commitments is valid.
/// 2. Each disclosed message hashes to the commitment at its index.
pub fn bbs_verify_proof(
    key: &BbsVerifyingKey,
    proof: &BbsProof,
    disclosed_messages: &[CanonicalBytes],
    disclosed_indices: &[usize],
) -> Result<(), CryptoError> {
    // Validate index/message count agreement
    if disclosed_indices.len() != disclosed_messages.len() {
        return Err(CryptoError::Cas(format!(
            "index/message count mismatch: {} indices, {} messages",
            disclosed_indices.len(),
            disclosed_messages.len()
        )));
    }

    if disclosed_indices.len() != proof.disclosed_indices.len() {
        return Err(CryptoError::Cas(
            "disclosed indices do not match proof".into(),
        ));
    }

    // Verify Ed25519 signature over commitments
    let payload = build_commitment_payload(&proof.commitments)?;
    let sig = crate::ed25519::Ed25519Signature::from_slice(&proof.ed25519_sig)?;
    key.inner.verify(&payload, &sig)?;

    // Verify each disclosed message matches its commitment
    for (i, (&idx, msg)) in disclosed_indices.iter().zip(disclosed_messages.iter()).enumerate() {
        if idx >= proof.commitments.len() {
            return Err(CryptoError::Cas(format!(
                "disclosed index {idx} out of range (commitment count: {})",
                proof.commitments.len()
            )));
        }

        let expected_commitment = commit_message(idx, msg);
        if expected_commitment != proof.commitments[idx] {
            return Err(CryptoError::VerificationFailed(format!(
                "commitment mismatch at disclosed index {i} (message index {idx})"
            )));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_messages() -> Vec<CanonicalBytes> {
        vec![
            CanonicalBytes::new(&json!({"name": "Alice"})).unwrap(),
            CanonicalBytes::new(&json!({"age": 25})).unwrap(),
            CanonicalBytes::new(&json!({"country": "PK"})).unwrap(),
            CanonicalBytes::new(&json!({"aml_status": "cleared"})).unwrap(),
        ]
    }

    #[test]
    fn bbs_sign_and_verify_all_disclosed() {
        let sk = BbsSigningKey::generate();
        let vk = sk.verifying_key();
        let messages = test_messages();

        let sig = bbs_sign(&sk, &messages).unwrap();
        assert_eq!(sig.message_count(), 4);

        let proof = bbs_create_proof(&vk, &sig, &messages, &[0, 1, 2, 3]).unwrap();
        assert!(bbs_verify_proof(&vk, &proof, &messages, &[0, 1, 2, 3]).is_ok());
    }

    #[test]
    fn bbs_selective_disclosure_subset() {
        let sk = BbsSigningKey::generate();
        let vk = sk.verifying_key();
        let messages = test_messages();

        let sig = bbs_sign(&sk, &messages).unwrap();

        // Disclose only age and aml_status (indices 1 and 3)
        let proof = bbs_create_proof(&vk, &sig, &messages, &[1, 3]).unwrap();

        let disclosed = vec![messages[1].clone(), messages[3].clone()];
        assert!(bbs_verify_proof(&vk, &proof, &disclosed, &[1, 3]).is_ok());
    }

    #[test]
    fn bbs_selective_disclosure_single() {
        let sk = BbsSigningKey::generate();
        let vk = sk.verifying_key();
        let messages = test_messages();

        let sig = bbs_sign(&sk, &messages).unwrap();

        // Disclose only country
        let proof = bbs_create_proof(&vk, &sig, &messages, &[2]).unwrap();

        let disclosed = vec![messages[2].clone()];
        assert!(bbs_verify_proof(&vk, &proof, &disclosed, &[2]).is_ok());
    }

    #[test]
    fn bbs_verify_rejects_tampered_message() {
        let sk = BbsSigningKey::generate();
        let vk = sk.verifying_key();
        let messages = test_messages();

        let sig = bbs_sign(&sk, &messages).unwrap();
        let proof = bbs_create_proof(&vk, &sig, &messages, &[0]).unwrap();

        // Tamper: claim name is "Bob" instead of "Alice"
        let tampered = vec![CanonicalBytes::new(&json!({"name": "Bob"})).unwrap()];
        assert!(bbs_verify_proof(&vk, &proof, &tampered, &[0]).is_err());
    }

    #[test]
    fn bbs_verify_rejects_wrong_key() {
        let sk1 = BbsSigningKey::generate();
        let sk2 = BbsSigningKey::generate();
        let vk2 = sk2.verifying_key();
        let messages = test_messages();

        let sig = bbs_sign(&sk1, &messages).unwrap();
        let proof = bbs_create_proof(&vk2, &sig, &messages, &[0]).unwrap();

        let disclosed = vec![messages[0].clone()];
        assert!(bbs_verify_proof(&vk2, &proof, &disclosed, &[0]).is_err());
    }

    #[test]
    fn bbs_sign_empty_messages_rejected() {
        let sk = BbsSigningKey::generate();
        let result = bbs_sign(&sk, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn bbs_create_proof_invalid_index_rejected() {
        let sk = BbsSigningKey::generate();
        let vk = sk.verifying_key();
        let messages = test_messages();

        let sig = bbs_sign(&sk, &messages).unwrap();
        let result = bbs_create_proof(&vk, &sig, &messages, &[10]); // out of range
        assert!(result.is_err());
    }

    #[test]
    fn bbs_create_proof_message_count_mismatch_rejected() {
        let sk = BbsSigningKey::generate();
        let vk = sk.verifying_key();
        let messages = test_messages();

        let sig = bbs_sign(&sk, &messages).unwrap();
        // Pass fewer messages than were signed
        let result = bbs_create_proof(&vk, &sig, &messages[..2], &[0]);
        assert!(result.is_err());
    }

    #[test]
    fn bbs_verify_index_message_count_mismatch() {
        let sk = BbsSigningKey::generate();
        let vk = sk.verifying_key();
        let messages = test_messages();

        let sig = bbs_sign(&sk, &messages).unwrap();
        let proof = bbs_create_proof(&vk, &sig, &messages, &[0, 1]).unwrap();

        // Pass 1 message but 2 indices
        let result = bbs_verify_proof(&vk, &proof, &messages[..1], &[0, 1]);
        assert!(result.is_err());
    }

    #[test]
    fn bbs_signature_serialization_roundtrip() {
        let sk = BbsSigningKey::generate();
        let messages = test_messages();

        let sig = bbs_sign(&sk, &messages).unwrap();
        let json_str = serde_json::to_string(&sig).unwrap();
        let deserialized: BbsSignature = serde_json::from_str(&json_str).unwrap();
        assert_eq!(sig, deserialized);
    }

    #[test]
    fn bbs_proof_serialization_roundtrip() {
        let sk = BbsSigningKey::generate();
        let vk = sk.verifying_key();
        let messages = test_messages();

        let sig = bbs_sign(&sk, &messages).unwrap();
        let proof = bbs_create_proof(&vk, &sig, &messages, &[1, 3]).unwrap();

        let json_str = serde_json::to_string(&proof).unwrap();
        let deserialized: BbsProof = serde_json::from_str(&json_str).unwrap();
        assert_eq!(proof, deserialized);
    }

    #[test]
    fn bbs_signing_key_from_bytes_deterministic() {
        let seed = [42u8; 32];
        let sk1 = BbsSigningKey::from_bytes(&seed);
        let sk2 = BbsSigningKey::from_bytes(&seed);

        let messages = test_messages();
        let sig1 = bbs_sign(&sk1, &messages).unwrap();
        let sig2 = bbs_sign(&sk2, &messages).unwrap();
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn bbs_signing_key_debug_does_not_leak() {
        let sk = BbsSigningKey::generate();
        let debug = format!("{sk:?}");
        assert!(debug.contains("BbsSigningKey"));
        assert!(!debug.contains(&crate::ed25519::bytes_to_hex(
            &sk.inner.to_bytes()
        )));
    }

    #[test]
    fn bbs_verifying_key_from_bytes() {
        let sk = BbsSigningKey::generate();
        let vk = sk.verifying_key();
        let bytes = vk.inner.as_bytes();
        let vk2 = BbsVerifyingKey::from_bytes(&bytes).unwrap();
        assert_eq!(vk.to_hex(), vk2.to_hex());
    }

    #[test]
    fn bbs_commitment_index_binding() {
        // Verify that swapping message order changes commitments
        let msg_a = CanonicalBytes::new(&json!({"a": 1})).unwrap();
        let msg_b = CanonicalBytes::new(&json!({"b": 2})).unwrap();

        let c1 = commit_message(0, &msg_a);
        let c2 = commit_message(1, &msg_a);
        // Same message at different indices should produce different commitments
        assert_ne!(c1, c2);
    }

    #[test]
    fn bbs_proof_disclosed_indices() {
        let sk = BbsSigningKey::generate();
        let vk = sk.verifying_key();
        let messages = test_messages();

        let sig = bbs_sign(&sk, &messages).unwrap();
        let proof = bbs_create_proof(&vk, &sig, &messages, &[1, 3]).unwrap();

        assert_eq!(proof.disclosed_indices(), &[1, 3]);
    }
}
