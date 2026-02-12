//! # Ed25519 Signing and Verification
//!
//! Provides Ed25519 digital signatures for Verifiable Credentials,
//! corridor attestations, and watcher bonds.
//!
//! ## Security Invariant
//!
//! Signing operations take [`CanonicalBytes`](msez_core::CanonicalBytes) to
//! ensure the signed payload was properly canonicalized. This prevents
//! signature malleability from non-canonical serialization.

/// An Ed25519 digital signature (64 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ed25519Signature(pub Vec<u8>);

/// An Ed25519 signing (private) key.
///
/// Wraps `ed25519_dalek::SigningKey` with SEZ Stack conventions.
pub struct SigningKey {
    _inner: ed25519_dalek::SigningKey,
}


/// An Ed25519 verifying (public) key.
///
/// Used to verify signatures on VCs, attestations, and corridor proofs.
#[derive(Debug, Clone)]
pub struct VerifyingKey {
    _inner: ed25519_dalek::VerifyingKey,
}
