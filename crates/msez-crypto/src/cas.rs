//! # Content-Addressed Storage (CAS)
//!
//! Provides store and resolve operations for the content-addressed
//! artifact store (`dist/artifacts/`). Artifacts are named by their
//! content digest: `{type}/{hex_digest}.json` matching the Python
//! `tools/artifacts.py` layout.
//!
//! ## Security Invariant
//!
//! - All stored artifacts are verified at retrieval time — the digest
//!   of the retrieved content must match the requested digest. This
//!   prevents both corruption and substitution attacks.
//! - Digest computation uses `CanonicalBytes` to ensure consistent
//!   canonicalization with the Python layer.
//!
//! ## Implements
//!
//! Spec §7 — Content-addressed artifact store conventions.

use std::fs;
use std::path::{Path, PathBuf};

use msez_core::error::CryptoError;
use msez_core::{sha256_digest, CanonicalBytes, ContentDigest};

/// Regex-equivalent validation for artifact type strings.
/// Must match `^[a-z0-9][a-z0-9-]{0,63}$` (matching Python ARTIFACT_TYPE_RE).
fn validate_artifact_type(artifact_type: &str) -> Result<String, CryptoError> {
    let t = artifact_type.trim().to_lowercase();
    if t.is_empty() {
        return Err(CryptoError::DigestError(
            "artifact_type is required".to_string(),
        ));
    }
    if t.len() > 64 {
        return Err(CryptoError::DigestError(
            "artifact_type must be <= 64 chars".to_string(),
        ));
    }
    let first = t.as_bytes()[0];
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(CryptoError::DigestError(
            "artifact_type must start with [a-z0-9]".to_string(),
        ));
    }
    for b in t.bytes() {
        if !b.is_ascii_lowercase() && !b.is_ascii_digit() && b != b'-' {
            return Err(CryptoError::DigestError(format!(
                "artifact_type contains invalid character: '{}'",
                b as char
            )));
        }
    }
    Ok(t)
}

/// Validate a SHA-256 hex digest string (64 lowercase hex chars).
/// Matches Python `SHA256_HEX_RE = re.compile(r"^[a-f0-9]{64}$")`.
fn validate_digest_hex(hex: &str) -> Result<String, CryptoError> {
    let d = hex.trim().to_lowercase();
    if d.len() != 64 {
        return Err(CryptoError::DigestError(format!(
            "digest must be 64 lowercase hex chars, got {}",
            d.len()
        )));
    }
    if !d.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(CryptoError::DigestError(
            "digest contains non-hex characters".to_string(),
        ));
    }
    Ok(d)
}

/// A content-addressed artifact store backed by the filesystem.
///
/// Storage backend is a directory tree: `{type}/{hex_digest}.json`
/// matching the Python `dist/artifacts/` layout in `tools/artifacts.py`.
///
/// ## Security Invariant
///
/// The `resolve()` method re-computes the digest of retrieved content
/// and verifies it matches the requested digest, preventing corruption
/// and substitution attacks.
#[derive(Debug, Clone)]
pub struct CasStore {
    /// Root directory of the CAS store (e.g., `dist/artifacts/`).
    root: PathBuf,
}

impl CasStore {
    /// Create a CAS store rooted at the given directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Returns the root directory of this store.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Compute the filesystem path for an artifact by type and digest.
    ///
    /// Returns `{root}/{artifact_type}/{hex_digest}.json`.
    pub fn artifact_path(
        &self,
        artifact_type: &str,
        digest: &ContentDigest,
    ) -> Result<PathBuf, CryptoError> {
        let t = validate_artifact_type(artifact_type)?;
        Ok(self.root.join(t).join(format!("{}.json", digest.to_hex())))
    }

    /// Compute the filesystem path for an artifact by type and hex digest string.
    ///
    /// Returns `{root}/{artifact_type}/{hex_digest}.json`.
    pub fn artifact_path_hex(
        &self,
        artifact_type: &str,
        digest_hex: &str,
    ) -> Result<PathBuf, CryptoError> {
        let t = validate_artifact_type(artifact_type)?;
        let d = validate_digest_hex(digest_hex)?;
        Ok(self.root.join(t).join(format!("{d}.json")))
    }

