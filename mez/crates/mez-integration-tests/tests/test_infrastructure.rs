//! Tests for infrastructure components: content-addressed store, Merkle
//! Mountain Range, Ed25519 key operations, and identifier types.

use mez_core::{sha256_digest, CanonicalBytes, EntityId, JurisdictionId};
use mez_crypto::{ContentAddressedStore, MerkleMountainRange, SigningKey};
use rand_core::OsRng;
use serde_json::json;

// ---------------------------------------------------------------------------
// Content-Addressed Store
// ---------------------------------------------------------------------------

#[test]
fn cas_store_and_resolve_cycle() {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    let data = json!({"contract_id": "C-2026-001", "amount": "150000"});

    let artifact_ref = store.store("lawpack", &data).unwrap();
    let resolved = store.resolve_ref(&artifact_ref).unwrap();

    assert!(resolved.is_some(), "Stored artifact must be resolvable");
}

#[test]
fn cas_store_deterministic_ref() {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    let data = json!({"key": "value"});

    let ref1 = store.store("lawpack", &data).unwrap();
    let ref2 = store.store("lawpack", &data).unwrap();

    assert_eq!(
        ref1.digest.to_hex(),
        ref2.digest.to_hex(),
        "Same content must produce same artifact ref"
    );
}

#[test]
fn cas_store_different_content_different_ref() {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());

    let ref1 = store.store("lawpack", &json!({"a": 1})).unwrap();
    let ref2 = store.store("lawpack", &json!({"a": 2})).unwrap();

    assert_ne!(
        ref1.digest.to_hex(),
        ref2.digest.to_hex(),
        "Different content must produce different artifact refs"
    );
}

// ---------------------------------------------------------------------------
// Merkle Mountain Range
// ---------------------------------------------------------------------------

#[test]
fn mmr_append_and_root() {
    let mut mmr = MerkleMountainRange::new();
    assert_eq!(mmr.size(), 0);

    let c1 = CanonicalBytes::new(&json!({"leaf": 1})).unwrap();
    let d1 = sha256_digest(&c1);
    mmr.append(&d1.to_hex()).unwrap();
    assert_eq!(mmr.size(), 1);

    let root1 = mmr.root().unwrap();

    let c2 = CanonicalBytes::new(&json!({"leaf": 2})).unwrap();
    let d2 = sha256_digest(&c2);
    mmr.append(&d2.to_hex()).unwrap();
    assert_eq!(mmr.size(), 2);

    let root2 = mmr.root().unwrap();
    assert_ne!(root1, root2, "Adding a leaf must change the root");
}

#[test]
fn mmr_deterministic_root() {
    let mut mmr1 = MerkleMountainRange::new();
    let mut mmr2 = MerkleMountainRange::new();

    for i in 0..5 {
        let c = CanonicalBytes::new(&json!({"leaf": i})).unwrap();
        let d = sha256_digest(&c);
        let hex = d.to_hex();
        mmr1.append(&hex).unwrap();
        mmr2.append(&hex).unwrap();
    }

    assert_eq!(
        mmr1.root().unwrap(),
        mmr2.root().unwrap(),
        "MMRs with same leaves must have same root"
    );
}

// ---------------------------------------------------------------------------
// Ed25519 key generation and signing
// ---------------------------------------------------------------------------

#[test]
fn ed25519_keygen_sign_verify() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let data = CanonicalBytes::new(&json!({"test": "message"})).unwrap();
    let signature = signing_key.sign(&data);

    assert!(
        verifying_key.verify(&data, &signature).is_ok(),
        "Signature verification must succeed for correct data"
    );
}

#[test]
fn ed25519_wrong_message_fails_verification() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let data = CanonicalBytes::new(&json!({"correct": true})).unwrap();
    let signature = signing_key.sign(&data);

    let wrong_data = CanonicalBytes::new(&json!({"correct": false})).unwrap();
    assert!(
        verifying_key.verify(&wrong_data, &signature).is_err(),
        "Signature verification must fail for wrong data"
    );
}

// ---------------------------------------------------------------------------
// Identifier types
// ---------------------------------------------------------------------------

#[test]
fn entity_id_uniqueness() {
    let id1 = EntityId::new();
    let id2 = EntityId::new();
    assert_ne!(id1, id2, "Two new EntityIds must be distinct");
}

#[test]
fn jurisdiction_id_validation() {
    let valid = JurisdictionId::new("PK-REZ");
    assert!(valid.is_ok(), "PK-REZ should be a valid JurisdictionId");

    let also_valid = JurisdictionId::new("AE-DIFC");
    assert!(
        also_valid.is_ok(),
        "AE-DIFC should be a valid JurisdictionId"
    );

    // Empty string should be rejected.
    let empty = JurisdictionId::new("");
    assert!(empty.is_err(), "Empty string should be rejected");
}

#[test]
fn jurisdiction_id_display() {
    let jid = JurisdictionId::new("PK-REZ").unwrap();
    assert_eq!(jid.as_str(), "PK-REZ");
}
