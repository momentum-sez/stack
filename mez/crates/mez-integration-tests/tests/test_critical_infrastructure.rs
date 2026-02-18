//! # Critical Infrastructure Components Test
//!
//! Tests critical infrastructure invariants: all 20 compliance domains exist
//! and are unique, tensor commitment is deterministic, domain count matches
//! the specification, and float rejection in canonical serialization.

use mez_core::{CanonicalBytes, ComplianceDomain, JurisdictionId};
use mez_tensor::{commitment_digest, ComplianceState, ComplianceTensor, DefaultJurisdiction};

fn test_jurisdiction() -> DefaultJurisdiction {
    DefaultJurisdiction::new(JurisdictionId::new("PK-RSEZ").unwrap())
}

// ---------------------------------------------------------------------------
// 1. All 20 compliance domains exist and are unique
// ---------------------------------------------------------------------------

#[test]
fn all_20_compliance_domains_exist() {
    let domains = ComplianceDomain::all();
    assert_eq!(domains.len(), 20);

    // All unique
    let unique: std::collections::HashSet<_> = domains.iter().collect();
    assert_eq!(unique.len(), 20);

    // Specific domains from the spec
    let domain_names: Vec<&str> = domains.iter().map(|d| d.as_str()).collect();
    assert!(domain_names.contains(&"aml"));
    assert!(domain_names.contains(&"kyc"));
    assert!(domain_names.contains(&"sanctions"));
    assert!(domain_names.contains(&"tax"));
    assert!(domain_names.contains(&"securities"));
    assert!(domain_names.contains(&"corporate"));
    assert!(domain_names.contains(&"custody"));
    assert!(domain_names.contains(&"data_privacy"));
    assert!(domain_names.contains(&"licensing"));
    assert!(domain_names.contains(&"banking"));
    assert!(domain_names.contains(&"payments"));
    assert!(domain_names.contains(&"clearing"));
    assert!(domain_names.contains(&"settlement"));
    assert!(domain_names.contains(&"digital_assets"));
    assert!(domain_names.contains(&"employment"));
    assert!(domain_names.contains(&"immigration"));
    assert!(domain_names.contains(&"ip"));
    assert!(domain_names.contains(&"consumer_protection"));
    assert!(domain_names.contains(&"arbitration"));
    assert!(domain_names.contains(&"trade"));
}

// ---------------------------------------------------------------------------
// 2. Tensor commitment is deterministic
// ---------------------------------------------------------------------------

#[test]
fn tensor_commitment_deterministic() {
    let mut tensor = ComplianceTensor::new(test_jurisdiction());
    tensor.set(
        ComplianceDomain::Aml,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    tensor.set(
        ComplianceDomain::Kyc,
        ComplianceState::Pending,
        vec![],
        None,
    );

    let c1 = tensor.commit().unwrap();
    let c2 = tensor.commit().unwrap();
    assert_eq!(c1.to_hex(), c2.to_hex());
    assert_eq!(c1.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// 3. Domain count constant matches actual count
// ---------------------------------------------------------------------------

#[test]
fn domain_count_is_20() {
    assert_eq!(ComplianceDomain::COUNT, 20);
    assert_eq!(ComplianceDomain::all().len(), ComplianceDomain::COUNT);

    // Tensor initialized with all domains
    let tensor = ComplianceTensor::new(test_jurisdiction());
    assert_eq!(tensor.cell_count(), 20);
}

// ---------------------------------------------------------------------------
// 4. Float rejection in canonical serialization
// ---------------------------------------------------------------------------

#[test]
fn canonical_bytes_float_rejection() {
    // Floats must be rejected to prevent canonicalization divergence
    let float_data = serde_json::json!({"amount": 3.15});
    assert!(CanonicalBytes::new(&float_data).is_err());

    // Nested floats also rejected
    let nested_float = serde_json::json!({"outer": {"inner": 2.719}});
    assert!(CanonicalBytes::new(&nested_float).is_err());

    // Integer amounts work fine
    let int_data = serde_json::json!({"amount": 314});
    assert!(CanonicalBytes::new(&int_data).is_ok());

    // String representations of numbers work fine
    let str_data = serde_json::json!({"amount": "3.14"});
    assert!(CanonicalBytes::new(&str_data).is_ok());
}

// ---------------------------------------------------------------------------
// 5. Commitment digest standalone function
// ---------------------------------------------------------------------------

#[test]
fn commitment_digest_standalone() {
    let states: Vec<_> = ComplianceDomain::all()
        .iter()
        .map(|&d| (d, ComplianceState::Pending))
        .collect();

    let d1 = commitment_digest("PK-RSEZ", &states).unwrap();
    let d2 = commitment_digest("PK-RSEZ", &states).unwrap();
    assert_eq!(d1, d2);
    assert_eq!(d1.to_hex().len(), 64);

    // Different jurisdiction produces different digest
    let d3 = commitment_digest("AE-DIFC", &states).unwrap();
    assert_ne!(d1, d3);
}

// ---------------------------------------------------------------------------
// 6. Domain string roundtrip
// ---------------------------------------------------------------------------

#[test]
fn domain_string_roundtrip() {
    for domain in ComplianceDomain::all() {
        let s = domain.as_str();
        let parsed: ComplianceDomain = s.parse().unwrap();
        assert_eq!(*domain, parsed);
    }
}
