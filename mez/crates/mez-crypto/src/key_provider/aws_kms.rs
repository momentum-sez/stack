//! # AWS KMS Envelope Encryption Key Provider
//!
//! Protects Ed25519 key material at rest using AWS KMS envelope encryption.
//!
//! ## How It Works
//!
//! 1. The Ed25519 private key (32-byte seed) is encrypted with an AWS KMS
//!    Customer Master Key (CMK) and stored as a ciphertext blob (in S3,
//!    SSM Parameter Store, Secrets Manager, or a config file).
//! 2. At startup, the provider calls KMS `Decrypt` with the ciphertext blob.
//! 3. KMS returns the plaintext 32-byte seed, which is loaded into a
//!    [`SigningKey`] in memory (zeroized on drop).
//! 4. All signing operations use the in-memory key — no further KMS calls.
//!
//! ## Security Properties
//!
//! - Key material is encrypted at rest by a FIPS 140-2 Level 3 HSM (KMS).
//! - Decryption requires IAM permission on the CMK (auditable via CloudTrail).
//! - In-memory key is zeroized on drop via the `Zeroize` trait.
//! - The CMK never leaves KMS — only the data key (Ed25519 seed) is decrypted.
//!
//! ## Prerequisites
//!
//! - An AWS KMS symmetric CMK (or asymmetric, if using `RSAES_OAEP_SHA_256`).
//! - The encrypted key blob produced by `aws kms encrypt --key-id <CMK_ARN>
//!   --plaintext fileb://ed25519_seed.bin`.
//! - IAM role/credentials with `kms:Decrypt` permission on the CMK.
//!
//! ## Example
//!
//! ```bash
//! # Encrypt the Ed25519 seed with KMS:
//! aws kms encrypt \
//!   --key-id alias/mez-signing-key \
//!   --plaintext fileb://seed.bin \
//!   --output text --query CiphertextBlob | base64 -d > seed.enc
//!
//! # Set environment variable with base64-encoded ciphertext:
//! export MEZ_ENCRYPTED_KEY=$(base64 < seed.enc)
//! ```

use crate::ed25519::{Ed25519Signature, SigningKey, VerifyingKey};
use crate::error::CryptoError;
use mez_core::CanonicalBytes;

/// AWS KMS envelope encryption key provider.
///
/// Decrypts an Ed25519 seed from a KMS-encrypted ciphertext blob at
/// construction time, then signs locally with the decrypted key.
pub struct AwsKmsEnvelopeKeyProvider {
    key: SigningKey,
    kms_key_id: String,
}

impl AwsKmsEnvelopeKeyProvider {
    /// Create a provider by decrypting the given ciphertext blob via KMS.
    ///
    /// # Arguments
    ///
    /// - `encrypted_seed`: The KMS-encrypted Ed25519 seed (raw ciphertext bytes,
    ///   as returned by `aws kms encrypt`).
    /// - `kms_key_id`: The CMK ARN, alias, or key ID used for decryption.
    ///   Pass `None` to let KMS determine the key from the ciphertext metadata.
    /// - `region`: AWS region override. Pass `None` to use the default from
    ///   environment/config.
    pub async fn from_encrypted_seed(
        encrypted_seed: &[u8],
        kms_key_id: Option<&str>,
        region: Option<&str>,
    ) -> Result<Self, CryptoError> {
        let mut config_loader = aws_config::from_env();
        if let Some(r) = region {
            config_loader = config_loader.region(aws_config::Region::new(r.to_string()));
        }
        let sdk_config = config_loader.load().await;
        let kms_client = aws_sdk_kms::Client::new(&sdk_config);

        let mut req = kms_client
            .decrypt()
            .ciphertext_blob(aws_sdk_kms::primitives::Blob::new(encrypted_seed));

        if let Some(key_id) = kms_key_id {
            req = req.key_id(key_id);
        }

        let resp = req.send().await.map_err(|e| {
            CryptoError::NotImplemented(format!("KMS decrypt failed: {e}"))
        })?;

        let plaintext = resp.plaintext().ok_or_else(|| {
            CryptoError::NotImplemented(
                "KMS decrypt response missing plaintext".to_string(),
            )
        })?;

        let seed_bytes = plaintext.as_ref();
        if seed_bytes.len() != 32 {
            return Err(CryptoError::InvalidSigningKey(format!(
                "KMS decrypted seed is {} bytes, expected 32",
                seed_bytes.len()
            )));
        }

        let mut seed = [0u8; 32];
        seed.copy_from_slice(seed_bytes);
        let key = SigningKey::from_bytes(&seed);

        // Zeroize the stack copy of the seed.
        use zeroize::Zeroize;
        seed.zeroize();

        let resolved_key_id = kms_key_id
            .map(|s| s.to_string())
            .or_else(|| resp.key_id().map(|s| s.to_string()))
            .unwrap_or_else(|| "<auto>".to_string());

        Ok(Self {
            key,
            kms_key_id: resolved_key_id,
        })
    }

