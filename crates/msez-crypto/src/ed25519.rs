//! # Ed25519 Signing and Verification
//!
//! Provides Ed25519 key generation, signing, and verification for
//! Verifiable Credential proofs and corridor attestations.
//!
//! ## Security Invariant
//!
//! - Signing input MUST be `&CanonicalBytes` — you cannot sign raw bytes.
//!   This enforces that all signed data has been canonicalized through the
//!   JCS pipeline, preventing the canonicalization split defect.
//! - Private keys are never serialized or logged. `Ed25519KeyPair` does
//!   not implement `Serialize` or expose the private key bytes.
//! - Verification accepts `&CanonicalBytes` + `&Ed25519Signature` +
//!   `&ed25519_dalek::VerifyingKey`, enforcing type-level correctness.
//!
//! ## Serde
//!
//! - Public keys serialize/deserialize as hex-encoded strings.
//! - Signatures serialize/deserialize as hex-encoded strings.
//!
//! ## Implements
//!
//! Spec §9 — Ed25519 digital signatures for VC proofs.

use ed25519_dalek::{Signer, Verifier};
use msez_core::error::CryptoError;
use msez_core::CanonicalBytes;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An Ed25519 public key (32 bytes) for signature verification.
///
/// Serializes as a hex-encoded string for JSON interoperability.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Ed25519PublicKey(pub [u8; 32]);

/// An Ed25519 signature (64 bytes).
///
/// Wrapped in a newtype from msez-core to enforce that signatures are
/// produced only from `CanonicalBytes` input. Serializes as a hex-encoded
/// string.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Ed25519Signature(pub [u8; 64]);

/// An Ed25519 key pair for signing operations.
///
/// Does not implement `Serialize` — private keys must not be accidentally
/// serialized into logs, responses, or artifacts.
pub struct Ed25519KeyPair {
    signing_key: ed25519_dalek::SigningKey,
}

// ---------------------------------------------------------------------------
// Ed25519PublicKey impls
// ---------------------------------------------------------------------------

impl Ed25519PublicKey {
    /// Create a public key from raw 32 bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Return the raw 32-byte public key.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Render the public key as a lowercase hex string.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Parse a public key from a 64-character hex string.
    pub fn from_hex(hex: &str) -> Result<Self, CryptoError> {
        let hex = hex.trim().to_lowercase();
        if hex.len() != 64 {
            return Err(CryptoError::KeyError(format!(
                "public key hex must be 64 chars, got {}",
                hex.len()
            )));
        }
        let bytes = hex_to_bytes(&hex).map_err(|e| CryptoError::KeyError(e.to_string()))?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Convert to an `ed25519_dalek::VerifyingKey` for verification operations.
    pub fn to_verifying_key(&self) -> Result<ed25519_dalek::VerifyingKey, CryptoError> {
        ed25519_dalek::VerifyingKey::from_bytes(&self.0)
            .map_err(|e| CryptoError::KeyError(format!("invalid public key: {e}")))
    }
}

impl Serialize for Ed25519PublicKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Ed25519PublicKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hex = String::deserialize(deserializer)?;
        Self::from_hex(&hex).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Debug for Ed25519PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ed25519PublicKey({}...)", hex_prefix(&self.0))
    }
}

impl std::fmt::Display for Ed25519PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

// ---------------------------------------------------------------------------
// Ed25519Signature impls
// ---------------------------------------------------------------------------

