//! # Application State
//!
//! Shared state for the Axum application, passed to all route handlers
//! via the `State` extractor.
//!
//! ## Phase 1: In-Memory Storage
//!
//! All data is stored in `Arc<RwLock<HashMap>>` behind a generic [`Store`].
//! This provides thread-safe concurrent access without a database dependency.
//! The storage layer is designed so that a `sqlx::PgPool`-backed implementation
//! can replace it without changing route handler signatures.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Generic In-Memory Store ─────────────────────────────────────────

/// Thread-safe, cloneable in-memory key-value store.
///
/// All operations are synchronous (the RwLock is `std::sync`, not `tokio::sync`)
/// because we never hold the lock across `.await` points.
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
        self.data
            .write()
            .expect("store lock poisoned")
            .insert(id, value)
    }

    /// Retrieve a record by ID.
    pub fn get(&self, id: &Uuid) -> Option<T> {
        self.data
            .read()
            .expect("store lock poisoned")
            .get(id)
            .cloned()
    }

    /// List all records.
    pub fn list(&self) -> Vec<T> {
        self.data
            .read()
            .expect("store lock poisoned")
            .values()
            .cloned()
            .collect()
    }

    /// Update a record in place. Returns the updated record, or `None` if not found.
    pub fn update(&self, id: &Uuid, f: impl FnOnce(&mut T)) -> Option<T> {
        let mut guard = self.data.write().expect("store lock poisoned");
        if let Some(entry) = guard.get_mut(id) {
            f(entry);
            Some(entry.clone())
        } else {
            None
        }
    }

    /// Remove a record by ID.
    #[allow(dead_code)]
    pub fn remove(&self, id: &Uuid) -> Option<T> {
        self.data.write().expect("store lock poisoned").remove(id)
    }

    /// Check if a record exists.
    #[allow(dead_code)]
    pub fn contains(&self, id: &Uuid) -> bool {
        self.data
            .read()
            .expect("store lock poisoned")
            .contains_key(id)
    }

    /// Return the number of records.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.data.read().expect("store lock poisoned").len()
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

// ── Stored Record Types ─────────────────────────────────────────────

/// Entity record in storage.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityRecord {
    pub id: Uuid,
    pub entity_type: String,
    pub legal_name: String,
    pub jurisdiction_id: String,
    pub status: String,
    #[serde(default)]
    pub beneficial_owners: Vec<BeneficialOwner>,
    pub dissolution_stage: Option<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Beneficial owner of an entity.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BeneficialOwner {
    pub name: String,
    pub ownership_percentage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ntn: Option<String>,
}

/// Cap table record in storage.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CapTableRecord {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub share_classes: Vec<ShareClass>,
    pub transfers: Vec<OwnershipTransfer>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Share class definition.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShareClass {
    pub name: String,
    pub authorized_shares: u64,
    pub issued_shares: u64,
    pub par_value: Option<String>,
    pub voting_rights: bool,
}

/// Ownership transfer event.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OwnershipTransfer {
    pub id: Uuid,
    pub from_holder: String,
    pub to_holder: String,
    pub share_class: String,
    pub quantity: u64,
    pub price_per_share: Option<String>,
    pub transferred_at: DateTime<Utc>,
}

