//! # Ed25519 Signing and Verification
//!
//! Provides Ed25519 key generation, signing, and verification for
//! Verifiable Credential proofs and corridor attestations.
//!
//! ## Security Invariant
//!
//! Private keys are never serialized or logged. The `Ed25519KeyPair`
//! type intentionally does not implement `Serialize` or `Debug` for
//! the private component.
//!
//! ## Implements
//!
//! Spec §9 — Ed25519 digital signatures for VC proofs.

use serde::{Deserialize, Serialize};

/// An Ed25519 public key for signature verification.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ed25519PublicKey(pub [u8; 32]);

/// An Ed25519 signature (64 bytes).
///
/// Uses `Vec<u8>` internally for serde compatibility; the signature
/// is always exactly 64 bytes when valid.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ed25519Signature(pub Vec<u8>);

/// An Ed25519 key pair for signing operations.
///
/// Does not implement `Serialize` — private keys must not be accidentally
/// serialized into logs, responses, or artifacts.
pub struct Ed25519KeyPair {
    _private: (), // Placeholder — will hold ed25519_dalek::SigningKey
}

impl Ed25519KeyPair {
    /// Generate a new random Ed25519 key pair.
    pub fn generate() -> Self {
        // TODO: Implement with ed25519_dalek::SigningKey::generate()
        Self { _private: () }
    }

    /// Get the public key from this key pair.
    pub fn public_key(&self) -> Ed25519PublicKey {
        // TODO: Derive from signing key
        Ed25519PublicKey([0u8; 32])
    }
}

impl std::fmt::Debug for Ed25519PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ed25519PublicKey({}...)", hex_prefix(&self.0))
    }
}

impl std::fmt::Debug for Ed25519Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ed25519Signature({}...)", hex_prefix(&self.0))
    }
}

impl std::fmt::Debug for Ed25519KeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ed25519KeyPair(<private>)")
    }
}

fn hex_prefix(bytes: &[u8]) -> String {
    bytes.iter().take(4).map(|b| format!("{b:02x}")).collect()
}
