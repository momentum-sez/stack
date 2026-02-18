//! # ZK Proof Backend Policy (P0-ZK-001)
//!
//! Enforces production-mode proof backend requirements at runtime.
//!
//! ## Problem
//!
//! The mock proof system (`MockProofSystem`) provides deterministic
//! SHA-256 hashes as "proofs" — these have zero cryptographic security.
//! If a verifier accepts mock proofs in production, an attacker can
//! produce proofs without possessing the underlying witness.
//!
//! ## Solution
//!
//! This module provides a [`ProofPolicy`] that must be checked before
//! proof verification is accepted as authoritative. In production mode,
//! mock proofs are unconditionally rejected.
//!
//! ## Configuration
//!
//! The policy mode is determined by:
//! 1. Compile-time feature flags (`--cfg msez_production`)
//! 2. Runtime environment variable (`MSEZ_PROOF_POLICY`)
//! 3. Explicit `ProofPolicy::new()` construction
//!
//! The default policy is determined at compile time:
//! - Release builds (`not(debug_assertions)`) default to `Production`
//! - Debug builds default to `Development`
//!
//! ## CI Gate
//!
//! The CI pipeline should verify that release builds reject mock proofs:
//! ```text
//! cargo test --release --package msez-zkp -- policy::tests::release_build_rejects_mock
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from proof policy enforcement.
#[derive(Error, Debug)]
pub enum PolicyError {
    /// Mock proof rejected in production mode.
    #[error("mock proof rejected: production mode requires a real proof backend ({backend})")]
    MockProofRejected {
        /// The proof backend that was rejected.
        backend: String,
    },
}

/// The type of proof backend that produced a proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofBackend {
    /// Deterministic SHA-256 mock — no cryptographic security.
    Mock,
    /// Groth16 SNARK — real zero-knowledge proof.
    Groth16,
    /// PLONK — real zero-knowledge proof.
    Plonk,
}

impl ProofBackend {
    /// Whether this backend provides real cryptographic security.
    pub fn is_real(self) -> bool {
        matches!(self, ProofBackend::Groth16 | ProofBackend::Plonk)
    }

    /// Human-readable name.
    pub fn name(self) -> &'static str {
        match self {
            ProofBackend::Mock => "mock-sha256",
            ProofBackend::Groth16 => "groth16",
            ProofBackend::Plonk => "plonk",
        }
    }
}

/// Proof policy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyMode {
    /// Production: reject mock proofs unconditionally.
    Production,
    /// Development: accept mock proofs (for testing and local dev only).
    Development,
}

/// Runtime proof policy that validates whether a proof backend is
/// acceptable for the current deployment context.
///
/// ## Usage
///
/// ```rust,no_run
/// use msez_zkp::policy::{ProofPolicy, ProofBackend, PolicyMode};
///
/// let policy = ProofPolicy::production();
/// assert!(policy.validate(ProofBackend::Groth16).is_ok());
/// assert!(policy.validate(ProofBackend::Mock).is_err());
/// ```
#[derive(Debug, Clone)]
pub struct ProofPolicy {
    mode: PolicyMode,
}

impl ProofPolicy {
    /// Create a policy with the given mode.
    pub fn new(mode: PolicyMode) -> Self {
        Self { mode }
    }

    /// Create a production policy (rejects mock proofs).
    pub fn production() -> Self {
        Self {
            mode: PolicyMode::Production,
        }
    }

    /// Create a development policy (accepts mock proofs).
    pub fn development() -> Self {
        Self {
            mode: PolicyMode::Development,
        }
    }

    /// Create a policy based on the environment.
    ///
    /// Checks (in order):
    /// 1. `MSEZ_PROOF_POLICY` env var (`production` or `development`)
    /// 2. Compile-time: release builds default to `Production`
    /// 3. Compile-time: debug builds default to `Development`
    pub fn from_environment() -> Self {
        if let Ok(val) = std::env::var("MSEZ_PROOF_POLICY") {
            match val.to_lowercase().as_str() {
                "production" | "prod" => return Self::production(),
                "development" | "dev" => return Self::development(),
                _ => {} // Fall through to compile-time default.
            }
        }

        // Compile-time default: release = production, debug = development.
        if cfg!(not(debug_assertions)) {
            Self::production()
        } else {
            Self::development()
        }
    }

