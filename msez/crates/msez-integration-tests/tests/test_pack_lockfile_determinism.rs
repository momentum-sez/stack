//! # Pack Lockfile Determinism Test
//!
//! Verifies that lawpack digest computation is deterministic:
//! running the same input through the canonicalization pipeline twice
//! produces byte-identical output.
//!
//! Also tests that the pack trilogy (lawpack, regpack, licensepack)
//! all produce stable, content-addressed identifiers.

use msez_core::{sha256_digest, CanonicalBytes};
use msez_pack::lawpack::LawpackRef;
use serde_json::json;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// 1. Lawpack digest computation is deterministic
// ---------------------------------------------------------------------------

#[test]
fn lawpack_digest_is_deterministic() {
    // Simulate the lawpack digest protocol:
    // SHA256( b"msez-lawpack-v1\0" + for each path in sorted(paths):
    //     path.encode("utf-8") + b"\0" + canonical_bytes + b"\0" )
    use sha2::{Digest, Sha256};

    let mut paths_data: BTreeMap<&str, serde_json::Value> = BTreeMap::new();
    paths_data.insert(
        "modules/tax/withholding.yaml",
        json!({"name": "withholding", "version": "1.0", "domain": "tax"}),
    );
    paths_data.insert(
        "modules/aml/screening.yaml",
        json!({"name": "screening", "version": "1.0", "domain": "aml"}),
    );
    paths_data.insert(
        "modules/kyc/identity.yaml",
        json!({"name": "identity", "version": "2.1", "domain": "kyc"}),
    );

    let compute_digest = || {
        let mut hasher = Sha256::new();
        hasher.update(b"msez-lawpack-v1\0");
        for (path, data) in &paths_data {
            hasher.update(path.as_bytes());
            hasher.update(b"\0");
            let canonical = CanonicalBytes::new(data).unwrap();
            hasher.update(canonical.as_bytes());
            hasher.update(b"\0");
        }
        let result = hasher.finalize();
        hex::encode(result)
    };

    // Compute twice — must be identical
    let d1 = compute_digest();
    let d2 = compute_digest();
    assert_eq!(d1, d2, "lawpack digest must be deterministic across runs");
    assert_eq!(d1.len(), 64);
}

/// Hex encoding helper (avoiding external `hex` crate)
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
    }
}

// ---------------------------------------------------------------------------
// 2. LawpackRef parsing and roundtrip
// ---------------------------------------------------------------------------

#[test]
fn lawpack_ref_parse_valid() {
    let ref_str = &format!("PK-RSEZ:financial:{}", "ab".repeat(32));
    let parsed = LawpackRef::parse(ref_str).unwrap();
    assert_eq!(parsed.jurisdiction_id, "PK-RSEZ");
    assert_eq!(parsed.domain, "financial");
    assert_eq!(parsed.lawpack_digest_sha256, "ab".repeat(32));
}

#[test]
fn lawpack_ref_parse_rejects_invalid() {
    // Too few parts
    assert!(LawpackRef::parse("PK-RSEZ:financial").is_err());
    // Too many parts
    assert!(LawpackRef::parse(&format!("PK-RSEZ:financial:{}:extra", "ab".repeat(32))).is_err());
    // Invalid digest length
    assert!(LawpackRef::parse("PK-RSEZ:financial:tooshort").is_err());
}

// ---------------------------------------------------------------------------
// 3. Canonical bytes determinism for pack data
// ---------------------------------------------------------------------------

#[test]
fn canonical_bytes_determinism_for_zone_config() {
    let zone = json!({
        "jurisdiction_id": "PK-RSEZ",
        "zone_name": "Reko Diq Special Economic Zone",
        "lawpack_refs": [
            "PK-RSEZ:financial:aabbccdd",
            "PK-RSEZ:corporate:eeff0011"
        ],
        "modules": {
            "corporate/formation": {"version": "1.0"},
            "tax/withholding": {"version": "2.1"}
        },
        "profiles": ["default", "trade-zone"]
    });

    let cb1 = CanonicalBytes::new(&zone).unwrap();
    let cb2 = CanonicalBytes::new(&zone).unwrap();
    assert_eq!(cb1.as_bytes(), cb2.as_bytes());

    let d1 = sha256_digest(&cb1);
    let d2 = sha256_digest(&cb2);
    assert_eq!(d1, d2);
}

