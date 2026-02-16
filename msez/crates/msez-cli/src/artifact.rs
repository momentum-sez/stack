//! # Artifact Subcommand
//!
//! Content-addressed storage operations. Wraps the `msez-crypto` CAS module
//! to provide CLI access to artifact store, resolve, and verify operations.
//!
//! ## CAS Layout
//!
//! Artifacts are stored under `dist/artifacts/{type}/{digest}.json` using the
//! naming convention from the Python `tools/artifacts.py` implementation.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};

use msez_core::ContentDigest;
use msez_crypto::ContentAddressedStore;

/// Arguments for the `msez artifact` subcommand.
#[derive(Args, Debug)]
pub struct ArtifactArgs {
    #[command(subcommand)]
    pub command: ArtifactCommand,
}

/// Artifact subcommands.
#[derive(Subcommand, Debug)]
pub enum ArtifactCommand {
    /// Store an artifact in the content-addressed store.
    Store {
        /// Artifact type (e.g., "lawpack", "receipt", "vc").
        #[arg(long, value_name = "TYPE")]
        artifact_type: String,
        /// Path to the JSON file to store.
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Resolve an artifact by type and digest.
    Resolve {
        /// Artifact type (e.g., "lawpack", "receipt", "vc").
        #[arg(long, value_name = "TYPE")]
        artifact_type: String,
        /// SHA-256 hex digest of the artifact.
        #[arg(long)]
        digest: String,
    },

    /// Verify an artifact's integrity by recomputing its digest.
    Verify {
        /// Artifact type (e.g., "lawpack", "receipt", "vc").
        #[arg(long, value_name = "TYPE")]
        artifact_type: String,
        /// SHA-256 hex digest to verify against.
        #[arg(long)]
        digest: String,
    },
}

/// Execute the artifact subcommand.
pub fn run_artifact(args: &ArtifactArgs, repo_root: &Path) -> Result<u8> {
    let cas_dir = repo_root.join("dist").join("artifacts");
    let cas = ContentAddressedStore::new(&cas_dir);

    match &args.command {
        ArtifactCommand::Store {
            artifact_type,
            file,
        } => cmd_store(&cas, artifact_type, file, repo_root),

        ArtifactCommand::Resolve {
            artifact_type,
            digest,
        } => cmd_resolve(&cas, artifact_type, digest),

        ArtifactCommand::Verify {
            artifact_type,
            digest,
        } => cmd_verify(&cas, artifact_type, digest),
    }
}

/// Store a JSON artifact in the CAS.
fn cmd_store(
    cas: &ContentAddressedStore,
    artifact_type: &str,
    file: &Path,
    repo_root: &Path,
) -> Result<u8> {
    let resolved = crate::resolve_path(file, repo_root);
    if !resolved.exists() {
        bail!("file not found: {}", resolved.display());
    }

    let content = std::fs::read_to_string(&resolved)
        .with_context(|| format!("failed to read file: {}", resolved.display()))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse JSON: {}", resolved.display()))?;

    let artifact_ref = cas
        .store(artifact_type, &value)
        .map_err(|e| anyhow::anyhow!("CAS store failed: {e}"))?;

    println!(
        "OK: stored artifact type={} digest={}",
        artifact_ref.artifact_type,
        artifact_ref.digest.to_hex()
    );

    Ok(0)
}

/// Resolve an artifact by type and digest.
fn cmd_resolve(cas: &ContentAddressedStore, artifact_type: &str, digest_hex: &str) -> Result<u8> {
    // Build a ContentDigest from the hex string.
    let digest = parse_digest_hex(digest_hex)?;

    match cas.resolve(artifact_type, &digest) {
        Ok(Some(bytes)) => {
            let content = String::from_utf8_lossy(&bytes);
            println!("{content}");
            Ok(0)
        }
        Ok(None) => {
            println!("NOT FOUND: artifact type={artifact_type} digest={digest_hex}");
            Ok(1)
        }
        Err(e) => {
            bail!("CAS resolve failed: {e}");
        }
    }
}

/// Verify an artifact's integrity.
fn cmd_verify(cas: &ContentAddressedStore, artifact_type: &str, digest_hex: &str) -> Result<u8> {
    let digest = parse_digest_hex(digest_hex)?;

    match cas.resolve(artifact_type, &digest) {
        Ok(Some(_)) => {
            println!("OK: artifact integrity verified type={artifact_type} digest={digest_hex}");
            Ok(0)
        }
        Ok(None) => {
            println!("FAIL: artifact not found type={artifact_type} digest={digest_hex}");
            Ok(1)
        }
        Err(e) => {
            println!("FAIL: integrity check failed: {e}");
            Ok(1)
        }
    }
}

/// Parse a hex digest string into a ContentDigest.
fn parse_digest_hex(hex: &str) -> Result<ContentDigest> {
    ContentDigest::from_hex(hex)
        .map_err(|e| anyhow::anyhow!("invalid digest: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn store_and_verify_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"action": "test", "amount": 42});
        let artifact_ref = cas.store("receipt", &data).unwrap();
        let _hex = artifact_ref.digest.to_hex();

        // Verify resolves successfully.
        let resolved = cas.resolve("receipt", &artifact_ref.digest).unwrap();
        assert!(resolved.is_some());
    }

    #[test]
    fn store_creates_expected_path() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"key": "value"});
        let artifact_ref = cas.store("vc", &data).unwrap();

