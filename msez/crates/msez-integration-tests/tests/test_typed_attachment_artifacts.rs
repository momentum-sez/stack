//! # Typed Attachment Artifacts
//!
//! Tests typed attachment artifacts with different content types stored in CAS.
//! Verifies that attachment type metadata is preserved through storage, that
//! different attachment types produce different digests, and that attachments
//! round-trip correctly through the content-addressed store.

use msez_core::{sha256_digest, CanonicalBytes};
use msez_crypto::ContentAddressedStore;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

#[test]
fn attachment_with_type_field() {
    let (_dir, store) = make_store();

    let attachment = json!({
        "attachment_type": "regulatory-filing",
        "content_type": "application/json",
        "jurisdiction": "PK-RSEZ",
        "payload": {
            "filing_number": "FBR-2026-001",
            "entity_ntn": "1234567-8",
            "tax_year": 2026
        }
    });

    let artifact_ref = store.store("attachment", &attachment).unwrap();
    assert_eq!(artifact_ref.artifact_type, "attachment");

    let resolved = store.resolve("attachment", &artifact_ref.digest).unwrap();
    assert!(resolved.is_some());

    let parsed: serde_json::Value = serde_json::from_slice(&resolved.unwrap()).unwrap();
    assert_eq!(parsed["attachment_type"], "regulatory-filing");
    assert_eq!(parsed["content_type"], "application/json");
    assert_eq!(parsed["payload"]["filing_number"], "FBR-2026-001");
}

#[test]
fn different_attachment_types_different_digests() {
    let filing = json!({
        "attachment_type": "regulatory-filing",
        "content": "filing-data"
    });

    let certificate = json!({
        "attachment_type": "incorporation-certificate",
        "content": "certificate-data"
    });

    let license = json!({
        "attachment_type": "business-license",
        "content": "license-data"
    });

    let digest_filing = sha256_digest(&CanonicalBytes::new(&filing).unwrap());
    let digest_cert = sha256_digest(&CanonicalBytes::new(&certificate).unwrap());
    let digest_license = sha256_digest(&CanonicalBytes::new(&license).unwrap());

    assert_ne!(digest_filing, digest_cert);
    assert_ne!(digest_filing, digest_license);
    assert_ne!(digest_cert, digest_license);
}

#[test]
fn attachment_roundtrip() {
    let (_dir, store) = make_store();

    let attachment = json!({
        "attachment_type": "watcher-bond-receipt",
        "watcher_id": "watcher-pk-001",
        "bond_amount": "1000000",
        "currency": "PKR",
        "timestamp": "2026-01-15T12:00:00Z",
        "proof_of_deposit": {
            "bank": "HBL",
            "reference": "TX-20260115-001"
        }
    });

    // Store
    let artifact_ref = store.store("attachment", &attachment).unwrap();

    // Resolve
    let resolved = store.resolve("attachment", &artifact_ref.digest).unwrap();
    assert!(resolved.is_some());

    // Verify canonical equivalence
    let original_canonical = CanonicalBytes::new(&attachment).unwrap();
    let resolved_value: serde_json::Value = serde_json::from_slice(&resolved.unwrap()).unwrap();
    let resolved_canonical = CanonicalBytes::new(&resolved_value).unwrap();

    assert_eq!(original_canonical, resolved_canonical);
}

#[test]
fn attachment_type_preserved_in_cas() {
    let (_dir, store) = make_store();

    // Store multiple attachment types
    let types = ["filing", "certificate", "license", "bond-receipt"];
    let refs: Vec<_> = types
        .iter()
        .map(|t| {
            let data = json!({
                "attachment_type": t,
                "data": format!("payload-for-{t}")
            });
            store.store("attachment", &data).unwrap()
        })
        .collect();

    // All should be listed under the "attachment" type
    let digests = store.list_digests("attachment").unwrap();
    assert_eq!(digests.len(), 4);

    // Each ref should be findable
    for r in &refs {
        assert!(store.contains("attachment", &r.digest).unwrap());
    }
}