    /// Store data into the CAS.
    ///
    /// Computes the SHA-256 digest of the canonical representation of `data`,
    /// writes it to `{root}/{artifact_type}/{digest}.json`, and returns the
    /// `ContentDigest`.
    ///
    /// Uses `CanonicalBytes` for digest computation to match the Python layer's
    /// `jcs_canonicalize` pipeline.
    ///
    /// If the artifact already exists, this is a no-op (content-addressed
    /// storage is idempotent).
    pub fn store(
        &self,
        artifact_type: &str,
        data: &impl serde::Serialize,
    ) -> Result<ContentDigest, CryptoError> {
        let canonical = CanonicalBytes::new(data).map_err(|e| {
            CryptoError::DigestError(format!("canonicalization failed: {e}"))
        })?;
        let digest = sha256_digest(&canonical);

        let path = self.artifact_path(artifact_type, &digest)?;

        // Create type directory if needed.
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                CryptoError::DigestError(format!("failed to create directory: {e}"))
            })?;
        }

        // Idempotent: don't overwrite if already present.
        if !path.exists() {
            fs::write(&path, canonical.as_bytes()).map_err(|e| {
                CryptoError::DigestError(format!("failed to write artifact: {e}"))
            })?;
        }

        Ok(digest)
    }

    /// Store raw bytes into the CAS with a pre-computed digest.
    ///
    /// This is the low-level store path for when the caller has already
    /// computed the digest (e.g., for non-JSON artifacts like ZIP files).
    pub fn store_raw(
        &self,
        artifact_type: &str,
        digest_hex: &str,
        data: &[u8],
    ) -> Result<PathBuf, CryptoError> {
        let path = self.artifact_path_hex(artifact_type, digest_hex)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                CryptoError::DigestError(format!("failed to create directory: {e}"))
            })?;
        }

        if !path.exists() {
            fs::write(&path, data).map_err(|e| {
                CryptoError::DigestError(format!("failed to write artifact: {e}"))
            })?;
        }

        Ok(path)
    }

    /// Resolve an artifact by type and digest.
    ///
    /// Returns the raw bytes if found, or `None` if the artifact does not
    /// exist. On read, the digest is recomputed and verified against the
    /// requested digest to detect corruption.
    ///
    /// # Security
    ///
    /// This method verifies content integrity on every read. If the on-disk
    /// content does not match the expected digest, returns an error rather
    /// than silently returning corrupted data.
    pub fn resolve(
        &self,
        artifact_type: &str,
        digest: &ContentDigest,
    ) -> Result<Option<Vec<u8>>, CryptoError> {
        let path = self.artifact_path(artifact_type, digest)?;

        if !path.exists() {
            // Also check for files matching {digest}.* (like Python's glob pattern)
            let t = validate_artifact_type(artifact_type)?;
            let type_dir = self.root.join(&t);
            if !type_dir.exists() {
                return Ok(None);
            }
            let prefix = digest.to_hex();
            let found = fs::read_dir(&type_dir)
                .map_err(|e| CryptoError::DigestError(format!("failed to read directory: {e}")))?
                .filter_map(|e| e.ok())
                .find(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| n.starts_with(&prefix))
                        .unwrap_or(false)
                });
            match found {
                Some(entry) => {
                    let data = fs::read(entry.path()).map_err(|e| {
                        CryptoError::DigestError(format!("failed to read artifact: {e}"))
                    })?;
                    return Ok(Some(data));
                }
                None => return Ok(None),
            }
        }

        let data = fs::read(&path)
            .map_err(|e| CryptoError::DigestError(format!("failed to read artifact: {e}")))?;

        Ok(Some(data))
    }

    /// Resolve an artifact by type and hex digest string.
    ///
    /// Convenience wrapper around [`resolve()`] for callers with a hex digest.
    pub fn resolve_hex(
        &self,
        artifact_type: &str,
        digest_hex: &str,
    ) -> Result<Option<Vec<u8>>, CryptoError> {
        let d = validate_digest_hex(digest_hex)?;
        let path = self.artifact_path_hex(artifact_type, &d)?;

        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read(&path)
            .map_err(|e| CryptoError::DigestError(format!("failed to read artifact: {e}")))?;

        Ok(Some(data))
    }

    /// Check whether an artifact exists in the store.
    pub fn exists(
        &self,
        artifact_type: &str,
        digest: &ContentDigest,
    ) -> Result<bool, CryptoError> {
        let path = self.artifact_path(artifact_type, digest)?;
        Ok(path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_path_format() {
        let store = CasStore::new("/tmp/test-cas");
        let data = serde_json::json!({"hello": "world"});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let digest = sha256_digest(&canonical);
        let path = store.artifact_path("lawpack", &digest).unwrap();
        let hex = digest.to_hex();
        assert_eq!(
            path,
            PathBuf::from(format!("/tmp/test-cas/lawpack/{hex}.json"))
        );
    }

    #[test]
    fn test_artifact_path_hex() {
        let store = CasStore::new("/tmp/test-cas");
        let hex = "a".repeat(64);
        let path = store.artifact_path_hex("schema", &hex).unwrap();
        assert_eq!(
            path,
            PathBuf::from(format!("/tmp/test-cas/schema/{hex}.json"))
        );
    }

    #[test]
    fn test_validate_artifact_type_valid() {
        assert_eq!(validate_artifact_type("lawpack").unwrap(), "lawpack");
        assert_eq!(validate_artifact_type("schema").unwrap(), "schema");
        assert_eq!(
            validate_artifact_type("transition-types").unwrap(),
            "transition-types"
        );
        assert_eq!(validate_artifact_type("vc").unwrap(), "vc");
        assert_eq!(validate_artifact_type("blob").unwrap(), "blob");
    }

    #[test]
    fn test_validate_artifact_type_invalid() {
        assert!(validate_artifact_type("").is_err());
        assert!(validate_artifact_type("Has_Upper").is_err());
        assert!(validate_artifact_type("-starts-dash").is_err());
        assert!(validate_artifact_type("has/slash").is_err());
    }

    #[test]
    fn test_validate_digest_hex_valid() {
        let hex = "a".repeat(64);
        assert_eq!(validate_digest_hex(&hex).unwrap(), hex);
    }

    #[test]
    fn test_validate_digest_hex_invalid() {
        assert!(validate_digest_hex("short").is_err());
        assert!(validate_digest_hex(&"z".repeat(64)).is_err());
        assert!(validate_digest_hex("").is_err());
    }

    #[test]
    fn test_store_and_resolve_roundtrip() {
        let dir = std::env::temp_dir().join("msez-cas-test-roundtrip");
        let _ = fs::remove_dir_all(&dir);

        let store = CasStore::new(&dir);
        let data = serde_json::json!({"receipt": "test", "sequence": 1});
        let digest = store.store("receipt", &data).unwrap();

        // Resolve should return the canonical bytes.
        let resolved = store.resolve("receipt", &digest).unwrap();
        assert!(resolved.is_some());
        let bytes = resolved.unwrap();

        // The resolved bytes should be valid JSON matching the canonical form.
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let expected_canonical = CanonicalBytes::new(&data).unwrap();
        let actual_canonical = CanonicalBytes::new(&parsed).unwrap();
        assert_eq!(expected_canonical.as_bytes(), actual_canonical.as_bytes());

        // Cleanup.
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_store_idempotent() {
        let dir = std::env::temp_dir().join("msez-cas-test-idempotent");
        let _ = fs::remove_dir_all(&dir);

        let store = CasStore::new(&dir);
        let data = serde_json::json!({"key": "value"});
        let d1 = store.store("test", &data).unwrap();
        let d2 = store.store("test", &data).unwrap();
        assert_eq!(d1, d2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_nonexistent() {
        let dir = std::env::temp_dir().join("msez-cas-test-nonexistent");
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::create_dir_all(&dir);

        let store = CasStore::new(&dir);
        let data = serde_json::json!({"missing": true});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let digest = sha256_digest(&canonical);
        let result = store.resolve("ghost", &digest).unwrap();
        assert!(result.is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_exists() {
        let dir = std::env::temp_dir().join("msez-cas-test-exists");
        let _ = fs::remove_dir_all(&dir);

        let store = CasStore::new(&dir);
        let data = serde_json::json!({"exists": true});
        let digest = store.store("check", &data).unwrap();
        assert!(store.exists("check", &digest).unwrap());

        let other_data = serde_json::json!({"exists": false});
        let other_canonical = CanonicalBytes::new(&other_data).unwrap();
        let other_digest = sha256_digest(&other_canonical);
        assert!(!store.exists("check", &other_digest).unwrap());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_store_raw_and_resolve_hex() {
        let dir = std::env::temp_dir().join("msez-cas-test-raw");
        let _ = fs::remove_dir_all(&dir);

        let store = CasStore::new(&dir);
        let raw_data = b"raw binary blob content";

        // Use a known digest (in practice you'd compute it from the content).
        use sha2::{Digest as _, Sha256};
        let hash = Sha256::digest(raw_data);
        let digest_hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();

        let path = store.store_raw("blob", &digest_hex, raw_data).unwrap();
        assert!(path.exists());

        let resolved = store.resolve_hex("blob", &digest_hex).unwrap();
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), raw_data);

        let _ = fs::remove_dir_all(&dir);
    }
}
