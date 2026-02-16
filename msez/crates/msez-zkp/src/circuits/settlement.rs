//! # Settlement Proof Circuits
//!
//! Circuit definitions for proving payment and settlement properties
//! without revealing transaction amounts, counterparty details, or
//! settlement rail specifics.
//!
//! ## Circuit Types
//!
//! - [`RangeProofCircuit`]: Proves a value lies within a specified range
//!   (used for amount validation without disclosure).
//! - [`MerkleMembershipCircuit`]: Proves inclusion in a Merkle tree
//!   (used for receipt chain verification).
//! - [`NettingValidityCircuit`]: Proves that a multilateral netting
//!   computation is correct (net positions sum to zero).
//!
//! ## Phase 1 Status
//!
//! Data model only — no real constraint system.
//!
//! ## Spec Reference
//!
//! Settlement netting: `tools/netting.py`.
//! Receipt chain: `tools/phoenix/bridge.py` corridor receipts.
//! Python circuit refs: `tools/phoenix/zkp.py` CircuitType enum.

use serde::{Deserialize, Serialize};

/// Circuit proving a value lies within `[lower, upper]` without revealing it.
///
/// Public inputs:
/// - `lower_bound`: Minimum acceptable value.
/// - `upper_bound`: Maximum acceptable value.
/// - `value_commitment`: Pedersen-style commitment to the hidden value.
///
/// Witness (private):
/// - `value`: The actual value (must satisfy `lower <= value <= upper`).
/// - `blinding_factor`: Randomness used in the commitment scheme.
///
/// Approximate constraint count: 512 (bit decomposition + range check).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RangeProofCircuit {
    // -- Public inputs --
    /// Minimum acceptable value (inclusive).
    pub lower_bound: u64,
    /// Maximum acceptable value (inclusive).
    pub upper_bound: u64,
    /// Commitment to the hidden value (e.g., Pedersen commitment).
    pub value_commitment: [u8; 32],

    // -- Witness (private inputs) --
    /// The actual value being range-checked.
    pub value: u64,
    /// Blinding factor for the commitment scheme.
    pub blinding_factor: [u8; 32],
}

/// Circuit proving inclusion of an element in a Merkle tree.
///
/// Public inputs:
/// - `merkle_root`: Root hash of the Merkle tree.
/// - `leaf_hash`: Hash of the element whose membership is being proven.
///
/// Witness (private):
/// - `merkle_proof`: Sibling hashes along the path from leaf to root.
/// - `path_indices`: Direction indicators for each level of the tree.
///
/// Approximate constraint count: 256 * tree_depth (hash per level).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerkleMembershipCircuit {
    // -- Public inputs --
    /// Root hash of the Merkle tree.
    pub merkle_root: [u8; 32],
    /// Hash of the leaf element whose membership is asserted.
    pub leaf_hash: [u8; 32],

    // -- Witness (private inputs) --
    /// Sibling hashes along the authentication path.
    pub merkle_proof: Vec<[u8; 32]>,
    /// Path direction indicators: `false` = left child, `true` = right child.
    pub path_indices: Vec<bool>,
}

