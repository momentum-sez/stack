//! # Key Provider Abstraction
//!
//! Abstracts Ed25519 key storage and signing behind a trait, enabling
//! multiple backends:
//!
//! - [`LocalKeyProvider`]: In-memory key for development and testing.
//! - [`EnvKeyProvider`]: Loads key material from an environment variable
//!   (hex-encoded 32-byte Ed25519 seed). Suitable for container deployments
//!   where secrets are injected via environment.
//! - [`AwsKmsEnvelopeKeyProvider`]: Key material encrypted at rest by an
//!   AWS KMS Customer Master Key (CMK). At startup, the encrypted key
//!   blob is decrypted via KMS, producing the Ed25519 seed in memory.
//!   Key material is zeroized on drop. Requires the `aws-kms` feature.
//!
//! ## Design Decision: Envelope Encryption
//!
//! AWS KMS does not natively support Ed25519 signing. The envelope
//! encryption pattern protects the Ed25519 private key at rest using
//! KMS while performing signing locally with the decrypted key.
//! This is the standard approach for non-native key types in KMS.
//!
//! ## Security Invariants
//!
//! - All key material implements `Zeroize + Drop` for secure cleanup.
//! - `KeyProvider` is `Send + Sync` for use across async tasks.
//! - Signing input is `&CanonicalBytes` (never raw bytes).

use crate::ed25519::{Ed25519Signature, SigningKey, VerifyingKey};
use crate::error::CryptoError;
use mez_core::CanonicalBytes;

/// Trait for Ed25519 key storage and signing backends.
///
/// Implementations must be `Send + Sync` for use in multi-threaded
/// async runtimes. Signing input must be `&CanonicalBytes` to prevent
/// signature malleability from non-canonical serialization.
pub trait KeyProvider: Send + Sync {
    /// Sign canonicalized data with the managed Ed25519 key.
    fn sign(&self, data: &CanonicalBytes) -> Result<Ed25519Signature, CryptoError>;

    /// Return the Ed25519 verifying (public) key.
    fn verifying_key(&self) -> Result<VerifyingKey, CryptoError>;

    /// Human-readable name for this provider (for diagnostics/logging).
    fn provider_name(&self) -> &str;
}

// ─── LocalKeyProvider ────────────────────────────────────────────────────

/// In-memory Ed25519 key provider for development and testing.
///
/// Wraps a [`SigningKey`] directly. Key material lives in process memory
/// and is zeroized on drop.
pub struct LocalKeyProvider {
    key: SigningKey,
}

impl LocalKeyProvider {
    /// Create from an existing signing key.
    pub fn new(key: SigningKey) -> Self {
        Self { key }
    }

    /// Generate a new random key using the OS CSPRNG.
    pub fn generate() -> Self {
        Self {
            key: SigningKey::generate(&mut rand_core::OsRng),
        }
    }

    /// Create from raw 32-byte seed.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        Self {
            key: SigningKey::from_bytes(seed),
        }
    }
}

impl KeyProvider for LocalKeyProvider {
    fn sign(&self, data: &CanonicalBytes) -> Result<Ed25519Signature, CryptoError> {
        Ok(self.key.sign(data))
    }

    fn verifying_key(&self) -> Result<VerifyingKey, CryptoError> {
        Ok(self.key.verifying_key())
    }

    fn provider_name(&self) -> &str {
        "LocalKeyProvider"
    }
}

// ─── EnvKeyProvider ──────────────────────────────────────────────────────

/// Loads an Ed25519 signing key from an environment variable.
///
/// The environment variable must contain a 64-character hex string
/// encoding the 32-byte Ed25519 seed. The key is loaded once at
/// construction and held in memory (zeroized on drop).
///
/// ## Example
///
/// ```bash
/// export MEZ_SIGNING_KEY="deadbeef..."  # 64 hex chars
/// ```
pub struct EnvKeyProvider {
    key: SigningKey,
    var_name: String,
}

impl EnvKeyProvider {
    /// Load the signing key from the named environment variable.
    ///
    /// Returns `CryptoError::NotImplemented` if the variable is not set
    /// or contains invalid hex.
    pub fn from_env(var_name: &str) -> Result<Self, CryptoError> {
        let hex = std::env::var(var_name).map_err(|_| {
            CryptoError::NotImplemented(format!(
                "environment variable {var_name} not set"
            ))
        })?;

        let bytes = crate::ed25519::hex_to_bytes(&hex)?;
        let seed: [u8; 32] = bytes.try_into().map_err(|_| {
            CryptoError::InvalidSigningKey(format!(
                "expected 32 bytes (64 hex chars) in {var_name}, got {} bytes",
                hex.len() / 2
            ))
        })?;

        let key = SigningKey::from_bytes(&seed);
        Ok(Self {
            key,
            var_name: var_name.to_string(),
        })
    }
}

