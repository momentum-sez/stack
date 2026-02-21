//! # HSM/KMS Key Provider Abstraction
//!
//! Trait-based abstraction for key management that supports multiple backends:
//!
//! - **Software**: In-process Ed25519 via `ed25519-dalek` (dev/test/Phase 1-2).
//! - **AWS KMS**: Delegate signing to AWS KMS (Phase 3 production).
//! - **AWS CloudHSM**: FIPS 140-2 Level 3 hardware key storage (Phase 3+).
//!
//! ## Design Principles
//!
//! 1. **Private keys never leave the provider.** The `KeyProvider` trait exposes
//!    `sign()` and `verify()` but not raw key bytes. Hardware backends enforce
//!    this physically; the software backend enforces it by API contract.
//!
//! 2. **All signing takes `CanonicalBytes`.** This preserves the EZ Stack
//!    canonicalization invariant regardless of backend.
//!
//! 3. **Key identifiers are opaque strings.** AWS KMS uses ARNs, CloudHSM uses
//!    key handles, software uses hex-encoded public keys. The provider maps
//!    identifiers to its internal representation.
//!
//! 4. **Zeroize on drop.** The software provider zeroizes key material when
//!    the provider is dropped. Hardware backends handle this in firmware.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use mez_crypto::hsm::{KeyProvider, SoftwareKeyProvider, KeyAlgorithm};
//! use mez_core::CanonicalBytes;
//! use serde_json::json;
//!
//! let mut provider = SoftwareKeyProvider::new();
//! let key_id = provider.generate_key(KeyAlgorithm::Ed25519).unwrap();
//! let data = CanonicalBytes::new(&json!({"action": "transfer"})).unwrap();
//! let signature = provider.sign(&key_id, &data).unwrap();
//! assert!(provider.verify(&key_id, &data, &signature).is_ok());
//! ```

use mez_core::CanonicalBytes;
use std::collections::HashMap;
use zeroize::Zeroize;

use crate::ed25519::{bytes_to_hex, Ed25519Signature};
use crate::error::CryptoError;

// ---------------------------------------------------------------------------
// Key algorithm enumeration
// ---------------------------------------------------------------------------

/// Cryptographic algorithms supported by key providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAlgorithm {
    /// Ed25519 — used for VCs, corridor attestations, watcher bonds.
    Ed25519,
}

impl std::fmt::Display for KeyAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ed25519 => write!(f, "Ed25519"),
        }
    }
}

// ---------------------------------------------------------------------------
// Key metadata
// ---------------------------------------------------------------------------

/// Metadata about a managed key.
#[derive(Debug, Clone)]
pub struct KeyMetadata {
    /// Opaque key identifier (ARN, handle, or hex public key).
    pub key_id: String,
    /// Algorithm of the key.
    pub algorithm: KeyAlgorithm,
    /// Provider type that manages this key.
    pub provider: ProviderType,
    /// Whether the key is enabled for signing operations.
    pub enabled: bool,
}

/// Type of key provider backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderType {
    /// In-process software keys (ed25519-dalek).
    Software,
    /// AWS Key Management Service.
    AwsKms,
    /// AWS CloudHSM (FIPS 140-2 Level 3).
    AwsCloudHsm,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Software => write!(f, "software"),
            Self::AwsKms => write!(f, "aws-kms"),
            Self::AwsCloudHsm => write!(f, "aws-cloudhsm"),
        }
    }
}

// ---------------------------------------------------------------------------
// KeyProvider trait
// ---------------------------------------------------------------------------

/// Trait for key management backends.
///
/// Implementations MUST:
/// - Never expose raw private key bytes through this trait.
/// - Zeroize any in-memory key material on drop.
/// - Return `CryptoError` with diagnostic context on failure.
///
/// The trait is object-safe for use with `Box<dyn KeyProvider>`.
pub trait KeyProvider: Send + Sync {
    /// Generate a new key pair and return its identifier.
    ///
    /// The key is immediately available for signing and verification.
    fn generate_key(&mut self, algorithm: KeyAlgorithm) -> Result<String, CryptoError>;

