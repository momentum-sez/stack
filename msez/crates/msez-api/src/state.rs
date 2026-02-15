//! # Application State
//!
//! Shared state for the Axum application, passed to all route handlers
//! via the `State` extractor.
//!
//! ## Architecture
//!
//! AppState holds only SEZ-Stack-owned concerns:
//! - **Corridors** — cross-border corridor lifecycle (SEZ Stack domain)
//! - **Smart Assets** — smart asset lifecycle (SEZ Stack domain)
//! - **Attestations** — compliance attestations for regulator queries (SEZ Stack domain)
//! - **Tax Events** — tax collection pipeline events and withholding records (SEZ Stack domain)
//! - **Mass API client** — typed client delegating primitive operations to live Mass APIs
//!
//! Entity, ownership, fiscal, identity, and consent data is NOT stored here.
//! That data lives in the Mass APIs and is accessed via `msez-mass-client`.
//! See CLAUDE.md Section II.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use msez_agentic::{PolicyEngine, TaxPipeline};
use msez_corridor::ReceiptChain;
use msez_crypto::SigningKey;
use msez_state::{DynCorridorState, TransitionRecord};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

// -- Generic In-Memory Store --------------------------------------------------

/// Thread-safe, cloneable in-memory key-value store.
///
/// All operations are synchronous (the RwLock is `parking_lot`, not `tokio::sync`)
/// because we never hold the lock across `.await` points. `parking_lot::RwLock`
/// is non-poisonable — a panicking writer does not permanently corrupt the store.
#[derive(Debug)]
pub struct Store<T: Clone + Send + Sync> {
    data: Arc<RwLock<HashMap<Uuid, T>>>,
}

impl<T: Clone + Send + Sync> Clone for Store<T> {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
        }
    }
}

impl<T: Clone + Send + Sync> Store<T> {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert a record, returning the previous value if the key existed.
    pub fn insert(&self, id: Uuid, value: T) -> Option<T> {
        self.data.write().insert(id, value)
    }

    /// Retrieve a record by ID.
    pub fn get(&self, id: &Uuid) -> Option<T> {
        self.data.read().get(id).cloned()
    }

    /// List all records.
    pub fn list(&self) -> Vec<T> {
        self.data.read().values().cloned().collect()
    }

    /// Update a record in place. Returns the updated record, or `None` if not found.
    pub fn update(&self, id: &Uuid, f: impl FnOnce(&mut T)) -> Option<T> {
        let mut guard = self.data.write();
        if let Some(entry) = guard.get_mut(id) {
            f(entry);
            Some(entry.clone())
        } else {
            None
        }
    }

    /// Atomically read-validate-update a record.
    ///
    /// The closure receives a `&mut T` and may inspect the current state,
    /// validate preconditions, mutate the record, and return `Ok(R)` or
    /// `Err(E)`. The entire operation runs under a single write lock,
    /// eliminating TOCTOU races between read and update.
    ///
    /// Returns `None` if the record doesn't exist, or `Some(result)` with
    /// the closure's `Result`.
    pub fn try_update<R, E>(
        &self,
        id: &Uuid,
        f: impl FnOnce(&mut T) -> Result<R, E>,
    ) -> Option<Result<R, E>> {
        self.data.write().get_mut(id).map(f)
    }

    /// Remove a record by ID.
    #[allow(dead_code)]
    pub fn remove(&self, id: &Uuid) -> Option<T> {
        self.data.write().remove(id)
    }

    /// Check if a record exists.
    #[allow(dead_code)]
    pub fn contains(&self, id: &Uuid) -> bool {
        self.data.read().contains_key(id)
    }

    /// Return the number of records.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.data.read().len()
    }

    /// Whether the store is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T: Clone + Send + Sync> Default for Store<T> {
    fn default() -> Self {
        Self::new()
    }
}

// -- SEZ-Stack-Owned Record Types ---------------------------------------------
//
// These types represent data that genuinely belongs to the SEZ Stack,
// NOT Mass primitive data. Mass primitives are accessed via msez-mass-client.

