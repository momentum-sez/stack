//! # Migration Saga with Provable Compensation (P0-MIGRATION-001)
//!
//! Implements a corridor migration state machine with formally provable
//! safety properties:
//!
//! 1. **Inverse compensation:** `forward + compensate = pre-state`
//! 2. **Idempotent compensation:** `compensate(compensate(s)) == compensate(s)`
//! 3. **No asset duplication:** `¬(asset_exists_source ∧ asset_exists_dest)`
//! 4. **Timeout triggers compensation:** expired migrations auto-compensate
//!
//! ## Side-Effect Model
//!
//! Each migration step records explicit forward side-effects (`Lock`,
//! `Unlock`, `Mint`, `Burn`) and their inverses. The compensation
//! function deterministically inverts all recorded effects.
//!
//! ## Concurrency Safety
//!
//! Uses CAS (Compare-And-Swap) versioning: each state transition
//! increments the version counter. Concurrent `advance()` calls on
//! the same migration will fail with `VersionConflict`.
//!
//! ## Spec Reference
//!
//! Implements migration saga protocol per `spec/40-corridors.md` §26-30.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Trait for executing migration side-effects against external systems.
///
/// Implementations bridge the saga state machine to real zone infrastructure:
/// - Lock/unlock assets at source zones
/// - Mint/burn representations at destination zones
///
/// The executor is called during `advance_with_executor()` for forward effects
/// and during `compensate_with_executor()` for compensation effects.
///
/// ## Idempotency Contract
///
/// Implementations MUST be idempotent: executing the same effect twice
/// must produce the same result without error. This is required because
/// compensation retry loops may re-execute effects after partial failure.
pub trait EffectExecutor {
    /// Execute a single side-effect. Returns `Ok(())` on success
    /// or an error description on failure.
    fn execute(&mut self, migration_id: &Uuid, effect: &SideEffect) -> Result<(), String>;
}

/// A no-op executor that records effects without executing them.
///
/// Used when callers only need effect tracking (the original `advance()`
/// and `compensate()` behavior). Also useful in tests.
#[derive(Debug, Default)]
pub struct RecordingExecutor {
    /// All effects that were passed to `execute()`.
    pub executed: Vec<SideEffect>,
}

impl EffectExecutor for RecordingExecutor {
    fn execute(&mut self, _migration_id: &Uuid, effect: &SideEffect) -> Result<(), String> {
        self.executed.push(effect.clone());
        Ok(())
    }
}

/// Errors in migration saga operations.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum MigrationError {
    /// Attempted to advance a migration in a terminal state.
    #[error("migration {id} is in terminal state {state:?}")]
    AlreadyTerminal {
        /// Migration identifier.
        id: Uuid,
        /// Current terminal state.
        state: MigrationState,
    },

    /// CAS version conflict — another concurrent operation modified
    /// the migration between read and write.
    #[error("version conflict on migration {id}: expected {expected}, found {found}")]
    VersionConflict {
        /// Migration identifier.
        id: Uuid,
        /// The version the caller expected.
        expected: u64,
        /// The actual current version.
        found: u64,
    },

    /// Migration deadline has passed. Must compensate.
    #[error("migration {id} has timed out (deadline={deadline})")]
    TimedOut {
        /// Migration identifier.
        id: Uuid,
        /// The deadline that was exceeded.
        deadline: DateTime<Utc>,
    },

    /// Invalid state transition.
    #[error("invalid migration transition from {from:?} to {to:?}")]
    InvalidTransition {
        /// Current state.
        from: MigrationState,
        /// Attempted target state.
        to: MigrationState,
    },

    /// A side-effect execution failed.
    #[error("effect execution failed for migration {id}: {reason}")]
    EffectFailed {
        /// Migration identifier.
        id: Uuid,
        /// Description of the failure.
        reason: String,
    },
}