impl EnvKeyProvider {
    /// Return the environment variable name this provider was loaded from.
    pub fn var_name(&self) -> &str {
        &self.var_name
    }
}

impl KeyProvider for EnvKeyProvider {
    fn sign(&self, data: &CanonicalBytes) -> Result<Ed25519Signature, CryptoError> {
        Ok(self.key.sign(data))
    }

    fn verifying_key(&self) -> Result<VerifyingKey, CryptoError> {
        Ok(self.key.verifying_key())
    }

    fn provider_name(&self) -> &str {
        "EnvKeyProvider"
    }
}

// ─── AWS KMS Envelope Key Provider ──────────────────────────────────────

#[cfg(feature = "aws-kms")]
mod aws_kms;

#[cfg(feature = "aws-kms")]
pub use aws_kms::AwsKmsEnvelopeKeyProvider;

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_provider_sign_and_verify() {
        let provider = LocalKeyProvider::generate();
        let data = CanonicalBytes::new(&serde_json::json!({"action": "test"}))
            .expect("canonical");
        let sig = provider.sign(&data).expect("sign");
        let vk = provider.verifying_key().expect("vk");
        assert!(vk.verify(&data, &sig).is_ok());
    }

    #[test]
    fn local_provider_from_seed_deterministic() {
        let seed = [42u8; 32];
        let p1 = LocalKeyProvider::from_seed(&seed);
        let p2 = LocalKeyProvider::from_seed(&seed);
        assert_eq!(
            p1.verifying_key().expect("vk1"),
            p2.verifying_key().expect("vk2"),
        );
    }

    #[test]
    fn local_provider_name() {
        let provider = LocalKeyProvider::generate();
        assert_eq!(provider.provider_name(), "LocalKeyProvider");
    }

    #[test]
    fn local_provider_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LocalKeyProvider>();
    }

    #[test]
    fn env_provider_missing_var() {
        let result = EnvKeyProvider::from_env("MEZ_TEST_KEY_THAT_DOES_NOT_EXIST_12345");
        assert!(result.is_err());
    }

    #[test]
    fn env_provider_name() {
        // Set a temp env var for testing.
        let seed = [0xab_u8; 32];
        let hex: String = seed.iter().map(|b| format!("{b:02x}")).collect();
        let var = "MEZ_TEST_KEY_PROVIDER_TEST";
        std::env::set_var(var, &hex);

        let provider = EnvKeyProvider::from_env(var).expect("from_env");
        assert_eq!(provider.provider_name(), "EnvKeyProvider");

        // Verify signing works.
        let data = CanonicalBytes::new(&serde_json::json!({"env": true}))
            .expect("canonical");
        let sig = provider.sign(&data).expect("sign");
        let vk = provider.verifying_key().expect("vk");
        assert!(vk.verify(&data, &sig).is_ok());

        // Clean up.
        std::env::remove_var(var);
    }

    #[test]
    fn env_provider_invalid_hex() {
        let var = "MEZ_TEST_KEY_PROVIDER_BAD_HEX";
        std::env::set_var(var, "not-valid-hex");
        let result = EnvKeyProvider::from_env(var);
        assert!(result.is_err());
        std::env::remove_var(var);
    }

    #[test]
    fn env_provider_wrong_length() {
        let var = "MEZ_TEST_KEY_PROVIDER_SHORT";
        std::env::set_var(var, "aabbccdd"); // 4 bytes, not 32
        let result = EnvKeyProvider::from_env(var);
        assert!(result.is_err());
        std::env::remove_var(var);
    }

    #[test]
    fn key_provider_trait_object_safe() {
        let provider = LocalKeyProvider::generate();
        let _boxed: Box<dyn KeyProvider> = Box::new(provider);
    }

    #[test]
    fn different_providers_same_seed_same_output() {
        let seed = [0x99_u8; 32];
        let hex: String = seed.iter().map(|b| format!("{b:02x}")).collect();
        let var = "MEZ_TEST_KEY_PROVIDER_COMPAT";
        std::env::set_var(var, &hex);

        let local = LocalKeyProvider::from_seed(&seed);
        let env = EnvKeyProvider::from_env(var).expect("from_env");

        let data = CanonicalBytes::new(&serde_json::json!({"compat": true}))
            .expect("canonical");
        let sig_local = local.sign(&data).expect("sign local");
        let sig_env = env.sign(&data).expect("sign env");

        assert_eq!(sig_local, sig_env);
        assert_eq!(
            local.verifying_key().expect("vk"),
            env.verifying_key().expect("vk"),
        );

        std::env::remove_var(var);
    }

    #[test]
    fn local_provider_zeroizes_on_drop() {
        // Verify that Drop runs without panicking.
        let provider = LocalKeyProvider::generate();
        let _vk = provider.verifying_key().expect("vk");
        drop(provider);
    }
}
