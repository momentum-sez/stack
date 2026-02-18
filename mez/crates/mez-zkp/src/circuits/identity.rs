//! # Identity Verification Circuits
//!
//! Circuit definitions for proving identity properties without revealing
//! sensitive personal data (KYC/KYB attestation verification).
//!
//! ## Circuit Types
//!
//! - [`KycAttestationCircuit`]: Proves a valid KYC attestation exists from
//!   an approved issuer without revealing attestation details.
//! - [`AttestationValidityCircuit`]: Proves an attestation (of any type)
//!   is currently valid (not expired, not revoked).
//! - [`ThresholdSignatureCircuit`]: Proves that a threshold number of
//!   authorized signers have endorsed a statement.
//!
//! ## Phase 1 Status
//!
//! Data model only — no real constraint system.
//!
//! ## Spec Reference
//!
//! Python equivalent: `tools/phoenix/zkp.py:build_kyc_attestation_circuit()`.
//! Identity primitive API: `apis/identity.openapi.yaml` (future).

use serde::{Deserialize, Serialize};

/// Circuit proving a valid KYC attestation from an approved issuer.
///
/// Public inputs:
/// - `approved_issuers_root`: Merkle root of the approved KYC issuers list.
/// - `min_kyc_level`: Minimum KYC level required (1=basic, 2=enhanced, 3=full).
/// - `verification_timestamp`: Timestamp of the verification request.
///
/// Witness (private):
/// - `attestation_hash`: Hash of the KYC attestation document.
/// - `issuer_signature`: Issuer's digital signature over the attestation.
/// - `issuer_pubkey`: Issuer's public key (verified against approved list).
/// - `kyc_level`: Actual KYC level in the attestation.
/// - `issuer_merkle_proof`: Proof that the issuer is in the approved list.
///
/// Approximate constraint count: 4096 (signature verification + Merkle proof).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycAttestationCircuit {
    // -- Public inputs --
    /// Merkle root of the approved KYC issuers list.
    pub approved_issuers_root: [u8; 32],
    /// Minimum required KYC verification level.
    pub min_kyc_level: u8,
    /// UTC epoch seconds of the verification request.
    pub verification_timestamp: u64,

    // -- Witness (private inputs) --
    /// SHA-256 hash of the KYC attestation document.
    pub attestation_hash: [u8; 32],
    /// Digital signature bytes from the issuer.
    pub issuer_signature: Vec<u8>,
    /// Issuer's public key bytes.
    pub issuer_pubkey: Vec<u8>,
    /// Actual KYC level granted (must be >= min_kyc_level).
    pub kyc_level: u8,
    /// Merkle inclusion proof for the issuer in the approved list.
    pub issuer_merkle_proof: Vec<[u8; 32]>,
}

/// Circuit proving an attestation is currently valid (not expired, not revoked).
///
/// Public inputs:
/// - `attestation_commitment`: Commitment to the attestation.
/// - `current_timestamp`: Current time for expiry checking.
/// - `revocation_root`: Root of the revocation accumulator.
///
/// Witness (private):
/// - `attestation_data`: The full attestation data.
/// - `expiry_timestamp`: When the attestation expires.
/// - `revocation_non_membership`: Proof of non-revocation.
///
/// Approximate constraint count: 2048 (timestamp comparison + accumulator check).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationValidityCircuit {
    // -- Public inputs --
    /// Commitment to the attestation being verified.
    pub attestation_commitment: [u8; 32],
    /// Current UTC epoch seconds for expiry comparison.
    pub current_timestamp: u64,
    /// Root of the revocation accumulator (e.g., Merkle tree of revoked IDs).
    pub revocation_root: [u8; 32],

    // -- Witness (private inputs) --
    /// SHA-256 hash of the full attestation data.
    pub attestation_hash: [u8; 32],
    /// UTC epoch seconds when the attestation expires.
    pub expiry_timestamp: u64,
    /// Non-membership proof in the revocation accumulator.
    pub revocation_non_membership: Vec<[u8; 32]>,
}