/// A forward side-effect recorded during migration execution.
///
/// Each effect has a defined inverse for compensation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideEffect {
    /// Lock assets at source zone. Inverse: `Unlock`.
    Lock {
        /// Asset identifier.
        asset_id: String,
        /// Zone where the asset is locked.
        zone_id: String,
    },
    /// Unlock assets at source zone. Inverse: `Lock`.
    Unlock {
        /// Asset identifier.
        asset_id: String,
        /// Zone where the asset is unlocked.
        zone_id: String,
    },
    /// Mint (create) representation at destination zone. Inverse: `Burn`.
    Mint {
        /// Asset identifier.
        asset_id: String,
        /// Zone where the representation is minted.
        zone_id: String,
    },
    /// Burn (destroy) representation at destination zone. Inverse: `Mint`.
    Burn {
        /// Asset identifier.
        asset_id: String,
        /// Zone where the representation is burned.
        zone_id: String,
    },
}

impl SideEffect {
    /// Return the inverse side-effect for compensation.
    ///
    /// This is the core of the provable compensation model:
    /// `inverse(inverse(e)) == e` for all effects.
    pub fn inverse(&self) -> Self {
        match self {
            SideEffect::Lock { asset_id, zone_id } => SideEffect::Unlock {
                asset_id: asset_id.clone(),
                zone_id: zone_id.clone(),
            },
            SideEffect::Unlock { asset_id, zone_id } => SideEffect::Lock {
                asset_id: asset_id.clone(),
                zone_id: zone_id.clone(),
            },
            SideEffect::Mint { asset_id, zone_id } => SideEffect::Burn {
                asset_id: asset_id.clone(),
                zone_id: zone_id.clone(),
            },
            SideEffect::Burn { asset_id, zone_id } => SideEffect::Mint {
                asset_id: asset_id.clone(),
                zone_id: zone_id.clone(),
            },
        }
    }
}

/// Migration saga states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MigrationState {
    /// Migration initiated, no side-effects yet.
    Initiated,
    /// Source assets locked. Forward effect: `Lock`.
    SourceLocked,
    /// Destination representation minted. Forward effects: `Lock`, `Mint`.
    DestinationMinted,
    /// Migration completed successfully. Terminal.
    Completed,
    /// Migration compensated (rolled back). Terminal. Idempotent.
    Compensated,
    /// Migration timed out. Terminal. Compensation executed.
    TimedOut,
}

impl MigrationState {
    /// Whether this state is terminal (no further transitions allowed).
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            MigrationState::Completed
                | MigrationState::Compensated
                | MigrationState::TimedOut
        )
    }
}

/// A migration saga instance with explicit side-effect tracking and
/// CAS versioning for concurrent safety.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSaga {
    /// Unique migration identifier.
    pub id: Uuid,
    /// CAS version — incremented on every state transition.
    pub version: u64,
    /// Current state of the migration.
    pub state: MigrationState,
    /// Source zone identifier.
    pub source_zone: String,
    /// Destination zone identifier.
    pub dest_zone: String,
    /// Asset being migrated.
    pub asset_id: String,
    /// Deadline for migration completion. After this, compensation
    /// is automatically triggered.
    pub deadline: DateTime<Utc>,
    /// Ordered log of forward side-effects that have been executed.
    pub forward_effects: Vec<SideEffect>,
    /// Ordered log of compensation side-effects that were executed
    /// during rollback. Empty unless compensated.
    pub compensation_effects: Vec<SideEffect>,
    /// Timestamp of saga creation.
    pub created_at: DateTime<Utc>,
    /// Timestamp of last state change.
    pub updated_at: DateTime<Utc>,
}

