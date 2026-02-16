//! # Migration Evidence Circuits
//!
//! Circuit definitions for proving properties of cross-jurisdiction asset
//! migration without revealing sensitive migration details.
//!
//! ## Circuit Types
//!
//! - [`MigrationEvidenceCircuit`]: Proves that a migration saga completed
//!   all required phases with valid evidence at each transition.
//! - [`OwnershipChainCircuit`]: Proves valid chain of title across
//!   jurisdictional boundaries.
//! - [`CompensationValidityCircuit`]: Proves that compensation actions
//!   in a failed migration were correctly executed.
//!
//! ## Phase 1 Status
//!
//! Data model only — no real constraint system.
//!
//! ## Spec Reference
//!
//! Migration saga phases are defined in the spec's migration chapter.
//! Python equivalent: `tools/phoenix/migration.py` state machine.

use serde::{Deserialize, Serialize};

/// Circuit proving a migration saga completed all required phases.
///
/// Public inputs:
/// - `source_jurisdiction`: Source jurisdiction identifier hash.
/// - `target_jurisdiction`: Target jurisdiction identifier hash.
/// - `migration_id`: Unique migration identifier hash.
/// - `final_state_commitment`: Commitment to the final saga state.
///
/// Witness (private):
/// - `phase_evidence`: Evidence hashes for each completed phase.
/// - `transition_timestamps`: UTC timestamps of each phase transition.
/// - `approval_signatures`: Regulatory approval signature data.
///
/// Approximate constraint count: 4096 (multi-phase verification chain).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationEvidenceCircuit {
    // -- Public inputs --
    /// SHA-256 hash of the source jurisdiction identifier.
    pub source_jurisdiction: [u8; 32],
    /// SHA-256 hash of the target jurisdiction identifier.
    pub target_jurisdiction: [u8; 32],
    /// SHA-256 hash of the migration identifier.
    pub migration_id: [u8; 32],
    /// Commitment to the final saga state after all phases.
    pub final_state_commitment: [u8; 32],

    // -- Witness (private inputs) --
    /// Evidence hash for each completed migration phase.
    /// Ordered: INITIATED, VALIDATED, APPROVED, LOCKED, IN_TRANSIT,
    /// RECEIVED, VERIFIED, COMPLETED.
    pub phase_evidence: Vec<[u8; 32]>,
    /// UTC epoch seconds for each phase transition.
    pub transition_timestamps: Vec<u64>,
    /// Regulatory approval signature bytes for each required approval.
    pub approval_signatures: Vec<Vec<u8>>,
}

/// Circuit proving valid chain of title across jurisdictional boundaries.
///
/// Public inputs:
/// - `asset_digest`: Content-addressed digest of the asset.
/// - `current_owner_commitment`: Commitment to the current owner.
/// - `chain_root`: Merkle root of the ownership chain.
///
/// Witness (private):
/// - `ownership_entries`: Sequence of (owner_hash, timestamp, evidence_hash).
/// - `transfer_proofs`: Merkle proofs for each ownership transfer.
///
/// Approximate constraint count: 2048 per chain link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnershipChainCircuit {
    // -- Public inputs --
    /// Content-addressed digest of the asset being tracked.
    pub asset_digest: [u8; 32],
    /// Commitment to the current owner identity.
    pub current_owner_commitment: [u8; 32],
    /// Merkle root of the complete ownership chain.
    pub chain_root: [u8; 32],

    // -- Witness (private inputs) --
    /// Ownership chain entries: (owner_hash, timestamp_epoch, evidence_hash).
    pub ownership_entries: Vec<OwnershipEntry>,
    /// Merkle inclusion proofs for each transfer in the chain.
    pub transfer_proofs: Vec<Vec<[u8; 32]>>,
}

/// A single entry in the ownership chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnershipEntry {
    /// SHA-256 hash of the owner identity.
    pub owner_hash: [u8; 32],
    /// UTC epoch seconds when ownership was acquired.
    pub timestamp: u64,
    /// SHA-256 hash of the transfer evidence.
    pub evidence_hash: [u8; 32],
}

/// Circuit proving compensation actions in a failed migration were valid.
///
/// Public inputs:
/// - `migration_id`: Hash of the failed migration.
/// - `compensation_commitment`: Commitment to all compensation records.
///
/// Witness (private):
/// - `compensation_records`: Details of each compensation action.
/// - `failure_evidence`: Evidence of the migration failure trigger.
///
/// Approximate constraint count: 1024 per compensation action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompensationValidityCircuit {
    // -- Public inputs --
    /// SHA-256 hash of the failed migration identifier.
    pub migration_id: [u8; 32],
    /// Commitment to all compensation records.
    pub compensation_commitment: [u8; 32],

    // -- Witness (private inputs) --
    /// Compensation action records.
    pub compensation_records: Vec<CompensationRecord>,
    /// Evidence hash of the failure that triggered compensation.
    pub failure_evidence: [u8; 32],
}

