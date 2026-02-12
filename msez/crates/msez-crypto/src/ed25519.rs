//! # Ed25519 Signing and Verification
//!
//! Provides Ed25519 digital signatures for Verifiable Credentials,
//! corridor attestations, and watcher bonds using the `ed25519-dalek` crate.
//!
//! ## Security Invariant
//!
//! Signing operations take [`CanonicalBytes`](msez_core::CanonicalBytes) to
//! ensure the signed payload was properly canonicalized. This prevents
//! signature malleability from non-canonical serialization. You **cannot**
//! sign raw bytes — the type system enforces this.
//!
//! ## Serde
//!
//! Public keys and signatures serialize as lowercase hex strings.
//!
//! ## Spec Reference
//!
//! Implements Ed25519 signing per `tools/vc.py` which uses
//! `canonicalize_json()` → `jcs_canonicalize()` for signing input.

use ed25519_dalek::{Signer, Verifier};
use msez_core::CanonicalBytes;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::CryptoError;

// ---------------------------------------------------------------------------
// Hex encoding/decoding helpers (avoid adding `hex` crate dependency)
// ---------------------------------------------------------------------------

/// Encode bytes as lowercase hex string.
pub(crate) fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Decode a hex string into bytes.
pub(crate) fn hex_to_bytes(s: &str) -> Result<Vec<u8>, CryptoError> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(CryptoError::HexDecode(format!(
            "hex string has odd length: {}",
            s.len()
        )));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| {
                CryptoError::HexDecode(format!("invalid hex at position {i}: {e}"))
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Ed25519Signature
// ---------------------------------------------------------------------------

/// An Ed25519 digital signature (64 bytes).
///
/// Wraps the raw 64-byte signature value. Serializes as a lowercase hex string
/// for JSON interoperability with the Python `tools/vc.py` layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ed25519Signature([u8; 64]);

impl Ed25519Signature {
    /// Construct from raw 64-byte signature.
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    /// Construct from a byte slice, validating length.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, CryptoError> {
        let arr: [u8; 64] = bytes.try_into().map_err(|_| {
            CryptoError::InvalidSignatureLength(bytes.len())
        })?;
        Ok(Self(arr))
    }

    /// Access the raw 64-byte signature value.
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    /// Encode the signature as a lowercase hex string.
    pub fn to_hex(&self) -> String {
        bytes_to_hex(&self.0)
    }

    /// Decode a signature from a hex string (128 hex chars → 64 bytes).
    pub fn from_hex(s: &str) -> Result<Self, CryptoError> {
        let bytes = hex_to_bytes(s)?;
        Self::from_slice(&bytes)
    }
}

impl Serialize for Ed25519Signature {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Ed25519Signature {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// SigningKey
// ---------------------------------------------------------------------------

/// An Ed25519 signing (private) key.
///
/// Wraps `ed25519_dalek::SigningKey` with SEZ Stack conventions.
/// Signing input **must** be `&CanonicalBytes` — raw byte signing is not exposed.
///
/// ## Security
///
/// This type intentionally does **not** implement `Serialize`. Private keys
/// must not be casually serialized. Use [`SigningKey::to_bytes()`] for
/// explicit key export when required.
pub struct SigningKey {
    inner: ed25519_dalek::SigningKey,
}

impl SigningKey {
    /// Generate a new random Ed25519 signing key.
    ///
    /// Uses the provided cryptographically secure random number generator.
    pub fn generate<R: rand_core::CryptoRngCore>(csprng: &mut R) -> Self {
        Self {
            inner: ed25519_dalek::SigningKey::generate(csprng),
        }
    }

    /// Construct from raw 32-byte private key material.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            inner: ed25519_dalek::SigningKey::from_bytes(bytes),
        }
    }

    /// Derive the corresponding public verifying key.
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey {
            inner: self.inner.verifying_key(),
        }
    }

    /// Sign canonicalized data.
    ///
    /// The input **must** be `&CanonicalBytes`, not raw bytes. This ensures
    /// the signed payload was produced by the JCS canonicalization pipeline,
    /// preventing signature malleability from non-canonical serialization.
    ///
    /// Matches the signing path in `tools/vc.py:signing_input()` →
    /// `canonicalize_json()` → `jcs_canonicalize()`.
    pub fn sign(&self, data: &CanonicalBytes) -> Ed25519Signature {
        let sig = self.inner.sign(data.as_bytes());
        Ed25519Signature(sig.to_bytes())
    }

    /// Export the raw 32-byte private key material.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }
}