impl MigrationSaga {
    /// Create a new migration saga in the `Initiated` state.
    pub fn new(
        source_zone: String,
        dest_zone: String,
        asset_id: String,
        deadline: DateTime<Utc>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            version: 0,
            state: MigrationState::Initiated,
            source_zone,
            dest_zone,
            asset_id,
            deadline,
            forward_effects: Vec::new(),
            compensation_effects: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Advance the migration to the next state.
    ///
    /// Uses CAS versioning: the caller must provide the expected version.
    /// If another operation modified the saga concurrently, this returns
    /// `VersionConflict`.
    ///
    /// Checks the deadline and auto-compensates if expired.
    ///
    /// This is the recording-only variant. For executing effects against
    /// external systems, use [`advance_with_executor`](Self::advance_with_executor).
    pub fn advance(&mut self, expected_version: u64) -> Result<MigrationState, MigrationError> {
        self.advance_with_executor(expected_version, &mut RecordingExecutor::default())
    }

    /// Advance the migration to the next state, executing effects via the
    /// provided [`EffectExecutor`].
    ///
    /// The executor is called for each forward side-effect produced by
    /// the state transition. If the executor fails, the saga state is
    /// NOT advanced (the effect is not recorded).
    ///
    /// On timeout, compensation effects are computed and executed via
    /// the same executor.
    pub fn advance_with_executor(
        &mut self,
        expected_version: u64,
        executor: &mut dyn EffectExecutor,
    ) -> Result<MigrationState, MigrationError> {
        // CAS check.
        if self.version != expected_version {
            return Err(MigrationError::VersionConflict {
                id: self.id,
                expected: expected_version,
                found: self.version,
            });
        }

        // Terminal state check.
        if self.state.is_terminal() {
            return Err(MigrationError::AlreadyTerminal {
                id: self.id,
                state: self.state,
            });
        }

        // Deadline check — if expired, execute compensation.
        let now = Utc::now();
        if now > self.deadline {
            self.execute_compensation_with_executor(executor)?;
            self.state = MigrationState::TimedOut;
            self.version += 1;
            self.updated_at = now;
            return Err(MigrationError::TimedOut {
                id: self.id,
                deadline: self.deadline,
            });
        }

        // Determine the next effect.
        let effect = match self.state {
            MigrationState::Initiated => {
                // Step 1: Lock source assets.
                SideEffect::Lock {
                    asset_id: self.asset_id.clone(),
                    zone_id: self.source_zone.clone(),
                }
            }
            MigrationState::SourceLocked => {
                // Step 2: Mint at destination.
                SideEffect::Mint {
                    asset_id: self.asset_id.clone(),
                    zone_id: self.dest_zone.clone(),
                }
            }
            MigrationState::DestinationMinted => {
                // Step 3: Complete — burn the source lock (finalize).
                SideEffect::Burn {
                    asset_id: self.asset_id.clone(),
                    zone_id: self.source_zone.clone(),
                }
            }
            // Terminal states handled above.
            MigrationState::Completed
            | MigrationState::Compensated
            | MigrationState::TimedOut => unreachable!(),
        };

        // Execute the effect. If the executor fails, do NOT record
        // the effect or transition state.
        executor
            .execute(&self.id, &effect)
            .map_err(|reason| MigrationError::EffectFailed {
                id: self.id,
                reason,
            })?;

        let next_state = match self.state {
            MigrationState::Initiated => MigrationState::SourceLocked,
            MigrationState::SourceLocked => MigrationState::DestinationMinted,
            MigrationState::DestinationMinted => MigrationState::Completed,
            _ => unreachable!(),
        };

        self.forward_effects.push(effect);
        self.state = next_state;
        self.version += 1;
        self.updated_at = Utc::now();
        Ok(next_state)
    }

    /// Compensate the migration by reversing all forward side-effects.
    ///
    /// ## CAS Versioning
    ///
    /// Like `advance()`, `compensate()` uses CAS versioning to prevent
    /// concurrent modification. The caller must supply `expected_version`.
    ///
    /// ## Idempotency (P0-MIGRATION-001 requirement)
    ///
    /// Calling `compensate()` on an already-compensated or timed-out
    /// migration is a **no-op** that returns `Ok(Compensated)`.
    /// This is required for financial safety: retry loops must not
    /// produce errors on re-compensation. The version check is skipped
    /// for idempotent no-op returns.
    ///
    /// This is the recording-only variant. For executing effects against
    /// external systems, use [`compensate_with_executor`](Self::compensate_with_executor).
    pub fn compensate(&mut self, expected_version: u64) -> Result<MigrationState, MigrationError> {
        self.compensate_with_executor(expected_version, &mut RecordingExecutor::default())
    }

    /// Compensate with an [`EffectExecutor`] that executes each
    /// compensation effect against external systems.
    ///
    /// Idempotency: if the saga is already compensated or timed out,
    /// this is a no-op (the executor is NOT called again).
    pub fn compensate_with_executor(
        &mut self,
        expected_version: u64,
        executor: &mut dyn EffectExecutor,
    ) -> Result<MigrationState, MigrationError> {
        // Idempotent: already compensated or timed out = no-op.
        // No CAS check on idempotent path — retry loops must succeed.
        if self.state == MigrationState::Compensated
            || self.state == MigrationState::TimedOut
        {
            return Ok(self.state);
        }

        // CAS check (only on non-idempotent path).
        if self.version != expected_version {
            return Err(MigrationError::VersionConflict {
                id: self.id,
                expected: expected_version,
                found: self.version,
            });
        }

        // Cannot compensate a completed migration through this path.
        if self.state == MigrationState::Completed {
            return Err(MigrationError::AlreadyTerminal {
                id: self.id,
                state: self.state,
            });
        }

        self.execute_compensation_with_executor(executor)?;
        self.state = MigrationState::Compensated;
        self.version += 1;
        self.updated_at = Utc::now();
        Ok(MigrationState::Compensated)
    }

    /// Compute and execute compensation effects.
    ///
    /// Compensation effects are the inverse of forward effects in reverse
    /// order. Each effect is executed via the provided executor.
    ///
    /// If compensation effects were already computed (idempotent retry),
    /// they are re-executed but not recomputed.
    fn execute_compensation_with_executor(
        &mut self,
        executor: &mut dyn EffectExecutor,
    ) -> Result<(), MigrationError> {
        if self.compensation_effects.is_empty() {
            // Compute compensation effects: reverse order, inverse of each.
            self.compensation_effects = self
                .forward_effects
                .iter()
                .rev()
                .map(|e| e.inverse())
                .collect();
        }

        // Execute each compensation effect.
        for effect in &self.compensation_effects {
            executor
                .execute(&self.id, effect)
                .map_err(|reason| MigrationError::EffectFailed {
                    id: self.id,
                    reason,
                })?;
        }
        Ok(())
    }

    /// Check the no-duplicate invariant:
    /// `¬(asset_exists_source ∧ asset_exists_dest)`
    ///
    /// Returns `true` if the invariant holds (no duplication).
    pub fn no_duplicate_invariant(&self) -> bool {
        let mut source_locked = false;
        let mut dest_minted = false;

        // Replay forward effects.
        for effect in &self.forward_effects {
            match effect {
                SideEffect::Lock { .. } => source_locked = true,
                SideEffect::Unlock { .. } => source_locked = false,
                SideEffect::Mint { .. } => dest_minted = true,
                SideEffect::Burn { .. } => dest_minted = false,
            }
        }

        // Replay compensation effects (if any).
        for effect in &self.compensation_effects {
            match effect {
                SideEffect::Lock { .. } => source_locked = true,
                SideEffect::Unlock { .. } => source_locked = false,
                SideEffect::Mint { .. } => dest_minted = true,
                SideEffect::Burn { .. } => dest_minted = false,
            }
        }

        // In a valid state, the asset should not be simultaneously
        // available in both source and destination zones.
        // After locking source and minting dest: source_locked=true, dest_minted=true
        // which is OK because the source is LOCKED (not available).
        // The invariant fails when source is UNLOCKED AND dest is MINTED.
        source_locked || !dest_minted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn make_saga() -> MigrationSaga {
        MigrationSaga::new(
            "zone-pk-01".to_string(),
            "zone-ae-01".to_string(),
            "asset-001".to_string(),
            Utc::now() + chrono::Duration::hours(1),
        )
    }

    fn make_expired_saga() -> MigrationSaga {
        MigrationSaga::new(
            "zone-pk-01".to_string(),
            "zone-ae-01".to_string(),
            "asset-002".to_string(),
            Utc::now() - chrono::Duration::hours(1), // Already expired
        )
    }

    /// Asset state in a zone — models the external world for property tests.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum AssetState {
        /// Asset is locked (cannot be transferred).
        Locked,
        /// Asset representation has been minted.
        Minted,
    }

    /// A simple ledger that tracks per-zone asset state.
    /// Used by property tests to verify `forward + compensate = pre-state`.
    #[derive(Debug, Clone)]
    struct AssetLedger {
        /// (zone_id, asset_id) → state
        state: BTreeMap<(String, String), AssetState>,
    }

    impl AssetLedger {
        fn new() -> Self {
            Self {
                state: BTreeMap::new(),
            }
        }

        fn apply(&mut self, effect: &SideEffect) {
            match effect {
                SideEffect::Lock { asset_id, zone_id } => {
                    self.state
                        .insert((zone_id.clone(), asset_id.clone()), AssetState::Locked);
                }
                SideEffect::Unlock { asset_id, zone_id } => {
                    // Unlock removes the lock — returns to pre-lock state
                    // (absent from ledger). This ensures forward + compensate = pre-state.
                    self.state.remove(&(zone_id.clone(), asset_id.clone()));
                }
                SideEffect::Mint { asset_id, zone_id } => {
                    self.state
                        .insert((zone_id.clone(), asset_id.clone()), AssetState::Minted);
                }
                SideEffect::Burn { asset_id, zone_id } => {
                    // Burn removes the minted representation.
                    self.state.remove(&(zone_id.clone(), asset_id.clone()));
                }
            }
        }

        fn apply_all(&mut self, effects: &[SideEffect]) {
            for e in effects {
                self.apply(e);
            }
        }
    }

    /// An executor that applies effects to an AssetLedger.
    struct LedgerExecutor<'a> {
        ledger: &'a mut AssetLedger,
    }

