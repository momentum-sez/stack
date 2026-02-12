//! # Content-Addressed Storage (CAS)
//!
//! Provides store and resolve operations for the content-addressed
//! artifact store (`dist/artifacts/`). Artifacts are named by their
//! content digest: `{type}/{digest}.json`.
//!
//! ## Security Invariant
//!
//! All stored artifacts are verified at retrieval time — the digest
//! of the retrieved content must match the requested digest. This
//! prevents both corruption and substitution attacks.
//!
//! ## Implements
//!
//! Spec §7 — Content-addressed artifact store conventions.

use msez_core::ContentDigest;
use std::path::PathBuf;

/// A content-addressed artifact store backed by the filesystem.
///
/// Placeholder — full implementation will provide store, resolve,
/// verify, and graph operations matching `tools/artifacts.py`.
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
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Compute the filesystem path for an artifact by type and digest.
    pub fn artifact_path(&self, artifact_type: &str, digest: &ContentDigest) -> PathBuf {
        self.root
            .join(artifact_type)
            .join(format!("{}.json", digest.to_hex()))
    }
}
