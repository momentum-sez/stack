//! Rust counterpart of tests/scenarios/test_comprehensive_library_scaffold.py
//! Tests that all library modules are importable and functional.

use mez_agentic::policy::standard_policies;
use mez_agentic::{PolicyEngine, TriggerType};
use mez_core::{
    sha256_digest, CanonicalBytes, Cnic, ComplianceDomain, CorridorId, Did, EntityId,
    JurisdictionId, MigrationId, Ntn, PassportNumber, Timestamp, WatcherId,
};
use mez_crypto::{ContentAddressedStore, MerkleMountainRange, SigningKey};
use mez_state::{Corridor, Draft, Entity, EntityLifecycleState, License, LicenseState};
use mez_zkp::{Cdb, MockProofSystem};
use rand_core::OsRng;
use serde_json::json;

#[test]
fn all_core_types_constructible() {
    let _eid = EntityId::new();
    let _mid = MigrationId::new();
    let _wid = WatcherId::new();
    let _jid = JurisdictionId::new("PK-REZ").unwrap();
    let _cid = CorridorId::new();
    let _ts = Timestamp::now();
    let _did = Did::new("did:key:z6MkTest123").unwrap();
    let _cnic = Cnic::new("1234567890123").unwrap();
    let _ntn = Ntn::new("1234567").unwrap();
    let _pp = PassportNumber::new("AB1234567").unwrap();
}

#[test]
fn all_crypto_primitives_functional() {
    let sk = SigningKey::generate(&mut OsRng);
    let _vk = sk.verifying_key();
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    let aref = store.store("test", &json!({"test": true})).unwrap();
    assert!(store.contains("test", &aref.digest).unwrap());
    let mut mmr = MerkleMountainRange::new();
    let leaf = sha256_digest(&CanonicalBytes::new(&json!({"leaf": 0})).unwrap()).to_hex();
    mmr.append(&leaf).unwrap();
    assert_eq!(mmr.size(), 1);
}

#[test]
fn all_state_machines_initializable() {
    let ja = JurisdictionId::new("PK-REZ").unwrap();
    let jb = JurisdictionId::new("AE-DIFC").unwrap();
    let corridor = Corridor::<Draft>::new(CorridorId::new(), ja.clone(), jb);
    assert_eq!(corridor.state_name(), "DRAFT");
    let entity = Entity::new(EntityId::new());
    assert_eq!(entity.state, EntityLifecycleState::Applied);
    let license = License::new("lic-001");
    assert_eq!(license.state, LicenseState::Applied);
}

#[test]
fn all_compliance_domains_enumerable() {
    let domains = ComplianceDomain::all();
    assert_eq!(domains.len(), 20);
    for d in domains {
        assert!(!d.as_str().is_empty());
    }
}

#[test]
fn agentic_engine_initializable() {
    let _engine = PolicyEngine::with_standard_policies();
    assert_eq!(standard_policies().len(), 4);
    assert_eq!(TriggerType::all().len(), 20);
}

#[test]
fn zkp_mock_system_usable() {
    let _ps = MockProofSystem;
    let digest = sha256_digest(&CanonicalBytes::new(&json!({"test": "cdb"})).unwrap());
    let _cdb = Cdb::new(digest);
}
