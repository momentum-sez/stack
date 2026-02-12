//! # Signing Subcommand
//!
//! Ed25519 key generation, VC signing, and signature verification.
//!
//! Wraps the `msez-crypto` Ed25519 module to provide CLI access to
//! cryptographic operations.
//!
//! ## Security Invariant
//!
//! All signing operations take canonicalized data via `CanonicalBytes`.
//! The type system prevents signing raw bytes, ensuring signature
//! malleability from non-canonical serialization is impossible.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use rand_core::OsRng;

use msez_core::CanonicalBytes;
use msez_crypto::{Ed25519Signature, SigningKey, VerifyingKey};

/// Arguments for the `msez vc` (signing) subcommand.
#[derive(Args, Debug)]
pub struct SigningArgs {
    #[command(subcommand)]
    pub command: SigningCommand,
}

/// Signing subcommands.
#[derive(Subcommand, Debug)]
pub enum SigningCommand {
    /// Generate a new Ed25519 keypair.
    Keygen {
        /// Output directory for the keypair files.
        #[arg(long, short, default_value = ".")]
        output: PathBuf,
        /// Prefix for the key filenames (default: "msez").
        #[arg(long, default_value = "msez")]
        prefix: String,
    },

    /// Sign a JSON document with Ed25519.
    Sign {
        /// Path to the private key file (hex-encoded 32-byte key).
        #[arg(long)]
        key: PathBuf,
        /// Path to the JSON document to sign.
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Verify an Ed25519 signature over a JSON document.
    Verify {
        /// Path to the public key file (hex-encoded 32-byte key).
        #[arg(long)]
        pubkey: PathBuf,
        /// Path to the JSON document.
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// The signature to verify (hex-encoded 64-byte signature).
        #[arg(long)]
        signature: String,
    },
}

/// Execute the signing subcommand.
pub fn run_signing(args: &SigningArgs, repo_root: &Path) -> Result<u8> {
    match &args.command {
        SigningCommand::Keygen { output, prefix } => {
            let resolved_output = crate::resolve_path(output, repo_root);
            cmd_keygen(&resolved_output, prefix)
        }
        SigningCommand::Sign { key, file } => {
            let resolved_key = crate::resolve_path(key, repo_root);
            let resolved_file = crate::resolve_path(file, repo_root);
            cmd_sign(&resolved_key, &resolved_file)
        }
        SigningCommand::Verify {
            pubkey,
            file,
            signature,
        } => {
            let resolved_pubkey = crate::resolve_path(pubkey, repo_root);
            let resolved_file = crate::resolve_path(file, repo_root);
            cmd_verify(&resolved_pubkey, &resolved_file, signature)
        }
    }
}

/// Generate a new Ed25519 keypair and write to files.
fn cmd_keygen(output_dir: &Path, prefix: &str) -> Result<u8> {
    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let sk_hex = sk
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    let vk_hex = vk.to_hex();

    let sk_path = output_dir.join(format!("{prefix}.key"));
    let vk_path = output_dir.join(format!("{prefix}.pub"));

    std::fs::write(&sk_path, &sk_hex)
        .with_context(|| format!("failed to write private key: {}", sk_path.display()))?;
    std::fs::write(&vk_path, &vk_hex)
        .with_context(|| format!("failed to write public key: {}", vk_path.display()))?;

    println!("OK: generated Ed25519 keypair");
    println!("  Private key: {}", sk_path.display());
    println!("  Public key:  {}", vk_path.display());
    println!("  Public key (hex): {vk_hex}");

    Ok(0)
}

/// Sign a JSON document with an Ed25519 private key.
fn cmd_sign(key_path: &Path, file_path: &Path) -> Result<u8> {
    if !key_path.exists() {
        bail!("private key file not found: {}", key_path.display());
    }
    if !file_path.exists() {
        bail!("document file not found: {}", file_path.display());
    }

    // Read private key (hex-encoded).
    let sk_hex = std::fs::read_to_string(key_path)
        .with_context(|| format!("failed to read private key: {}", key_path.display()))?;
    let sk_hex = sk_hex.trim();

    let sk_bytes = hex_to_bytes(sk_hex).context("invalid private key hex")?;
    if sk_bytes.len() != 32 {
        bail!(
            "private key must be 32 bytes (64 hex chars), got {} bytes",
            sk_bytes.len()
        );
    }
    let mut sk_arr = [0u8; 32];
    sk_arr.copy_from_slice(&sk_bytes);
    let sk = SigningKey::from_bytes(&sk_arr);

    // Read and canonicalize the document.
    let content = std::fs::read_to_string(file_path)
        .with_context(|| format!("failed to read document: {}", file_path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse JSON: {}", file_path.display()))?;
    let canonical = CanonicalBytes::new(&value).context("failed to canonicalize document")?;

    // Sign.
    let signature = sk.sign(&canonical);
    let sig_hex = signature.to_hex();

    println!("{sig_hex}");

    Ok(0)
}

/// Verify an Ed25519 signature over a JSON document.
fn cmd_verify(pubkey_path: &Path, file_path: &Path, sig_hex: &str) -> Result<u8> {
    if !pubkey_path.exists() {
        bail!("public key file not found: {}", pubkey_path.display());
    }
    if !file_path.exists() {
        bail!("document file not found: {}", file_path.display());
    }

    // Read public key (hex-encoded).
    let vk_hex = std::fs::read_to_string(pubkey_path)
        .with_context(|| format!("failed to read public key: {}", pubkey_path.display()))?;
    let vk_hex = vk_hex.trim();

    let vk =
        VerifyingKey::from_hex(vk_hex).map_err(|e| anyhow::anyhow!("invalid public key: {e}"))?;

    // Read and canonicalize the document.
    let content = std::fs::read_to_string(file_path)
        .with_context(|| format!("failed to read document: {}", file_path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse JSON: {}", file_path.display()))?;
    let canonical = CanonicalBytes::new(&value).context("failed to canonicalize document")?;

    // Parse signature.
    let signature = Ed25519Signature::from_hex(sig_hex.trim())
        .map_err(|e| anyhow::anyhow!("invalid signature: {e}"))?;

    // Verify.
    match vk.verify(&canonical, &signature) {
        Ok(()) => {
            println!("OK: signature is valid");
            Ok(0)
        }
        Err(e) => {
            println!("FAIL: signature verification failed: {e}");
            Ok(1)
        }
    }
}

/// Decode a hex string into bytes.
fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
    if hex.len() % 2 != 0 {
        bail!("hex string has odd length: {}", hex.len());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .with_context(|| format!("invalid hex at position {i}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keygen_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_keygen(dir.path(), "test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        assert!(dir.path().join("test.key").exists());
        assert!(dir.path().join("test.pub").exists());

        let key_content = std::fs::read_to_string(dir.path().join("test.key")).unwrap();
        assert_eq!(key_content.len(), 64); // 32 bytes as hex

        let pub_content = std::fs::read_to_string(dir.path().join("test.pub")).unwrap();
        assert_eq!(pub_content.len(), 64); // 32 bytes as hex
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let dir = tempfile::tempdir().unwrap();

        // Generate keypair.
        cmd_keygen(dir.path(), "test").unwrap();

        // Create test document.
        let doc_path = dir.path().join("test.json");
        let doc = json!({"action": "transfer", "amount": 1000});
        std::fs::write(&doc_path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();

        // Sign the document.
        let key_path = dir.path().join("test.key");
        let sk_hex = std::fs::read_to_string(&key_path).unwrap();
        let sk_bytes = hex_to_bytes(sk_hex.trim()).unwrap();
        let mut sk_arr = [0u8; 32];
        sk_arr.copy_from_slice(&sk_bytes);
        let sk = SigningKey::from_bytes(&sk_arr);

        let content = std::fs::read_to_string(&doc_path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&content).unwrap();
        let canonical = CanonicalBytes::new(&value).unwrap();
        let sig = sk.sign(&canonical);
        let sig_hex = sig.to_hex();

        // Verify.
        let pub_path = dir.path().join("test.pub");
        let result = cmd_verify(&pub_path, &doc_path, &sig_hex);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn hex_to_bytes_valid() {
        let bytes = hex_to_bytes("deadbeef").unwrap();
        assert_eq!(bytes, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn hex_to_bytes_odd_length_rejected() {
        assert!(hex_to_bytes("abc").is_err());
    }

    // ── Additional coverage tests ────────────────────────────────────

    #[test]
    fn hex_to_bytes_empty_string() {
        let bytes = hex_to_bytes("").unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn hex_to_bytes_invalid_chars() {
        let result = hex_to_bytes("zzzz");
        assert!(result.is_err());
    }

    #[test]
    fn hex_to_bytes_uppercase() {
        let bytes = hex_to_bytes("DEADBEEF").unwrap();
        assert_eq!(bytes, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn hex_to_bytes_all_zeros() {
        let bytes = hex_to_bytes("00000000").unwrap();
        assert_eq!(bytes, vec![0, 0, 0, 0]);
    }

    #[test]
    fn hex_to_bytes_all_ff() {
        let bytes = hex_to_bytes("ffffffff").unwrap();
        assert_eq!(bytes, vec![0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn keygen_creates_directory_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path().join("deep").join("nested").join("keys");
        let result = cmd_keygen(&output_dir, "mykey");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert!(output_dir.join("mykey.key").exists());
        assert!(output_dir.join("mykey.pub").exists());
    }

    #[test]
    fn keygen_custom_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_keygen(dir.path(), "custom_prefix");
        assert!(result.is_ok());
        assert!(dir.path().join("custom_prefix.key").exists());
        assert!(dir.path().join("custom_prefix.pub").exists());
    }

    #[test]
    fn keygen_keys_are_valid_hex() {
        let dir = tempfile::tempdir().unwrap();
        cmd_keygen(dir.path(), "test").unwrap();

        let key_hex = std::fs::read_to_string(dir.path().join("test.key")).unwrap();
        assert!(key_hex.chars().all(|c| c.is_ascii_hexdigit()));

        let pub_hex = std::fs::read_to_string(dir.path().join("test.pub")).unwrap();
        assert!(pub_hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn cmd_sign_key_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let doc_path = dir.path().join("doc.json");
        std::fs::write(&doc_path, r#"{"key":"val"}"#).unwrap();

        let result = cmd_sign(
            &dir.path().join("nonexistent.key"),
            &doc_path,
        );
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("private key file not found"));
    }

    #[test]
    fn cmd_sign_doc_not_found() {
        let dir = tempfile::tempdir().unwrap();
        cmd_keygen(dir.path(), "test").unwrap();

        let result = cmd_sign(
            &dir.path().join("test.key"),
            &dir.path().join("nonexistent.json"),
        );
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("document file not found"));
    }

    #[test]
    fn cmd_sign_invalid_key_hex() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("bad.key");
        std::fs::write(&key_path, "not_valid_hex_at_all").unwrap();
        let doc_path = dir.path().join("doc.json");
        std::fs::write(&doc_path, r#"{"key":"val"}"#).unwrap();

        let result = cmd_sign(&key_path, &doc_path);
        assert!(result.is_err());
    }

    #[test]
    fn cmd_sign_key_wrong_length() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("short.key");
        // 16 bytes hex = 32 chars, but key should be 32 bytes = 64 chars.
        std::fs::write(&key_path, "abcdef0123456789abcdef0123456789").unwrap();
        let doc_path = dir.path().join("doc.json");
        std::fs::write(&doc_path, r#"{"key":"val"}"#).unwrap();

        let result = cmd_sign(&key_path, &doc_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("32 bytes"));
    }

    #[test]
    fn cmd_sign_invalid_json_doc() {
        let dir = tempfile::tempdir().unwrap();
        cmd_keygen(dir.path(), "test").unwrap();

        let doc_path = dir.path().join("bad.json");
        std::fs::write(&doc_path, "not json {{{").unwrap();

        let result = cmd_sign(&dir.path().join("test.key"), &doc_path);
        assert!(result.is_err());
    }

    #[test]
    fn cmd_sign_returns_hex_signature() {
        let dir = tempfile::tempdir().unwrap();
        cmd_keygen(dir.path(), "test").unwrap();

        let doc_path = dir.path().join("doc.json");
        std::fs::write(&doc_path, r#"{"key":"value"}"#).unwrap();

        let result = cmd_sign(&dir.path().join("test.key"), &doc_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn cmd_verify_pubkey_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let doc_path = dir.path().join("doc.json");
        std::fs::write(&doc_path, r#"{"key":"val"}"#).unwrap();

        let result = cmd_verify(
            &dir.path().join("nonexistent.pub"),
            &doc_path,
            "aabbccdd",
        );
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("public key file not found"));
    }

    #[test]
    fn cmd_verify_doc_not_found() {
        let dir = tempfile::tempdir().unwrap();
        cmd_keygen(dir.path(), "test").unwrap();

        let result = cmd_verify(
            &dir.path().join("test.pub"),
            &dir.path().join("nonexistent.json"),
            "aabbccdd",
        );
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("document file not found"));
    }

    #[test]
    fn cmd_verify_invalid_pubkey_hex() {
        let dir = tempfile::tempdir().unwrap();
        let pub_path = dir.path().join("bad.pub");
        std::fs::write(&pub_path, "not_hex").unwrap();
        let doc_path = dir.path().join("doc.json");
        std::fs::write(&doc_path, r#"{"key":"val"}"#).unwrap();

        let result = cmd_verify(&pub_path, &doc_path, "aa".repeat(64).as_str());
        assert!(result.is_err());
    }

    #[test]
    fn cmd_verify_invalid_signature_hex() {
        let dir = tempfile::tempdir().unwrap();
        cmd_keygen(dir.path(), "test").unwrap();
        let doc_path = dir.path().join("doc.json");
        std::fs::write(&doc_path, r#"{"key":"val"}"#).unwrap();

        let result = cmd_verify(
            &dir.path().join("test.pub"),
            &doc_path,
            "not_valid_hex",
        );
        assert!(result.is_err());
    }

    #[test]
    fn cmd_verify_wrong_signature_returns_1() {
        let dir = tempfile::tempdir().unwrap();
        cmd_keygen(dir.path(), "test").unwrap();

        let doc_path = dir.path().join("doc.json");
        std::fs::write(&doc_path, r#"{"key":"val"}"#).unwrap();

        // A valid 128-char hex string that is NOT the correct signature.
        let wrong_sig = "ab".repeat(64);

        let result = cmd_verify(
            &dir.path().join("test.pub"),
            &doc_path,
            &wrong_sig,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1, "Wrong signature should return exit code 1");
    }

    #[test]
    fn cmd_verify_invalid_json_doc() {
        let dir = tempfile::tempdir().unwrap();
        cmd_keygen(dir.path(), "test").unwrap();

        let doc_path = dir.path().join("bad.json");
        std::fs::write(&doc_path, "not json").unwrap();

        let wrong_sig = "ab".repeat(64);
        let result = cmd_verify(
            &dir.path().join("test.pub"),
            &doc_path,
            &wrong_sig,
        );
        assert!(result.is_err());
    }

    #[test]
    fn run_signing_keygen_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        let args = SigningArgs {
            command: SigningCommand::Keygen {
                output: dir.path().to_path_buf(),
                prefix: "mykey".to_string(),
            },
        };
        let result = run_signing(&args, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert!(dir.path().join("mykey.key").exists());
        assert!(dir.path().join("mykey.pub").exists());
    }

    #[test]
    fn run_signing_sign_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        cmd_keygen(dir.path(), "test").unwrap();

        let doc_path = dir.path().join("doc.json");
        std::fs::write(&doc_path, r#"{"key":"value"}"#).unwrap();

        let args = SigningArgs {
            command: SigningCommand::Sign {
                key: dir.path().join("test.key"),
                file: doc_path,
            },
        };
        let result = run_signing(&args, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_signing_verify_subcommand_wrong_sig() {
        let dir = tempfile::tempdir().unwrap();
        cmd_keygen(dir.path(), "test").unwrap();

        let doc_path = dir.path().join("doc.json");
        std::fs::write(&doc_path, r#"{"key":"value"}"#).unwrap();

        let wrong_sig = "cd".repeat(64);
        let args = SigningArgs {
            command: SigningCommand::Verify {
                pubkey: dir.path().join("test.pub"),
                file: doc_path,
                signature: wrong_sig,
            },
        };
        let result = run_signing(&args, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // Verification failure.
    }

    #[test]
    fn full_sign_verify_via_run_signing() {
        let dir = tempfile::tempdir().unwrap();

        // Generate keys.
        let keygen_args = SigningArgs {
            command: SigningCommand::Keygen {
                output: dir.path().to_path_buf(),
                prefix: "signer".to_string(),
            },
        };
        run_signing(&keygen_args, dir.path()).unwrap();

        // Create document.
        let doc_path = dir.path().join("contract.json");
        let doc = json!({"parties": ["alice", "bob"], "amount": 5000});
        std::fs::write(&doc_path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();

        // Sign it using the internal function to get the sig hex.
        let sk_hex = std::fs::read_to_string(dir.path().join("signer.key")).unwrap();
        let sk_bytes = hex_to_bytes(sk_hex.trim()).unwrap();
        let mut sk_arr = [0u8; 32];
        sk_arr.copy_from_slice(&sk_bytes);
        let sk = SigningKey::from_bytes(&sk_arr);
        let content = std::fs::read_to_string(&doc_path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&content).unwrap();
        let canonical = CanonicalBytes::new(&value).unwrap();
        let sig = sk.sign(&canonical);

        // Verify via run_signing.
        let verify_args = SigningArgs {
            command: SigningCommand::Verify {
                pubkey: dir.path().join("signer.pub"),
                file: doc_path,
                signature: sig.to_hex(),
            },
        };
        let result = run_signing(&verify_args, dir.path()).unwrap();
        assert_eq!(result, 0, "Valid signature should verify successfully");
    }

    #[test]
    fn keygen_generates_different_keys_each_time() {
        let dir = tempfile::tempdir().unwrap();
        cmd_keygen(dir.path(), "key1").unwrap();
        cmd_keygen(dir.path(), "key2").unwrap();

        let key1 = std::fs::read_to_string(dir.path().join("key1.key")).unwrap();
        let key2 = std::fs::read_to_string(dir.path().join("key2.key")).unwrap();
        assert_ne!(key1, key2, "Two keygen calls should produce different keys");
    }

    #[test]
    fn cmd_sign_key_with_whitespace() {
        let dir = tempfile::tempdir().unwrap();
        cmd_keygen(dir.path(), "test").unwrap();

        // Add whitespace around the key content.
        let key_path = dir.path().join("test.key");
        let key_content = std::fs::read_to_string(&key_path).unwrap();
        std::fs::write(&key_path, format!("  {key_content}  \n")).unwrap();

        let doc_path = dir.path().join("doc.json");
        std::fs::write(&doc_path, r#"{"key":"val"}"#).unwrap();

        let result = cmd_sign(&key_path, &doc_path);
        assert!(result.is_ok(), "Should handle whitespace-padded key file");
    }
}
