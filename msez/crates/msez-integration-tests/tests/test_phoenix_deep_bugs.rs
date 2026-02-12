//! Deep regression tests for subtle phoenix issues.
//!
//! Covers tensor commitment sensitivity to domain state changes,
//! migration deadline boundary conditions, Unicode handling in
//! canonicalization, and digest collision resistance.

use msez_core::{CanonicalBytes, sha256_digest, ComplianceDomain, JurisdictionId, MigrationId};
use msez_tensor::{ComplianceTensor, DefaultJurisdiction, ComplianceState, TensorCommitment};
use msez_state::{MigrationBuilder, MigrationState};
use chrono::{Utc, Duration};
use serde_json::json;

// ---------------------------------------------------------------------------
// Tensor commitment sensitivity
// ---------------------------------------------------------------------------

#[test]
fn tensor_commitment_changes_with_domain_state() {
    // Changing a single domain's compliance state must change the
    // tensor commitment digest.
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let config = DefaultJurisdiction::new(jid);
    let mut tensor = ComplianceTensor::new(config);

    let commitment_before = TensorCommitment::compute(&tensor).unwrap();

    tensor.set(ComplianceDomain::Aml, ComplianceState::Compliant, vec![], None);

    let commitment_after = TensorCommitment::compute(&tensor).unwrap();

    assert_ne!(
        commitment_before.digest().to_hex(),
        commitment_after.digest().to_hex(),
        "Changing a domain state must change the tensor commitment"
    );
}

#[test]
fn tensor_commitment_sensitive_to_each_domain() {
    // Each domain change must produce a unique commitment.
    let jid = JurisdictionId::new("AE-DIFC").unwrap();
    let config = DefaultJurisdiction::new(jid);
    let base_tensor = ComplianceTensor::new(config.clone());
    let base_commitment = TensorCommitment::compute(&base_tensor).unwrap();

    let mut digests = std::collections::HashSet::new();
    digests.insert(base_commitment.digest().to_hex());

    for domain in ComplianceDomain::all() {
        let mut tensor = ComplianceTensor::new(config.clone());
        tensor.set(*domain, ComplianceState::Compliant, vec![], None);
        let commitment = TensorCommitment::compute(&tensor).unwrap();
        digests.insert(commitment.digest().to_hex());
    }

    // base + 20 domains = 21 unique commitments
    assert_eq!(
        digests.len(),
        ComplianceDomain::COUNT + 1,
        "Each domain state change must produce a unique commitment"
    );
}

// ---------------------------------------------------------------------------
// Migration deadline boundary
// ---------------------------------------------------------------------------

#[test]
fn migration_deadline_boundary_exact() {
    // A migration with a future deadline should advance normally.
    let id = MigrationId::new();
    let deadline = Utc::now() + Duration::hours(24);
    let jid_src = JurisdictionId::new("PK-RSEZ").unwrap();
    let jid_dst = JurisdictionId::new("AE-DIFC").unwrap();

    let mut saga = MigrationBuilder::new(id)
        .deadline(deadline)
        .source(jid_src)
        .destination(jid_dst)
        .build();

    assert_eq!(saga.state, MigrationState::Initiated);

    // Advance should succeed with future deadline.
    let next = saga.advance().unwrap();
    assert_eq!(next, MigrationState::ComplianceCheck);
}

#[test]
fn migration_expired_deadline_triggers_timeout() {
    // A migration with an expired deadline must transition to TimedOut.
    let id = MigrationId::new();
    let deadline = Utc::now() - Duration::hours(1);
    let jid_src = JurisdictionId::new("PK-RSEZ").unwrap();
    let jid_dst = JurisdictionId::new("AE-DIFC").unwrap();

    let mut saga = MigrationBuilder::new(id)
        .deadline(deadline)
        .source(jid_src)
        .destination(jid_dst)
        .build();

    let result = saga.advance();
    assert!(result.is_err(), "Expired deadline should cause timeout error");
    assert_eq!(
        saga.state,
        MigrationState::TimedOut,
        "State should transition to TimedOut"
    );
}

// ---------------------------------------------------------------------------
// Unicode handling
// ---------------------------------------------------------------------------

#[test]
fn canonical_bytes_unicode_handling() {
    // Unicode strings must be handled consistently in canonicalization.
    let data = json!({
        "name": "\u{00E9}\u{00E8}\u{00EA}",
        "city": "\u{5317}\u{4EAC}",
        "symbol": "\u{20AC}"
    });

    let c1 = CanonicalBytes::new(&data).unwrap();
    let c2 = CanonicalBytes::new(&data).unwrap();

    assert_eq!(
        sha256_digest(&c1).to_hex(),
        sha256_digest(&c2).to_hex(),
        "Unicode strings must canonicalize deterministically"
    );
}

#[test]
fn canonical_bytes_emoji_handling() {
    let data = json!({"emoji": "\u{1F600}\u{1F4A9}"});
    let c1 = CanonicalBytes::new(&data).unwrap();
    let c2 = CanonicalBytes::new(&data).unwrap();
    assert_eq!(sha256_digest(&c1).to_hex(), sha256_digest(&c2).to_hex());
}

// ---------------------------------------------------------------------------
// Digest collision resistance
// ---------------------------------------------------------------------------

#[test]
fn digest_collision_resistance() {
    // Similar but distinct inputs must produce distinct digests.
    let inputs: Vec<serde_json::Value> = (0..100)
        .map(|i| json!({"index": i, "data": format!("item-{}", i)}))
        .collect();

    let mut digests = std::collections::HashSet::new();
    for input in &inputs {
        let canonical = CanonicalBytes::new(input).unwrap();
        let digest = sha256_digest(&canonical).to_hex();
        assert!(
            digests.insert(digest),
            "Digest collision detected for input: {}",
            input
        );
    }

    assert_eq!(digests.len(), 100, "All 100 inputs must produce unique digests");
}