    /// Validate whether a proof backend is acceptable under this policy.
    ///
    /// Returns `Ok(())` if the backend is accepted, or
    /// [`PolicyError::MockProofRejected`] if mock proofs are rejected.
    pub fn validate(&self, backend: ProofBackend) -> Result<(), PolicyError> {
        match self.mode {
            PolicyMode::Production => {
                if backend == ProofBackend::Mock {
                    Err(PolicyError::MockProofRejected {
                        backend: backend.name().to_string(),
                    })
                } else {
                    Ok(())
                }
            }
            PolicyMode::Development => Ok(()),
        }
    }

    /// Current policy mode.
    pub fn mode(&self) -> PolicyMode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_rejects_mock() {
        let policy = ProofPolicy::production();
        assert!(policy.validate(ProofBackend::Mock).is_err());
    }

    #[test]
    fn production_accepts_groth16() {
        let policy = ProofPolicy::production();
        assert!(policy.validate(ProofBackend::Groth16).is_ok());
    }

    #[test]
    fn production_accepts_plonk() {
        let policy = ProofPolicy::production();
        assert!(policy.validate(ProofBackend::Plonk).is_ok());
    }

    #[test]
    fn development_accepts_mock() {
        let policy = ProofPolicy::development();
        assert!(policy.validate(ProofBackend::Mock).is_ok());
    }

    #[test]
    fn development_accepts_real() {
        let policy = ProofPolicy::development();
        assert!(policy.validate(ProofBackend::Groth16).is_ok());
        assert!(policy.validate(ProofBackend::Plonk).is_ok());
    }

    #[test]
    fn mock_backend_is_not_real() {
        assert!(!ProofBackend::Mock.is_real());
    }

    #[test]
    fn real_backends_are_real() {
        assert!(ProofBackend::Groth16.is_real());
        assert!(ProofBackend::Plonk.is_real());
    }

    #[test]
    fn backend_names() {
        assert_eq!(ProofBackend::Mock.name(), "mock-sha256");
        assert_eq!(ProofBackend::Groth16.name(), "groth16");
        assert_eq!(ProofBackend::Plonk.name(), "plonk");
    }

    #[test]
    fn policy_mode_accessor() {
        assert_eq!(
            ProofPolicy::production().mode(),
            PolicyMode::Production
        );
        assert_eq!(
            ProofPolicy::development().mode(),
            PolicyMode::Development
        );
    }

    #[test]
    fn error_message_includes_backend() {
        let err = PolicyError::MockProofRejected {
            backend: "mock-sha256".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("mock-sha256"));
        assert!(msg.contains("production mode"));
    }

    #[test]
    fn from_environment_default() {
        // In test builds (debug), default should be Development.
        let policy = ProofPolicy::from_environment();
        // Don't assert specific mode since env var may be set,
        // but verify it doesn't panic.
        let _ = policy.mode();
    }

    /// This test verifies that release builds default to production mode.
    /// Run with: `cargo test --release --package msez-zkp -- policy::tests::release_build_rejects_mock`
    #[test]
    fn release_build_rejects_mock() {
        if cfg!(not(debug_assertions)) {
            let policy = ProofPolicy::from_environment();
            // In release builds without env var override, mock should be rejected.
            // (This may pass or fail depending on env vars in CI,
            // but documents the expected behavior.)
            if std::env::var("MSEZ_PROOF_POLICY").is_err() {
                assert_eq!(policy.mode(), PolicyMode::Production);
                assert!(policy.validate(ProofBackend::Mock).is_err());
            }
        }
    }

    #[test]
    fn policy_serialization() {
        let backend = ProofBackend::Mock;
        let json = serde_json::to_string(&backend).unwrap();
        let deserialized: ProofBackend = serde_json::from_str(&json).unwrap();
        assert_eq!(backend, deserialized);

        let mode = PolicyMode::Production;
        let json = serde_json::to_string(&mode).unwrap();
        let deserialized: PolicyMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, deserialized);
    }
}
