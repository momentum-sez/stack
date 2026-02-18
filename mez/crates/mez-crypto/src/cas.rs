//! # Content-Addressed Storage (CAS)
//!
//! Utilities for storing and resolving artifacts by their content digest.
//! Artifacts are stored using the naming convention `{type}/{digest}.json`
//! matching the Python `dist/artifacts/` layout.
//!
//! ## Integrity Invariant
//!
//! Every stored artifact's filename encodes its content digest. On retrieval,
//! the digest is recomputed and verified against the filename. Corruption
//! or tampering is detected at read time.
//!
//! ## Artifact Type Validation
//!
//! Artifact types must match `^[a-z0-9][a-z0-9-]{0,63}$` — lowercase
//! alphanumerics and hyphens, starting with an alphanumeric character.
//! This matches the Python convention in `tools/artifacts.py`.
//!
//! ## Spec Reference
//!
//! Implements the CAS store/resolve pattern from `tools/artifacts.py`.
//! Digest computation uses `CanonicalBytes` → `sha256_digest()` to ensure
//! cross-language digest agreement with the Python layer.

use mez_core::{sha256_digest, CanonicalBytes, ContentDigest};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use subtle::ConstantTimeEq;

use crate::error::CryptoError;

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Validate and normalize an artifact type string.
///
/// Must match `^[a-z0-9][a-z0-9-]{0,63}$` per the Python `artifacts.py` convention.
fn validate_artifact_type(artifact_type: &str) -> Result<String, CryptoError> {
    let t = artifact_type.trim().to_lowercase();
    if t.is_empty() {
        return Err(CryptoError::Cas("artifact_type is required".into()));
    }
    if t.len() > 64 {
        return Err(CryptoError::Cas(format!(
            "artifact_type too long: {} chars (max 64)",
            t.len()
        )));
    }
    let mut chars = t.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => {
            return Err(CryptoError::Cas(format!(
                "artifact_type must start with [a-z0-9], got: {t:?}"
            )));
        }
    }
    for c in chars {
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(CryptoError::Cas(format!(
                "artifact_type contains invalid character {c:?}: must match [a-z0-9-]"
            )));
        }
    }
    Ok(t)
}

/// Validate a hex digest string (64 lowercase hex chars).
fn validate_digest_hex(digest_hex: &str) -> Result<String, CryptoError> {
    let d = digest_hex.trim().to_lowercase();
    if d.len() != 64 {
        return Err(CryptoError::Cas(format!(
            "digest must be 64 hex chars, got {} chars",
            d.len()
        )));
    }
    if !d.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CryptoError::Cas(
            "digest contains non-hex characters".into(),
        ));
    }
    Ok(d)
}

// ---------------------------------------------------------------------------
// ArtifactRef
// ---------------------------------------------------------------------------

/// A validated artifact type string.
///
/// Wraps a `String` that has been validated against the CAS naming
/// convention: `^[a-z0-9][a-z0-9-]{0,63}$`. The inner value cannot be
/// mutated after construction, guaranteeing the invariant holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactType(String);

impl ArtifactType {
    /// Create a new validated artifact type.
    ///
    /// Returns an error if the string doesn't match `^[a-z0-9][a-z0-9-]{0,63}$`.
    pub fn new(s: &str) -> Result<Self, CryptoError> {
        let validated = validate_artifact_type(s)?;
        Ok(Self(validated))
    }

    /// Return the artifact type as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ArtifactType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<&str> for ArtifactType {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<str> for ArtifactType {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

/// A reference to a content-addressed artifact.
///
/// Combines the artifact type (e.g., `"lawpack"`, `"receipt"`, `"vc"`) with
/// its content digest. This pair uniquely identifies an artifact in the CAS
/// and determines its storage path: `{base_dir}/{artifact_type}/{digest_hex}.json`.
#[derive(Debug, Clone)]
pub struct ArtifactRef {
    /// The validated artifact type (e.g., "lawpack", "receipt", "vc").
    pub artifact_type: ArtifactType,
    /// The content digest of the artifact.
    pub digest: ContentDigest,
}

impl ArtifactRef {
    /// Construct a new artifact reference.
    ///
    /// Validates the artifact type against the CAS naming convention.
    pub fn new(artifact_type: &str, digest: ContentDigest) -> Result<Self, CryptoError> {
        let t = ArtifactType::new(artifact_type)?;
        Ok(Self {
            artifact_type: t,
            digest,
        })
    }

