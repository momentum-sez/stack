//! # Strict Smart Asset Artifact Graph
//!
//! Tests that smart asset artifacts maintain strict graph structure with
//! required fields. Verifies that smart asset artifacts include entity and
//! jurisdiction bindings, and that their digests are deterministic and
//! correctly track jurisdiction-specific compliance data.

use mez_core::{sha256_digest, CanonicalBytes, EntityId, JurisdictionId};
use mez_crypto::ContentAddressedStore;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

#[test]
fn smart_asset_artifact_has_required_fields() {
    let (_dir, store) = make_store();
    let entity_id = EntityId::new();
    let jurisdiction = JurisdictionId::new("PK-RSEZ").unwrap();

    let smart_asset = json!({
        "asset_type": "SmartAsset",
        "entity_id": entity_id.to_string(),
        "jurisdiction": jurisdiction.as_str(),
        "compliance_status": "evaluated",
        "lawpack_digest": "aa".repeat(32),
        "registry_digest": "bb".repeat(32),
        "created_at": "2026-01-15T12:00:00Z"
    });

    let artifact_ref = store.store("smart-asset", &smart_asset).unwrap();
    assert_eq!(artifact_ref.artifact_type, "smart-asset");

    // Resolve and verify required fields are preserved
    let resolved = store
        .resolve("smart-asset", &artifact_ref.digest)
        .unwrap()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&resolved).unwrap();

    assert_eq!(parsed["asset_type"], "SmartAsset");
    assert!(parsed["entity_id"].is_string());
    assert_eq!(parsed["jurisdiction"], "PK-RSEZ");
    assert_eq!(parsed["compliance_status"], "evaluated");
    assert!(parsed["lawpack_digest"].is_string());
    assert!(parsed["registry_digest"].is_string());
}

#[test]
fn smart_asset_artifact_digest_deterministic() {
    let entity_id = EntityId::new();
    let jurisdiction = JurisdictionId::new("AE-DIFC").unwrap();

    let smart_asset = json!({
        "asset_type": "SmartAsset",
        "entity_id": entity_id.to_string(),
        "jurisdiction": jurisdiction.as_str(),
        "compliance_domains": ["aml", "kyc", "sanctions"],
        "evaluation_result": "pass"
    });

    let canonical_1 = CanonicalBytes::new(&smart_asset).unwrap();
    let digest_1 = sha256_digest(&canonical_1);

    let canonical_2 = CanonicalBytes::new(&smart_asset).unwrap();
    let digest_2 = sha256_digest(&canonical_2);

    assert_eq!(digest_1, digest_2);
    assert_eq!(digest_1.to_hex().len(), 64);
}

#[test]
fn smart_asset_jurisdiction_binding_in_graph() {
    let (_dir, store) = make_store();
    let ja = JurisdictionId::new("PK-RSEZ").unwrap();
    let jb = JurisdictionId::new("AE-DIFC").unwrap();

    // Create jurisdiction-specific compliance artifacts
    let compliance_a = json!({
        "jurisdiction": ja.as_str(),
        "domains": ["aml", "kyc"],
        "status": "compliant"
    });
    let compliance_b = json!({
        "jurisdiction": jb.as_str(),
        "domains": ["sanctions", "tax"],
        "status": "compliant"
    });

    let ref_a = store.store("compliance", &compliance_a).unwrap();
    let ref_b = store.store("compliance", &compliance_b).unwrap();

    // Create a smart asset graph binding both jurisdictions
    let asset_graph = json!({
        "asset_type": "CrossBorderSmartAsset",
        "jurisdiction_bindings": [
            {"jurisdiction": ja.as_str(), "compliance_digest": ref_a.digest.to_hex()},
            {"jurisdiction": jb.as_str(), "compliance_digest": ref_b.digest.to_hex()}
        ]
    });

    let graph_ref = store.store("smart-asset-graph", &asset_graph).unwrap();

    // All artifacts should be independently resolvable
    assert!(store.contains("compliance", &ref_a.digest).unwrap());
    assert!(store.contains("compliance", &ref_b.digest).unwrap());
    assert!(store
        .contains("smart-asset-graph", &graph_ref.digest)
        .unwrap());

    // The two compliance artifacts must have different digests
    assert_ne!(ref_a.digest, ref_b.digest);
}

#[test]
fn smart_asset_graph_rejects_float_amounts() {
    // Amounts must be integers or strings, never floats.
    // CanonicalBytes::new rejects floats per canonicalization rules.
    let asset_with_float = json!({
        "asset_type": "SmartAsset",
        "value": 1000.50
    });

    let result = CanonicalBytes::new(&asset_with_float);
    assert!(
        result.is_err(),
        "floats must be rejected by canonicalization"
    );
}