/// Corridor record (API-layer representation).
///
/// Uses [`DynCorridorState`] from `msez-state` for the corridor state, ensuring
/// only spec-aligned state names are representable. The transition log uses
/// [`TransitionRecord`] which carries `Option<ContentDigest>` evidence.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CorridorRecord {
    pub id: Uuid,
    pub jurisdiction_a: String,
    pub jurisdiction_b: String,
    /// Current corridor lifecycle state (DRAFT, PENDING, ACTIVE, HALTED, SUSPENDED, DEPRECATED).
    #[schema(value_type = String)]
    pub state: DynCorridorState,
    /// Audit trail of state transitions with evidence digests.
    #[schema(value_type = Vec<Object>)]
    pub transition_log: Vec<TransitionRecord>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Compliance status for a smart asset.
///
/// This is a simplified classification derived from the compliance tensor's
/// [`ComplianceState`] lattice. The tensor performs algebraic evaluation across
/// 20 compliance domains; this enum collapses that into an API-layer status
/// suitable for storage and display. The conversion discards lattice semantics
/// (meet, join, absorbing element) that are not needed at rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetComplianceStatus {
    /// All applicable domains are passing.
    Compliant,
    /// Evaluation has not been performed or is incomplete.
    Pending,
    /// At least one domain is non-compliant.
    NonCompliant,
    /// At least one domain is pending but none are non-compliant.
    PartiallyCompliant,
    /// Compliance has not been evaluated for this asset.
    Unevaluated,
}

/// Smart asset lifecycle status.
///
/// Represents the stages of a smart asset's lifecycle from genesis through
/// retirement. Uses `SCREAMING_CASE` for serialization to match the API
/// contract and prevent the defective-string problem that plagued corridor
/// states in the Python v1 implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssetStatus {
    /// Initial creation — genesis document submitted.
    Genesis,
    /// Registry VC bound to the asset.
    Registered,
    /// Asset is operational and actively managed.
    Active,
    /// Awaiting compliance evaluation or other prerequisite.
    Pending,
    /// Temporarily suspended (compliance hold, dispute, etc.).
    Suspended,
    /// End of lifecycle — asset decommissioned.
    Retired,
}

impl AssetStatus {
    /// Return the string representation of this status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Genesis => "GENESIS",
            Self::Registered => "REGISTERED",
            Self::Active => "ACTIVE",
            Self::Pending => "PENDING",
            Self::Suspended => "SUSPENDED",
            Self::Retired => "RETIRED",
        }
    }
}

impl std::fmt::Display for AssetStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Validated smart asset type.
///
/// Serializes/deserializes as a plain string for backward compatibility.
/// Validated on construction via [`SmartAssetType::new`] to ensure non-empty
/// and within length limits (max 255 characters).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = String)]
pub struct SmartAssetType(String);

impl SmartAssetType {
    /// Create a validated smart asset type.
    ///
    /// Returns an error if the string is empty or exceeds 255 characters.
    pub fn new(s: impl Into<String>) -> Result<Self, String> {
        let s = s.into();
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            return Err("asset_type must not be empty".to_string());
        }
        if trimmed.len() > 255 {
            return Err("asset_type must not exceed 255 characters".to_string());
        }
        Ok(Self(trimmed))
    }

    /// Return the asset type as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SmartAssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<&str> for SmartAssetType {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// Smart asset record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SmartAssetRecord {
    pub id: Uuid,
    pub asset_type: SmartAssetType,
    pub jurisdiction_id: String,
    pub status: AssetStatus,
    pub genesis_digest: Option<String>,
    pub compliance_status: AssetComplianceStatus,
    pub metadata: serde_json::Value,
    /// The entity that created this asset. Used for IDOR protection.
    /// `None` for assets created before RBAC was enabled (legacy).
    #[serde(default)]
    pub owner_entity_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Attestation lifecycle status.
///
/// Represents the state of a compliance attestation throughout its
/// validity period. Prevents invalid string values from being stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttestationStatus {
    /// Attestation is current and valid.
    Active,
    /// Attestation is awaiting verification or issuance.
    Pending,
    /// Attestation has been explicitly revoked.
    Revoked,
    /// Attestation has passed its expiry date.
    Expired,
}

impl AttestationStatus {
    /// Return the string representation of this status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Pending => "PENDING",
            Self::Revoked => "REVOKED",
            Self::Expired => "EXPIRED",
        }
    }
}

impl std::fmt::Display for AttestationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Attestation record for regulator queries.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttestationRecord {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub attestation_type: String,
    pub issuer: String,
    pub status: AttestationStatus,
    pub jurisdiction_id: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub details: serde_json::Value,
}

