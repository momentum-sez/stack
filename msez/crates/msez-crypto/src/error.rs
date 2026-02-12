//! # Cryptographic Error Types
//!
//! Structured errors for all cryptographic operations in `msez-crypto`.
//! Uses `thiserror` for ergonomic error definitions with diagnostic context.

use thiserror::Error;

/// Errors from cryptographic operations in the SEZ Stack.
#[derive(Error, Debug)]
pub enum CryptoError {
    /// Ed25519 signature verification failed.
    #[error("Ed25519 verification failed: {0}")]
    VerificationFailed(String),

    /// Invalid Ed25519 signature length.
    #[error("invalid Ed25519 signature length: expected 64 bytes, got {0}")]
    InvalidSignatureLength(usize),

    /// Invalid Ed25519 public key.
    #[error("invalid Ed25519 public key: {0}")]
    InvalidPublicKey(String),

    /// Hex decoding error.
    #[error("hex decode error: {0}")]
    HexDecode(String),

    /// MMR operation error.
    #[error("MMR error: {0}")]
    Mmr(String),

    /// CAS operation error.
    #[error("CAS error: {0}")]
    Cas(String),

    /// I/O error (CAS filesystem operations).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