// Implement Debug manually to avoid leaking key material.
impl std::fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigningKey")
            .field("public", &self.verifying_key().to_hex())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// VerifyingKey
// ---------------------------------------------------------------------------

/// An Ed25519 verifying (public) key.
///
/// Used to verify signatures on VCs, attestations, and corridor proofs.
/// Serializes as a lowercase hex string (64 hex chars = 32 bytes).
#[derive(Debug, Clone)]
pub struct VerifyingKey {
    inner: ed25519_dalek::VerifyingKey,
}

impl VerifyingKey {
    /// Construct from raw 32-byte public key.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, CryptoError> {
        let inner = ed25519_dalek::VerifyingKey::from_bytes(bytes)
            .map_err(|e| CryptoError::InvalidPublicKey(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Construct from a hex string (64 hex chars → 32 bytes).
    pub fn from_hex(s: &str) -> Result<Self, CryptoError> {
        let bytes = hex_to_bytes(s)?;
        let arr: [u8; 32] = bytes.try_into().map_err(|_| {
            CryptoError::InvalidPublicKey(format!(
                "expected 32 bytes (64 hex chars), got {} bytes",
                s.len() / 2
            ))
        })?;
        Self::from_bytes(&arr)
    }

    /// Access the raw 32-byte public key value.
    pub fn as_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }

    /// Encode the public key as a lowercase hex string.
    pub fn to_hex(&self) -> String {
        bytes_to_hex(&self.inner.to_bytes())
    }

    /// Verify an Ed25519 signature over canonicalized data.
    ///
    /// The data **must** be `&CanonicalBytes`, not raw bytes. This ensures the
    /// verification is against properly canonicalized data, matching the signing
    /// path.
    ///
    /// Matches `tools/vc.py:verify_credential()` which verifies against
    /// `signing_input()` → `canonicalize_json()` → `jcs_canonicalize()`.
    pub fn verify(
        &self,
        data: &CanonicalBytes,
        signature: &Ed25519Signature,
    ) -> Result<(), CryptoError> {
        let sig = ed25519_dalek::Signature::from_bytes(signature.as_bytes());
        self.inner
            .verify(data.as_bytes(), &sig)
            .map_err(|e| CryptoError::VerificationFailed(e.to_string()))
    }
}

impl PartialEq for VerifyingKey {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for VerifyingKey {}

impl Serialize for VerifyingKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for VerifyingKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// Convenience functions
// ---------------------------------------------------------------------------

/// Sign canonical bytes with a signing key, returning the signature.
///
/// Equivalent to `key.sign(data)` — provided as a free function for
/// consistency with `verify()`.
pub fn sign(key: &SigningKey, data: &CanonicalBytes) -> Ed25519Signature {
    key.sign(data)
}

/// Verify a signature over canonical bytes using a verifying key.
///
/// Equivalent to `key.verify(data, signature)` — provided as a free function
/// for consistency with `sign()`.
pub fn verify(
    key: &VerifyingKey,
    data: &CanonicalBytes,
    signature: &Ed25519Signature,
) -> Result<(), CryptoError> {
    key.verify(data, signature)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;
    use serde_json::json;

    #[test]
    fn keypair_generation_produces_valid_keys() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();
        assert_eq!(vk.as_bytes().len(), 32);
        assert_eq!(sk.to_bytes().len(), 32);
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();

        let data = CanonicalBytes::new(&json!({"action": "transfer", "amount": 1000})).unwrap();
        let sig = sk.sign(&data);

        assert!(vk.verify(&data, &sig).is_ok());
    }

