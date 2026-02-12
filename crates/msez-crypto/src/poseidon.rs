//! # Poseidon2 — ZK-Friendly Hash Function (Phase 4 Stub)
//!
//! This module activates behind the `poseidon2` feature flag. It contains
//! only type signatures and `unimplemented!()` bodies until a real Poseidon2
//! backend is integrated in Phase 4 of the SEZ Stack roadmap.
//!
//! ## Purpose
//!
//! Poseidon2 is an arithmetic-circuit-native hash function optimized for
//! zero-knowledge proof systems. It replaces SHA-256 in ZK contexts where
//! proving SHA-256 constraints is prohibitively expensive.
//!
//! ## Security Invariant
//!
//! Like SHA-256 paths, all Poseidon2 digest computation accepts only
//! `&CanonicalBytes` to prevent the canonicalization split defect.
//!
//! ## Implements
//!
//! Spec §22 — ZK-friendly hash function for Phase 4 proof circuits.

use msez_core::{CanonicalBytes, ContentDigest, DigestAlgorithm};

/// Compute a Poseidon2 content digest from canonical bytes.
///
/// # Panics
///
/// Always panics — Poseidon2 is not yet implemented. This function exists
/// as a forward-declaration for Phase 4 ZK proof system integration.
///
/// When implemented, this will produce a `ContentDigest` with
/// `DigestAlgorithm::Poseidon2` suitable for use in ZK circuits.
pub fn poseidon2_digest(_data: &CanonicalBytes) -> ContentDigest {
    unimplemented!(
        "Poseidon2 digest is Phase 4 — ZK proof system integration pending. \
         See spec §22 for the target hash specification."
    )
}

/// Poseidon2 hash parameters for ZK circuit compatibility.
///
/// Placeholder — will hold the Poseidon2 configuration (round constants,
/// MDS matrix, etc.) when the backend is integrated.
pub struct Poseidon2Params {
    _private: (),
}

impl Poseidon2Params {
    /// Create default Poseidon2 parameters.
    ///
    /// # Panics
    ///
    /// Always panics — parameters are not yet defined.
    pub fn default_params() -> Self {
        unimplemented!(
            "Poseidon2 parameters are Phase 4 — ZK proof system integration pending"
        )
    }
}

/// A Poseidon2 hash state for incremental hashing.
///
/// Placeholder — will implement `update()` / `finalize()` API when the
/// Poseidon2 backend is integrated.
pub struct Poseidon2State {
    _private: (),
}

impl Poseidon2State {
    /// Create a new Poseidon2 hash state.
    ///
    /// # Panics
    ///
    /// Always panics — not yet implemented.
    pub fn new(_params: &Poseidon2Params) -> Self {
        unimplemented!("Poseidon2 state is Phase 4")
    }

    /// Update the hash state with additional data.
    ///
    /// # Panics
    ///
    /// Always panics — not yet implemented.
    pub fn update(&mut self, _data: &CanonicalBytes) {
        unimplemented!("Poseidon2 update is Phase 4")
    }

    /// Finalize and produce the content digest.
    ///
    /// # Panics
    ///
    /// Always panics — not yet implemented.
    pub fn finalize(self) -> ContentDigest {
        unimplemented!("Poseidon2 finalize is Phase 4")
    }
}

// Suppress unused import warning — DigestAlgorithm is referenced in docs
// and will be used when poseidon2_digest is implemented.
const _: () = {
    fn _assert_digest_algorithm_exists() {
        let _ = DigestAlgorithm::Poseidon2;
    }
};
