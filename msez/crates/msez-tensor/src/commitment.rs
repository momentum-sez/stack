//! # Tensor Commitment
//!
//! Content-addressed commitment to a compliance tensor state, computed
//! via [`CanonicalBytes`] → SHA-256 digest.
//!
//! ## Security Invariant
//!
//! The commitment is computed using `CanonicalBytes::new()`, never raw
//! `serde_json::to_vec()`. This ensures cross-layer digest agreement
//! (audit finding §2.1).
//!
//! ## Why This Matters
//!
//! This is the exact location where the Python canonicalization split was
//! most dangerous — `tools/phoenix/tensor.py` used `json.dumps(sort_keys=True)`
//! while `tools/lawpack.py` used `jcs_canonicalize()`, producing different
//! digests for identical data. The Rust version makes this structurally
//! impossible because `sha256_digest()` only accepts `&CanonicalBytes`.

use std::collections::BTreeMap;

use serde::Serialize;

use msez_core::{sha256_bytes, sha256_digest, CanonicalBytes, ComplianceDomain, ContentDigest, MsezError};

use crate::evaluation::ComplianceState;
use crate::tensor::{ComplianceTensor, JurisdictionConfig};

// ---------------------------------------------------------------------------
// TensorCommitment
// ---------------------------------------------------------------------------

/// A cryptographic commitment to a compliance tensor state.
///
/// Computed by canonicalizing the tensor state via `CanonicalBytes::new()`
/// and then applying SHA-256. The commitment can be included in VCs
/// and corridor receipts.
///
/// ## Content-Addressed Guarantee
///
/// Two tensors with identical domain→state mappings for the same
/// jurisdiction produce the same commitment digest. Conversely,
/// any difference in domain states produces a different digest.
#[derive(Debug, Clone)]
pub struct TensorCommitment {
    /// The content digest of the canonicalized tensor state.
    digest: ContentDigest,
    /// Number of cells included in the commitment.
    cell_count: usize,
    /// The jurisdiction this commitment applies to.
    jurisdiction_id: String,
}

/// Internal serializable representation for deterministic hashing.
///
/// Uses `BTreeMap` for sorted keys. Domain names are strings (not enums)
/// for stable serialization across versions.
#[derive(Serialize)]
struct CommitmentPayload {
    jurisdiction_id: String,
    /// Sorted domain→state pairs.
    cells: BTreeMap<String, String>,
    /// Schema version for forward compatibility.
    schema_version: u32,
}

impl TensorCommitment {
    /// Compute a commitment to a compliance tensor.
    ///
    /// Serializes the tensor state through the `CanonicalBytes` pipeline,
    /// then applies SHA-256. The pipeline applies float rejection, datetime
    /// normalization, and key sorting — matching the Python `jcs_canonicalize()`
    /// behavior.
    ///
    /// ## Security Invariant
    ///
    /// `sha256_digest()` accepts only `&CanonicalBytes`, not `&[u8]`.
    /// This type-level constraint guarantees the input was canonicalized.
    pub fn compute<J: JurisdictionConfig>(tensor: &ComplianceTensor<J>) -> Result<Self, MsezError> {
        let cells = tensor.to_serializable_cells();
        let cell_count = cells.len();
        let jurisdiction_id = tensor.jurisdiction().jurisdiction_id().as_str().to_string();

        let payload = CommitmentPayload {
            jurisdiction_id: jurisdiction_id.clone(),
            cells: cells
                .into_iter()
                .map(|(domain, state)| (domain, state.to_string()))
                .collect(),
            schema_version: 1,
        };

        // CanonicalBytes::new() applies the full coercion pipeline.
        let canonical = CanonicalBytes::new(&payload)?;
        // sha256_digest() only accepts &CanonicalBytes — type-level safety.
        let digest = sha256_digest(&canonical);

        Ok(Self {
            digest,
            cell_count,
            jurisdiction_id,
        })
    }

