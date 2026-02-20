// SPDX-License-Identifier: BUSL-1.1
//! In-memory storage backend using DashMap.
//!
//! Each resource type (organizations, treasuries, consents, investments,
//! templates) gets its own `DashMap<Uuid, serde_json::Value>`.

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
}

/// Shared application state holding all in-memory stores.
///
/// Cheaply cloneable via `Arc` — all clones share the same data.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                organizations: DashMap::new(),
                treasuries: DashMap::new(),
                consents: DashMap::new(),
                investments: DashMap::new(),
            }),
        }
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
}