/// Fiscal account record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FiscalAccountRecord {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub account_type: String,
    pub currency: String,
    pub balance: String,
    pub ntn: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Payment record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentRecord {
    pub id: Uuid,
    pub from_account_id: Uuid,
    pub to_account_id: Option<Uuid>,
    pub amount: String,
    pub currency: String,
    pub reference: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Tax event record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxEventRecord {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub event_type: String,
    pub amount: String,
    pub currency: String,
    pub tax_year: String,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Identity record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IdentityRecord {
    pub id: Uuid,
    pub identity_type: String,
    pub status: String,
    pub linked_ids: Vec<LinkedExternalId>,
    pub attestations: Vec<IdentityAttestation>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// External ID linked to an identity.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LinkedExternalId {
    pub id_type: String,
    pub id_value: String,
    pub verified: bool,
    pub linked_at: DateTime<Utc>,
}

/// Identity attestation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IdentityAttestation {
    pub id: Uuid,
    pub attestation_type: String,
    pub issuer: String,
    pub status: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Consent record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConsentRecord {
    pub id: Uuid,
    pub consent_type: String,
    pub description: String,
    pub parties: Vec<ConsentParty>,
    pub status: String,
    pub audit_trail: Vec<ConsentAuditEntry>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Party involved in a consent request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConsentParty {
    pub entity_id: Uuid,
    pub role: String,
    pub decision: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
}

/// Audit trail entry for a consent.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConsentAuditEntry {
    pub action: String,
    pub actor_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub details: Option<String>,
}

/// Corridor record (API-layer representation).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CorridorRecord {
    pub id: Uuid,
    pub jurisdiction_a: String,
    pub jurisdiction_b: String,
    pub state: String,
    pub transition_log: Vec<CorridorTransitionEntry>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Corridor transition log entry.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CorridorTransitionEntry {
    pub from_state: String,
    pub to_state: String,
    pub timestamp: DateTime<Utc>,
    pub evidence_digest: Option<String>,
}

/// Smart asset record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SmartAssetRecord {
    pub id: Uuid,
    pub asset_type: String,
    pub jurisdiction_id: String,
    pub status: String,
    pub genesis_digest: Option<String>,
    pub compliance_status: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Attestation record for regulator queries.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttestationRecord {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub attestation_type: String,
    pub issuer: String,
    pub status: String,
    pub jurisdiction_id: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub details: serde_json::Value,
}

// ── Application State ───────────────────────────────────────────────

/// Application configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Port to bind the HTTP server to.
    pub port: u16,
    /// Static bearer token for Phase 1 authentication.
    /// If `None`, authentication is disabled.
    pub auth_token: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            auth_token: None,
        }
    }
}

/// Shared application state accessible to all route handlers.
///
/// Contains in-memory stores for each domain primitive and application
/// configuration. Clone-friendly via `Arc` internals in each `Store`.
#[derive(Debug, Clone)]
pub struct AppState {
    pub entities: Store<EntityRecord>,
    pub cap_tables: Store<CapTableRecord>,
    pub fiscal_accounts: Store<FiscalAccountRecord>,
    pub payments: Store<PaymentRecord>,
    pub tax_events: Store<TaxEventRecord>,
    pub identities: Store<IdentityRecord>,
    pub consents: Store<ConsentRecord>,
    pub corridors: Store<CorridorRecord>,
    pub smart_assets: Store<SmartAssetRecord>,
    pub attestations: Store<AttestationRecord>,
    pub config: AppConfig,
}

impl AppState {
    /// Create a new application state with default configuration.
    pub fn new() -> Self {
        Self::with_config(AppConfig::default())
    }

    /// Create a new application state with the given configuration.
    pub fn with_config(config: AppConfig) -> Self {
        Self {
            entities: Store::new(),
            cap_tables: Store::new(),
            fiscal_accounts: Store::new(),
            payments: Store::new(),
            tax_events: Store::new(),
            identities: Store::new(),
            consents: Store::new(),
            corridors: Store::new(),
            smart_assets: Store::new(),
            attestations: Store::new(),
            config,
        }
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

    /// Helper: create a minimal EntityRecord for store tests.
    fn sample_entity(id: Uuid) -> EntityRecord {
        let now = Utc::now();
        EntityRecord {
            id,
            entity_type: "llc".to_string(),
            legal_name: "Acme Corp".to_string(),
            jurisdiction_id: "pk-sez-01".to_string(),
            status: "active".to_string(),
            beneficial_owners: vec![],
            dissolution_stage: None,
            created_at: now,
            updated_at: now,
        }
    }

    // ── Store tests ───────────────────────────────────────────────

    #[test]
    fn store_new_creates_empty_store() {
        let store: Store<EntityRecord> = Store::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.list().is_empty());
    }