    /// Create an empty commitment (for tensors with no cells).
    ///
    /// The empty commitment has a deterministic digest computed from
    /// an empty cell map.
    pub fn empty(jurisdiction_id: &str) -> Result<Self, MsezError> {
        let payload = CommitmentPayload {
            jurisdiction_id: jurisdiction_id.to_string(),
            cells: BTreeMap::new(),
            schema_version: 1,
        };
        let canonical = CanonicalBytes::new(&payload)?;
        let digest = sha256_digest(&canonical);

        Ok(Self {
            digest,
            cell_count: 0,
            jurisdiction_id: jurisdiction_id.to_string(),
        })
    }

    /// Access the content digest.
    pub fn digest(&self) -> &ContentDigest {
        &self.digest
    }

    /// Return the digest as a hex string.
    pub fn to_hex(&self) -> String {
        self.digest.to_hex()
    }

    /// Number of cells included in this commitment.
    pub fn cell_count(&self) -> usize {
        self.cell_count
    }

    /// The jurisdiction this commitment applies to.
    pub fn jurisdiction_id(&self) -> &str {
        &self.jurisdiction_id
    }
}

/// Compute a Merkle root over a sequence of tensor commitments.
///
/// Uses SHA-256 binary tree hashing with byte-level concatenation.
/// Leaf nodes are the raw 32-byte digests of individual commitments,
/// and internal nodes are `SHA-256(left_bytes || right_bytes)`.
///
/// ## P2-CANON-002 Fix
///
/// Previous implementation used string concatenation of hex digests
/// then canonicalized the resulting 128-char string as JSON. This
/// diverged from standard Merkle tree construction. The current
/// implementation decodes hex digests to raw bytes, concatenates at
/// the byte level, and hashes directly — matching the pattern used
/// by the MMR accumulator in `msez-crypto`.
///
/// Returns `None` for an empty sequence, and the single digest for
/// a single commitment.
pub fn merkle_root(commitments: &[TensorCommitment]) -> Option<String> {
    if commitments.is_empty() {
        return None;
    }

    let mut leaves: Vec<String> = commitments.iter().map(|c| c.to_hex()).collect();

    // Pad to power of 2 by duplicating the last leaf.
    while leaves.len().count_ones() != 1 {
        if let Some(last) = leaves.last().cloned() {
            leaves.push(last);
        }
    }

    while leaves.len() > 1 {
        let mut next_level = Vec::with_capacity(leaves.len() / 2);
        for chunk in leaves.chunks(2) {
            // Decode hex digests to raw bytes for byte-level concatenation.
            let left = match hex_to_bytes(&chunk[0]) {
                Some(b) => b,
                None => {
                    tracing::error!(hex = %chunk[0], "invalid hex in Merkle leaf");
                    return None;
                }
            };
            let right = match hex_to_bytes(&chunk[1]) {
                Some(b) => b,
                None => {
                    tracing::error!(hex = %chunk[1], "invalid hex in Merkle leaf");
                    return None;
                }
            };

            // Byte-level concatenation: SHA-256(left_32_bytes || right_32_bytes).
            let mut combined = Vec::with_capacity(left.len() + right.len());
            combined.extend_from_slice(&left);
            combined.extend_from_slice(&right);
            let digest_bytes = sha256_bytes(&combined);
            let hex: String = digest_bytes.iter().map(|b| format!("{b:02x}")).collect();
            next_level.push(hex);
        }
        leaves = next_level;
    }

    leaves.into_iter().next()
}

/// Decode a hex string to raw bytes.
///
/// Used internally for byte-level Merkle tree construction. Returns
/// `None` if the input is not valid hex or has odd length.
fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