    impl EffectExecutor for LedgerExecutor<'_> {
        fn execute(&mut self, _migration_id: &Uuid, effect: &SideEffect) -> Result<(), String> {
            self.ledger.apply(effect);
            Ok(())
        }
    }

    /// An executor that fails on the Nth call, for testing partial failure.
    struct FailingExecutor {
        call_count: usize,
        fail_on: usize,
    }

    impl EffectExecutor for FailingExecutor {
        fn execute(&mut self, _migration_id: &Uuid, _effect: &SideEffect) -> Result<(), String> {
            self.call_count += 1;
            if self.call_count == self.fail_on {
                return Err("simulated failure".to_string());
            }
            Ok(())
        }
    }

    #[test]
    fn full_migration_lifecycle() {
        let mut saga = make_saga();
        assert_eq!(saga.state, MigrationState::Initiated);
        assert_eq!(saga.version, 0);

        // Step 1: Initiated → SourceLocked
        let state = saga.advance(0).unwrap();
        assert_eq!(state, MigrationState::SourceLocked);
        assert_eq!(saga.version, 1);
        assert_eq!(saga.forward_effects.len(), 1);
        assert!(matches!(saga.forward_effects[0], SideEffect::Lock { .. }));

        // Step 2: SourceLocked → DestinationMinted
        let state = saga.advance(1).unwrap();
        assert_eq!(state, MigrationState::DestinationMinted);
        assert_eq!(saga.version, 2);
        assert_eq!(saga.forward_effects.len(), 2);
        assert!(matches!(saga.forward_effects[1], SideEffect::Mint { .. }));

        // Step 3: DestinationMinted → Completed
        let state = saga.advance(2).unwrap();
        assert_eq!(state, MigrationState::Completed);
        assert_eq!(saga.version, 3);
        assert_eq!(saga.forward_effects.len(), 3);
        assert!(matches!(saga.forward_effects[2], SideEffect::Burn { .. }));

        // Terminal — no further advance.
        let err = saga.advance(3).unwrap_err();
        assert!(matches!(err, MigrationError::AlreadyTerminal { .. }));
    }

    #[test]
    fn compensation_reverses_forward_effects() {
        let mut saga = make_saga();
        saga.advance(0).unwrap(); // → SourceLocked
        saga.advance(1).unwrap(); // → DestinationMinted

        // Now compensate (before completing). Version is 2 after two advances.
        let state = saga.compensate(2).unwrap();
        assert_eq!(state, MigrationState::Compensated);

        // Compensation effects should be reverse of forward effects.
        assert_eq!(saga.compensation_effects.len(), 2);
        // Last forward was Mint, so first compensation is Burn.
        assert!(matches!(
            saga.compensation_effects[0],
            SideEffect::Burn { .. }
        ));
        // First forward was Lock, so second compensation is Unlock.
        assert!(matches!(
            saga.compensation_effects[1],
            SideEffect::Unlock { .. }
        ));
    }

    #[test]
    fn compensation_is_idempotent() {
        let mut saga = make_saga();
        saga.advance(0).unwrap(); // → SourceLocked

        // First compensation. Version is 1 after one advance.
        let state1 = saga.compensate(1).unwrap();
        assert_eq!(state1, MigrationState::Compensated);
        let v1 = saga.version;

        // Second compensation — must be no-op, not error.
        // Version argument is ignored on idempotent path.
        let state2 = saga.compensate(999).unwrap();
        assert_eq!(state2, MigrationState::Compensated);
        // Version should NOT increment on idempotent no-op.
        assert_eq!(saga.version, v1);
    }

    #[test]
    fn timeout_triggers_compensation() {
        let mut saga = make_expired_saga();

        // Advance on expired saga should fail with TimedOut
        // AND execute compensation.
        let err = saga.advance(0).unwrap_err();
        assert!(matches!(err, MigrationError::TimedOut { .. }));
        assert_eq!(saga.state, MigrationState::TimedOut);

        // Compensation on timed-out = idempotent no-op.
        // Version argument is ignored on idempotent path.
        let state = saga.compensate(999).unwrap();
        assert_eq!(state, MigrationState::TimedOut);
    }

    #[test]
    fn version_conflict_prevents_concurrent_advance() {
        let mut saga = make_saga();
        saga.advance(0).unwrap(); // version becomes 1

        // Try to advance with wrong version.
        let err = saga.advance(0).unwrap_err();
        assert!(matches!(
            err,
            MigrationError::VersionConflict {
                expected: 0,
                found: 1,
                ..
            }
        ));
    }

    #[test]
    fn no_duplicate_invariant_holds_throughout() {
        let mut saga = make_saga();
        assert!(saga.no_duplicate_invariant());

        saga.advance(0).unwrap(); // Lock source
        assert!(saga.no_duplicate_invariant());

        saga.advance(1).unwrap(); // Mint dest (source locked, dest minted = OK)
        assert!(saga.no_duplicate_invariant());

        saga.advance(2).unwrap(); // Burn source lock (complete)
        assert!(saga.no_duplicate_invariant());
    }

    #[test]
    fn no_duplicate_invariant_holds_after_compensation() {
        let mut saga = make_saga();
        saga.advance(0).unwrap(); // Lock source
        saga.advance(1).unwrap(); // Mint dest

        saga.compensate(2).unwrap(); // Should burn dest, unlock source (version=2 after 2 advances)
        assert!(saga.no_duplicate_invariant());
    }

    #[test]
    fn side_effect_inverse_is_involution() {
        // inverse(inverse(e)) == e for all effect types
        let effects = vec![
            SideEffect::Lock {
                asset_id: "a".into(),
                zone_id: "z".into(),
            },
            SideEffect::Unlock {
                asset_id: "a".into(),
                zone_id: "z".into(),
            },
            SideEffect::Mint {
                asset_id: "a".into(),
                zone_id: "z".into(),
            },
            SideEffect::Burn {
                asset_id: "a".into(),
                zone_id: "z".into(),
            },
        ];

        for effect in &effects {
            assert_eq!(
                effect.inverse().inverse(),
                *effect,
                "inverse must be involution for {:?}",
                effect
            );
        }
    }

    #[test]
    fn completed_migration_cannot_be_compensated() {
        let mut saga = make_saga();
        saga.advance(0).unwrap();
        saga.advance(1).unwrap();
        saga.advance(2).unwrap(); // Completed

        let err = saga.compensate(3).unwrap_err();
        assert!(matches!(err, MigrationError::AlreadyTerminal { .. }));
    }

    #[test]
    fn initiated_state_compensation() {
        let mut saga = make_saga();
        // Compensate from Initiated — no forward effects to reverse. Version=0.
        let state = saga.compensate(0).unwrap();
        assert_eq!(state, MigrationState::Compensated);
        assert!(saga.compensation_effects.is_empty());
    }

    #[test]
    fn compensate_version_conflict() {
        let mut saga = make_saga();
        saga.advance(0).unwrap(); // version becomes 1

        // Wrong version — must fail.
        let err = saga.compensate(0).unwrap_err();
        assert!(matches!(
            err,
            MigrationError::VersionConflict {
                expected: 0,
                found: 1,
                ..
            }
        ));

        // Correct version — must succeed.
        let state = saga.compensate(1).unwrap();
        assert_eq!(state, MigrationState::Compensated);
    }

    #[test]
    fn migration_serialization_roundtrip() {
        let mut saga = make_saga();
        saga.advance(0).unwrap();

        let json = serde_json::to_string(&saga).unwrap();
        let deserialized: MigrationSaga = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.state, saga.state);
        assert_eq!(deserialized.version, saga.version);
        assert_eq!(deserialized.forward_effects.len(), saga.forward_effects.len());
    }

    #[test]
    fn terminal_states_are_terminal() {
        assert!(MigrationState::Completed.is_terminal());
        assert!(MigrationState::Compensated.is_terminal());
        assert!(MigrationState::TimedOut.is_terminal());
        assert!(!MigrationState::Initiated.is_terminal());
        assert!(!MigrationState::SourceLocked.is_terminal());
        assert!(!MigrationState::DestinationMinted.is_terminal());
    }

    // ── EffectExecutor tests ────────────────────────────────────────────

    #[test]
    fn advance_with_executor_applies_effects_to_ledger() {
        let mut saga = make_saga();
        let mut ledger = AssetLedger::new();
        let mut exec = LedgerExecutor { ledger: &mut ledger };

        saga.advance_with_executor(0, &mut exec).unwrap();
        assert_eq!(
            exec.ledger
                .state
                .get(&("zone-pk-01".to_string(), "asset-001".to_string())),
            Some(&AssetState::Locked)
        );

        saga.advance_with_executor(1, &mut exec).unwrap();
        assert_eq!(
            exec.ledger
                .state
                .get(&("zone-ae-01".to_string(), "asset-001".to_string())),
            Some(&AssetState::Minted)
        );
    }

    #[test]
    fn compensate_with_executor_applies_inverse_effects() {
        let mut saga = make_saga();
        let mut ledger = AssetLedger::new();
        let mut exec = LedgerExecutor { ledger: &mut ledger };

        saga.advance_with_executor(0, &mut exec).unwrap(); // Lock
        saga.advance_with_executor(1, &mut exec).unwrap(); // Mint

        // Ledger: source locked, dest minted.
        assert_eq!(
            exec.ledger
                .state
                .get(&("zone-pk-01".to_string(), "asset-001".to_string())),
            Some(&AssetState::Locked)
        );
        assert_eq!(
            exec.ledger
                .state
                .get(&("zone-ae-01".to_string(), "asset-001".to_string())),
            Some(&AssetState::Minted)
        );

        // Compensate: should burn dest, unlock source.
        saga.compensate_with_executor(2, &mut exec).unwrap();

        // Source should be unlocked (removed from ledger = pre-state).
        assert_eq!(
            exec.ledger
                .state
                .get(&("zone-pk-01".to_string(), "asset-001".to_string())),
            None
        );
        // Dest should be burned (removed from ledger = pre-state).
        assert_eq!(
            exec.ledger
                .state
                .get(&("zone-ae-01".to_string(), "asset-001".to_string())),
            None
        );
    }

    #[test]
    fn executor_failure_prevents_state_transition() {
        let mut saga = make_saga();
        let mut exec = FailingExecutor {
            call_count: 0,
            fail_on: 1, // Fail on first call.
        };

        let err = saga.advance_with_executor(0, &mut exec).unwrap_err();
        assert!(matches!(err, MigrationError::EffectFailed { .. }));

        // State should NOT have advanced.
        assert_eq!(saga.state, MigrationState::Initiated);
        assert_eq!(saga.version, 0);
        assert!(saga.forward_effects.is_empty());
    }

    #[test]
    fn recording_executor_captures_all_effects() {
        let mut saga = make_saga();
        let mut exec = RecordingExecutor::default();

        saga.advance_with_executor(0, &mut exec).unwrap();
        saga.advance_with_executor(1, &mut exec).unwrap();
        assert_eq!(exec.executed.len(), 2);
        assert!(matches!(exec.executed[0], SideEffect::Lock { .. }));
        assert!(matches!(exec.executed[1], SideEffect::Mint { .. }));
    }

    // ── Property tests: forward + compensate = pre-state ────────────────

    #[test]
    fn forward_plus_compensate_equals_pre_state() {
        // Property: for any partial migration (0..N steps), compensating
        // must return the asset ledger to its pre-migration state.
        for steps in 0..=2 {
            let mut saga = make_saga();
            let mut ledger = AssetLedger::new();
            let pre_state = ledger.state.clone();

            // Advance N steps.
            let mut exec = LedgerExecutor { ledger: &mut ledger };
            for v in 0..steps {
                saga.advance_with_executor(v, &mut exec).unwrap();
            }

            // Compensate (if not already terminal from completion).
            if !saga.state.is_terminal() {
                saga.compensate_with_executor(steps, &mut exec).unwrap();
            }

            // After compensation, replay compensation effects on a fresh
            // ledger alongside forward effects to verify net-zero.
            let mut verify_ledger = AssetLedger::new();
            verify_ledger.apply_all(&saga.forward_effects);
            verify_ledger.apply_all(&saga.compensation_effects);

            assert_eq!(
                verify_ledger.state, pre_state,
                "forward + compensate must equal pre-state for {steps} steps"
            );
        }
    }

    #[test]
    fn no_duplicate_across_all_partial_states() {
        // Property: at every intermediate state, the no-duplicate invariant holds.
        let mut saga = make_saga();
        assert!(saga.no_duplicate_invariant(), "initial state");

        for v in 0..3u64 {
            saga.advance(v).unwrap();
            assert!(
                saga.no_duplicate_invariant(),
                "after advance to {:?}",
                saga.state
            );
        }
    }

    #[test]
    fn compensation_at_every_stage_preserves_no_dup() {
        // Property: compensating at any stage preserves the no-duplicate invariant.
        for stop_at in 0..=2 {
            let mut saga = make_saga();
            for v in 0..stop_at {
                saga.advance(v).unwrap();
            }
            if !saga.state.is_terminal() {
                saga.compensate(stop_at).unwrap();
                assert!(
                    saga.no_duplicate_invariant(),
                    "no-dup after compensating at step {stop_at}"
                );
            }
        }
    }

    #[test]
    fn idempotent_compensate_with_executor_does_not_re_execute() {
        let mut saga = make_saga();
        let mut ledger = AssetLedger::new();
        let mut exec = LedgerExecutor { ledger: &mut ledger };

        saga.advance_with_executor(0, &mut exec).unwrap();
        saga.compensate_with_executor(1, &mut exec).unwrap();
        let state_after_first = exec.ledger.state.clone();

        // Second compensate — idempotent no-op. Executor should NOT be
        // called (we pass a failing executor to prove it).
        let mut fail_exec = FailingExecutor {
            call_count: 0,
            fail_on: 1,
        };
        let state = saga
            .compensate_with_executor(999, &mut fail_exec)
            .unwrap();
        assert_eq!(state, MigrationState::Compensated);
        assert_eq!(fail_exec.call_count, 0); // No calls made.

        // Ledger unchanged.
        assert_eq!(exec.ledger.state, state_after_first);
    }
}