    /// Return the filesystem path for this artifact relative to a CAS base directory.
    pub fn path_in(&self, base_dir: &Path) -> PathBuf {
        base_dir
            .join(self.artifact_type.as_str())
            .join(format!("{}.json", self.digest.to_hex()))
    }
}

// ---------------------------------------------------------------------------
// ContentAddressedStore
// ---------------------------------------------------------------------------

/// A content-addressed artifact store backed by the filesystem.
///
/// Artifacts are stored at `{base_dir}/{artifact_type}/{digest_hex}.json`.
/// This matches the Python `dist/artifacts/` directory layout.
///
/// ## Integrity
///
/// On retrieval via [`resolve()`](ContentAddressedStore::resolve), the content
/// digest is recomputed from the stored bytes and verified against the
/// filename. If the digest does not match, the artifact is considered
/// corrupted and an error is returned.
///
/// ## Store Path
///
/// All digest computation flows through `CanonicalBytes::new()` →
/// `sha256_digest()`, ensuring cross-language agreement with the Python
/// layer's `jcs_canonicalize()` → `hashlib.sha256()` path.
#[derive(Debug, Clone)]
pub struct ContentAddressedStore {
    /// The root directory for CAS storage.
    base_dir: PathBuf,
}

impl ContentAddressedStore {
    /// Create a new CAS store rooted at the given directory.
    ///
    /// The directory does not need to exist yet — it will be created
    /// on the first [`store()`](ContentAddressedStore::store) call.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Return the base directory path.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Store a serializable value as a content-addressed JSON artifact.
    ///
    /// The value is canonicalized via `CanonicalBytes::new()`, its SHA-256
    /// digest is computed, and the canonical bytes are written to
    /// `{base_dir}/{artifact_type}/{digest_hex}.json`.
    ///
    /// Returns an [`ArtifactRef`] containing the artifact type and digest.
    ///
    /// If an artifact with the same digest already exists, the file is not
    /// overwritten (content-addressed storage is idempotent).
    pub fn store(
        &self,
        artifact_type: &str,
        data: &impl Serialize,
    ) -> Result<ArtifactRef, CryptoError> {
        let t = ArtifactType::new(artifact_type)?;
        let canonical = CanonicalBytes::new(data)
            .map_err(|e| CryptoError::Cas(format!("canonicalization failed: {e}")))?;
        let digest = sha256_digest(&canonical);
        let artifact_ref = ArtifactRef {
            artifact_type: t.clone(),
            digest,
        };

        let dir = self.base_dir.join(t.as_str());
        fs::create_dir_all(&dir)?;

        let path = artifact_ref.path_in(&self.base_dir);
        // Atomic create-if-absent: OpenOptions::create_new(true) fails with
        // AlreadyExists if the file exists, eliminating the TOCTOU race
        // between exists() and write() under concurrent access.
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                use std::io::Write;
                f.write_all(canonical.as_bytes())?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Content-addressed: identical digest means identical content.
            }
            Err(e) => return Err(e.into()),
        }

