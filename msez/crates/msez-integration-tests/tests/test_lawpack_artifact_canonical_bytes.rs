//! Tests for lawpack artifact canonical byte computation.
//!
//! Validates that lawpack data produces deterministic, content-sensitive
//! digests through the canonical bytes pipeline. Uses the same lawpack
//! digest protocol as `tools/lawpack.py`.

use msez_core::{sha256_digest, CanonicalBytes, Sha256Accumulator};
use serde_json::json;
use std::collections::BTreeMap;

/// Compute lawpack digest using the v1 protocol.
fn compute_lawpack_digest(paths_data: &BTreeMap<&str, serde_json::Value>) -> String {
    let mut acc = Sha256Accumulator::new();
    acc.update(b"msez-lawpack-v1\0");
    for (path, data) in paths_data {
        acc.update(path.as_bytes());
        acc.update(b"\0");
        let canonical = CanonicalBytes::new(data).unwrap();
        acc.update(canonical.as_bytes());
        acc.update(b"\0");
    }
    acc.finalize_hex()
}

// ---------------------------------------------------------------------------
// Lawpack digest uses canonical bytes
// ---------------------------------------------------------------------------

#[test]
fn lawpack_digest_uses_canonical_bytes() {
    let mut paths_data = BTreeMap::new();
    paths_data.insert(
        "modules/tax/withholding.yaml",
        json!({"name": "withholding", "version": "1.0"}),
    );

    let digest = compute_lawpack_digest(&paths_data);
    assert_eq!(
        digest.len(),
        64,
        "Lawpack digest must be a 64-char hex string"
    );

    // Must be deterministic.
    let digest2 = compute_lawpack_digest(&paths_data);
    assert_eq!(digest, digest2, "Lawpack digest must be deterministic");
}

// ---------------------------------------------------------------------------
// Lawpack digest deterministic
// ---------------------------------------------------------------------------

#[test]
fn lawpack_digest_deterministic() {
    let mut paths_data = BTreeMap::new();
    paths_data.insert(
        "modules/aml/screening.yaml",
        json!({"name": "screening", "version": "1.0", "domain": "aml"}),
    );
    paths_data.insert(
        "modules/kyc/identity.yaml",
        json!({"name": "identity", "version": "2.1", "domain": "kyc"}),
    );

    let d1 = compute_lawpack_digest(&paths_data);
    let d2 = compute_lawpack_digest(&paths_data);
    assert_eq!(d1, d2);
}

// ---------------------------------------------------------------------------
// Lawpack digest changes with content
// ---------------------------------------------------------------------------

#[test]
fn lawpack_digest_changes_with_content() {
    let mut paths_data_v1 = BTreeMap::new();
    paths_data_v1.insert(
        "modules/tax/withholding.yaml",
        json!({"name": "withholding", "version": "1.0"}),
    );

    let mut paths_data_v2 = BTreeMap::new();
    paths_data_v2.insert(
        "modules/tax/withholding.yaml",
        json!({"name": "withholding", "version": "2.0"}),
    );

    let d1 = compute_lawpack_digest(&paths_data_v1);
    let d2 = compute_lawpack_digest(&paths_data_v2);

    assert_ne!(
        d1, d2,
        "Different content must produce different lawpack digest"
    );
}

// ---------------------------------------------------------------------------
// Standalone canonical bytes for lawpack-like data
// ---------------------------------------------------------------------------

#[test]
fn lawpack_data_canonical_bytes() {
    // Verify that lawpack-like data produces consistent canonical bytes.
    let data = json!({
        "jurisdiction_id": "PK-RSEZ",
        "version": "1.0",
        "statutes": {
            "companies-act": {"title": "Companies Act"}
        }
    });

    let c1 = CanonicalBytes::new(&data).unwrap();
    let c2 = CanonicalBytes::new(&data).unwrap();

    assert_eq!(sha256_digest(&c1).to_hex(), sha256_digest(&c2).to_hex(),);
}

#[test]
fn lawpack_digest_changes_with_additional_path() {
    let mut paths_data = BTreeMap::new();
    paths_data.insert(
        "modules/tax/withholding.yaml",
        json!({"name": "withholding", "version": "1.0"}),
    );
    let d1 = compute_lawpack_digest(&paths_data);

    paths_data.insert(
        "modules/aml/screening.yaml",
        json!({"name": "screening", "version": "1.0"}),
    );
    let d2 = compute_lawpack_digest(&paths_data);

    assert_ne!(d1, d2, "Adding a path must change the lawpack digest");
}
