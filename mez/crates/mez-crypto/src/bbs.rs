//! # BBS+ Selective Disclosure — Phase 4 Stub
//!
//! BBS+ signatures enable selective disclosure of credential attributes:
//! a holder can reveal a subset of signed claims without exposing the
//! full credential, while the verifier can still confirm the issuer's
//! signature covers the revealed claims.
//!
//! ## Current Status
//!
//! This module is gated behind the `bbs-plus` Cargo feature flag.
//! All public types and functions have correct signatures but return
//! `Err(CryptoError::NotImplemented)` at runtime. This allows downstream
//! crates to reference BBS+ types for compile-time checking.
//!
//! ## Activation Plan
//!
//! Phase 4 will provide a concrete BBS+ implementation via an external
//! crate (e.g., `bbs` or `bbs-plus`). At that point, the error-returning
//! stubs will be replaced with real cryptographic operations.
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
//! ## Spec Reference
//!
//! See `spec/` Phase 4 ZKP chapters for the BBS+ parameter selection
//! and credential binding conventions.

use mez_core::CanonicalBytes;

/// A BBS+ signature over a set of messages (attributes).
///
/// ## Phase 4
///
/// The internal representation will be determined by the chosen BBS+
/// library. This placeholder uses a fixed-size byte array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BbsSignature {
    bytes: Vec<u8>,
}

impl BbsSignature {
    /// Access the raw signature bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// A BBS+ proof of selective disclosure.
///
/// Contains the zero-knowledge proof that a subset of signed messages
/// was part of a valid BBS+ signature, without revealing the other
/// messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BbsProof {
    bytes: Vec<u8>,
}

impl BbsProof {
    /// Access the raw proof bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// A BBS+ signing key for issuers.
///
/// ## Phase 4 — Not Yet Implemented
#[derive(Debug)]
pub struct BbsSigningKey {
    _private: (),
}

/// A BBS+ verifying key for verifiers.
///
/// ## Phase 4 — Not Yet Implemented
#[derive(Debug, Clone)]
pub struct BbsVerifyingKey {
    _private: (),
}

/// Sign a set of canonical messages with BBS+.
///
/// ## Phase 4 — Not Yet Implemented
///
/// Each message must be [`CanonicalBytes`] to maintain the
/// canonicalization invariant across the entire stack.
pub fn bbs_sign(
    _key: &BbsSigningKey,
    _messages: &[CanonicalBytes],
) -> Result<BbsSignature, crate::error::CryptoError> {
    Err(crate::error::CryptoError::NotImplemented(
        "BBS+ signing available in Phase 4".into(),
    ))
}

/// Create a selective disclosure proof from a BBS+ signature.
///
/// ## Phase 4 — Not Yet Implemented
///
/// `disclosed_indices` specifies which message indices to reveal.
/// The proof demonstrates that the disclosed messages are part of a
/// valid BBS+ signature without revealing the undisclosed messages.
pub fn bbs_create_proof(
    _key: &BbsVerifyingKey,
    _signature: &BbsSignature,
    _messages: &[CanonicalBytes],
    _disclosed_indices: &[usize],
) -> Result<BbsProof, crate::error::CryptoError> {
    Err(crate::error::CryptoError::NotImplemented(
        "BBS+ proof creation available in Phase 4".into(),
    ))
}

/// Verify a BBS+ selective disclosure proof.
///
/// ## Phase 4 — Not Yet Implemented
///
/// Verifies that the disclosed messages at the given indices were
/// signed by the holder of the corresponding signing key.
pub fn bbs_verify_proof(
    _key: &BbsVerifyingKey,
    _proof: &BbsProof,
    _disclosed_messages: &[CanonicalBytes],
    _disclosed_indices: &[usize],
) -> Result<(), crate::error::CryptoError> {
    Err(crate::error::CryptoError::NotImplemented(
        "BBS+ proof verification available in Phase 4".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbs_sign_returns_not_implemented() {
        let key = BbsSigningKey { _private: () };
        let msg = CanonicalBytes::new(&serde_json::json!({"claim": "over_18"})).unwrap();
        let result = bbs_sign(&key, &[msg]);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("not implemented"));
    }

    #[test]
    fn bbs_create_proof_returns_not_implemented() {
        let key = BbsVerifyingKey { _private: () };
        let sig = BbsSignature { bytes: vec![] };
        let msg = CanonicalBytes::new(&serde_json::json!({"claim": "over_18"})).unwrap();
        let result = bbs_create_proof(&key, &sig, &[msg], &[0]);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("not implemented"));
    }

    #[test]
    fn bbs_verify_proof_returns_not_implemented() {
        let key = BbsVerifyingKey { _private: () };
        let proof = BbsProof { bytes: vec![] };
        let msg = CanonicalBytes::new(&serde_json::json!({"claim": "over_18"})).unwrap();
        let result = bbs_verify_proof(&key, &proof, &[msg], &[0]);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("not implemented"));
    }
}
