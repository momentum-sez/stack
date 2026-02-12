//! # Compliance Attestation Circuits
//!
//! Circuit definitions for proving regulatory compliance properties without
//! revealing sensitive entity or transaction data.
//!
//! ## Circuit Types
//!
//! - [`BalanceSufficiencyCircuit`]: Proves `balance >= threshold` without
//!   revealing the actual balance.
//! - [`SanctionsClearanceCircuit`]: Proves an entity is NOT on a sanctions
//!   list via Merkle non-membership proof.
//! - [`TensorInclusionCircuit`]: Proves a specific compliance state exists
//!   at a given coordinate in the compliance tensor.
//!
//! ## Phase 1 Status
//!
//! Data model only — no real constraint system. The structs define the
//! public inputs and witness fields that a real circuit would use.
//!
//! ## Spec Reference
//!
//! Python equivalent: `tools/phoenix/zkp.py` circuit builders
//! (`build_balance_sufficiency_circuit`, `build_sanctions_clearance_circuit`,
//! `build_compliance_tensor_inclusion_circuit`).

use serde::{Deserialize, Serialize};

/// Circuit proving `balance >= threshold` without revealing the actual balance.
///
/// Public inputs:
/// - `threshold`: The minimum balance required (public or private depending
///   on `threshold_public` flag).
/// - `result_commitment`: SHA-256 commitment to the comparison result.
///
/// Witness (private):
/// - `balance`: The actual balance value.
///
/// Approximate constraint count: 256 (range check).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSufficiencyCircuit {
    // -- Public inputs --
    /// Minimum balance threshold. Public when `threshold_public` is true.
    pub threshold: u64,
    /// Whether the threshold is a public input (visible to verifier).
    pub threshold_public: bool,
    /// SHA-256 commitment to the comparison result.
    pub result_commitment: [u8; 32],

    // -- Witness (private inputs) --
    /// The actual balance. Never revealed in the proof.
    pub balance: u64,
}

/// Circuit proving entity is NOT on a sanctions list via Merkle non-membership.
///
/// Public inputs:
/// - `sanctions_root`: Root hash of the sanctions Merkle tree.
/// - `verification_timestamp`: Timestamp of the sanctions list snapshot.
///
/// Witness (private):
/// - `entity_hash`: Hash of the entity being checked.
/// - `merkle_proof`: Non-membership proof path.
/// - `merkle_path_indices`: Path direction indicators (left/right).
///
/// Approximate constraint count: 2048 (Merkle path verification).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsClearanceCircuit {
    // -- Public inputs --
    /// Root hash of the sanctions Merkle tree.
    pub sanctions_root: [u8; 32],
    /// Timestamp of the sanctions list snapshot (UTC epoch seconds).
    pub verification_timestamp: u64,

    // -- Witness (private inputs) --
    /// Hash of the entity being verified.
    pub entity_hash: [u8; 32],
    /// Merkle non-membership proof nodes.
    pub merkle_proof: Vec<[u8; 32]>,
    /// Path direction indicators: `false` = left, `true` = right.
    pub merkle_path_indices: Vec<bool>,
}

/// Circuit proving a specific compliance state at a tensor coordinate.
///
/// Public inputs:
/// - `tensor_commitment`: Commitment to the entire compliance tensor.
/// - `claimed_state`: The compliance state claimed at this coordinate.
///
/// Witness (private):
/// - `asset_id`: Identifier of the asset.
/// - `jurisdiction_id`: Jurisdiction where compliance is asserted.
/// - `domain`: Compliance domain (AML, KYC, TAX, etc.).
/// - `time_quantum`: Time period for the compliance assertion.
/// - `merkle_proof`: Inclusion proof within the tensor commitment.
///
/// Approximate constraint count: 2048 (Merkle inclusion + coordinate binding).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorInclusionCircuit {
    // -- Public inputs --
    /// Commitment to the full compliance tensor.
    pub tensor_commitment: [u8; 32],
    /// The compliance state claimed at the target coordinate.
    pub claimed_state: u8,

    // -- Witness (private inputs) --
    /// Asset identifier (hashed in real circuit).
    pub asset_id: String,
    /// Jurisdiction identifier.
    pub jurisdiction_id: String,
    /// Compliance domain ordinal (maps to `ComplianceDomain` enum).
    pub domain: u8,
    /// Time quantum index.
    pub time_quantum: u64,
    /// Merkle inclusion proof nodes.
    pub merkle_proof: Vec<[u8; 32]>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_sufficiency_circuit_construction() {
        let circuit = BalanceSufficiencyCircuit {
            threshold: 1000,
            threshold_public: true,
            result_commitment: [0u8; 32],
            balance: 5000,
        };
        assert!(circuit.balance >= circuit.threshold);
    }

    #[test]
    fn sanctions_clearance_circuit_construction() {
        let circuit = SanctionsClearanceCircuit {
            sanctions_root: [0xab; 32],
            verification_timestamp: 1738281600,
            entity_hash: [0xcd; 32],
            merkle_proof: vec![[0x01; 32], [0x02; 32]],
            merkle_path_indices: vec![false, true],
        };
        assert_eq!(circuit.merkle_proof.len(), circuit.merkle_path_indices.len());
    }

    #[test]
    fn tensor_inclusion_circuit_construction() {
        let circuit = TensorInclusionCircuit {
            tensor_commitment: [0xff; 32],
            claimed_state: 1,
            asset_id: "SA-PK-001".to_string(),
            jurisdiction_id: "PK".to_string(),
            domain: 0, // AML
            time_quantum: 2026_01,
            merkle_proof: vec![[0x10; 32]],
        };
        assert_eq!(circuit.jurisdiction_id, "PK");
    }

    #[test]
    fn balance_sufficiency_serialization_roundtrip() {
        let circuit = BalanceSufficiencyCircuit {
            threshold: 500,
            threshold_public: false,
            result_commitment: [0xaa; 32],
            balance: 1000,
        };
        let json = serde_json::to_string(&circuit).unwrap();
        let deserialized: BalanceSufficiencyCircuit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.threshold, 500);
        assert_eq!(deserialized.balance, 1000);
    }
}