/// Circuit proving threshold multi-party signature endorsement.
///
/// Public inputs:
/// - `statement_hash`: Hash of the statement being endorsed.
/// - `threshold`: Minimum number of required signers.
/// - `authorized_signers_root`: Merkle root of authorized signers.
///
/// Witness (private):
/// - `signatures`: Individual signature bytes from each signer.
/// - `signer_pubkeys`: Public keys of the signers.
/// - `signer_merkle_proofs`: Merkle proofs that each signer is authorized.
///
/// Approximate constraint count: 4096 * threshold (one signature verification
/// per required signer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdSignatureCircuit {
    // -- Public inputs --
    /// SHA-256 hash of the statement being endorsed.
    pub statement_hash: [u8; 32],
    /// Minimum number of valid signatures required.
    pub threshold: u32,
    /// Merkle root of the authorized signers list.
    pub authorized_signers_root: [u8; 32],

    // -- Witness (private inputs) --
    /// Individual signature bytes from participating signers.
    pub signatures: Vec<Vec<u8>>,
    /// Public keys of the participating signers.
    pub signer_pubkeys: Vec<Vec<u8>>,
    /// Merkle inclusion proofs for each signer.
    pub signer_merkle_proofs: Vec<Vec<[u8; 32]>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kyc_attestation_circuit_construction() {
        let circuit = KycAttestationCircuit {
            approved_issuers_root: [0xaa; 32],
            min_kyc_level: 2,
            verification_timestamp: 1738281600,
            attestation_hash: [0xbb; 32],
            issuer_signature: vec![0xcc; 64],
            issuer_pubkey: vec![0xdd; 32],
            kyc_level: 3,
            issuer_merkle_proof: vec![[0xee; 32], [0xff; 32]],
        };
        assert!(circuit.kyc_level >= circuit.min_kyc_level);
    }

    #[test]
    fn attestation_validity_circuit_construction() {
        let circuit = AttestationValidityCircuit {
            attestation_commitment: [0x11; 32],
            current_timestamp: 1738281600,
            revocation_root: [0x22; 32],
            attestation_hash: [0x33; 32],
            expiry_timestamp: 1769817600,
            revocation_non_membership: vec![[0x44; 32]],
        };
        assert!(circuit.expiry_timestamp > circuit.current_timestamp);
    }

    #[test]
    fn threshold_signature_circuit_construction() {
        let circuit = ThresholdSignatureCircuit {
            statement_hash: [0xab; 32],
            threshold: 3,
            authorized_signers_root: [0xcd; 32],
            signatures: vec![vec![0x01; 64]; 3],
            signer_pubkeys: vec![vec![0x02; 32]; 3],
            signer_merkle_proofs: vec![vec![[0x03; 32]; 2]; 3],
        };
        assert!(circuit.signatures.len() >= circuit.threshold as usize);
    }

    // ── KycAttestationCircuit comprehensive tests ───────────────

    #[test]
    fn kyc_attestation_circuit_serialization_roundtrip() {
        let circuit = KycAttestationCircuit {
            approved_issuers_root: [0xaa; 32],
            min_kyc_level: 2,
            verification_timestamp: 1738281600,
            attestation_hash: [0xbb; 32],
            issuer_signature: vec![0xcc; 64],
            issuer_pubkey: vec![0xdd; 32],
            kyc_level: 3,
            issuer_merkle_proof: vec![[0xee; 32]],
        };
        let json = serde_json::to_string(&circuit).unwrap();
        let deserialized: KycAttestationCircuit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.min_kyc_level, 2);
        assert_eq!(deserialized.kyc_level, 3);
        assert_eq!(deserialized.verification_timestamp, 1738281600);
    }

    #[test]
    fn kyc_attestation_circuit_all_levels() {
        for level in 1u8..=3 {
            let circuit = KycAttestationCircuit {
                approved_issuers_root: [0xaa; 32],
                min_kyc_level: level,
                verification_timestamp: 1738281600,
                attestation_hash: [0xbb; 32],
                issuer_signature: vec![0xcc; 64],
                issuer_pubkey: vec![0xdd; 32],
                kyc_level: level,
                issuer_merkle_proof: vec![[0xee; 32]],
            };
            assert!(circuit.kyc_level >= circuit.min_kyc_level);
        }
    }

    #[test]
    fn kyc_attestation_deep_merkle_proof() {
        let circuit = KycAttestationCircuit {
            approved_issuers_root: [0xaa; 32],
            min_kyc_level: 1,
            verification_timestamp: 1738281600,
            attestation_hash: [0xbb; 32],
            issuer_signature: vec![0xcc; 64],
            issuer_pubkey: vec![0xdd; 32],
            kyc_level: 3,
            issuer_merkle_proof: (0..10).map(|i| [i as u8; 32]).collect(),
        };
        assert_eq!(circuit.issuer_merkle_proof.len(), 10);
    }

    // ── AttestationValidityCircuit comprehensive tests ──────────

    #[test]
    fn attestation_validity_circuit_serialization_roundtrip() {
        let circuit = AttestationValidityCircuit {
            attestation_commitment: [0x11; 32],
            current_timestamp: 1738281600,
            revocation_root: [0x22; 32],
            attestation_hash: [0x33; 32],
            expiry_timestamp: 1769817600,
            revocation_non_membership: vec![[0x44; 32], [0x55; 32]],
        };
        let json = serde_json::to_string(&circuit).unwrap();
        let deserialized: AttestationValidityCircuit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.current_timestamp, 1738281600);
        assert_eq!(deserialized.expiry_timestamp, 1769817600);
    }

    #[test]
    fn attestation_validity_not_expired() {
        let circuit = AttestationValidityCircuit {
            attestation_commitment: [0x11; 32],
            current_timestamp: 1738281600,
            revocation_root: [0x22; 32],
            attestation_hash: [0x33; 32],
            expiry_timestamp: 1769817600,
            revocation_non_membership: vec![],
        };
        assert!(
            circuit.expiry_timestamp > circuit.current_timestamp,
            "Attestation must not be expired"
        );
    }

    #[test]
    fn attestation_validity_expired() {
        let circuit = AttestationValidityCircuit {
            attestation_commitment: [0x11; 32],
            current_timestamp: 1800000000,
            revocation_root: [0x22; 32],
            attestation_hash: [0x33; 32],
            expiry_timestamp: 1769817600,
            revocation_non_membership: vec![],
        };
        assert!(
            circuit.current_timestamp > circuit.expiry_timestamp,
            "This attestation is expired"
        );
    }

    // ── ThresholdSignatureCircuit comprehensive tests ────────────

    #[test]
    fn threshold_signature_circuit_serialization_roundtrip() {
        let circuit = ThresholdSignatureCircuit {
            statement_hash: [0xab; 32],
            threshold: 2,
            authorized_signers_root: [0xcd; 32],
            signatures: vec![vec![0x01; 64]; 2],
            signer_pubkeys: vec![vec![0x02; 32]; 2],
            signer_merkle_proofs: vec![vec![[0x03; 32]]; 2],
        };
        let json = serde_json::to_string(&circuit).unwrap();
        let deserialized: ThresholdSignatureCircuit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.threshold, 2);
        assert_eq!(deserialized.signatures.len(), 2);
    }

    #[test]
    fn threshold_signature_exceeds_minimum() {
        let circuit = ThresholdSignatureCircuit {
            statement_hash: [0xab; 32],
            threshold: 2,
            authorized_signers_root: [0xcd; 32],
            signatures: vec![vec![0x01; 64]; 5],
            signer_pubkeys: vec![vec![0x02; 32]; 5],
            signer_merkle_proofs: vec![vec![[0x03; 32]]; 5],
        };
        assert!(circuit.signatures.len() > circuit.threshold as usize);
    }

    #[test]
    fn threshold_signature_single_signer() {
        let circuit = ThresholdSignatureCircuit {
            statement_hash: [0xab; 32],
            threshold: 1,
            authorized_signers_root: [0xcd; 32],
            signatures: vec![vec![0x01; 64]],
            signer_pubkeys: vec![vec![0x02; 32]],
            signer_merkle_proofs: vec![vec![[0x03; 32]]],
        };
        assert_eq!(circuit.threshold, 1);
        assert_eq!(circuit.signatures.len(), 1);
    }
}