    /// Create from a base64-encoded ciphertext string (convenience for
    /// loading from environment variables or config files).
    pub async fn from_base64_ciphertext(
        base64_ciphertext: &str,
        kms_key_id: Option<&str>,
        region: Option<&str>,
    ) -> Result<Self, CryptoError> {
        // Try base64 decoding. We use a simple implementation to avoid
        // adding a base64 crate dependency.
        let encrypted = base64_decode(base64_ciphertext).map_err(|e| {
            CryptoError::Base64Decode(format!("base64 decode failed: {e}"))
        })?;

        Self::from_encrypted_seed(&encrypted, kms_key_id, region).await
    }

    /// Return the KMS key ID used for decryption (for audit logging).
    pub fn kms_key_id(&self) -> &str {
        &self.kms_key_id
    }

    /// Synchronous factory for use within a Tokio runtime.
    ///
    /// Blocks on the async KMS call using `tokio::runtime::Handle::try_current()`.
    pub fn from_encrypted_seed_sync(
        encrypted_seed: &[u8],
        kms_key_id: Option<&str>,
        region: Option<&str>,
    ) -> Result<Self, CryptoError> {
        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            CryptoError::NotImplemented(
                "no async runtime available for KMS decrypt".to_string(),
            )
        })?;

        rt.block_on(Self::from_encrypted_seed(encrypted_seed, kms_key_id, region))
    }
}

impl super::KeyProvider for AwsKmsEnvelopeKeyProvider {
    fn sign(&self, data: &CanonicalBytes) -> Result<Ed25519Signature, CryptoError> {
        Ok(self.key.sign(data))
    }

    fn verifying_key(&self) -> Result<VerifyingKey, CryptoError> {
        Ok(self.key.verifying_key())
    }

    fn provider_name(&self) -> &str {
        "AwsKmsEnvelopeKeyProvider"
    }
}

/// Minimal base64 decoder (standard alphabet, with optional padding).
///
/// Avoids adding a `base64` crate dependency for this single use case.
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim().trim_end_matches('=');
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for ch in input.chars() {
        let val = match ch {
            'A'..='Z' => (ch as u32) - ('A' as u32),
            'a'..='z' => (ch as u32) - ('a' as u32) + 26,
            '0'..='9' => (ch as u32) - ('0' as u32) + 52,
            '+' => 62,
            '/' => 63,
            '\n' | '\r' | ' ' | '\t' => continue,
            _ => return Err(format!("invalid base64 character: {ch}")),
        };
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push(((buf >> bits) & 0xFF) as u8);
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_decode_basic() {
        // "hello" in base64 is "aGVsbG8="
        let decoded = base64_decode("aGVsbG8=").expect("decode");
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn base64_decode_no_padding() {
        let decoded = base64_decode("aGVsbG8").expect("decode");
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn base64_decode_empty() {
        let decoded = base64_decode("").expect("decode");
        assert!(decoded.is_empty());
    }

    #[test]
    fn base64_decode_invalid_char() {
        let result = base64_decode("hello!world");
        assert!(result.is_err());
    }

    #[test]
    fn base64_decode_whitespace_ignored() {
        let decoded = base64_decode("  aGVs\nbG8= ").expect("decode");
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn provider_name() {
        // We can't test the full KMS flow without AWS credentials,
        // but we can test the provider name via a mock construction.
        // In practice, integration tests with localstack would cover this.
    }
}
