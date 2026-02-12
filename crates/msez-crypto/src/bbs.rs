//! # BBS+ Selective Disclosure Signatures (Phase 4 Stub)
//!
//! This module activates behind the `bbs-plus` feature flag. It contains
//! only type signatures until a real BBS+ backend is integrated in Phase 4.
//!
//! ## Purpose
//!
//! BBS+ signatures enable selective disclosure of Verifiable Credential
//! attributes. A holder can prove they possess a VC signed by an issuer
//! while revealing only a subset of the credential's claims.
//!
//! This is critical for privacy-preserving KYC/KYB flows where an entity
//! needs to prove compliance (e.g., "I have a valid business license")
//! without revealing the full credential contents.
//!
//! ## Security Invariant
//!
//! All BBS+ signing operations accept only `&CanonicalBytes` as input,
//! matching the Ed25519 signing invariant.
//!
//! ## Implements
//!
//! Spec §23 — BBS+ selective disclosure for privacy-preserving VCs.

use msez_core::error::CryptoError;
use msez_core::CanonicalBytes;

/// A BBS+ public key for signature verification and selective disclosure.
///
/// Placeholder — will hold the actual BBS+ public key when the backend
/// is integrated.
#[derive(Debug, Clone)]
pub struct BbsPlusPublicKey {
    _private: (),
}

/// A BBS+ secret key for signing.
///
/// Does not implement `Serialize` — secret keys must not be accidentally
/// serialized, matching the Ed25519 key pair policy.
pub struct BbsPlusSecretKey {
    _private: (),
}

impl std::fmt::Debug for BbsPlusSecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BbsPlusSecretKey(<private>)")
    }
}

/// A BBS+ signature over a set of messages.
///
/// Placeholder — will hold the actual BBS+ signature when the backend
/// is integrated.
#[derive(Debug, Clone)]
pub struct BbsPlusSignature {
    _private: (),
}

/// A BBS+ selective disclosure proof.
///
/// Proves knowledge of a BBS+ signature while selectively revealing
/// only a subset of the signed messages.
#[derive(Debug, Clone)]
pub struct BbsPlusProof {
    _private: (),
}

/// Generate a new BBS+ key pair.
///
/// # Panics
///
/// Always panics — BBS+ is not yet implemented.
pub fn generate_keypair() -> (BbsPlusPublicKey, BbsPlusSecretKey) {
    unimplemented!("BBS+ key generation is Phase 4")
}

/// Sign a set of messages (canonical bytes) with a BBS+ secret key.
///
/// Each message must be `&CanonicalBytes` to prevent the canonicalization
/// split defect.
///
/// # Panics
///
/// Always panics — BBS+ is not yet implemented.
pub fn sign(
    _secret_key: &BbsPlusSecretKey,
    _public_key: &BbsPlusPublicKey,
    _messages: &[&CanonicalBytes],
) -> BbsPlusSignature {
    unimplemented!("BBS+ signing is Phase 4")
}

/// Verify a BBS+ signature over a set of messages.
///
/// # Panics
///
/// Always panics — BBS+ is not yet implemented.
pub fn verify_signature(
    _public_key: &BbsPlusPublicKey,
    _signature: &BbsPlusSignature,
    _messages: &[&CanonicalBytes],
) -> Result<(), CryptoError> {
    unimplemented!("BBS+ verification is Phase 4")
}

/// Create a selective disclosure proof.
///
/// Given a BBS+ signature and a set of messages, creates a proof that
/// reveals only the messages at the specified indices.
///
/// # Panics
///
/// Always panics — BBS+ is not yet implemented.
pub fn create_proof(
    _public_key: &BbsPlusPublicKey,
    _signature: &BbsPlusSignature,
    _messages: &[&CanonicalBytes],
    _revealed_indices: &[usize],
) -> BbsPlusProof {
    unimplemented!("BBS+ proof creation is Phase 4")
}

/// Verify a selective disclosure proof.
///
/// Verifies that the proof is valid for the revealed messages at the
/// specified indices.
///
/// # Panics
///
/// Always panics — BBS+ is not yet implemented.
pub fn verify_proof(
    _public_key: &BbsPlusPublicKey,
    _proof: &BbsPlusProof,
    _revealed_messages: &[&CanonicalBytes],
    _revealed_indices: &[usize],
) -> Result<(), CryptoError> {
    unimplemented!("BBS+ proof verification is Phase 4")
}