    #[test]
    fn verification_fails_with_wrong_key() {
        let sk1 = SigningKey::generate(&mut OsRng);
        let sk2 = SigningKey::generate(&mut OsRng);
        let vk2 = sk2.verifying_key();

        let data = CanonicalBytes::new(&json!({"msg": "hello"})).unwrap();
        let sig = sk1.sign(&data);

        assert!(vk2.verify(&data, &sig).is_err());
    }

    #[test]
    fn verification_fails_with_tampered_data() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();

        let original = CanonicalBytes::new(&json!({"val": 42})).unwrap();
        let tampered = CanonicalBytes::new(&json!({"val": 43})).unwrap();
        let sig = sk.sign(&original);

        assert!(vk.verify(&original, &sig).is_ok());
        assert!(vk.verify(&tampered, &sig).is_err());
    }

    #[test]
    fn signature_hex_roundtrip() {
        let sk = SigningKey::generate(&mut OsRng);
        let data = CanonicalBytes::new(&json!({"key": "value"})).unwrap();
        let sig = sk.sign(&data);

        let hex = sig.to_hex();
        assert_eq!(hex.len(), 128);
        let recovered = Ed25519Signature::from_hex(&hex).unwrap();
        assert_eq!(sig, recovered);
    }

    #[test]
    fn verifying_key_hex_roundtrip() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();

        let hex = vk.to_hex();
        assert_eq!(hex.len(), 64);
        let recovered = VerifyingKey::from_hex(&hex).unwrap();
        assert_eq!(vk, recovered);
    }

    #[test]
    fn signature_serde_roundtrip() {
        let sk = SigningKey::generate(&mut OsRng);
        let data = CanonicalBytes::new(&json!({"x": 1})).unwrap();
        let sig = sk.sign(&data);

        let json_str = serde_json::to_string(&sig).unwrap();
        assert!(json_str.starts_with('"'));
        assert!(json_str.ends_with('"'));
        let deserialized: Ed25519Signature = serde_json::from_str(&json_str).unwrap();
        assert_eq!(sig, deserialized);
    }

    #[test]
    fn verifying_key_serde_roundtrip() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();

        let json_str = serde_json::to_string(&vk).unwrap();
        let deserialized: VerifyingKey = serde_json::from_str(&json_str).unwrap();
        assert_eq!(vk, deserialized);
    }

    #[test]
    fn signing_key_from_bytes_roundtrip() {
        let sk = SigningKey::generate(&mut OsRng);
        let bytes = sk.to_bytes();
        let sk2 = SigningKey::from_bytes(&bytes);
        assert_eq!(sk.verifying_key(), sk2.verifying_key());
    }

    #[test]
    fn signing_key_debug_does_not_leak_private_key() {
        let sk = SigningKey::generate(&mut OsRng);
        let debug_str = format!("{sk:?}");
        assert!(debug_str.contains("SigningKey"));
        let private_hex = bytes_to_hex(&sk.to_bytes());
        assert!(!debug_str.contains(&private_hex));
    }

    #[test]
    fn invalid_signature_length_rejected() {
        let result = Ed25519Signature::from_slice(&[0u8; 32]);
        assert!(result.is_err());
        match result.unwrap_err() {
            CryptoError::InvalidSignatureLength(len) => assert_eq!(len, 32),
            other => panic!("expected InvalidSignatureLength, got: {other}"),
        }
    }

    #[test]
    fn invalid_hex_rejected() {
        assert!(Ed25519Signature::from_hex("not_hex").is_err());
        assert!(VerifyingKey::from_hex("xyz").is_err());
        assert!(Ed25519Signature::from_hex("abc").is_err());
    }

    #[test]
    fn free_functions_match_method_calls() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();
        let data = CanonicalBytes::new(&json!({"test": true})).unwrap();

        let sig_method = sk.sign(&data);
        let sig_free = sign(&sk, &data);
        assert_eq!(sig_method, sig_free);

        assert!(verify(&vk, &data, &sig_method).is_ok());
    }

    #[test]
    fn deterministic_signing() {
        let sk = SigningKey::generate(&mut OsRng);
        let data = CanonicalBytes::new(&json!({"deterministic": true})).unwrap();
        let sig1 = sk.sign(&data);
        let sig2 = sk.sign(&data);
        assert_eq!(sig1, sig2);
    }
}