/// Tax event record stored by the tax collection pipeline.
///
/// Wraps a [`msez_agentic::TaxEvent`] with withholding results and pipeline
/// status for API-layer persistence. Tax events are SEZ-Stack-owned data —
/// they represent the jurisdictional tax awareness applied to Mass fiscal
/// operations, not Mass fiscal CRUD.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxEventRecord {
    /// Unique identifier (matches the inner event_id).
    pub id: Uuid,
    /// Entity subject to this tax event.
    pub entity_id: Uuid,
    /// Event type classification.
    pub event_type: String,
    /// Tax category.
    pub tax_category: String,
    /// Jurisdiction where the tax obligation arises.
    pub jurisdiction_id: String,
    /// Gross amount of the economic activity.
    pub gross_amount: String,
    /// Total computed withholding amount.
    pub withholding_amount: String,
    /// Net amount after withholding.
    pub net_amount: String,
    /// Currency code.
    pub currency: String,
    /// Tax year.
    pub tax_year: String,
    /// Entity NTN, if registered.
    pub ntn: Option<String>,
    /// Filing status of the entity.
    pub filer_status: String,
    /// Statutory section reference.
    pub statutory_section: Option<String>,
    /// Whether withholding has been executed via Mass fiscal API.
    pub withholding_executed: bool,
    /// Reference to the originating Mass payment.
    pub mass_payment_id: Option<Uuid>,
    /// Number of withholding rules that matched.
    pub rules_applied: usize,
    /// When the event was recorded.
    pub created_at: DateTime<Utc>,
}

// -- Application State --------------------------------------------------------

/// Application configuration.
///
/// Custom `Debug` redacts the `auth_token` to prevent credential leakage in logs.
#[derive(Clone)]
pub struct AppConfig {
    /// Port to bind the HTTP server to.
    pub port: u16,
    /// Static bearer token for Phase 1 authentication.
    /// If `None`, authentication is disabled.
    pub auth_token: Option<String>,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("port", &self.port)
            .field(
                "auth_token",
                &self.auth_token.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            auth_token: None,
        }
    }
}

/// Decode a hex string into bytes.
fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(format!("hex string has odd length: {}", s.len()));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| format!("invalid hex at position {i}: {e}"))
        })
        .collect()
}

/// Error loading the zone signing key from the environment.
#[derive(Debug)]
pub enum ZoneKeyError {
    /// `ZONE_SIGNING_KEY_HEX` contained invalid hex characters.
    InvalidHex(String),
    /// `ZONE_SIGNING_KEY_HEX` decoded to the wrong number of bytes.
    InvalidLength { expected: usize, actual: usize },
}

impl std::fmt::Display for ZoneKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHex(msg) => write!(f, "ZONE_SIGNING_KEY_HEX invalid hex: {msg}"),
            Self::InvalidLength { expected, actual } => write!(
                f,
                "ZONE_SIGNING_KEY_HEX must be exactly {} hex chars ({expected} bytes), got {actual} bytes",
                expected * 2
            ),
        }
    }
}

impl std::error::Error for ZoneKeyError {}

