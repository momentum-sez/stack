//! # Trade Corridor Instruments — End-to-End Integration Tests
//!
//! Exercises the full trade flow lifecycle through the HTTP API:
//! flow creation, transition submission, compliance evaluation,
//! VC issuance, and attestation storage.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use mez_api::state::AppState;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_app() -> axum::Router {
    let state = AppState::new();
    mez_api::app(state)
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn json_post(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

fn json_get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

// ---------------------------------------------------------------------------
// Test: 7-step Export flow lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn export_flow_full_lifecycle() {
    let app = test_app();

    // Step 1: Create export flow
    let resp = app
        .clone()
        .oneshot(json_post(
            "/v1/trade/flows",
            serde_json::json!({
                "flow_type": "Export",
                "seller": {
                    "party_id": "pk-seller-001",
                    "name": "Pakistan Textiles Ltd"
                },
                "buyer": {
                    "party_id": "ae-buyer-001",
                    "name": "Dubai Import Corp"
                },
                "jurisdiction_id": "pk-sifc"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert!(body.get("compliance").is_some(), "should have compliance summary");
    assert!(body.get("credential").is_some(), "should have VC credential");
    assert!(body.get("attestation_id").is_some(), "should have attestation ID");

    let flow = &body["flow"];
    let flow_id = flow["flow_id"].as_str().expect("flow_id");
    assert_eq!(flow["state"].as_str(), Some("Created"));
    assert_eq!(flow["flow_type"].as_str(), Some("Export"));

    // Step 2: Invoice issue
    let resp = app
        .clone()
        .oneshot(json_post(
            &format!("/v1/trade/flows/{flow_id}/transitions"),
            serde_json::json!({
                "payload": {
                    "kind": "trade.invoice.issue.v1",
                    "invoice": {
                        "invoice_id": "INV-001",
                        "issue_date": "2026-02-20",
                        "seller": { "party_id": "pk-seller-001" },
                        "buyer": { "party_id": "ae-buyer-001" },
                        "total": { "currency": "USD", "value": "50000.00" }
                    }
                }
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["flow"]["state"].as_str(), Some("InvoiceIssued"));
    assert!(body.get("compliance").is_some());

    // Step 3: Invoice accept
    let resp = app
        .clone()
        .oneshot(json_post(
            &format!("/v1/trade/flows/{flow_id}/transitions"),
            serde_json::json!({
                "payload": {
                    "kind": "trade.invoice.accept.v1",
                    "accepted_by_party_id": "ae-buyer-001",
                    "accepted_at": "2026-02-20T12:00:00Z"
                }
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["flow"]["state"].as_str(), Some("InvoiceAccepted"));

    // Step 4: BOL issue
    let resp = app
        .clone()
        .oneshot(json_post(
            &format!("/v1/trade/flows/{flow_id}/transitions"),
            serde_json::json!({
                "payload": {
                    "kind": "trade.bol.issue.v1",
                    "bol": {
                        "bol_id": "BOL-001",
                        "issue_date": "2026-02-21",
                        "carrier": { "party_id": "carrier-001" },
                        "shipper": { "party_id": "pk-seller-001" },
                        "consignee": { "party_id": "ae-buyer-001" },
                        "port_of_loading": "PKQCT",
                        "port_of_discharge": "AEJEA",
                        "goods": [{
                            "description": "Cotton textiles",
                            "packages": "200 cartons"
                        }]
                    }
                }
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["flow"]["state"].as_str(), Some("GoodsShipped"));

    // Step 5: BOL endorse
    let resp = app
        .clone()
        .oneshot(json_post(
            &format!("/v1/trade/flows/{flow_id}/transitions"),
            serde_json::json!({
                "payload": {
                    "kind": "trade.bol.endorse.v1",
                    "from_party_id": "pk-seller-001",
                    "to_party_id": "ae-buyer-001",
                    "endorsed_at": "2026-02-22T10:00:00Z"
                }
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["flow"]["state"].as_str(), Some("BolEndorsed"));

    // Step 6: BOL release
    let resp = app
        .clone()
        .oneshot(json_post(
            &format!("/v1/trade/flows/{flow_id}/transitions"),
            serde_json::json!({
                "payload": {
                    "kind": "trade.bol.release.v1",
                    "released_at": "2026-02-23T10:00:00Z",
                    "released_to_party_id": "ae-buyer-001"
                }
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["flow"]["state"].as_str(), Some("GoodsReleased"));

    // Step 7: Invoice settle
    let resp = app
        .clone()
        .oneshot(json_post(
            &format!("/v1/trade/flows/{flow_id}/transitions"),
            serde_json::json!({
                "payload": {
                    "kind": "trade.invoice.settle.v1",
                    "settled_at": "2026-02-25T10:00:00Z",
                    "amount": { "currency": "USD", "value": "50000.00" }
                }
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["flow"]["state"].as_str(), Some("Settled"));

    // Verify transition count
    let transitions = &body["flow"]["transitions"];
    assert_eq!(transitions.as_array().map(|a| a.len()), Some(6));

    // Verify flow is retrievable via GET
    let resp = app
        .clone()
        .oneshot(json_get(&format!("/v1/trade/flows/{flow_id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["state"].as_str(), Some("Settled"));

    // Verify transitions list
    let resp = app
        .clone()
        .oneshot(json_get(&format!(
            "/v1/trade/flows/{flow_id}/transitions"
        )))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["total"].as_u64(), Some(6));
}

// ---------------------------------------------------------------------------
// Test: LC flow lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn lc_flow_lifecycle() {
    let app = test_app();

    // Create LC flow
    let resp = app
        .clone()
        .oneshot(json_post(
            "/v1/trade/flows",
            serde_json::json!({
                "flow_type": "LetterOfCredit",
                "seller": { "party_id": "pk-seller-001" },
                "buyer": { "party_id": "ae-buyer-001" }
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let flow_id = body["flow"]["flow_id"].as_str().unwrap();

    // lc.issue
    let resp = app
        .clone()
        .oneshot(json_post(
            &format!("/v1/trade/flows/{flow_id}/transitions"),
            serde_json::json!({
                "payload": {
                    "kind": "trade.lc.issue.v1",
                    "lc": {
                        "lc_id": "LC-001",
                        "issue_date": "2026-02-20",
                        "expiry_date": "2026-05-20",
                        "applicant": { "party_id": "ae-buyer-001" },
                        "beneficiary": { "party_id": "pk-seller-001" },
                        "issuing_bank": { "party_id": "bank-001" },
                        "amount": { "currency": "USD", "value": "50000.00" }
                    }
                }
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["flow"]["state"].as_str(), Some("LcIssued"));

    // bol.issue
    let resp = app
        .clone()
        .oneshot(json_post(
            &format!("/v1/trade/flows/{flow_id}/transitions"),
            serde_json::json!({
                "payload": {
                    "kind": "trade.bol.issue.v1",
                    "bol": {
                        "bol_id": "BOL-LC-001",
                        "issue_date": "2026-03-01",
                        "carrier": { "party_id": "carrier-001" },
                        "shipper": { "party_id": "pk-seller-001" },
                        "consignee": { "party_id": "ae-buyer-001" },
                        "port_of_loading": "PKQCT",
                        "port_of_discharge": "AEJEA",
                        "goods": [{ "description": "Textiles", "packages": "100 cartons" }]
                    }
                }
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // lc.present
    let resp = app
        .clone()
        .oneshot(json_post(
            &format!("/v1/trade/flows/{flow_id}/transitions"),
            serde_json::json!({
                "payload": {
                    "kind": "trade.lc.present.v1",
                    "presented_at": "2026-03-10T10:00:00Z",
                    "presented_by_party_id": "pk-seller-001",
                    "document_refs": [{
                        "artifact_type": "invoice",
                        "digest_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    }]
                }
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // lc.honor
    let resp = app
        .clone()
        .oneshot(json_post(
            &format!("/v1/trade/flows/{flow_id}/transitions"),
            serde_json::json!({
                "payload": {
                    "kind": "trade.lc.honor.v1",
                    "decision": "honor",
                    "decision_at": "2026-03-12T10:00:00Z"
                }
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // invoice.settle
    let resp = app
        .clone()
        .oneshot(json_post(
            &format!("/v1/trade/flows/{flow_id}/transitions"),
            serde_json::json!({
                "payload": {
                    "kind": "trade.invoice.settle.v1",
                    "settled_at": "2026-03-15T10:00:00Z",
                    "amount": { "currency": "USD", "value": "50000.00" }
                }
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["flow"]["state"].as_str(), Some("Settled"));
}

// ---------------------------------------------------------------------------
// Negative tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn invalid_transition_returns_422() {
    let app = test_app();

    // Create flow
    let resp = app
        .clone()
        .oneshot(json_post(
            "/v1/trade/flows",
            serde_json::json!({
                "flow_type": "Export",
                "seller": { "party_id": "seller" },
                "buyer": { "party_id": "buyer" }
            }),
        ))
        .await
        .unwrap();
    let body = body_json(resp).await;
    let flow_id = body["flow"]["flow_id"].as_str().unwrap();

    // Try to settle without issuing invoice first
    let resp = app
        .clone()
        .oneshot(json_post(
            &format!("/v1/trade/flows/{flow_id}/transitions"),
            serde_json::json!({
                "payload": {
                    "kind": "trade.invoice.settle.v1",
                    "settled_at": "2026-02-25T10:00:00Z",
                    "amount": { "currency": "USD", "value": "50000.00" }
                }
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn get_nonexistent_flow_returns_404() {
    let app = test_app();
    let resp = app
        .clone()
        .oneshot(json_get(
            "/v1/trade/flows/00000000-0000-0000-0000-000000000000",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn transition_to_nonexistent_flow_returns_404() {
    let app = test_app();
    let resp = app
        .clone()
        .oneshot(json_post(
            "/v1/trade/flows/00000000-0000-0000-0000-000000000000/transitions",
            serde_json::json!({
                "payload": {
                    "kind": "trade.invoice.issue.v1"
                }
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_flows_returns_empty_initially() {
    let app = test_app();
    let resp = app
        .clone()
        .oneshot(json_get("/v1/trade/flows"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["total"].as_u64(), Some(0));
}

#[tokio::test]
async fn list_flows_after_creation() {
    let app = test_app();

    // Create two flows
    app.clone()
        .oneshot(json_post(
            "/v1/trade/flows",
            serde_json::json!({
                "flow_type": "Export",
                "seller": { "party_id": "s1" },
                "buyer": { "party_id": "b1" }
            }),
        ))
        .await
        .unwrap();

    app.clone()
        .oneshot(json_post(
            "/v1/trade/flows",
            serde_json::json!({
                "flow_type": "Import",
                "seller": { "party_id": "s2" },
                "buyer": { "party_id": "b2" }
            }),
        ))
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(json_get("/v1/trade/flows"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["total"].as_u64(), Some(2));
}

// ---------------------------------------------------------------------------
// Test: Document digest computation is deterministic
// ---------------------------------------------------------------------------

#[test]
fn trade_document_digest_determinism() {
    use mez_corridor::trade::{TradeAmount, TradeInvoice, TradeParty, compute_trade_document_digest};

    let make_invoice = || TradeInvoice {
        invoice_id: "INV-DET-001".to_string(),
        invoice_number: None,
        issue_date: "2026-02-20".to_string(),
        due_date: None,
        seller: TradeParty {
            party_id: "seller".to_string(),
            name: None,
            lei: None,
            did: None,
            account_id: None,
            agent_id: None,
            address: None,
            meta: None,
        },
        buyer: TradeParty {
            party_id: "buyer".to_string(),
            name: None,
            lei: None,
            did: None,
            account_id: None,
            agent_id: None,
            address: None,
            meta: None,
        },
        total: TradeAmount {
            currency: "PKR".to_string(),
            value: "1000000.00".to_string(),
            scale: None,
        },
        tax: None,
        line_items: None,
        purchase_order_ref: None,
        contract_ref: None,
        incoterms: None,
        shipment_ref: None,
        governing_law: None,
        jurisdiction_tags: None,
        attachment_refs: None,
        meta: None,
    };

    let d1 = compute_trade_document_digest(&make_invoice()).unwrap().to_hex();
    let d2 = compute_trade_document_digest(&make_invoice()).unwrap().to_hex();
    assert_eq!(d1, d2, "same invoice must produce identical digests");
    assert_eq!(d1.len(), 64, "SHA-256 hex digest must be 64 characters");
}
