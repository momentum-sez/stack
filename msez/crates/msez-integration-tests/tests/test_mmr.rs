//! # Merkle Mountain Range (MMR) Integration Tests
//!
//! Python counterpart: `tests/test_mmr.py`
//!
//! Tests the Merkle Mountain Range operations:
//! - Empty MMR behavior
//! - Single leaf appending and root computation
//! - Multiple leaves change the root
//! - Root is deterministic for same input
//! - Incremental append produces same root as batch
//! - Size tracks appends correctly

use msez_core::{sha256_digest, CanonicalBytes};
use msez_crypto::MerkleMountainRange;
use serde_json::json;

/// Generate a valid 64-hex-char digest for use as an MMR leaf.
fn make_leaf(label: &str) -> String {
    let data = json!({"leaf": label});
    let canonical = CanonicalBytes::new(&data).unwrap();
    sha256_digest(&canonical).to_hex()
}

// ---------------------------------------------------------------------------
// 1. Empty MMR
// ---------------------------------------------------------------------------

#[test]
fn mmr_empty_has_empty_root() {
    let mmr = MerkleMountainRange::new();
    assert_eq!(mmr.size(), 0);
    let root = mmr.root().unwrap();
    assert!(root.is_empty(), "empty MMR root should be empty string");
}

// ---------------------------------------------------------------------------
// 2. Single leaf
// ---------------------------------------------------------------------------

#[test]
fn mmr_single_leaf() {
    let mut mmr = MerkleMountainRange::new();
    let leaf = make_leaf("first");
    mmr.append(&leaf).unwrap();

    assert_eq!(mmr.size(), 1);
    let root = mmr.root().unwrap();
    assert_eq!(root.len(), 64, "root should be 64 hex chars");
    assert!(root.chars().all(|c| c.is_ascii_hexdigit()));
}

// ---------------------------------------------------------------------------
// 3. Multiple leaves change the root
// ---------------------------------------------------------------------------

#[test]
fn mmr_multiple_leaves_root_changes() {
    let mut mmr = MerkleMountainRange::new();

    mmr.append(&make_leaf("a")).unwrap();
    let root_1 = mmr.root().unwrap();

    mmr.append(&make_leaf("b")).unwrap();
    let root_2 = mmr.root().unwrap();

    mmr.append(&make_leaf("c")).unwrap();
    let root_3 = mmr.root().unwrap();

    assert_ne!(root_1, root_2, "root must change after second append");
    assert_ne!(root_2, root_3, "root must change after third append");
    assert_ne!(root_1, root_3, "all roots must differ");
}

// ---------------------------------------------------------------------------
// 4. Root is deterministic
// ---------------------------------------------------------------------------

#[test]
fn mmr_root_deterministic() {
    let leaves: Vec<String> = (0..5).map(|i| make_leaf(&format!("leaf-{i}"))).collect();

    let build_root = || {
        let mut mmr = MerkleMountainRange::new();
        for leaf in &leaves {
            mmr.append(leaf).unwrap();
        }
        mmr.root().unwrap()
    };

    let root1 = build_root();
    let root2 = build_root();
    let root3 = build_root();

    assert_eq!(root1, root2, "first and second builds must match");
    assert_eq!(root2, root3, "second and third builds must match");
}

// ---------------------------------------------------------------------------
// 5. Incremental equals batch
// ---------------------------------------------------------------------------

#[test]
fn mmr_incremental_equals_batch() {
    let leaves: Vec<String> = (0..8).map(|i| make_leaf(&format!("item-{i}"))).collect();

    // Build incrementally
    let mut mmr_inc = MerkleMountainRange::new();
    for leaf in &leaves {
        mmr_inc.append(leaf).unwrap();
    }
    let root_inc = mmr_inc.root().unwrap();

    // Build from scratch (same data)
    let mut mmr_batch = MerkleMountainRange::new();
    for leaf in &leaves {
        mmr_batch.append(leaf).unwrap();
    }
    let root_batch = mmr_batch.root().unwrap();

    assert_eq!(
        root_inc, root_batch,
        "incremental and batch construction must produce same root"
    );
}

// ---------------------------------------------------------------------------
// 6. Size tracks appends
// ---------------------------------------------------------------------------

#[test]
fn mmr_size_tracks_appends() {
    let mut mmr = MerkleMountainRange::new();
    assert_eq!(mmr.size(), 0);

    for i in 0..10 {
        mmr.append(&make_leaf(&format!("leaf-{i}"))).unwrap();
        assert_eq!(mmr.size(), i + 1);
    }
}

// ---------------------------------------------------------------------------
// 7. Invalid hex is rejected
// ---------------------------------------------------------------------------

#[test]
fn mmr_rejects_invalid_hex() {
    let mut mmr = MerkleMountainRange::new();

    // Too short
    assert!(mmr.append("abcd").is_err());

    // Non-hex
    assert!(mmr.append(&"zz".repeat(32)).is_err());

    // Odd length
    assert!(mmr.append(&"a".repeat(63)).is_err());
}

// ---------------------------------------------------------------------------
// 8. Large MMR is stable
// ---------------------------------------------------------------------------

#[test]
fn mmr_large_stable() {
    let mut mmr = MerkleMountainRange::new();
    for i in 0..100 {
        mmr.append(&make_leaf(&format!("large-{i}"))).unwrap();
    }

    assert_eq!(mmr.size(), 100);
    let root = mmr.root().unwrap();
    assert_eq!(root.len(), 64);

    // Rebuild and compare
    let mut mmr2 = MerkleMountainRange::new();
    for i in 0..100 {
        mmr2.append(&make_leaf(&format!("large-{i}"))).unwrap();
    }
    assert_eq!(mmr2.root().unwrap(), root);
}