impl Ed25519Signature {
    /// Create a signature from raw 64 bytes.
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    /// Return the raw 64-byte signature.
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    /// Render the signature as a lowercase hex string.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Parse a signature from a 128-character hex string.
    pub fn from_hex(hex: &str) -> Result<Self, CryptoError> {
        let hex = hex.trim().to_lowercase();
        if hex.len() != 128 {
            return Err(CryptoError::VerificationFailed(format!(
                "signature hex must be 128 chars, got {}",
                hex.len()
            )));
        }
        let bytes = hex_to_bytes(&hex)
            .map_err(|e| CryptoError::VerificationFailed(e.to_string()))?;
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl Serialize for Ed25519Signature {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Ed25519Signature {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hex = String::deserialize(deserializer)?;
        Self::from_hex(&hex).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Debug for Ed25519Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ed25519Signature({}...)", hex_prefix(&self.0))
    }
}

impl std::fmt::Display for Ed25519Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

// ---------------------------------------------------------------------------
// Ed25519KeyPair impls
// ---------------------------------------------------------------------------

impl Ed25519KeyPair {
    /// Generate a new random Ed25519 key pair.
    pub fn generate() -> Self {
        let mut csprng = rand::rngs::OsRng;
        let signing_key = ed25519_dalek::SigningKey::generate(&mut csprng);
        Self { signing_key }
    }

    /// Create a key pair from raw 32-byte private key seed.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(seed);
        Self { signing_key }
    }

    /// Get the public key from this key pair.
    pub fn public_key(&self) -> Ed25519PublicKey {
        let vk = self.signing_key.verifying_key();
        Ed25519PublicKey(vk.to_bytes())
    }

    /// Sign canonical bytes.
    ///
    /// The signing input MUST be `&CanonicalBytes` to enforce that all
    /// signed data has been canonicalized through the JCS pipeline.
    ///
    /// # Security Invariant
    ///
    /// You cannot sign raw `&[u8]` — this prevents signing non-canonical
    /// data which would cause verification failures across implementations.
    ///
    /// Implements Spec §9 — Ed25519 signing over canonical input.
    pub fn sign(&self, data: &CanonicalBytes) -> Ed25519Signature {
        let sig = self.signing_key.sign(data.as_bytes());
        Ed25519Signature(sig.to_bytes())
    }
}

impl std::fmt::Debug for Ed25519KeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ed25519KeyPair(<private>)")
    }
}

// ---------------------------------------------------------------------------
// Verification
// ---------------------------------------------------------------------------

/// Verify an Ed25519 signature over canonical bytes.
///
/// Accepts `&CanonicalBytes` (the signed message), `&Ed25519Signature`,
/// and a `&ed25519_dalek::VerifyingKey`. Returns `Ok(())` if valid,
/// `Err(CryptoError::VerificationFailed)` otherwise.
///
/// # Security Invariant
///
/// The message parameter is `&CanonicalBytes`, enforcing at compile time
/// that only canonicalized data can be verified. This prevents the
/// canonicalization split defect by construction.
///
/// Implements Spec §9 — Ed25519 signature verification.
pub fn verify(
    data: &CanonicalBytes,
    signature: &Ed25519Signature,
    verifying_key: &ed25519_dalek::VerifyingKey,
) -> Result<(), CryptoError> {
    let sig = ed25519_dalek::Signature::from_bytes(&signature.0);
    verifying_key
        .verify(data.as_bytes(), &sig)
        .map_err(|e| CryptoError::VerificationFailed(format!("Ed25519 verification failed: {e}")))
}

/// Convenience verification using `Ed25519PublicKey` instead of dalek key.
///
/// Parses the public key into a verifying key and delegates to [`verify()`].
pub fn verify_with_public_key(
    data: &CanonicalBytes,
    signature: &Ed25519Signature,
    public_key: &Ed25519PublicKey,
) -> Result<(), CryptoError> {
    let vk = public_key.to_verifying_key()?;
    verify(data, signature, &vk)
}

// ---------------------------------------------------------------------------
// Hex utilities (no external hex crate dependency)
// ---------------------------------------------------------------------------