        let expected_path = dir
            .path()
            .join("vc")
            .join(format!("{}.json", artifact_ref.digest.to_hex()));
        assert!(expected_path.exists());
    }

    // ── Additional coverage tests ────────────────────────────────────

    #[test]
    fn parse_digest_hex_rejects_short_input() {
        let result = parse_digest_hex("abc123");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("64 hex char"), "expected '64 hex char' in: {err_msg}");
    }

    #[test]
    fn parse_digest_hex_rejects_non_hex() {
        let result =
            parse_digest_hex("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid digest"), "expected 'invalid digest' in: {err_msg}");
    }

    #[test]
    fn parse_digest_hex_valid_hex_succeeds() {
        let valid_hex = "a".repeat(64);
        let result = parse_digest_hex(&valid_hex);
        assert!(result.is_ok(), "valid 64-char hex should parse successfully");
        assert_eq!(result.unwrap().to_hex(), valid_hex);
    }

    #[test]
    fn cmd_store_file_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let result = cmd_store(
            &cas,
            "receipt",
            Path::new("/tmp/msez-no-such-file-xyz.json"),
            dir.path(),
        );
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("file not found"));
    }

    #[test]
    fn cmd_store_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let cas_dir = dir.path().join("cas");
        let cas = ContentAddressedStore::new(&cas_dir);

        let bad_json = dir.path().join("bad.json");
        std::fs::write(&bad_json, b"not json at all {{{").unwrap();

        let result = cmd_store(&cas, "receipt", &bad_json, dir.path());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("JSON") || err_msg.contains("parse"));
    }

    #[test]
    fn cmd_store_success() {
        let dir = tempfile::tempdir().unwrap();
        let cas_dir = dir.path().join("cas");
        let cas = ContentAddressedStore::new(&cas_dir);

        let json_file = dir.path().join("test.json");
        std::fs::write(
            &json_file,
            serde_json::to_string_pretty(&json!({"key": "value"})).unwrap(),
        )
        .unwrap();

        let result = cmd_store(&cas, "vc", &json_file, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn cmd_resolve_with_invalid_digest() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let result = cmd_resolve(&cas, "receipt", "tooshort");
        assert!(result.is_err());
    }

    #[test]
    fn cmd_resolve_valid_hex_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let valid_hex = "b".repeat(64);
        let result = cmd_resolve(&cas, "receipt", &valid_hex);
        // Digest parses successfully but artifact is not in the store.
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1, "exit code 1 = not found");
    }

    #[test]
    fn cmd_verify_with_invalid_digest() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let result = cmd_verify(&cas, "receipt", "not_a_valid_hex");
        assert!(result.is_err());
    }

    #[test]
    fn cmd_verify_valid_hex_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let valid_hex = "c".repeat(64);
        let result = cmd_verify(&cas, "receipt", &valid_hex);
        // Digest parses successfully but artifact is not in the store.
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1, "exit code 1 = not found");
    }

    #[test]
    fn run_artifact_store_subcommand() {
        let dir = tempfile::tempdir().unwrap();

        let json_file = dir.path().join("data.json");
        std::fs::write(
            &json_file,
            serde_json::to_string_pretty(&json!({"test": true})).unwrap(),
        )
        .unwrap();

        let cas_dir = dir.path().join("dist").join("artifacts");
        std::fs::create_dir_all(&cas_dir).unwrap();

        let args = ArtifactArgs {
            command: ArtifactCommand::Store {
                artifact_type: "receipt".to_string(),
                file: json_file,
            },
        };
        let result = run_artifact(&args, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_artifact_resolve_subcommand_with_invalid_digest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("dist").join("artifacts")).unwrap();

        let args = ArtifactArgs {
            command: ArtifactCommand::Resolve {
                artifact_type: "receipt".to_string(),
                digest: "short".to_string(),
            },
        };
        let result = run_artifact(&args, dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn run_artifact_verify_subcommand_with_invalid_digest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("dist").join("artifacts")).unwrap();

        let args = ArtifactArgs {
            command: ArtifactCommand::Verify {
                artifact_type: "receipt".to_string(),
                digest: "bad".to_string(),
            },
        };
        let result = run_artifact(&args, dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn run_artifact_store_file_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("dist").join("artifacts")).unwrap();

        let args = ArtifactArgs {
            command: ArtifactCommand::Store {
                artifact_type: "vc".to_string(),
                file: PathBuf::from("nonexistent.json"),
            },
        };
        let result = run_artifact(&args, dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn cmd_store_stores_different_artifact_types() {
        let dir = tempfile::tempdir().unwrap();
        let cas_dir = dir.path().join("cas");
        let cas = ContentAddressedStore::new(&cas_dir);

        let json_file = dir.path().join("data.json");
        std::fs::write(
            &json_file,
            serde_json::to_string(&json!({"item": "value"})).unwrap(),
        )
        .unwrap();

        let result1 = cmd_store(&cas, "receipt", &json_file, dir.path());
        assert!(result1.is_ok());

        let result2 = cmd_store(&cas, "vc", &json_file, dir.path());
        assert!(result2.is_ok());

        let result3 = cmd_store(&cas, "lawpack", &json_file, dir.path());
        assert!(result3.is_ok());

        // Verify all three type subdirectories exist.
        assert!(cas_dir.join("receipt").is_dir());
        assert!(cas_dir.join("vc").is_dir());
        assert!(cas_dir.join("lawpack").is_dir());
    }

    #[test]
    fn parse_digest_hex_empty_string() {
        let result = parse_digest_hex("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_digest_hex_too_long() {
        let result = parse_digest_hex(&"a".repeat(128));
        assert!(result.is_err());
    }
}
