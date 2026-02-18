//! # Cryptographic Error Types
//!
//! Structured errors for all cryptographic operations in `mez-crypto`.
//! Uses `thiserror` for ergonomic error definitions with diagnostic context.

use thiserror::Error;

/// Errors from cryptographic operations in the EZ Stack.
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

    /// Feature is not yet implemented.
    #[error("not implemented: {0}")]
    NotImplemented(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_failed_display() {
        let err = CryptoError::VerificationFailed("bad sig".to_string());
        assert!(format!("{err}").contains("bad sig"));
    }

    #[test]
    fn invalid_signature_length_display() {
        let err = CryptoError::InvalidSignatureLength(32);
        let msg = format!("{err}");
        assert!(msg.contains("64 bytes"));
        assert!(msg.contains("32"));
    }

    #[test]
    fn invalid_public_key_display() {
        let err = CryptoError::InvalidPublicKey("too short".to_string());
        assert!(format!("{err}").contains("too short"));
    }

    #[test]
    fn hex_decode_display() {
        let err = CryptoError::HexDecode("invalid char".to_string());
        assert!(format!("{err}").contains("invalid char"));
    }

    #[test]
    fn mmr_error_display() {
        let err = CryptoError::Mmr("index out of range".to_string());
        assert!(format!("{err}").contains("index out of range"));
    }

    #[test]
    fn cas_error_display() {
        let err = CryptoError::Cas("artifact not found".to_string());
        assert!(format!("{err}").contains("artifact not found"));
    }

    #[test]
    fn io_error_from_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = CryptoError::from(io_err);
        assert!(format!("{err}").contains("file missing"));
    }

    #[test]
    fn all_variants_are_debug() {
        let variants: Vec<CryptoError> = vec![
            CryptoError::VerificationFailed("a".to_string()),
            CryptoError::InvalidSignatureLength(0),
            CryptoError::InvalidPublicKey("b".to_string()),
            CryptoError::HexDecode("c".to_string()),
            CryptoError::Mmr("d".to_string()),
            CryptoError::Cas("e".to_string()),
        ];
        for v in variants {
            assert!(!format!("{v:?}").is_empty());
        }
    }
}
