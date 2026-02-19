//! # Akoma Schema Validation Tests
//!
//! Tests that Akoma Ntoso-style legislative descriptors and nested
//! structures produce deterministic canonical bytes and digests.
//! Verifies that legislative reference formats are handled correctly
//! by the canonicalization pipeline.

use mez_core::{sha256_digest, CanonicalBytes};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Akoma-style descriptor produces deterministic canonical bytes
// ---------------------------------------------------------------------------

#[test]
fn akoma_style_descriptor_canonical() {
    let descriptor = json!({
        "an:FRBRWork": {
            "an:FRBRthis": "/pk/act/2001/income-tax-ordinance",
            "an:FRBRuri": "/pk/act/2001/income-tax-ordinance",
            "an:FRBRdate": {
                "date": "2001-09-13",
                "name": "enactment"
            },
            "an:FRBRauthor": {
                "href": "#president-of-pakistan",
                "as": "#author"
            }
        },
        "an:FRBRExpression": {
            "an:FRBRlanguage": "eng",
            "an:FRBRdate": {
                "date": "2026-01-01",
                "name": "amendment"
            }
        }
    });

    let cb1 = CanonicalBytes::new(&descriptor).unwrap();
    let cb2 = CanonicalBytes::new(&descriptor).unwrap();
    assert_eq!(
        cb1.as_bytes(),
        cb2.as_bytes(),
        "Akoma descriptor canonical bytes must be deterministic"
    );

    // Keys should be sorted in the canonical output
    let s = std::str::from_utf8(cb1.as_bytes()).unwrap();
    let e_pos = s.find("an:FRBRExpression").unwrap();
    let w_pos = s.find("an:FRBRWork").unwrap();
    assert!(
        e_pos < w_pos,
        "keys must be sorted: FRBRExpression before FRBRWork"
    );
}

#[test]
fn akoma_descriptor_digest_deterministic() {
    let descriptor = json!({
        "an:FRBRWork": {
            "an:FRBRthis": "/pk/act/1990/sales-tax",
            "an:FRBRuri": "/pk/act/1990/sales-tax"
        }
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&descriptor).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&descriptor).unwrap());
    assert_eq!(d1, d2);
    assert_eq!(d1.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// 2. Akoma nested structure is deterministic
// ---------------------------------------------------------------------------

#[test]
fn akoma_nested_structure_deterministic() {
    let nested = json!({
        "body": {
            "chapter": [
                {
                    "num": "80",
                    "heading": "Withholding of Tax",
                    "section": [
                        {
                            "num": "80(1)",
                            "content": "Every prescribed person making a prescribed payment..."
                        },
                        {
                            "num": "80(2)",
                            "content": "The rate of deduction shall be as prescribed..."
                        }
                    ]
                },
                {
                    "num": "114",
                    "heading": "Return of Income",
                    "section": [
                        {
                            "num": "114(1)",
                            "content": "Every person who has taxable income..."
                        }
                    ]
                }
            ]
        },
        "meta": {
            "lifecycle": {
                "eventRef": "enactment",
                "source": "#parliament"
            }
        }
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&nested).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&nested).unwrap());
    assert_eq!(
        d1, d2,
        "nested Akoma structure digest must be deterministic"
    );

    // Verify nested key ordering
    let cb = CanonicalBytes::new(&nested).unwrap();
    let s = std::str::from_utf8(cb.as_bytes()).unwrap();
    let body_pos = s.find("\"body\"").unwrap();
    let meta_pos = s.find("\"meta\"").unwrap();
    assert!(
        body_pos < meta_pos,
        "body must come before meta in sorted keys"
    );
}

#[test]
fn akoma_nested_with_different_key_order() {
    // Same data, different insertion order
    let v1 = json!({
        "z_section": {"num": "1"},
        "a_chapter": {"heading": "First"}
    });
    let v2 = json!({
        "a_chapter": {"heading": "First"},
        "z_section": {"num": "1"}
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&v1).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&v2).unwrap());
    assert_eq!(d1, d2, "key insertion order must not affect digest");
}

// ---------------------------------------------------------------------------
// 3. Akoma legislative reference format
// ---------------------------------------------------------------------------

#[test]
fn akoma_legislative_reference_format() {
    // Test that legislative references with various URI formats canonicalize correctly
    let references = json!({
        "references": [
            {
                "href": "/pk/act/2001/income-tax-ordinance#section80",
                "showAs": "Income Tax Ordinance 2001, Section 80"
            },
            {
                "href": "/pk/act/1990/sales-tax#section3",
                "showAs": "Sales Tax Act 1990, Section 3"
            },
            {
                "href": "/pk/regulation/2026/ez-rules#rule15",
                "showAs": "EZ Rules 2026, Rule 15"
            }
        ],
        "jurisdiction": "PK-REZ",
        "effective_date": "2026-01-01T00:00:00Z"
    });

    let canonical = CanonicalBytes::new(&references).unwrap();

    // Must be valid UTF-8
    assert!(std::str::from_utf8(canonical.as_bytes()).is_ok());

    // Digest should be deterministic
    let d1 = sha256_digest(&canonical);
    let d2 = sha256_digest(&CanonicalBytes::new(&references).unwrap());
    assert_eq!(d1, d2);
    assert_eq!(d1.to_hex().len(), 64);
}

#[test]
fn akoma_references_with_special_characters() {
    let ref_with_special = json!({
        "title": "Companies Act \u{00A7} 233",
        "uri": "/pk/act/2017/companies#section233",
        "notes": "Cross-reference: Ordinance LXIII of 2001"
    });

    let cb = CanonicalBytes::new(&ref_with_special).unwrap();
    assert!(std::str::from_utf8(cb.as_bytes()).is_ok());
    let digest = sha256_digest(&cb);
    assert_eq!(digest.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// 4. Different legislative references produce different digests
// ---------------------------------------------------------------------------

#[test]
fn different_references_produce_different_digests() {
    let ref1 = json!({"href": "/pk/act/2001/income-tax"});
    let ref2 = json!({"href": "/pk/act/1990/sales-tax"});

    let d1 = sha256_digest(&CanonicalBytes::new(&ref1).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&ref2).unwrap());
    assert_ne!(
        d1, d2,
        "different references must produce different digests"
    );
}
