//! # CONSENT Primitive — Consent Info API
//!
//! Routes:
//! - POST   /v1/consent/request — Request multi-party consent
//! - GET    /v1/consent/{consent_id} — Consent status
//! - POST   /v1/consent/{consent_id}/sign — Sign consent
//! - GET    /v1/consent/{consent_id}/audit-trail — Full audit history

/// Placeholder for consent router.
pub fn router() -> axum::Router<crate::AppState> {
    axum::Router::new()
}