    /// Import a key from an external identifier.
    ///
    /// For AWS KMS: the identifier is a key ARN or alias.
    /// For software: the identifier is a hex-encoded 32-byte seed.
    fn import_key(
        &mut self,
        algorithm: KeyAlgorithm,
        key_material: &[u8],
    ) -> Result<String, CryptoError>;

    /// Sign canonicalized data with the identified key.
    ///
    /// The data MUST be `CanonicalBytes` — raw byte signing is not exposed
    /// through the provider trait to maintain the canonicalization invariant.
    fn sign(
        &self,
        key_id: &str,
        data: &CanonicalBytes,
    ) -> Result<Vec<u8>, CryptoError>;

    /// Verify a signature over canonicalized data.
    fn verify(
        &self,
        key_id: &str,
        data: &CanonicalBytes,
        signature: &[u8],
    ) -> Result<(), CryptoError>;

    /// Get the public key bytes for the identified key.
    fn public_key(&self, key_id: &str) -> Result<Vec<u8>, CryptoError>;

    /// Get metadata about a managed key.
    fn key_metadata(&self, key_id: &str) -> Result<KeyMetadata, CryptoError>;

    /// List all key identifiers managed by this provider.
    fn list_keys(&self) -> Vec<String>;

    /// Disable a key (prevent further signing but allow verification).
    fn disable_key(&mut self, key_id: &str) -> Result<(), CryptoError>;

    /// The provider type.
    fn provider_type(&self) -> ProviderType;
}

// ---------------------------------------------------------------------------
// Software key entry (internal)
// ---------------------------------------------------------------------------

/// A software-managed key pair with zeroize-on-drop.
struct SoftwareKeyEntry {
    signing_key: crate::ed25519::SigningKey,
    algorithm: KeyAlgorithm,
    enabled: bool,
}

impl Drop for SoftwareKeyEntry {
    fn drop(&mut self) {
        // SigningKey already implements Zeroize + Drop, but we explicitly
        // call it here for defense-in-depth.
        self.signing_key.zeroize();
    }
}

// ---------------------------------------------------------------------------
// SoftwareKeyProvider
// ---------------------------------------------------------------------------

/// In-process key provider using `ed25519-dalek`.
///
/// Suitable for development, testing, and Phase 1-2 deployments.
/// For Phase 3 production, use `AwsKmsKeyProvider` or `AwsCloudHsmKeyProvider`.
///
/// ## Security
///
/// - Key material lives in process memory — protected by OS-level isolation only.
/// - All key material is zeroized on drop via the `Zeroize` trait.
/// - `Debug` never prints private key bytes.
pub struct SoftwareKeyProvider {
    keys: HashMap<String, SoftwareKeyEntry>,
}

impl SoftwareKeyProvider {
    /// Create a new empty software key provider.
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    /// Convenience: sign and return an `Ed25519Signature` (typed).
    pub fn sign_ed25519(
        &self,
        key_id: &str,
        data: &CanonicalBytes,
    ) -> Result<Ed25519Signature, CryptoError> {
        let raw = self.sign(key_id, data)?;
        Ed25519Signature::from_slice(&raw)
    }

    /// Convenience: verify using an `Ed25519Signature` (typed).
    pub fn verify_ed25519(
        &self,
        key_id: &str,
        data: &CanonicalBytes,
        signature: &Ed25519Signature,
    ) -> Result<(), CryptoError> {
        self.verify(key_id, data, signature.as_bytes())
    }
}