        Ok(artifact_ref)
    }

    /// Store raw bytes with a pre-computed digest.
    ///
    /// This is the low-level storage primitive. The caller is responsible
    /// for ensuring the digest matches the content. The store will verify
    /// the digest on retrieval.
    ///
    /// Use [`store()`](ContentAddressedStore::store) for the standard
    /// canonicalize-and-digest path.
    pub fn store_raw(
        &self,
        artifact_type: &str,
        digest: &ContentDigest,
        bytes: &[u8],
    ) -> Result<ArtifactRef, CryptoError> {
        let t = ArtifactType::new(artifact_type)?;
        let artifact_ref = ArtifactRef {
            artifact_type: t.clone(),
            digest: digest.clone(),
        };

        let dir = self.base_dir.join(t.as_str());
        fs::create_dir_all(&dir)?;

        let path = artifact_ref.path_in(&self.base_dir);
        // Atomic create-if-absent: same rationale as store().
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                use std::io::Write;
                f.write_all(bytes)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Content-addressed: identical digest means identical content.
            }
            Err(e) => return Err(e.into()),
        }

        Ok(artifact_ref)
    }

    /// Resolve an artifact by type and digest, returning the raw bytes.
    ///
    /// Returns `Ok(None)` if no artifact with the given type and digest
    /// exists. Returns `Ok(Some(bytes))` if found and integrity-verified.
    ///
    /// ## Integrity Verification
    ///
    /// After reading the file, the content is re-canonicalized and its
    /// SHA-256 digest is recomputed. If the digest does not match the
    /// filename, the file is considered corrupted and an error is returned.
    pub fn resolve(
        &self,
        artifact_type: &str,
        digest: &ContentDigest,
    ) -> Result<Option<Vec<u8>>, CryptoError> {
        let t = ArtifactType::new(artifact_type)?;
        let hex = digest.to_hex();
        let path = self.base_dir.join(t.as_str()).join(format!("{hex}.json"));

        if !path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&path)?;

        // Integrity check: recompute digest from the stored bytes.
        // Parse back to Value, canonicalize, and verify digest matches.
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            CryptoError::Cas(format!(
                "stored artifact at {} is not valid JSON: {e}",
                path.display()
            ))
        })?;
        let recanon = CanonicalBytes::from_value(value).map_err(|e| {
            CryptoError::Cas(format!(
                "stored artifact at {} failed re-canonicalization: {e}",
                path.display()
            ))
        })?;
        let recomputed = sha256_digest(&recanon);
        // Constant-time comparison to prevent timing side-channel on digest
        // verification. Comparing raw 32-byte digests avoids hex encoding
        // overhead and provides a fixed-length comparison.
        if !bool::from(recomputed.as_bytes().ct_eq(digest.as_bytes())) {
            return Err(CryptoError::Cas(format!(
                "integrity violation: artifact at {} has digest {} but filename says {}",
                path.display(),
                recomputed.to_hex(),
                hex,
            )));
        }

        Ok(Some(bytes))
    }

    /// Resolve an artifact by its [`ArtifactRef`].
    ///
    /// Convenience wrapper around [`resolve()`](ContentAddressedStore::resolve).
    pub fn resolve_ref(&self, artifact_ref: &ArtifactRef) -> Result<Option<Vec<u8>>, CryptoError> {
        self.resolve(artifact_ref.artifact_type.as_str(), &artifact_ref.digest)
    }

    /// Check whether an artifact exists in the store.
    pub fn contains(
        &self,
        artifact_type: &str,
        digest: &ContentDigest,
    ) -> Result<bool, CryptoError> {
        let t = ArtifactType::new(artifact_type)?;
        let path = self
            .base_dir
            .join(t.as_str())
            .join(format!("{}.json", digest.to_hex()));
        Ok(path.exists())
    }

    /// List all digests stored for a given artifact type.
    ///
    /// Returns hex digest strings extracted from filenames matching
    /// `{artifact_type}/*.json`.
    pub fn list_digests(&self, artifact_type: &str) -> Result<Vec<String>, CryptoError> {
        let t = ArtifactType::new(artifact_type)?;
        let dir = self.base_dir.join(t.as_str());

        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut digests = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if validate_digest_hex(stem).is_ok() {
                        digests.push(stem.to_string());
                    }
                }
            }
        }
        digests.sort();
        Ok(digests)
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
    fn validate_artifact_type_accepts_valid() {
        assert_eq!(validate_artifact_type("lawpack").unwrap(), "lawpack");
        assert_eq!(validate_artifact_type("receipt").unwrap(), "receipt");
        assert_eq!(
            validate_artifact_type("transition-types").unwrap(),
            "transition-types"
        );
        assert_eq!(validate_artifact_type("vc").unwrap(), "vc");
        assert_eq!(validate_artifact_type("abc123").unwrap(), "abc123");
    }

    #[test]
    fn validate_artifact_type_normalizes_case() {
        assert_eq!(validate_artifact_type("LawPack").unwrap(), "lawpack");
    }

    #[test]
    fn validate_artifact_type_rejects_empty() {
        assert!(validate_artifact_type("").is_err());
        assert!(validate_artifact_type("  ").is_err());
    }

    #[test]
    fn validate_artifact_type_rejects_invalid_chars() {
        assert!(validate_artifact_type("law/pack").is_err());
        assert!(validate_artifact_type("law pack").is_err());
        assert!(validate_artifact_type("law_pack").is_err());
        assert!(validate_artifact_type("-leading").is_err());
    }

    #[test]
    fn validate_digest_hex_accepts_valid() {
        let valid = "43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b888292777";
        assert_eq!(validate_digest_hex(valid).unwrap(), valid);
    }

    #[test]
    fn validate_digest_hex_rejects_wrong_length() {
        assert!(validate_digest_hex("abc123").is_err());
        assert!(validate_digest_hex("").is_err());
    }

    #[test]
    fn validate_digest_hex_rejects_non_hex() {
        let invalid = "43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b88829277g";
        assert!(validate_digest_hex(invalid).is_err());
    }

    #[test]
    fn store_and_resolve_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"action": "transfer", "amount": 1000});
        let artifact_ref = cas.store("receipt", &data).unwrap();

        assert_eq!(artifact_ref.artifact_type, "receipt");

        let resolved = cas.resolve("receipt", &artifact_ref.digest).unwrap();
        assert!(resolved.is_some());

        // The resolved bytes should parse to the same canonical form.
        let resolved_value: serde_json::Value = serde_json::from_slice(&resolved.unwrap()).unwrap();
        let original_canonical = CanonicalBytes::new(&data).unwrap();
        let resolved_canonical = CanonicalBytes::new(&resolved_value).unwrap();
        assert_eq!(original_canonical, resolved_canonical);
    }

    #[test]
    fn store_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"key": "value"});
        let ref1 = cas.store("vc", &data).unwrap();
        let ref2 = cas.store("vc", &data).unwrap();

        assert_eq!(ref1.digest, ref2.digest);
        assert_eq!(ref1.artifact_type, ref2.artifact_type);
    }

    #[test]
    fn resolve_nonexistent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        // Store something so the type directory exists
        let data = json!({"x": 1});
        let artifact_ref = cas.store("lawpack", &data).unwrap();

        // Try to resolve with a different digest
        let other_data = json!({"y": 2});
        let other_canonical = CanonicalBytes::new(&other_data).unwrap();
        let other_digest = sha256_digest(&other_canonical);

        assert_ne!(artifact_ref.digest, other_digest);
        let result = cas.resolve("lawpack", &other_digest).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn resolve_detects_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"important": "data"});
        let artifact_ref = cas.store("receipt", &data).unwrap();

        // Manually corrupt the file by writing different content
        let path = artifact_ref.path_in(dir.path());
        fs::write(&path, b"{\"important\":\"tampered\"}").unwrap();

        // Resolve should detect the integrity violation
        let result = cas.resolve("receipt", &artifact_ref.digest);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            format!("{err}").contains("integrity violation"),
            "expected integrity violation error, got: {err}"
        );
    }

    #[test]
    fn contains_check() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"test": true});
        let artifact_ref = cas.store("vc", &data).unwrap();

        assert!(cas.contains("vc", &artifact_ref.digest).unwrap());

        let other_canonical = CanonicalBytes::new(&json!({"other": true})).unwrap();
        let other_digest = sha256_digest(&other_canonical);
        assert!(!cas.contains("vc", &other_digest).unwrap());
    }

    #[test]
    fn list_digests_returns_stored() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        // Empty initially
        assert!(cas.list_digests("receipt").unwrap().is_empty());

        let d1 = json!({"a": 1});
        let d2 = json!({"b": 2});
        let ref1 = cas.store("receipt", &d1).unwrap();
        let ref2 = cas.store("receipt", &d2).unwrap();

        let digests = cas.list_digests("receipt").unwrap();
        assert_eq!(digests.len(), 2);
        assert!(digests.contains(&ref1.digest.to_hex()));
        assert!(digests.contains(&ref2.digest.to_hex()));
    }

    #[test]
    fn artifact_ref_path() {
        let data = json!({"test": "path"});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let digest = sha256_digest(&canonical);
        let hex = digest.to_hex();

        let artifact_ref = ArtifactRef::new("lawpack", digest).unwrap();
        let path = artifact_ref.path_in(Path::new("/cas/store"));

        assert_eq!(
            path,
            PathBuf::from(format!("/cas/store/lawpack/{hex}.json"))
        );
    }

    #[test]
    fn resolve_ref_convenience() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"via": "ref"});
        let artifact_ref = cas.store("vc", &data).unwrap();

        let resolved = cas.resolve_ref(&artifact_ref).unwrap();
        assert!(resolved.is_some());
    }

    #[test]
    fn different_types_are_separate_namespaces() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"shared": "data"});
        let ref_receipt = cas.store("receipt", &data).unwrap();
        let ref_vc = cas.store("vc", &data).unwrap();

        // Same data produces same digest
        assert_eq!(ref_receipt.digest, ref_vc.digest);

        // But they live in different directories
        assert!(cas.contains("receipt", &ref_receipt.digest).unwrap());
        assert!(cas.contains("vc", &ref_vc.digest).unwrap());

        let receipt_path = ref_receipt.path_in(dir.path());
        let vc_path = ref_vc.path_in(dir.path());
        assert_ne!(receipt_path, vc_path);
        assert!(receipt_path.exists());
        assert!(vc_path.exists());
    }

    #[test]
    fn store_raw_and_resolve() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"raw": "test"});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let digest = sha256_digest(&canonical);

        let artifact_ref = cas
            .store_raw("receipt", &digest, canonical.as_bytes())
            .unwrap();
        assert_eq!(artifact_ref.digest, digest);

        let resolved = cas.resolve("receipt", &digest).unwrap();
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), canonical.as_bytes());
    }

    #[test]
    fn store_creates_nested_directories() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("deep").join("nested").join("cas");
        let cas = ContentAddressedStore::new(&nested);

        let data = json!({"nested": true});
        let artifact_ref = cas.store("receipt", &data).unwrap();

        let path = artifact_ref.path_in(&nested);
        assert!(path.exists());
    }

    // ── Coverage expansion tests ─────────────────────────────────────

    #[test]
    fn validate_artifact_type_too_long() {
        let long_type = "a".repeat(65);
        let result = validate_artifact_type(&long_type);
        assert!(result.is_err());
    }

    #[test]
    fn validate_artifact_type_starts_with_hyphen() {
        let result = validate_artifact_type("-invalid");
        assert!(result.is_err());
    }

    #[test]
    fn validate_artifact_type_contains_uppercase() {
        // Should be lowercased and accepted
        let result = validate_artifact_type("Receipt");
        assert_eq!(result.unwrap(), "receipt");
    }

    #[test]
    fn validate_artifact_type_contains_special_char() {
        let result = validate_artifact_type("my_type");
        assert!(result.is_err());
    }

    #[test]
    fn base_dir_accessor() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());
        assert_eq!(cas.base_dir(), dir.path());
    }

    #[test]
    fn store_raw_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"idempotent": true});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let digest = sha256_digest(&canonical);

        // Store twice
        cas.store_raw("receipt", &digest, canonical.as_bytes())
            .unwrap();
        cas.store_raw("receipt", &digest, canonical.as_bytes())
            .unwrap();

        // Should still resolve correctly
        let resolved = cas.resolve("receipt", &digest).unwrap();
        assert!(resolved.is_some());
    }

    #[test]
    fn resolve_nonexistent_artifact() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"nonexistent": true});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let digest = sha256_digest(&canonical);

        let result = cas.resolve("receipt", &digest).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn resolve_corrupted_artifact() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        // Store a valid artifact
        let data = json!({"key": "value"});
        let artifact_ref = cas.store("receipt", &data).unwrap();

        // Corrupt the file
        let path = artifact_ref.path_in(dir.path());
        std::fs::write(&path, r#"{"key": "DIFFERENT"}"#).unwrap();

        // Resolve should detect the integrity violation
        let result = cas.resolve("receipt", &artifact_ref.digest);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_non_json_file() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"non_json": true});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let digest = sha256_digest(&canonical);

        // Write non-JSON content to the expected path
        let receipt_dir = dir.path().join("receipt");
        std::fs::create_dir_all(&receipt_dir).unwrap();
        std::fs::write(
            receipt_dir.join(format!("{}.json", digest.to_hex())),
            "not json at all",
        )
        .unwrap();

        let result = cas.resolve("receipt", &digest);
        assert!(result.is_err());
    }

    #[test]
    fn list_digests_empty_type_dir() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let digests = cas.list_digests("receipt").unwrap();
        assert!(digests.is_empty());
    }

    #[test]
    fn list_digests_skips_non_json_files() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        // Store a valid artifact
        cas.store("receipt", &json!({"a": 1})).unwrap();

        // Add a non-json file
        let receipt_dir = dir.path().join("receipt");
        std::fs::write(receipt_dir.join("readme.txt"), "not an artifact").unwrap();

        let digests = cas.list_digests("receipt").unwrap();
        assert_eq!(digests.len(), 1);
    }

    #[test]
    fn list_digests_skips_invalid_digest_filenames() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        // Store a valid artifact
        cas.store("receipt", &json!({"b": 2})).unwrap();

        // Add a JSON file with non-digest name
        let receipt_dir = dir.path().join("receipt");
        std::fs::write(receipt_dir.join("not-a-digest.json"), "{}").unwrap();

        let digests = cas.list_digests("receipt").unwrap();
        assert_eq!(digests.len(), 1);
    }

    #[test]
    fn contains_returns_false_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"absent": true});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let digest = sha256_digest(&canonical);

        assert!(!cas.contains("receipt", &digest).unwrap());
    }

    #[test]
    fn resolve_ref_delegates_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());

        let data = json!({"delegate": "test"});
        let artifact_ref = cas.store("receipt", &data).unwrap();

        let resolved = cas.resolve_ref(&artifact_ref).unwrap();
        assert!(resolved.is_some());
    }

    #[test]
    fn artifact_ref_new_validates_type() {
        let canonical = CanonicalBytes::new(&json!({})).unwrap();
        let digest = sha256_digest(&canonical);

        assert!(ArtifactRef::new("valid-type", digest.clone()).is_ok());
        assert!(ArtifactRef::new("", digest.clone()).is_err());
        assert!(ArtifactRef::new("-bad", digest).is_err());
    }

    #[test]
    fn artifact_ref_debug_format() {
        let canonical = CanonicalBytes::new(&json!({})).unwrap();
        let digest = sha256_digest(&canonical);
        let aref = ArtifactRef::new("receipt", digest).unwrap();
        let debug = format!("{aref:?}");
        assert!(debug.contains("ArtifactRef"));
        assert!(debug.contains("receipt"));
    }

    #[test]
    fn store_with_float_fails() {
        let dir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(dir.path());
        let result = cas.store("receipt", &json!({"amount": 3.15}));
        assert!(result.is_err());
    }

    #[test]
    fn validate_digest_hex_rejects_short() {
        assert!(validate_digest_hex("abcdef").is_err());
    }
}
