//! Tests for regpack operations.
//!
//! Validates regpack struct construction, digest computation via canonical
//! bytes, domain mapping, and sanctions checking.

use mez_core::{sha256_digest, CanonicalBytes, ComplianceDomain, JurisdictionId};
use mez_pack::regpack::{Regpack, SanctionsChecker, SanctionsEntry};

// ---------------------------------------------------------------------------
// Regpack creation
// ---------------------------------------------------------------------------

#[test]
fn regpack_creation() {
    let jid = JurisdictionId::new("PK-REZ").unwrap();
    let regpack = Regpack {
        jurisdiction: jid.clone(),
        name: "PK-REZ Regulatory Pack".to_string(),
        version: "1.0".to_string(),
        digest: None,
        metadata: None,
    };

    assert_eq!(regpack.jurisdiction.as_str(), "PK-REZ");
    assert_eq!(regpack.name, "PK-REZ Regulatory Pack");
    assert_eq!(regpack.version, "1.0");
    assert!(regpack.digest.is_none());
}

// ---------------------------------------------------------------------------
// Regpack digest deterministic
// ---------------------------------------------------------------------------

#[test]
fn regpack_digest_deterministic() {
    let jid = JurisdictionId::new("PK-REZ").unwrap();

    let rp1 = Regpack {
        jurisdiction: jid.clone(),
        name: "Test Pack".to_string(),
        version: "1.0".to_string(),
        digest: None,
        metadata: None,
    };

    let rp2 = Regpack {
        jurisdiction: jid,
        name: "Test Pack".to_string(),
        version: "1.0".to_string(),
        digest: None,
        metadata: None,
    };

    let json1 = serde_json::to_value(&rp1).unwrap();
    let json2 = serde_json::to_value(&rp2).unwrap();

    let c1 = CanonicalBytes::new(&json1).unwrap();
    let c2 = CanonicalBytes::new(&json2).unwrap();

    assert_eq!(
        sha256_digest(&c1).to_hex(),
        sha256_digest(&c2).to_hex(),
        "Same-parameter regpacks must canonicalize identically"
    );
}

#[test]
fn regpack_different_jurisdictions_different_digest() {
    let rp_a = Regpack {
        jurisdiction: JurisdictionId::new("PK-REZ").unwrap(),
        name: "Test".to_string(),
        version: "1.0".to_string(),
        digest: None,
        metadata: None,
    };
    let rp_b = Regpack {
        jurisdiction: JurisdictionId::new("AE-DIFC").unwrap(),
        name: "Test".to_string(),
        version: "1.0".to_string(),
        digest: None,
        metadata: None,
    };

    let ja = serde_json::to_value(&rp_a).unwrap();
    let jb = serde_json::to_value(&rp_b).unwrap();
    let ca = CanonicalBytes::new(&ja).unwrap();
    let cb = CanonicalBytes::new(&jb).unwrap();

    assert_ne!(
        sha256_digest(&ca).to_hex(),
        sha256_digest(&cb).to_hex(),
        "Different jurisdictions must produce different digests"
    );
}

// ---------------------------------------------------------------------------
// Domain mapping validation
// ---------------------------------------------------------------------------

#[test]
fn regpack_domain_mapping_validation() {
    // ComplianceDomain has 20 variants.
    let all_domains = ComplianceDomain::all();
    assert_eq!(all_domains.len(), 20);

    // Build a domain name set.
    let domain_names: Vec<&str> = all_domains.iter().map(|d| d.as_str()).collect();

    // Verify that known domains exist.
    assert!(domain_names.contains(&"aml"));
    assert!(domain_names.contains(&"kyc"));
    assert!(domain_names.contains(&"sanctions"));
    assert!(domain_names.contains(&"tax"));
}

// ---------------------------------------------------------------------------
// Sanctions check
// ---------------------------------------------------------------------------

#[test]
fn regpack_sanctions_check() {
    let entry = SanctionsEntry {
        entry_id: "OFAC-001".to_string(),
        entry_type: "entity".to_string(),
        source_lists: vec!["OFAC SDN".to_string()],
        primary_name: "Evil Corp International".to_string(),
        aliases: vec![],
        identifiers: vec![],
        addresses: vec![],
        nationalities: vec![],
        date_of_birth: None,
        programs: vec!["IRAN".to_string()],
        listing_date: Some("2024-01-01".to_string()),
        remarks: None,
    };

    let checker = SanctionsChecker::new(vec![entry], "snapshot-2026-01".to_string());

    // Exact match must return a hit.
    let result = checker.check_entity("Evil Corp International", None, 0.7);
    assert!(result.matched, "Exact name match must be detected");
    assert!(!result.matches.is_empty());
    assert_eq!(result.match_score, 1.0);

    // Non-matching name must not return a hit.
    let result2 = checker.check_entity("Good Corp Ltd", None, 0.7);
    assert!(!result2.matched, "Non-matching name must not be flagged");
}

#[test]
fn regpack_sanctions_fuzzy_match() {
    let entry = SanctionsEntry {
        entry_id: "UN-002".to_string(),
        entry_type: "individual".to_string(),
        source_lists: vec!["UN Consolidated".to_string()],
        primary_name: "John Smith Doe".to_string(),
        aliases: vec![],
        identifiers: vec![],
        addresses: vec![],
        nationalities: vec!["US".to_string()],
        date_of_birth: Some("1970-01-01".to_string()),
        programs: vec!["TERRORISM".to_string()],
        listing_date: None,
        remarks: None,
    };

    let checker = SanctionsChecker::new(vec![entry], "snapshot-2026-02".to_string());

    // Fuzzy match: partial name overlap.
    let result = checker.check_entity("John Smith", None, 0.5);
    assert!(
        result.match_score > 0.0,
        "Partial name should have non-zero match score"
    );
}