/// A single compensation action record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompensationRecord {
    /// Type of compensation action (e.g., "unlock_source", "refund_fees").
    pub action_type: String,
    /// Whether the compensation action succeeded.
    pub success: bool,
    /// SHA-256 hash of the compensation evidence.
    pub evidence_hash: [u8; 32],
    /// UTC epoch seconds when the action was executed.
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_evidence_circuit_construction() {
        let circuit = MigrationEvidenceCircuit {
            source_jurisdiction: [0x01; 32],
            target_jurisdiction: [0x02; 32],
            migration_id: [0x03; 32],
            final_state_commitment: [0x04; 32],
            phase_evidence: vec![[0x10; 32], [0x20; 32], [0x30; 32]],
            transition_timestamps: vec![1000, 2000, 3000],
            approval_signatures: vec![vec![0xaa; 64]],
        };
        assert_eq!(circuit.phase_evidence.len(), 3);
        assert_eq!(circuit.transition_timestamps.len(), 3);
    }

    #[test]
    fn ownership_chain_circuit_construction() {
        let circuit = OwnershipChainCircuit {
            asset_digest: [0xab; 32],
            current_owner_commitment: [0xcd; 32],
            chain_root: [0xef; 32],
            ownership_entries: vec![OwnershipEntry {
                owner_hash: [0x11; 32],
                timestamp: 1738281600,
                evidence_hash: [0x22; 32],
            }],
            transfer_proofs: vec![vec![[0x33; 32]]],
        };
        assert_eq!(circuit.ownership_entries.len(), 1);
    }

    #[test]
    fn compensation_validity_circuit_construction() {
        let circuit = CompensationValidityCircuit {
            migration_id: [0xaa; 32],
            compensation_commitment: [0xbb; 32],
            compensation_records: vec![CompensationRecord {
                action_type: "unlock_source".to_string(),
                success: true,
                evidence_hash: [0xcc; 32],
                timestamp: 1738281600,
            }],
            failure_evidence: [0xdd; 32],
        };
        assert!(circuit.compensation_records[0].success);
    }

    // ── MigrationEvidenceCircuit comprehensive tests ────────────

    #[test]
    fn migration_evidence_circuit_full_8_phase() {
        let circuit = MigrationEvidenceCircuit {
            source_jurisdiction: [0x01; 32],
            target_jurisdiction: [0x02; 32],
            migration_id: [0x03; 32],
            final_state_commitment: [0x04; 32],
            phase_evidence: vec![
                [0x10; 32], [0x20; 32], [0x30; 32], [0x40; 32], [0x50; 32], [0x60; 32], [0x70; 32],
                [0x80; 32],
            ],
            transition_timestamps: vec![1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000],
            approval_signatures: vec![vec![0xaa; 64], vec![0xbb; 64]],
        };
        assert_eq!(
            circuit.phase_evidence.len(),
            8,
            "All 8 phases must have evidence"
        );
        assert_eq!(circuit.transition_timestamps.len(), 8);
        assert_eq!(circuit.approval_signatures.len(), 2);
    }

    #[test]
    fn migration_evidence_circuit_serialization_roundtrip() {
        let circuit = MigrationEvidenceCircuit {
            source_jurisdiction: [0x01; 32],
            target_jurisdiction: [0x02; 32],
            migration_id: [0x03; 32],
            final_state_commitment: [0x04; 32],
            phase_evidence: vec![[0x10; 32]],
            transition_timestamps: vec![1738281600],
            approval_signatures: vec![vec![0xaa; 64]],
        };
        let json = serde_json::to_string(&circuit).unwrap();
        let deserialized: MigrationEvidenceCircuit = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.source_jurisdiction,
            circuit.source_jurisdiction
        );
        assert_eq!(
            deserialized.target_jurisdiction,
            circuit.target_jurisdiction
        );
        assert_eq!(deserialized.migration_id, circuit.migration_id);
    }

    #[test]
    fn migration_evidence_circuit_empty_phases() {
        let circuit = MigrationEvidenceCircuit {
            source_jurisdiction: [0x01; 32],
            target_jurisdiction: [0x02; 32],
            migration_id: [0x03; 32],
            final_state_commitment: [0x04; 32],
            phase_evidence: vec![],
            transition_timestamps: vec![],
            approval_signatures: vec![],
        };
        assert_eq!(circuit.phase_evidence.len(), 0);
    }

    #[test]
    fn migration_evidence_timestamps_monotonic() {
        let timestamps = vec![1000u64, 2000, 3000, 4000];
        let circuit = MigrationEvidenceCircuit {
            source_jurisdiction: [0x01; 32],
            target_jurisdiction: [0x02; 32],
            migration_id: [0x03; 32],
            final_state_commitment: [0x04; 32],
            phase_evidence: vec![[0; 32]; 4],
            transition_timestamps: timestamps.clone(),
            approval_signatures: vec![],
        };
        for i in 1..circuit.transition_timestamps.len() {
            assert!(
                circuit.transition_timestamps[i] > circuit.transition_timestamps[i - 1],
                "Timestamps must be monotonically increasing"
            );
        }
    }

    // ── OwnershipChainCircuit comprehensive tests ───────────────

    #[test]
    fn ownership_chain_circuit_multi_level() {
        let entries: Vec<OwnershipEntry> = (0..5)
            .map(|i| OwnershipEntry {
                owner_hash: [i as u8; 32],
                timestamp: 1738281600 + (i as u64 * 86400),
                evidence_hash: [(i + 0x10) as u8; 32],
            })
            .collect();
        let proofs: Vec<Vec<[u8; 32]>> = (0..5).map(|_| vec![[0x33; 32], [0x44; 32]]).collect();
        let circuit = OwnershipChainCircuit {
            asset_digest: [0xab; 32],
            current_owner_commitment: [0xcd; 32],
            chain_root: [0xef; 32],
            ownership_entries: entries,
            transfer_proofs: proofs,
        };
        assert_eq!(circuit.ownership_entries.len(), 5);
        assert_eq!(circuit.transfer_proofs.len(), 5);
    }

    #[test]
    fn ownership_chain_circuit_serialization_roundtrip() {
        let circuit = OwnershipChainCircuit {
            asset_digest: [0xab; 32],
            current_owner_commitment: [0xcd; 32],
            chain_root: [0xef; 32],
            ownership_entries: vec![OwnershipEntry {
                owner_hash: [0x11; 32],
                timestamp: 1738281600,
                evidence_hash: [0x22; 32],
            }],
            transfer_proofs: vec![vec![[0x33; 32]]],
        };
        let json = serde_json::to_string(&circuit).unwrap();
        let deserialized: OwnershipChainCircuit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.asset_digest, circuit.asset_digest);
        assert_eq!(deserialized.chain_root, circuit.chain_root);
    }

    #[test]
    fn ownership_entry_construction_and_serde() {
        let entry = OwnershipEntry {
            owner_hash: [0xaa; 32],
            timestamp: 1738281600,
            evidence_hash: [0xbb; 32],
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: OwnershipEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.owner_hash, entry.owner_hash);
        assert_eq!(deserialized.timestamp, entry.timestamp);
        assert_eq!(deserialized.evidence_hash, entry.evidence_hash);
    }

    // ── CompensationValidityCircuit comprehensive tests ─────────

    #[test]
    fn compensation_circuit_multiple_actions() {
        let records = vec![
            CompensationRecord {
                action_type: "unlock_source".to_string(),
                success: true,
                evidence_hash: [0x01; 32],
                timestamp: 1738281600,
            },
            CompensationRecord {
                action_type: "refund_fees".to_string(),
                success: true,
                evidence_hash: [0x02; 32],
                timestamp: 1738281601,
            },
            CompensationRecord {
                action_type: "notify_counterparties".to_string(),
                success: false,
                evidence_hash: [0x03; 32],
                timestamp: 1738281602,
            },
        ];
        let circuit = CompensationValidityCircuit {
            migration_id: [0xaa; 32],
            compensation_commitment: [0xbb; 32],
            compensation_records: records,
            failure_evidence: [0xdd; 32],
        };
        assert_eq!(circuit.compensation_records.len(), 3);
        assert!(circuit.compensation_records[0].success);
        assert!(!circuit.compensation_records[2].success);
    }

    #[test]
    fn compensation_circuit_serialization_roundtrip() {
        let circuit = CompensationValidityCircuit {
            migration_id: [0xaa; 32],
            compensation_commitment: [0xbb; 32],
            compensation_records: vec![CompensationRecord {
                action_type: "unlock_source".to_string(),
                success: true,
                evidence_hash: [0xcc; 32],
                timestamp: 1738281600,
            }],
            failure_evidence: [0xdd; 32],
        };
        let json = serde_json::to_string(&circuit).unwrap();
        let deserialized: CompensationValidityCircuit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.migration_id, circuit.migration_id);
        assert_eq!(deserialized.compensation_records.len(), 1);
    }

    #[test]
    fn compensation_record_construction_and_serde() {
        let record = CompensationRecord {
            action_type: "refund_fees".to_string(),
            success: false,
            evidence_hash: [0xff; 32],
            timestamp: 1738281600,
        };
        let json = serde_json::to_string(&record).unwrap();
        let deserialized: CompensationRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.action_type, "refund_fees");
        assert!(!deserialized.success);
        assert_eq!(deserialized.timestamp, 1738281600);
    }

    #[test]
    fn compensation_record_all_action_types() {
        let action_types = [
            "unlock_source",
            "refund_fees",
            "notify_counterparties",
            "rollback_state",
            "release_escrow",
        ];
        for action in action_types {
            let record = CompensationRecord {
                action_type: action.to_string(),
                success: true,
                evidence_hash: [0x00; 32],
                timestamp: 1738281600,
            };
            assert_eq!(record.action_type, action);
        }
    }
}