impl Default for SoftwareKeyProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SoftwareKeyProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SoftwareKeyProvider")
            .field("key_count", &self.keys.len())
            .field("key_ids", &self.keys.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Drop for SoftwareKeyProvider {
    fn drop(&mut self) {
        // Each SoftwareKeyEntry handles its own zeroize in its Drop impl.
        // We clear the HashMap to trigger drops immediately.
        self.keys.clear();
    }
}

impl KeyProvider for SoftwareKeyProvider {
    fn generate_key(&mut self, algorithm: KeyAlgorithm) -> Result<String, CryptoError> {
        match algorithm {
            KeyAlgorithm::Ed25519 => {
                let mut rng = rand_core::OsRng;
                let signing_key = crate::ed25519::SigningKey::generate(&mut rng);
                let key_id = bytes_to_hex(&signing_key.verifying_key().as_bytes());
                self.keys.insert(
                    key_id.clone(),
                    SoftwareKeyEntry {
                        signing_key,
                        algorithm,
                        enabled: true,
                    },
                );
                Ok(key_id)
            }
        }
    }

    fn import_key(
        &mut self,
        algorithm: KeyAlgorithm,
        key_material: &[u8],
    ) -> Result<String, CryptoError> {
        match algorithm {
            KeyAlgorithm::Ed25519 => {
                if key_material.len() != 32 {
                    return Err(CryptoError::InvalidPublicKey(format!(
                        "Ed25519 seed must be 32 bytes, got {}",
                        key_material.len()
                    )));
                }
                let mut seed = [0u8; 32];
                seed.copy_from_slice(key_material);
                let signing_key = crate::ed25519::SigningKey::from_bytes(&seed);
                seed.zeroize();
                let key_id = bytes_to_hex(&signing_key.verifying_key().as_bytes());
                self.keys.insert(
                    key_id.clone(),
                    SoftwareKeyEntry {
                        signing_key,
                        algorithm,
                        enabled: true,
                    },
                );
                Ok(key_id)
            }
        }
    }

    fn sign(
        &self,
        key_id: &str,
        data: &CanonicalBytes,
    ) -> Result<Vec<u8>, CryptoError> {
        let entry = self.keys.get(key_id).ok_or_else(|| {
            CryptoError::Cas(format!("key not found: {key_id}"))
        })?;
        if !entry.enabled {
            return Err(CryptoError::Cas(format!("key is disabled: {key_id}")));
        }
        let sig = entry.signing_key.sign(data);
        Ok(sig.as_bytes().to_vec())
    }

    fn verify(
        &self,
        key_id: &str,
        data: &CanonicalBytes,
        signature: &[u8],
    ) -> Result<(), CryptoError> {
        let entry = self.keys.get(key_id).ok_or_else(|| {
            CryptoError::Cas(format!("key not found: {key_id}"))
        })?;
        let sig = Ed25519Signature::from_slice(signature)?;
        let vk = entry.signing_key.verifying_key();
        vk.verify(data, &sig)
    }

    fn public_key(&self, key_id: &str) -> Result<Vec<u8>, CryptoError> {
        let entry = self.keys.get(key_id).ok_or_else(|| {
            CryptoError::Cas(format!("key not found: {key_id}"))
        })?;
        Ok(entry.signing_key.verifying_key().as_bytes().to_vec())
    }

    fn key_metadata(&self, key_id: &str) -> Result<KeyMetadata, CryptoError> {
        let entry = self.keys.get(key_id).ok_or_else(|| {
            CryptoError::Cas(format!("key not found: {key_id}"))
        })?;
        Ok(KeyMetadata {
            key_id: key_id.to_string(),
            algorithm: entry.algorithm,
            provider: ProviderType::Software,
            enabled: entry.enabled,
        })
    }

    fn list_keys(&self) -> Vec<String> {
        self.keys.keys().cloned().collect()
    }

    fn disable_key(&mut self, key_id: &str) -> Result<(), CryptoError> {
        let entry = self.keys.get_mut(key_id).ok_or_else(|| {
            CryptoError::Cas(format!("key not found: {key_id}"))
        })?;
        entry.enabled = false;
        Ok(())
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Software
    }
}

// ---------------------------------------------------------------------------
// AWS KMS Key Provider (stub — requires async runtime + AWS SDK)
// ---------------------------------------------------------------------------

/// AWS KMS key provider configuration.
///
/// Phase 3 production deployments use AWS KMS for key management. Keys are
/// identified by their ARN and never leave the KMS boundary.
///
/// ## Activation
///
/// Requires the `aws-sdk-kms` crate (not yet in workspace). The provider
/// delegates all cryptographic operations to KMS via the AWS SDK.
///
/// ## Configuration
///
/// ```text
/// MEZ_KEY_PROVIDER=aws-kms
/// MEZ_KMS_KEY_ARN=arn:aws:kms:us-east-1:123456789012:key/mrk-...
/// MEZ_KMS_REGION=us-east-1
/// ```
#[derive(Debug, Clone)]
pub struct AwsKmsConfig {
    /// AWS region for KMS API calls.
    pub region: String,
    /// Key ARN or alias for the zone's signing key.
    pub key_arn: String,
    /// Optional endpoint URL override (for LocalStack/testing).
    pub endpoint_url: Option<String>,
}

/// AWS CloudHSM key provider configuration.
///
/// Phase 3+ deployments requiring FIPS 140-2 Level 3 compliance use
/// CloudHSM for key storage and signing operations.
///
/// ## Activation
///
/// Requires the `aws-cloudhsm-pkcs11` integration. The provider
/// communicates with CloudHSM via the PKCS#11 interface.
///
/// ## Configuration
///
/// ```text
/// MEZ_KEY_PROVIDER=aws-cloudhsm
/// MEZ_HSM_CLUSTER_ID=cluster-...
/// MEZ_HSM_PIN=${HSM_PIN:?must be set}
/// ```
#[derive(Debug, Clone)]
pub struct AwsCloudHsmConfig {
    /// CloudHSM cluster ID.
    pub cluster_id: String,
    /// HSM user type (CU = Crypto User for signing).
    pub hsm_user: String,
    /// PKCS#11 library path.
    pub pkcs11_lib_path: String,
}

/// Select a key provider based on environment configuration.
///
/// Reads `MEZ_KEY_PROVIDER` environment variable:
/// - `"software"` (default): In-process ed25519-dalek.
/// - `"aws-kms"`: AWS KMS (Phase 3).
/// - `"aws-cloudhsm"`: AWS CloudHSM (Phase 3+).
///
/// Returns a boxed `SoftwareKeyProvider` for now. AWS providers will be
/// wired in when the AWS SDK dependencies are added.
pub fn create_key_provider_from_env() -> Result<Box<dyn KeyProvider>, CryptoError> {
    let provider_type = std::env::var("MEZ_KEY_PROVIDER").unwrap_or_else(|_| "software".into());
    match provider_type.as_str() {
        "software" => Ok(Box::new(SoftwareKeyProvider::new())),
        "aws-kms" => Err(CryptoError::NotImplemented(
            "AWS KMS provider requires aws-sdk-kms dependency (Phase 3)".into(),
        )),
        "aws-cloudhsm" => Err(CryptoError::NotImplemented(
            "AWS CloudHSM provider requires PKCS#11 integration (Phase 3+)".into(),
        )),
        other => Err(CryptoError::Cas(format!(
            "unknown key provider type: {other} (expected: software, aws-kms, aws-cloudhsm)"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn software_provider_generate_and_sign() {
        let mut provider = SoftwareKeyProvider::new();
        let key_id = provider.generate_key(KeyAlgorithm::Ed25519).unwrap();
        assert_eq!(key_id.len(), 64); // 32 bytes = 64 hex chars

        let data = CanonicalBytes::new(&json!({"action": "transfer", "amount": 1000})).unwrap();
        let sig = provider.sign(&key_id, &data).unwrap();
        assert_eq!(sig.len(), 64); // Ed25519 signature = 64 bytes

        assert!(provider.verify(&key_id, &data, &sig).is_ok());
    }

    #[test]
    fn software_provider_verify_rejects_wrong_data() {
        let mut provider = SoftwareKeyProvider::new();
        let key_id = provider.generate_key(KeyAlgorithm::Ed25519).unwrap();

        let data1 = CanonicalBytes::new(&json!({"val": 1})).unwrap();
        let data2 = CanonicalBytes::new(&json!({"val": 2})).unwrap();
        let sig = provider.sign(&key_id, &data1).unwrap();

        assert!(provider.verify(&key_id, &data2, &sig).is_err());
    }

    #[test]
    fn software_provider_verify_rejects_wrong_key() {
        let mut provider = SoftwareKeyProvider::new();
        let key1 = provider.generate_key(KeyAlgorithm::Ed25519).unwrap();
        let key2 = provider.generate_key(KeyAlgorithm::Ed25519).unwrap();

        let data = CanonicalBytes::new(&json!({"msg": "hello"})).unwrap();
        let sig = provider.sign(&key1, &data).unwrap();

        assert!(provider.verify(&key2, &data, &sig).is_err());
    }

    #[test]
    fn software_provider_import_key() {
        let mut provider = SoftwareKeyProvider::new();

        // Generate a seed and import it
        let seed = [42u8; 32];
        let key_id = provider
            .import_key(KeyAlgorithm::Ed25519, &seed)
            .unwrap();

        // Re-importing the same seed produces the same key_id
        let mut provider2 = SoftwareKeyProvider::new();
        let key_id2 = provider2
            .import_key(KeyAlgorithm::Ed25519, &seed)
            .unwrap();
        assert_eq!(key_id, key_id2);

        // Signing with both produces the same signature
        let data = CanonicalBytes::new(&json!({"test": true})).unwrap();
        let sig1 = provider.sign(&key_id, &data).unwrap();
        let sig2 = provider2.sign(&key_id2, &data).unwrap();
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn software_provider_import_rejects_wrong_length() {
        let mut provider = SoftwareKeyProvider::new();
        let result = provider.import_key(KeyAlgorithm::Ed25519, &[0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    fn software_provider_sign_unknown_key_fails() {
        let provider = SoftwareKeyProvider::new();
        let data = CanonicalBytes::new(&json!({})).unwrap();
        assert!(provider.sign("nonexistent", &data).is_err());
    }

    #[test]
    fn software_provider_disable_key() {
        let mut provider = SoftwareKeyProvider::new();
        let key_id = provider.generate_key(KeyAlgorithm::Ed25519).unwrap();
        let data = CanonicalBytes::new(&json!({"test": true})).unwrap();

        // Signing works before disable
        assert!(provider.sign(&key_id, &data).is_ok());

        // Disable the key
        provider.disable_key(&key_id).unwrap();

        // Signing fails after disable
        assert!(provider.sign(&key_id, &data).is_err());

        // Verification still works (disabled keys can still verify)
        // First, get a signature from before disable
        let mut provider2 = SoftwareKeyProvider::new();
        let seed = [99u8; 32];
        let kid = provider2.import_key(KeyAlgorithm::Ed25519, &seed).unwrap();
        let sig = provider2.sign(&kid, &data).unwrap();

        // Import same key, disable, and verify should still work
        let kid2 = provider.import_key(KeyAlgorithm::Ed25519, &seed).unwrap();
        provider.disable_key(&kid2).unwrap();
        assert!(provider.verify(&kid2, &data, &sig).is_ok());
    }

    #[test]
    fn software_provider_key_metadata() {
        let mut provider = SoftwareKeyProvider::new();
        let key_id = provider.generate_key(KeyAlgorithm::Ed25519).unwrap();

        let meta = provider.key_metadata(&key_id).unwrap();
        assert_eq!(meta.key_id, key_id);
        assert_eq!(meta.algorithm, KeyAlgorithm::Ed25519);
        assert_eq!(meta.provider, ProviderType::Software);
        assert!(meta.enabled);
    }

    #[test]
    fn software_provider_public_key() {
        let mut provider = SoftwareKeyProvider::new();
        let key_id = provider.generate_key(KeyAlgorithm::Ed25519).unwrap();

        let pk = provider.public_key(&key_id).unwrap();
        assert_eq!(pk.len(), 32);
    }

    #[test]
    fn software_provider_list_keys() {
        let mut provider = SoftwareKeyProvider::new();
        assert!(provider.list_keys().is_empty());

        let k1 = provider.generate_key(KeyAlgorithm::Ed25519).unwrap();
        let k2 = provider.generate_key(KeyAlgorithm::Ed25519).unwrap();

        let keys = provider.list_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&k1));
        assert!(keys.contains(&k2));
    }

    #[test]
    fn software_provider_debug_does_not_leak() {
        let mut provider = SoftwareKeyProvider::new();
        let key_id = provider.generate_key(KeyAlgorithm::Ed25519).unwrap();
        let debug = format!("{provider:?}");
        assert!(debug.contains("SoftwareKeyProvider"));
        assert!(debug.contains(&key_id));
        // Private key bytes should not appear
        let entry = provider.keys.get(&key_id).unwrap();
        let private_hex = bytes_to_hex(&entry.signing_key.to_bytes());
        assert!(!debug.contains(&private_hex));
    }

    #[test]
    fn software_provider_ed25519_convenience() {
        let mut provider = SoftwareKeyProvider::new();
        let key_id = provider.generate_key(KeyAlgorithm::Ed25519).unwrap();
        let data = CanonicalBytes::new(&json!({"convenience": true})).unwrap();

        let sig = provider.sign_ed25519(&key_id, &data).unwrap();
        assert!(provider.verify_ed25519(&key_id, &data, &sig).is_ok());
    }

    #[test]
    fn create_provider_from_env_defaults_to_software() {
        // Unset env var to get default
        std::env::remove_var("MEZ_KEY_PROVIDER");
        let provider = create_key_provider_from_env().unwrap();
        assert_eq!(provider.provider_type(), ProviderType::Software);
    }

    #[test]
    fn create_provider_from_env_kms_not_implemented() {
        std::env::set_var("MEZ_KEY_PROVIDER", "aws-kms");
        let result = create_key_provider_from_env();
        assert!(result.is_err());
        std::env::remove_var("MEZ_KEY_PROVIDER");
    }

    #[test]
    fn create_provider_from_env_unknown_type() {
        std::env::set_var("MEZ_KEY_PROVIDER", "magic-hsm");
        let result = create_key_provider_from_env();
        assert!(result.is_err());
        std::env::remove_var("MEZ_KEY_PROVIDER");
    }

    #[test]
    fn key_algorithm_display() {
        assert_eq!(format!("{}", KeyAlgorithm::Ed25519), "Ed25519");
    }

    #[test]
    fn provider_type_display() {
        assert_eq!(format!("{}", ProviderType::Software), "software");
        assert_eq!(format!("{}", ProviderType::AwsKms), "aws-kms");
        assert_eq!(format!("{}", ProviderType::AwsCloudHsm), "aws-cloudhsm");
    }

    #[test]
    fn disable_nonexistent_key_fails() {
        let mut provider = SoftwareKeyProvider::new();
        assert!(provider.disable_key("nonexistent").is_err());
    }

    #[test]
    fn metadata_nonexistent_key_fails() {
        let provider = SoftwareKeyProvider::new();
        assert!(provider.key_metadata("nonexistent").is_err());
    }

    #[test]
    fn public_key_nonexistent_fails() {
        let provider = SoftwareKeyProvider::new();
        assert!(provider.public_key("nonexistent").is_err());
    }
}
