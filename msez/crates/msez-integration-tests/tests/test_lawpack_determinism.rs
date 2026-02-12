//! Tests for lawpack digest determinism across multiple runs.
//!
//! Ensures that lawpack digests are stable across construction,
//! lock computation, and key reordering.

use msez_core::{CanonicalBytes, sha256_digest};
use sha2::{Digest as _, Sha256};
use serde_json::json;
use std::collections::BTreeMap;

mod hex_enc {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
    }
}

fn compute_lawpack_digest(paths_data: &BTreeMap<&str, serde_json::Value>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"msez-lawpack-v1\0");
    for (path, data) in paths_data {
        hasher.update(path.as_bytes());
        hasher.update(b"\0");
        let canonical = CanonicalBytes::new(data).unwrap();
        hasher.update(canonical.as_bytes());
        hasher.update(b"\0");
    }
    hex_enc::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// Same digest across runs
// ---------------------------------------------------------------------------

#[test]
fn lawpack_digest_same_across_runs() {
    let mut paths_data = BTreeMap::new();
    paths_data.insert(
        "modules/tax/withholding.yaml",
        json!({"name": "withholding", "version": "1.0"}),
    );
    paths_data.insert(
        "modules/aml/screening.yaml",
        json!({"name": "screening", "version": "1.0"}),
    );

    let digests: Vec<String> = (0..10)
        .map(|_| compute_lawpack_digest(&paths_data))
        .collect();

    let first = &digests[0];
    for (i, d) in digests.iter().enumerate() {
        assert_eq!(
            first, d,
            "Digest mismatch on run {}: expected {}, got {}",
            i, first, d
        );
    }
}

// ---------------------------------------------------------------------------
// Lock digest deterministic
// ---------------------------------------------------------------------------

#[test]
fn lawpack_lock_digest_deterministic() {
    // The same path data must produce the same digest every time.
    let mut paths_data = BTreeMap::new();
    paths_data.insert(
        "statutes/companies-act-2017.yaml",
        json!({"title": "Companies Act 2017", "citation": "Act No. XIX of 2017"}),
    );
    paths_data.insert(
        "statutes/income-tax-2001.yaml",
        json!({"title": "Income Tax Ordinance 2001", "citation": "Ordinance XLIX of 2001"}),
    );

    let d1 = compute_lawpack_digest(&paths_data);
    let d2 = compute_lawpack_digest(&paths_data);
    assert_eq!(d1, d2, "Same data must produce same digest");
}

// ---------------------------------------------------------------------------
// Key ordering invariant
// ---------------------------------------------------------------------------

#[test]
fn lawpack_key_ordering_invariant() {
    // Lawpack data with different key ordering must produce the same
    // canonical bytes and digest.
    let a = json!({
        "jurisdiction_id": "PK-RSEZ",
        "version": "1.0",
        "name": "Test Lawpack"
    });

    let b = json!({
        "name": "Test Lawpack",
        "jurisdiction_id": "PK-RSEZ",
        "version": "1.0"
    });

    let ca = CanonicalBytes::new(&a).unwrap();
    let cb = CanonicalBytes::new(&b).unwrap();

    assert_eq!(
        sha256_digest(&ca).to_hex(),
        sha256_digest(&cb).to_hex(),
        "Key ordering must not affect lawpack digest"
    );
}

#[test]
fn lawpack_nested_key_ordering_invariant() {
    let a = json!({
        "jurisdiction_id": "PK-RSEZ",
        "statutes": {
            "z_act": {"title": "Z Act"},
            "a_act": {"title": "A Act"}
        }
    });

    let b = json!({
        "statutes": {
            "a_act": {"title": "A Act"},
            "z_act": {"title": "Z Act"}
        },
        "jurisdiction_id": "PK-RSEZ"
    });

    let ca = CanonicalBytes::new(&a).unwrap();
    let cb = CanonicalBytes::new(&b).unwrap();

    assert_eq!(
        sha256_digest(&ca).to_hex(),
        sha256_digest(&cb).to_hex(),
    );
}

#[test]
fn lawpack_different_jurisdictions_different_digest() {
    // Different jurisdiction data must produce different digests.
    let mut paths_a = BTreeMap::new();
    paths_a.insert(
        "lawpack.yaml",
        json!({"jurisdiction_id": "PK-RSEZ", "domain": "financial"}),
    );

    let mut paths_b = BTreeMap::new();
    paths_b.insert(
        "lawpack.yaml",
        json!({"jurisdiction_id": "AE-DIFC", "domain": "financial"}),
    );

    let d1 = compute_lawpack_digest(&paths_a);
    let d2 = compute_lawpack_digest(&paths_b);

    assert_ne!(d1, d2, "Different jurisdictions must produce different digests");
}