fn hex_prefix(bytes: &[u8]) -> String {
    bytes.iter().take(4).map(|b| format!("{b:02x}")).collect()
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("hex string must have even length".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("invalid hex at position {i}: {e}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp = Ed25519KeyPair::generate();
        let pk = kp.public_key();
        assert_eq!(pk.as_bytes().len(), 32);
    }

    #[test]
    fn test_sign_and_verify() {
        let kp = Ed25519KeyPair::generate();
        let data = serde_json::json!({"message": "hello", "nonce": 42});
        let canonical = CanonicalBytes::new(&data).expect("should canonicalize");
        let sig = kp.sign(&canonical);
        assert_eq!(sig.as_bytes().len(), 64);

        let vk = kp.public_key().to_verifying_key().unwrap();
        verify(&canonical, &sig, &vk).expect("valid signature should verify");
    }

    #[test]
    fn test_verify_wrong_key_fails() {
        let kp1 = Ed25519KeyPair::generate();
        let kp2 = Ed25519KeyPair::generate();
        let data = serde_json::json!({"test": true});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let sig = kp1.sign(&canonical);

        let wrong_vk = kp2.public_key().to_verifying_key().unwrap();
        let result = verify(&canonical, &sig, &wrong_vk);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_wrong_message_fails() {
        let kp = Ed25519KeyPair::generate();
        let data1 = serde_json::json!({"msg": "original"});
        let data2 = serde_json::json!({"msg": "tampered"});
        let canonical1 = CanonicalBytes::new(&data1).unwrap();
        let canonical2 = CanonicalBytes::new(&data2).unwrap();
        let sig = kp.sign(&canonical1);

        let vk = kp.public_key().to_verifying_key().unwrap();
        let result = verify(&canonical2, &sig, &vk);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_with_public_key_convenience() {
        let kp = Ed25519KeyPair::generate();
        let data = serde_json::json!({"corridor_id": "test-corridor"});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let sig = kp.sign(&canonical);
        let pk = kp.public_key();

        verify_with_public_key(&canonical, &sig, &pk).expect("should verify");
    }

    #[test]
    fn test_deterministic_from_seed() {
        let seed = [42u8; 32];
        let kp1 = Ed25519KeyPair::from_seed(&seed);
        let kp2 = Ed25519KeyPair::from_seed(&seed);
        assert_eq!(kp1.public_key(), kp2.public_key());

        let data = serde_json::json!({"test": "deterministic"});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let sig1 = kp1.sign(&canonical);
        let sig2 = kp2.sign(&canonical);
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_public_key_hex_roundtrip() {
        let kp = Ed25519KeyPair::generate();
        let pk = kp.public_key();
        let hex = pk.to_hex();
        assert_eq!(hex.len(), 64);
        let pk2 = Ed25519PublicKey::from_hex(&hex).unwrap();
        assert_eq!(pk, pk2);
    }

    #[test]
    fn test_signature_hex_roundtrip() {
        let kp = Ed25519KeyPair::generate();
        let data = serde_json::json!({"x": 1});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let sig = kp.sign(&canonical);
        let hex = sig.to_hex();
        assert_eq!(hex.len(), 128);
        let sig2 = Ed25519Signature::from_hex(&hex).unwrap();
        assert_eq!(sig, sig2);
    }

    #[test]
    fn test_public_key_serde_json_roundtrip() {
        let kp = Ed25519KeyPair::generate();
        let pk = kp.public_key();
        let json = serde_json::to_string(&pk).unwrap();
        // Should be a quoted hex string
        assert!(json.starts_with('"'));
        assert!(json.ends_with('"'));
        assert_eq!(json.len(), 64 + 2); // 64 hex chars + 2 quotes

        let pk2: Ed25519PublicKey = serde_json::from_str(&json).unwrap();
        assert_eq!(pk, pk2);
    }

    #[test]
    fn test_signature_serde_json_roundtrip() {
        let kp = Ed25519KeyPair::generate();
        let data = serde_json::json!({"y": 2});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let sig = kp.sign(&canonical);
        let json = serde_json::to_string(&sig).unwrap();
        assert!(json.starts_with('"'));
        assert_eq!(json.len(), 128 + 2); // 128 hex chars + 2 quotes

        let sig2: Ed25519Signature = serde_json::from_str(&json).unwrap();
        assert_eq!(sig, sig2);
    }

    #[test]
    fn test_public_key_invalid_hex() {
        assert!(Ed25519PublicKey::from_hex("not-hex").is_err());
        assert!(Ed25519PublicKey::from_hex("aabb").is_err());
        assert!(Ed25519PublicKey::from_hex(&"zz".repeat(32)).is_err());
    }

    #[test]
    fn test_signature_invalid_hex() {
        assert!(Ed25519Signature::from_hex("not-hex").is_err());
        assert!(Ed25519Signature::from_hex("aabb").is_err());
    }

    #[test]
    fn test_debug_does_not_leak_private_key() {
        let kp = Ed25519KeyPair::generate();
        let debug = format!("{kp:?}");
        assert_eq!(debug, "Ed25519KeyPair(<private>)");
        assert!(!debug.contains("SigningKey"));
    }

    #[test]
    fn test_debug_public_key_shows_prefix() {
        let kp = Ed25519KeyPair::generate();
        let pk = kp.public_key();
        let debug = format!("{pk:?}");
        assert!(debug.starts_with("Ed25519PublicKey("));
        assert!(debug.ends_with("...)"));
    }
}