    #[test]
    fn store_insert_and_get_roundtrip() {
        let store = Store::new();
        let id = Uuid::new_v4();
        let entity = sample_entity(id);

        let prev = store.insert(id, entity.clone());
        assert!(prev.is_none(), "first insert should return None");

        let retrieved = store.get(&id);
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.legal_name, "Acme Corp");
    }

    #[test]
    fn store_insert_returns_previous_value() {
        let store = Store::new();
        let id = Uuid::new_v4();

        store.insert(id, sample_entity(id));
        let prev = store.insert(id, sample_entity(id));
        assert!(prev.is_some(), "second insert should return previous value");
    }

    #[test]
    fn store_list_returns_all_items() {
        let store = Store::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        store.insert(id1, sample_entity(id1));
        store.insert(id2, sample_entity(id2));
        store.insert(id3, sample_entity(id3));

        let all = store.list();
        assert_eq!(all.len(), 3);

        let ids: Vec<Uuid> = all.iter().map(|e| e.id).collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert!(ids.contains(&id3));
    }

    #[test]
    fn store_update_modifies_existing() {
        let store = Store::new();
        let id = Uuid::new_v4();
        store.insert(id, sample_entity(id));

        let updated = store.update(&id, |e| {
            e.legal_name = "Updated Corp".to_string();
            e.status = "suspended".to_string();
        });

        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.legal_name, "Updated Corp");
        assert_eq!(updated.status, "suspended");

        // Confirm the store itself reflects the change.
        let fetched = store.get(&id).unwrap();
        assert_eq!(fetched.legal_name, "Updated Corp");
    }

    #[test]
    fn store_update_returns_none_for_missing_key() {
        let store: Store<EntityRecord> = Store::new();
        let missing = Uuid::new_v4();
        let result = store.update(&missing, |e| {
            e.legal_name = "Ghost".to_string();
        });
        assert!(result.is_none());
    }

    #[test]
    fn store_remove_deletes_item() {
        let store = Store::new();
        let id = Uuid::new_v4();
        store.insert(id, sample_entity(id));
        assert_eq!(store.len(), 1);

        let removed = store.remove(&id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, id);

        assert!(store.is_empty());
        assert!(store.get(&id).is_none());
    }

    #[test]
    fn store_remove_returns_none_for_missing_key() {
        let store: Store<EntityRecord> = Store::new();
        let result = store.remove(&Uuid::new_v4());
        assert!(result.is_none());
    }

    #[test]
    fn store_contains_checks_existence() {
        let store = Store::new();
        let id = Uuid::new_v4();
        assert!(!store.contains(&id));

        store.insert(id, sample_entity(id));
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
        store.insert(id1, sample_entity(id1));
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);

        store.insert(id2, sample_entity(id2));
        assert_eq!(store.len(), 2);

        store.remove(&id1);
        assert_eq!(store.len(), 1);

        store.remove(&id2);
        assert!(store.is_empty());
    }

    #[test]
    fn store_default_is_empty() {
        let store: Store<EntityRecord> = Store::default();
        assert!(store.is_empty());
    }

    #[test]
    fn store_clone_shares_underlying_data() {
        let store = Store::new();
        let id = Uuid::new_v4();
        store.insert(id, sample_entity(id));

        let clone = store.clone();
        assert_eq!(clone.len(), 1);
        assert!(clone.contains(&id));

        // Mutations through the clone are visible from the original.
        let id2 = Uuid::new_v4();
        clone.insert(id2, sample_entity(id2));
        assert_eq!(store.len(), 2);
    }

    // ── AppState tests ────────────────────────────────────────────

    #[test]
    fn app_state_new_creates_empty_stores() {
        let state = AppState::new();
        assert!(state.entities.is_empty());
        assert!(state.cap_tables.is_empty());
        assert!(state.fiscal_accounts.is_empty());
        assert!(state.payments.is_empty());
        assert!(state.tax_events.is_empty());
        assert!(state.identities.is_empty());
        assert!(state.consents.is_empty());
        assert!(state.corridors.is_empty());
        assert!(state.smart_assets.is_empty());
        assert!(state.attestations.is_empty());
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
        let state = AppState::with_config(config);
        assert_eq!(state.config.port, 3000);
        assert_eq!(state.config.auth_token.as_deref(), Some("secret-token"));
        assert!(state.entities.is_empty());
    }

    #[test]
    fn app_state_default_equals_new() {
        let default_state = AppState::default();
        let new_state = AppState::new();
        assert_eq!(default_state.config.port, new_state.config.port);
        assert_eq!(default_state.config.auth_token, new_state.config.auth_token);
    }
}