/// Circuit proving multilateral netting computation correctness.
///
/// Public inputs:
/// - `gross_positions_commitment`: Commitment to the set of gross positions.
/// - `net_positions_commitment`: Commitment to the computed net positions.
/// - `participant_count`: Number of participants in the netting set.
///
/// Witness (private):
/// - `gross_positions`: Individual gross position amounts per participant.
/// - `net_positions`: Computed net positions (must sum to zero across all
///   participants for a balanced netting).
/// - `netting_matrix`: The bilateral obligation matrix.
///
/// Approximate constraint count: O(n^2) where n = participant_count.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NettingValidityCircuit {
    // -- Public inputs --
    /// Commitment to the gross positions vector.
    pub gross_positions_commitment: [u8; 32],
    /// Commitment to the net positions vector.
    pub net_positions_commitment: [u8; 32],
    /// Number of participants in the netting computation.
    pub participant_count: u32,

    // -- Witness (private inputs) --
    /// Gross position amounts per participant (signed: positive = receivable,
    /// negative = payable). Stored as i64 for signed arithmetic.
    pub gross_positions: Vec<i64>,
    /// Computed net positions per participant. Must sum to zero.
    pub net_positions: Vec<i64>,
    /// Bilateral obligation matrix (flattened, row-major).
    /// Size: participant_count * participant_count.
    pub netting_matrix: Vec<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_proof_circuit_construction() {
        let circuit = RangeProofCircuit {
            lower_bound: 100,
            upper_bound: 10_000,
            value_commitment: [0xaa; 32],
            value: 5_000,
            blinding_factor: [0xbb; 32],
        };
        assert!(circuit.value >= circuit.lower_bound);
        assert!(circuit.value <= circuit.upper_bound);
    }

    #[test]
    fn merkle_membership_circuit_construction() {
        let circuit = MerkleMembershipCircuit {
            merkle_root: [0x11; 32],
            leaf_hash: [0x22; 32],
            merkle_proof: vec![[0x33; 32], [0x44; 32], [0x55; 32]],
            path_indices: vec![false, true, false],
        };
        assert_eq!(circuit.merkle_proof.len(), circuit.path_indices.len());
    }

    #[test]
    fn netting_validity_circuit_construction() {
        // 3 participants, net positions must sum to zero.
        let circuit = NettingValidityCircuit {
            gross_positions_commitment: [0xaa; 32],
            net_positions_commitment: [0xbb; 32],
            participant_count: 3,
            gross_positions: vec![1000, -500, -500],
            net_positions: vec![500, -200, -300],
            netting_matrix: vec![0, 300, 200, -300, 0, 100, -200, -100, 0],
        };
        let net_sum: i64 = circuit.net_positions.iter().sum();
        assert_eq!(net_sum, 0, "Net positions must sum to zero");
    }

    #[test]
    fn range_proof_serialization_roundtrip() {
        let circuit = RangeProofCircuit {
            lower_bound: 0,
            upper_bound: u64::MAX,
            value_commitment: [0xff; 32],
            value: 42,
            blinding_factor: [0x00; 32],
        };
        let json = serde_json::to_string(&circuit).unwrap();
        let deserialized: RangeProofCircuit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.value, 42);
    }

    // ── RangeProofCircuit comprehensive tests ───────────────────

    #[test]
    fn range_proof_value_at_lower_bound() {
        let circuit = RangeProofCircuit {
            lower_bound: 100,
            upper_bound: 10_000,
            value_commitment: [0xaa; 32],
            value: 100,
            blinding_factor: [0xbb; 32],
        };
        assert!(circuit.value >= circuit.lower_bound);
        assert!(circuit.value <= circuit.upper_bound);
    }

    #[test]
    fn range_proof_value_at_upper_bound() {
        let circuit = RangeProofCircuit {
            lower_bound: 100,
            upper_bound: 10_000,
            value_commitment: [0xaa; 32],
            value: 10_000,
            blinding_factor: [0xbb; 32],
        };
        assert!(circuit.value <= circuit.upper_bound);
    }

    #[test]
    fn range_proof_zero_range() {
        let circuit = RangeProofCircuit {
            lower_bound: 500,
            upper_bound: 500,
            value_commitment: [0xaa; 32],
            value: 500,
            blinding_factor: [0xbb; 32],
        };
        assert_eq!(circuit.lower_bound, circuit.upper_bound);
        assert_eq!(circuit.value, circuit.lower_bound);
    }

    // ── MerkleMembershipCircuit comprehensive tests ─────────────

    #[test]
    fn merkle_membership_serialization_roundtrip() {
        let circuit = MerkleMembershipCircuit {
            merkle_root: [0x11; 32],
            leaf_hash: [0x22; 32],
            merkle_proof: vec![[0x33; 32], [0x44; 32]],
            path_indices: vec![false, true],
        };
        let json = serde_json::to_string(&circuit).unwrap();
        let deserialized: MerkleMembershipCircuit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.merkle_root, [0x11; 32]);
        assert_eq!(deserialized.leaf_hash, [0x22; 32]);
        assert_eq!(deserialized.merkle_proof.len(), 2);
    }

    #[test]
    fn merkle_membership_deep_tree() {
        let depth = 20;
        let circuit = MerkleMembershipCircuit {
            merkle_root: [0x11; 32],
            leaf_hash: [0x22; 32],
            merkle_proof: vec![[0x33; 32]; depth],
            path_indices: vec![false; depth],
        };
        assert_eq!(circuit.merkle_proof.len(), depth);
        assert_eq!(circuit.path_indices.len(), depth);
    }

    #[test]
    fn merkle_membership_empty_proof() {
        let circuit = MerkleMembershipCircuit {
            merkle_root: [0x22; 32],
            leaf_hash: [0x22; 32],
            merkle_proof: vec![],
            path_indices: vec![],
        };
        assert_eq!(
            circuit.merkle_root, circuit.leaf_hash,
            "Single-element tree: root == leaf"
        );
    }

    // ── NettingValidityCircuit comprehensive tests ──────────────

    #[test]
    fn netting_validity_serialization_roundtrip() {
        let circuit = NettingValidityCircuit {
            gross_positions_commitment: [0xaa; 32],
            net_positions_commitment: [0xbb; 32],
            participant_count: 2,
            gross_positions: vec![100, -100],
            net_positions: vec![100, -100],
            netting_matrix: vec![0, 100, -100, 0],
        };
        let json = serde_json::to_string(&circuit).unwrap();
        let deserialized: NettingValidityCircuit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.participant_count, 2);
        let net_sum: i64 = deserialized.net_positions.iter().sum();
        assert_eq!(net_sum, 0);
    }

    #[test]
    fn netting_validity_large_participant_set() {
        let n = 10u32;
        let gross: Vec<i64> = (0..n as i64)
            .map(|i| if i % 2 == 0 { 1000 } else { -1000 })
            .collect();
        let net: Vec<i64> = gross.clone();
        let matrix = vec![0i64; (n * n) as usize];
        let circuit = NettingValidityCircuit {
            gross_positions_commitment: [0xaa; 32],
            net_positions_commitment: [0xbb; 32],
            participant_count: n,
            gross_positions: gross,
            net_positions: net,
            netting_matrix: matrix,
        };
        assert_eq!(circuit.gross_positions.len(), n as usize);
        assert_eq!(circuit.netting_matrix.len(), (n * n) as usize);
        let net_sum: i64 = circuit.net_positions.iter().sum();
        assert_eq!(net_sum, 0);
    }

    #[test]
    fn netting_matrix_is_square() {
        let n = 4u32;
        let circuit = NettingValidityCircuit {
            gross_positions_commitment: [0xaa; 32],
            net_positions_commitment: [0xbb; 32],
            participant_count: n,
            gross_positions: vec![0; n as usize],
            net_positions: vec![0; n as usize],
            netting_matrix: vec![0; (n * n) as usize],
        };
        assert_eq!(
            circuit.netting_matrix.len(),
            (circuit.participant_count * circuit.participant_count) as usize,
            "Netting matrix must be square"
        );
    }
}