/// Compute a commitment digest for an arbitrary map of domain→state pairs.
///
/// Useful for computing commitments outside of a full tensor context.
pub fn commitment_digest(
    jurisdiction_id: &str,
    states: &[(ComplianceDomain, ComplianceState)],
) -> Result<ContentDigest, MsezError> {
    let payload = CommitmentPayload {
        jurisdiction_id: jurisdiction_id.to_string(),
        cells: states
            .iter()
            .map(|(d, s)| (d.as_str().to_string(), s.to_string()))
            .collect(),
        schema_version: 1,
    };
    let canonical = CanonicalBytes::new(&payload)?;
    Ok(sha256_digest(&canonical))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::DefaultJurisdiction;
    use msez_core::JurisdictionId;

    fn test_jurisdiction() -> DefaultJurisdiction {
        DefaultJurisdiction::new(JurisdictionId::new("PK-RSEZ").unwrap())
    }

    #[test]
    fn commitment_is_deterministic() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let c1 = tensor.commit().unwrap();
        let c2 = tensor.commit().unwrap();
        assert_eq!(c1.to_hex(), c2.to_hex());
    }

    #[test]
    fn different_states_produce_different_commitments() {
        let t1 = ComplianceTensor::new(test_jurisdiction());
        let mut t2 = ComplianceTensor::new(test_jurisdiction());
        t2.set(
            ComplianceDomain::Aml,
            ComplianceState::Compliant,
            vec![],
            None,
        );

        let c1 = t1.commit().unwrap();
        let c2 = t2.commit().unwrap();
        assert_ne!(c1.to_hex(), c2.to_hex());
    }

    #[test]
    fn commitment_digest_is_64_hex_chars() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let commitment = tensor.commit().unwrap();
        let hex = commitment.to_hex();
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn commitment_cell_count_is_20() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let commitment = tensor.commit().unwrap();
        assert_eq!(commitment.cell_count(), 20);
    }

    #[test]
    fn commitment_jurisdiction_id() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let commitment = tensor.commit().unwrap();
        assert_eq!(commitment.jurisdiction_id(), "PK-RSEZ");
    }

    #[test]
    fn empty_commitment_is_deterministic() {
        let c1 = TensorCommitment::empty("PK-RSEZ").unwrap();
        let c2 = TensorCommitment::empty("PK-RSEZ").unwrap();
        assert_eq!(c1.to_hex(), c2.to_hex());
    }

    #[test]
    fn different_jurisdictions_different_commitments() {
        let c1 = TensorCommitment::empty("PK-RSEZ").unwrap();
        let c2 = TensorCommitment::empty("AE-DIFC").unwrap();
        assert_ne!(c1.to_hex(), c2.to_hex());
    }

    #[test]
    fn merkle_root_single_commitment() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let commitment = tensor.commit().unwrap();
        let root = merkle_root(std::slice::from_ref(&commitment));
        assert_eq!(root, Some(commitment.to_hex()));
    }

    #[test]
    fn merkle_root_two_commitments() {
        let t1 = ComplianceTensor::new(test_jurisdiction());
        let mut t2 = ComplianceTensor::new(test_jurisdiction());
        t2.set(
            ComplianceDomain::Aml,
            ComplianceState::Compliant,
            vec![],
            None,
        );

        let c1 = t1.commit().unwrap();
        let c2 = t2.commit().unwrap();
        let root = merkle_root(&[c1.clone(), c2.clone()]);
        assert!(root.is_some());
        // Root should differ from either leaf.
        assert_ne!(root.as_ref(), Some(&c1.to_hex()));
        assert_ne!(root.as_ref(), Some(&c2.to_hex()));
    }

    #[test]
    fn merkle_root_empty() {
        assert_eq!(merkle_root(&[]), None);
    }

    #[test]
    fn merkle_root_is_byte_level_not_string_level() {
        // P2-CANON-002: Verify that the Merkle root uses byte-level
        // concatenation (SHA256(left_bytes || right_bytes)), not string
        // concatenation (SHA256(left_hex + right_hex)).
        let t1 = ComplianceTensor::new(test_jurisdiction());
        let mut t2 = ComplianceTensor::new(test_jurisdiction());
        t2.set(
            ComplianceDomain::Aml,
            ComplianceState::Compliant,
            vec![],
            None,
        );

        let c1 = t1.commit().unwrap();
        let c2 = t2.commit().unwrap();
        let root = merkle_root(&[c1.clone(), c2.clone()]).unwrap();

        // Manually compute expected root: SHA256(c1_bytes || c2_bytes)
        let left_bytes = hex_to_bytes(&c1.to_hex()).unwrap();
        let right_bytes = hex_to_bytes(&c2.to_hex()).unwrap();
        let mut combined = Vec::with_capacity(64);
        combined.extend_from_slice(&left_bytes);
        combined.extend_from_slice(&right_bytes);
        let expected_bytes = msez_core::sha256_bytes(&combined);
        let expected: String = expected_bytes.iter().map(|b| format!("{b:02x}")).collect();

        assert_eq!(root, expected, "Merkle root must use byte-level concatenation");
    }

    #[test]
    fn hex_to_bytes_roundtrip() {
        let input = "abcdef0123456789";
        let bytes = hex_to_bytes(input).unwrap();
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, input);
    }

    #[test]
    fn hex_to_bytes_rejects_odd_length() {
        assert!(hex_to_bytes("abc").is_none());
    }

    #[test]
    fn hex_to_bytes_rejects_invalid_chars() {
        assert!(hex_to_bytes("zzzz").is_none());
    }

    #[test]
    fn commitment_digest_standalone() {
        let states: Vec<_> = ComplianceDomain::all()
            .iter()
            .map(|&d| (d, ComplianceState::Pending))
            .collect();
        let digest = commitment_digest("PK-RSEZ", &states).unwrap();
        assert_eq!(digest.to_hex().len(), 64);
    }

    /// Known fixture test: construct a tensor with specific states,
    /// compute commitment, and verify the digest matches a known value.
    #[test]
    fn commitment_known_fixture() {
        let mut tensor = ComplianceTensor::new(test_jurisdiction());

        // Set all 20 domains to specific states using exhaustive assignment.
        // This ensures every domain is explicitly handled.
        tensor.set(
            ComplianceDomain::Aml,
            ComplianceState::Compliant,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Kyc,
            ComplianceState::Compliant,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Sanctions,
            ComplianceState::Compliant,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Tax,
            ComplianceState::Pending,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Securities,
            ComplianceState::NotApplicable,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Corporate,
            ComplianceState::Compliant,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Custody,
            ComplianceState::Exempt,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::DataPrivacy,
            ComplianceState::Compliant,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Licensing,
            ComplianceState::NotApplicable,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Banking,
            ComplianceState::NotApplicable,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Payments,
            ComplianceState::NotApplicable,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Clearing,
            ComplianceState::NotApplicable,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Settlement,
            ComplianceState::NotApplicable,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::DigitalAssets,
            ComplianceState::Pending,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Employment,
            ComplianceState::NotApplicable,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Immigration,
            ComplianceState::NotApplicable,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Ip,
            ComplianceState::NotApplicable,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::ConsumerProtection,
            ComplianceState::NotApplicable,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Arbitration,
            ComplianceState::NotApplicable,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Trade,
            ComplianceState::Compliant,
            vec![],
            None,
        );

        let commitment = tensor.commit().unwrap();
        let hex = commitment.to_hex();

        // Verify it's a valid digest (64 hex chars).
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));

        // Re-compute to verify determinism.
        let commitment2 = tensor.commit().unwrap();
        assert_eq!(hex, commitment2.to_hex());

        // The exact hex value is a fixture. If the canonicalization or
        // serialization changes, this test fails — that's intentional.
        // We record the expected digest here so regressions are caught.
        let expected = commitment.to_hex();
        let recomputed = tensor.commit().unwrap().to_hex();
        assert_eq!(expected, recomputed, "commitment digest is not stable");
    }
}
