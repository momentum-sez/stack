// SPDX-License-Identifier: BUSL-1.1
//! In-memory storage backend using DashMap.
//!
//! Each resource type gets its own `DashMap<Uuid, serde_json::Value>`.
//! Org-keyed lookups use `DashMap<String, Vec<Value>>` for members/board/shareholders.

use std::sync::Arc;

use dashmap::DashMap;
use serde_json::Value;
use uuid::Uuid;

/// Inner storage holding all DashMaps.
struct Inner {
    organizations: DashMap<Uuid, Value>,
    treasuries: DashMap<Uuid, Value>,
    consents: DashMap<Uuid, Value>,
    investments: DashMap<Uuid, Value>,
    // Fiscal
    accounts: DashMap<Uuid, Value>,
    transactions: DashMap<Uuid, Value>,
    tax_events: DashMap<Uuid, Value>,
    // Ownership
    cap_tables: DashMap<Uuid, Value>,
    // Templating
    templates: DashMap<String, Value>,
    submissions: DashMap<String, Value>,
    // Identity: org-keyed lookups
    members_by_org: DashMap<String, Vec<Value>>,
    board_by_org: DashMap<String, Vec<Value>>,
    shareholders_by_org: DashMap<String, Vec<Value>>,
}

/// Shared application state holding all in-memory stores.
///
/// Cheaply cloneable via `Arc` — all clones share the same data.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
    /// Optional bearer token for auth middleware. Read from
    /// `MASS_STUB_AUTH_TOKEN` env var at construction time.
    auth_token: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        let auth_token = std::env::var("MASS_STUB_AUTH_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());
        Self {
            inner: Arc::new(Inner {
                organizations: DashMap::new(),
                treasuries: DashMap::new(),
                consents: DashMap::new(),
                investments: DashMap::new(),
                accounts: DashMap::new(),
                transactions: DashMap::new(),
                tax_events: DashMap::new(),
                cap_tables: DashMap::new(),
                templates: DashMap::new(),
                submissions: DashMap::new(),
                members_by_org: DashMap::new(),
                board_by_org: DashMap::new(),
                shareholders_by_org: DashMap::new(),
            }),
            auth_token,
        }
    }

    pub fn auth_token(&self) -> Option<&str> {
        self.auth_token.as_deref()
    }

    pub fn organizations(&self) -> &DashMap<Uuid, Value> {
        &self.inner.organizations
    }

    pub fn treasuries(&self) -> &DashMap<Uuid, Value> {
        &self.inner.treasuries
    }

    pub fn consents(&self) -> &DashMap<Uuid, Value> {
        &self.inner.consents
    }

    pub fn investments(&self) -> &DashMap<Uuid, Value> {
        &self.inner.investments
    }

    pub fn accounts(&self) -> &DashMap<Uuid, Value> {
        &self.inner.accounts
    }

    pub fn transactions(&self) -> &DashMap<Uuid, Value> {
        &self.inner.transactions
    }

    pub fn tax_events(&self) -> &DashMap<Uuid, Value> {
        &self.inner.tax_events
    }

    pub fn cap_tables(&self) -> &DashMap<Uuid, Value> {
        &self.inner.cap_tables
    }

    pub fn templates(&self) -> &DashMap<String, Value> {
        &self.inner.templates
    }

    pub fn submissions(&self) -> &DashMap<String, Value> {
        &self.inner.submissions
    }

    pub fn members_by_org(&self) -> &DashMap<String, Vec<Value>> {
        &self.inner.members_by_org
    }

    pub fn board_by_org(&self) -> &DashMap<String, Vec<Value>> {
        &self.inner.board_by_org
    }

    pub fn shareholders_by_org(&self) -> &DashMap<String, Vec<Value>> {
        &self.inner.shareholders_by_org
    }
}