/// Load the zone signing key from the environment, or generate one for development.
///
/// In production, `ZONE_SIGNING_KEY_HEX` provides the 64-character hex-encoded
/// Ed25519 private key (32 bytes). In development (when the variable is absent),
/// a fresh key is generated and a warning is logged.
///
/// Returns `Err` if the environment variable is set but contains invalid data,
/// rather than panicking the server on startup.
fn load_or_generate_zone_key() -> Result<SigningKey, ZoneKeyError> {
    if let Ok(hex) = std::env::var("ZONE_SIGNING_KEY_HEX") {
        let bytes = hex_decode(&hex).map_err(ZoneKeyError::InvalidHex)?;
        if bytes.len() != 32 {
            return Err(ZoneKeyError::InvalidLength {
                expected: 32,
                actual: bytes.len(),
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(SigningKey::from_bytes(&arr))
    } else {
        tracing::warn!(
            "ZONE_SIGNING_KEY_HEX not set — generating ephemeral key. \
             VCs signed with this key will not be verifiable after restart."
        );
        Ok(SigningKey::generate(&mut rand_core::OsRng))
    }
}

/// Shared application state accessible to all route handlers.
///
/// Contains SEZ-Stack-owned stores (corridors, smart assets, attestations),
/// the Mass API client for primitive operations, the zone signing key for
/// VC issuance, and application configuration.
/// Clone-friendly via `Arc` internals in each `Store`.
///
/// ## What is NOT here
///
/// Entity, ownership, fiscal, identity, and consent stores have been removed.
/// That data lives in the Mass APIs and is accessed via `state.mass_client`.
#[derive(Debug, Clone)]
pub struct AppState {
    // -- SEZ Stack owned state --
    pub corridors: Store<CorridorRecord>,
    pub smart_assets: Store<SmartAssetRecord>,
    pub attestations: Store<AttestationRecord>,
    pub tax_events: Store<TaxEventRecord>,

    // -- Tax collection pipeline --
    /// The tax collection pipeline orchestrator. Contains the withholding
    /// computation engine with jurisdiction-specific rules loaded from regpacks.
    /// `parking_lot::Mutex` because the pipeline may be reconfigured at runtime
    /// (e.g., when new SRO rates are loaded).
    pub tax_pipeline: Arc<Mutex<TaxPipeline>>,

    /// Per-corridor receipt chains (append-only MMR accumulators).
    ///
    /// `ReceiptChain` is not `Clone` (it wraps a mutable `MerkleMountainRange`),
    /// so it cannot use the generic `Store<T>`. A direct `Arc<RwLock<HashMap>>`
    /// is used instead. Keyed by corridor UUID — each corridor has exactly one chain.
    pub receipt_chains: Arc<RwLock<HashMap<Uuid, ReceiptChain>>>,

    // -- Database persistence (optional) --
    /// PostgreSQL connection pool for durable state persistence.
    /// When `Some`, corridor, smart asset, attestation, and audit data is
    /// persisted to Postgres in addition to the in-memory stores.
    /// When `None`, the API operates in in-memory-only mode.
    pub db_pool: Option<PgPool>,

    // -- Mass API client (delegates primitive operations to live Mass APIs) --
    pub mass_client: Option<msez_mass_client::MassClient>,

    // -- Zone identity --
    /// The zone operator's Ed25519 signing key.
    /// Used to sign Verifiable Credentials issued by this zone.
    /// Wrapped in `Arc` because `SigningKey` is not `Clone` (it contains
    /// sensitive key material that shouldn't be casually duplicated).
    pub zone_signing_key: Arc<SigningKey>,

    /// The zone operator's DID, derived from the public key.
    /// Format: `"did:mass:zone:<hex-encoded-verifying-key>"`
    pub zone_did: String,

    // -- Agentic policy engine --
    /// The autonomous policy engine. `parking_lot::Mutex` because `PolicyEngine`
    /// is not `Sync` (it holds a mutable audit trail) and evaluation mutates
    /// internal state. `parking_lot::Mutex` never poisons on panic, eliminating
    /// the entire class of lock-poisoning runtime failures.
    pub policy_engine: Arc<Mutex<PolicyEngine>>,

    // -- Zone context (from bootstrap) --
    /// Zone context, if bootstrapped from a zone manifest.
    /// When present, the server operates as a configured zone node.
    /// When absent (generic mode), endpoints use default behavior.
    pub zone: Option<crate::bootstrap::ZoneContext>,

    // -- Configuration --
    pub config: AppConfig,
}

impl AppState {
    /// Create a new application state with default configuration and no Mass client.
    ///
    /// # Panics
    ///
    /// Panics if `ZONE_SIGNING_KEY_HEX` is set but contains invalid data.
    /// In production, prefer [`AppState::try_new`] for graceful error handling.
    pub fn new() -> Self {
        Self::try_with_config(AppConfig::default(), None, None)
            .expect("failed to initialize AppState (check ZONE_SIGNING_KEY_HEX)")
    }

    /// Create a new application state, returning `Err` if zone key loading fails.
    pub fn try_new() -> Result<Self, ZoneKeyError> {
        Self::try_with_config(AppConfig::default(), None, None)
    }

    /// Create a new application state with the given configuration and optional Mass client.
    ///
    /// # Panics
    ///
    /// Panics if `ZONE_SIGNING_KEY_HEX` is set but contains invalid data.
    pub fn with_config(
        config: AppConfig,
        mass_client: Option<msez_mass_client::MassClient>,
    ) -> Self {
        Self::try_with_config(config, mass_client, None)
            .expect("failed to initialize AppState (check ZONE_SIGNING_KEY_HEX)")
    }

    /// Create a new application state with the given configuration, optional Mass client,
    /// and optional database pool, returning `Err` on zone key loading failures.
    pub fn try_with_config(
        config: AppConfig,
        mass_client: Option<msez_mass_client::MassClient>,
        db_pool: Option<PgPool>,
    ) -> Result<Self, ZoneKeyError> {
        let zone_signing_key = load_or_generate_zone_key()?;
        let zone_did = format!(
            "did:mass:zone:{}",
            zone_signing_key.verifying_key().to_hex()
        );

        Ok(Self {
            corridors: Store::new(),
            smart_assets: Store::new(),
            attestations: Store::new(),
            tax_events: Store::new(),
            tax_pipeline: Arc::new(Mutex::new(TaxPipeline::pakistan())),
            receipt_chains: Arc::new(RwLock::new(HashMap::new())),
            db_pool,
            mass_client,
            zone_signing_key: Arc::new(zone_signing_key),
            zone_did,
            policy_engine: Arc::new(Mutex::new(PolicyEngine::with_extended_policies())),
            zone: None,
            config,
        })
    }

    /// Hydrate in-memory stores from the database.
    ///
    /// Called once on startup when a database pool is available. Loads all
    /// persisted corridors, smart assets, and attestations into the in-memory
    /// stores so that read operations remain fast and synchronous.
    pub async fn hydrate_from_db(&self) -> Result<(), String> {
        let pool = match &self.db_pool {
            Some(pool) => pool,
            None => return Ok(()),
        };

        // Load corridors
        let corridors = crate::db::corridors::load_all(pool)
            .await
            .map_err(|e| format!("failed to load corridors: {e}"))?;
        let corridor_count = corridors.len();
        for record in corridors {
            self.corridors.insert(record.id, record);
        }

        // Load smart assets
        let assets = crate::db::smart_assets::load_all(pool)
            .await
            .map_err(|e| format!("failed to load smart assets: {e}"))?;
        let asset_count = assets.len();
        for record in assets {
            self.smart_assets.insert(record.id, record);
        }

        // Load attestations
        let attestations = crate::db::attestations::load_all(pool)
            .await
            .map_err(|e| format!("failed to load attestations: {e}"))?;
        let attestation_count = attestations.len();
        for record in attestations {
            self.attestations.insert(record.id, record);
        }

        // Load tax events
        let tax_events = crate::db::tax_events::load_all(pool)
            .await
            .map_err(|e| format!("failed to load tax events: {e}"))?;
        let tax_event_count = tax_events.len();
        for record in tax_events {
            self.tax_events.insert(record.id, record);
        }

        tracing::info!(
            corridors = corridor_count,
            smart_assets = asset_count,
            attestations = attestation_count,
            tax_events = tax_event_count,
            "Hydrated in-memory stores from database"
        );

        Ok(())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// Helper: create a minimal CorridorRecord for store tests.
    fn sample_corridor(id: Uuid) -> CorridorRecord {
        let now = Utc::now();
        CorridorRecord {
            id,
            jurisdiction_a: "PK-PSEZ".to_string(),
            jurisdiction_b: "AE-DIFC".to_string(),
            state: DynCorridorState::Pending,
            transition_log: vec![],
            created_at: now,
            updated_at: now,
        }
    }

    // -- Store tests ----------------------------------------------------------

    #[test]
    fn store_new_creates_empty_store() {
        let store: Store<CorridorRecord> = Store::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.list().is_empty());
    }

    #[test]
    fn store_insert_and_get_roundtrip() {
        let store = Store::new();
        let id = Uuid::new_v4();
        let corridor = sample_corridor(id);

        let prev = store.insert(id, corridor);
        assert!(prev.is_none(), "first insert should return None");

        let retrieved = store.get(&id);
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.jurisdiction_a, "PK-PSEZ");
    }

    #[test]
    fn store_insert_returns_previous_value() {
        let store = Store::new();
        let id = Uuid::new_v4();

        store.insert(id, sample_corridor(id));
        let prev = store.insert(id, sample_corridor(id));
        assert!(prev.is_some(), "second insert should return previous value");
    }

    #[test]
    fn store_list_returns_all_items() {
        let store = Store::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        store.insert(id1, sample_corridor(id1));
        store.insert(id2, sample_corridor(id2));
        store.insert(id3, sample_corridor(id3));

        let all = store.list();
        assert_eq!(all.len(), 3);

        let ids: Vec<Uuid> = all.iter().map(|c| c.id).collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert!(ids.contains(&id3));
    }

    #[test]
    fn store_update_modifies_existing() {
        let store = Store::new();
        let id = Uuid::new_v4();
        store.insert(id, sample_corridor(id));

        let updated = store.update(&id, |c| {
            c.state = DynCorridorState::Active;
        });

        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.state, DynCorridorState::Active);

        let fetched = store.get(&id).unwrap();
        assert_eq!(fetched.state, DynCorridorState::Active);
    }

    #[test]
    fn store_update_returns_none_for_missing_key() {
        let store: Store<CorridorRecord> = Store::new();
        let missing = Uuid::new_v4();
        let result = store.update(&missing, |c| {
            c.state = DynCorridorState::Active;
        });
        assert!(result.is_none());
    }

    #[test]
    fn store_remove_deletes_item() {
        let store = Store::new();
        let id = Uuid::new_v4();
        store.insert(id, sample_corridor(id));
        assert_eq!(store.len(), 1);

        let removed = store.remove(&id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, id);

        assert!(store.is_empty());
        assert!(store.get(&id).is_none());
    }

    #[test]
    fn store_remove_returns_none_for_missing_key() {
        let store: Store<CorridorRecord> = Store::new();
        let result = store.remove(&Uuid::new_v4());
        assert!(result.is_none());
    }

    #[test]
    fn store_contains_checks_existence() {
        let store = Store::new();
        let id = Uuid::new_v4();
        assert!(!store.contains(&id));

        store.insert(id, sample_corridor(id));
        assert!(store.contains(&id));

        store.remove(&id);
        assert!(!store.contains(&id));
    }

    #[test]
    fn store_len_and_is_empty() {
        let store = Store::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        store.insert(id1, sample_corridor(id1));
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);

        store.insert(id2, sample_corridor(id2));
        assert_eq!(store.len(), 2);

        store.remove(&id1);
        assert_eq!(store.len(), 1);

        store.remove(&id2);
        assert!(store.is_empty());
    }

    #[test]
    fn store_default_is_empty() {
        let store: Store<CorridorRecord> = Store::default();
        assert!(store.is_empty());
    }

    #[test]
    fn store_clone_shares_underlying_data() {
        let store = Store::new();
        let id = Uuid::new_v4();
        store.insert(id, sample_corridor(id));

        let clone = store.clone();
        assert_eq!(clone.len(), 1);
        assert!(clone.contains(&id));

        // Mutations through the clone are visible from the original.
        let id2 = Uuid::new_v4();
        clone.insert(id2, sample_corridor(id2));
        assert_eq!(store.len(), 2);
    }

    // -- AppState tests -------------------------------------------------------

    #[test]
    fn app_state_new_creates_empty_stores() {
        let state = AppState::new();
        assert!(state.corridors.is_empty());
        assert!(state.smart_assets.is_empty());
        assert!(state.attestations.is_empty());
        assert!(state.tax_events.is_empty());
        assert!(state.mass_client.is_none());
    }

    #[test]
    fn app_state_new_uses_default_config() {
        let state = AppState::new();
        assert_eq!(state.config.port, 8080);
        assert!(state.config.auth_token.is_none());
    }

    #[test]
    fn app_state_with_config_applies_custom_config() {
        let config = AppConfig {
            port: 3000,
            auth_token: Some("secret-token".to_string()),
        };
        let state = AppState::with_config(config, None);
        assert_eq!(state.config.port, 3000);
        assert_eq!(state.config.auth_token.as_deref(), Some("secret-token"));
        assert!(state.corridors.is_empty());
    }

    #[test]
    fn app_state_default_equals_new() {
        let default_state = AppState::default();
        let new_state = AppState::new();
        assert_eq!(default_state.config.port, new_state.config.port);
        assert_eq!(default_state.config.auth_token, new_state.config.auth_token);
    }

    #[test]
    fn app_state_has_zone_signing_key() {
        let state = AppState::new();
        assert!(state.zone_did.starts_with("did:mass:zone:"));
        // DID should contain a 64-char hex-encoded verifying key.
        let hex_part = state.zone_did.strip_prefix("did:mass:zone:").unwrap();
        assert_eq!(hex_part.len(), 64);
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hex_decode_valid() {
        let result = super::hex_decode("deadbeef").unwrap();
        assert_eq!(result, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn hex_decode_odd_length_fails() {
        assert!(super::hex_decode("abc").is_err());
    }

    #[test]
    fn hex_decode_invalid_chars_fails() {
        assert!(super::hex_decode("zzzz").is_err());
    }
}