// ---------------------------------------------------------------------------
// 4. Pack data with sorted fields
// ---------------------------------------------------------------------------

#[test]
fn pack_data_key_ordering_invariant() {
    // Keys should be sorted regardless of insertion order
    let v1 = json!({
        "z_module": "last",
        "a_module": "first",
        "m_module": "middle"
    });
    let v2 = json!({
        "a_module": "first",
        "m_module": "middle",
        "z_module": "last"
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&v1).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&v2).unwrap());
    assert_eq!(d1, d2);
}

// ---------------------------------------------------------------------------
// 5. Content-addressed artifact consistency
// ---------------------------------------------------------------------------

#[test]
fn artifact_content_addressing_is_stable() {
    // Simulate what CAS does: serialize → canonicalize → digest → store
    let artifact = json!({
        "type": "lawpack",
        "version": "1.0",
        "jurisdiction_id": "PK-RSEZ",
        "statutes": [
            {"name": "Income Tax Ordinance 2001", "sections": [80, 114, 153]},
            {"name": "Sales Tax Act 1990", "sections": [3, 7, 8]}
        ],
        "effective_date": "2026-01-01T00:00:00Z"
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&artifact).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&artifact).unwrap());
    assert_eq!(d1, d2, "CAS artifact digest must be stable");

    // Changing any field must change the digest
    let mut modified = artifact.clone();
    modified["version"] = json!("1.1");
    let d3 = sha256_digest(&CanonicalBytes::new(&modified).unwrap());
    assert_ne!(d1, d3, "different content must produce different digest");
}

// ---------------------------------------------------------------------------
// 6. Multiple runs produce identical lockfile data
// ---------------------------------------------------------------------------

#[test]
fn lockfile_data_multiple_runs_identical() {
    let lockfile = json!({
        "schema_version": "1.0",
        "locked_at": "2026-02-12T00:00:00Z",
        "jurisdiction_id": "PK-RSEZ",
        "packs": {
            "lawpack": {
                "digest": "a".repeat(64),
                "modules": ["tax/withholding", "aml/screening"]
            },
            "regpack": {
                "digest": "b".repeat(64),
                "requirements": 15
            },
            "licensepack": {
                "digest": "c".repeat(64),
                "categories": 12
            }
        }
    });

    // Simulate 3 "runs"
    let results: Vec<_> = (0..3)
        .map(|_| {
            let cb = CanonicalBytes::new(&lockfile).unwrap();
            sha256_digest(&cb).to_hex()
        })
        .collect();

    assert_eq!(results[0], results[1]);
    assert_eq!(results[1], results[2]);
}

// ---------------------------------------------------------------------------
// 7. BTreeMap ordering for pack paths
// ---------------------------------------------------------------------------

#[test]
fn btree_map_ensures_path_ordering() {
    let mut paths = BTreeMap::new();
    paths.insert("z/module.yaml", json!({"z": true}));
    paths.insert("a/module.yaml", json!({"a": true}));
    paths.insert("m/module.yaml", json!({"m": true}));

    let keys: Vec<_> = paths.keys().copied().collect();
    assert_eq!(
        keys,
        vec!["a/module.yaml", "m/module.yaml", "z/module.yaml"]
    );

    // Serialized via CanonicalBytes, keys should also be sorted
    let data = json!(paths);
    let cb = CanonicalBytes::new(&data).unwrap();
    let s = std::str::from_utf8(cb.as_bytes()).unwrap();
    let a_pos = s.find("a/module.yaml").unwrap();
    let m_pos = s.find("m/module.yaml").unwrap();
    let z_pos = s.find("z/module.yaml").unwrap();
    assert!(a_pos < m_pos);
    assert!(m_pos < z_pos);
}
