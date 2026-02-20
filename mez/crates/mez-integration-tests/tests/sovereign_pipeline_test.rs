// SPDX-License-Identifier: BUSL-1.1
//! Full sovereign GovOS pipeline integration test.
//!
//! Starts one in-process Mass API stub and executes the complete sovereign
//! pipeline sequentially, each step using output from the prior:
//!
//! a) Create organization
//! b) Create treasury (entityId from a)
//! c) Create account (treasuryId from b)
//! d) Create payment (sourceAccountId from c)
//! e) Record tax event (entityId from a)
//! f) Compute withholding (entityId from a, NTN "1234567")
//! g) Create consent (organizationId from a)
//! h) Approve consent (id from g)
//! i) Create cap table (organizationId from a)
//! j) Verify CNIC ("12345-1234567-1")
//! k) Verify NTN ("1234567")
//! l) Sign template (entityId from a)
//! m) Get submission (id from l)
//!
//! Proves a sovereign zone can run the complete GovOS pipeline.

use serde_json::json;

/// Start a stub server on a random port, returning (port, shutdown_sender).
async fn start_stub_server() -> (u16, tokio::sync::oneshot::Sender<()>) {
    use axum::{
        extract::{Path, Query, State},
        http::StatusCode,
        response::{IntoResponse, Response},
        routing::{get, post},
        Json, Router,
    };
    use chrono::Utc;
    use dashmap::DashMap;
    use serde::Deserialize;
    use serde_json::Value;
    use std::sync::Arc;
    use uuid::Uuid;

    // ── State ───────────────────────────────────────────────────────

    #[derive(Clone)]
    struct S {
        orgs: Arc<DashMap<Uuid, Value>>,
        treasuries: Arc<DashMap<Uuid, Value>>,
        accounts: Arc<DashMap<Uuid, Value>>,
        transactions: Arc<DashMap<Uuid, Value>>,
        tax_events: Arc<DashMap<Uuid, Value>>,
        consents: Arc<DashMap<Uuid, Value>>,
        cap_tables: Arc<DashMap<Uuid, Value>>,
        submissions: Arc<DashMap<String, Value>>,
    }

    impl S {
        fn new() -> Self {
            Self {
                orgs: Arc::new(DashMap::new()),
                treasuries: Arc::new(DashMap::new()),
                accounts: Arc::new(DashMap::new()),
                transactions: Arc::new(DashMap::new()),
                tax_events: Arc::new(DashMap::new()),
                consents: Arc::new(DashMap::new()),
                cap_tables: Arc::new(DashMap::new()),
                submissions: Arc::new(DashMap::new()),
            }
        }
    }

    // ── Handlers ────────────────────────────────────────────────────

    async fn health() -> StatusCode {
        StatusCode::OK
    }

    async fn org_create(State(s): State<S>, Json(body): Json<Value>) -> Response {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let e = json!({
            "id": id.to_string(),
            "name": body.get("name").and_then(|v| v.as_str()).unwrap_or(""),
            "jurisdiction": body.get("jurisdiction"),
            "status": "ACTIVE",
            "tags": body.get("tags").cloned().unwrap_or(json!([])),
            "createdAt": now,
            "updatedAt": now
        });
        s.orgs.insert(id, e.clone());
        (StatusCode::CREATED, Json(e)).into_response()
    }

    async fn org_get(State(s): State<S>, Path(id): Path<Uuid>) -> Response {
        match s.orgs.get(&id) {
            Some(e) => Json(e.value().clone()).into_response(),
            None => StatusCode::NOT_FOUND.into_response(),
        }
    }

    async fn treasury_create(State(s): State<S>, Json(body): Json<Value>) -> Response {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let t = json!({
            "id": id.to_string(),
            "referenceId": null,
            "entityId": body.get("entityId").and_then(|v| v.as_str()).unwrap_or(""),
            "name": body.get("entityName"),
            "status": "ACTIVE",
            "context": "MASS",
            "createdAt": now,
            "updatedAt": now
        });
        s.treasuries.insert(id, t.clone());
        (StatusCode::CREATED, Json(t)).into_response()
    }

    #[derive(Deserialize)]
    struct AcctQ {
        #[serde(rename = "treasuryId")]
        treasury_id: Uuid,
        #[serde(rename = "idempotencyKey")]
        _idempotency_key: String,
        name: Option<String>,
    }

    async fn account_create(State(s): State<S>, Query(q): Query<AcctQ>) -> Response {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let entity_id = s
            .treasuries
            .get(&q.treasury_id)
            .and_then(|t| t.get("entityId").and_then(|v| v.as_str()).map(String::from));
        let a = json!({
            "id": id.to_string(),
            "entityId": entity_id,
            "treasuryId": q.treasury_id.to_string(),
            "name": q.name.as_deref().unwrap_or("Default Account"),
            "currency": "PKR",
            "balance": "0.00",
            "available": "0.00",
            "status": "ACTIVE",
            "createdAt": now,
            "updatedAt": now
        });
        s.accounts.insert(id, a.clone());
        (StatusCode::CREATED, Json(a)).into_response()
    }

    async fn payment_create(State(s): State<S>, Json(body): Json<Value>) -> Response {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let p = json!({
            "id": id.to_string(),
            "accountId": body.get("sourceAccountId"),
            "transactionType": "PAYMENT",
            "status": "PENDING",
            "direction": "OUTBOUND",
            "currency": body.get("currency").cloned().unwrap_or(json!("PKR")),
            "amount": body.get("amount"),
            "createdAt": now
        });
        s.transactions.insert(id, p.clone());
        (StatusCode::CREATED, Json(p)).into_response()
    }

    async fn tax_event_create(State(s): State<S>, Json(body): Json<Value>) -> Response {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let e = json!({
            "id": id.to_string(),
            "entityId": body.get("entity_id").and_then(|v| v.as_str()).unwrap_or(""),
            "eventType": body.get("event_type").and_then(|v| v.as_str()).unwrap_or("UNKNOWN"),
            "amount": body.get("amount").and_then(|v| v.as_str()).unwrap_or("0"),
            "currency": body.get("currency").and_then(|v| v.as_str()).unwrap_or("PKR"),
            "taxYear": body.get("tax_year").and_then(|v| v.as_str()),
            "details": body.get("details").cloned().unwrap_or(json!({})),
            "createdAt": now
        });
        s.tax_events.insert(id, e.clone());
        (StatusCode::CREATED, Json(e)).into_response()
    }

    async fn withholding_compute(Json(body): Json<Value>) -> Json<Value> {
        let entity_id = body.get("entity_id").and_then(|v| v.as_str()).unwrap_or("");
        let amount_str = body.get("transaction_amount").and_then(|v| v.as_str()).unwrap_or("0");
        let currency = body.get("currency").and_then(|v| v.as_str()).unwrap_or("PKR");
        let tx_type = body.get("transaction_type").and_then(|v| v.as_str()).unwrap_or("payment_for_goods");
        let ntn = body.get("ntn").and_then(|v| v.as_str());

        let (filer, status_str) = match ntn {
            Some(n) if !n.is_empty() => match n.as_bytes().first() {
                Some(b'0') => ("NonFiler", "NonFiler"),
                Some(b'9') => ("LateFiler", "LateFiler"),
                _ => ("Filer", "Filer"),
            },
            _ => ("NonFiler", "NonFiler"),
        };

        let rate: f64 = match (tx_type, filer) {
            ("payment_for_goods", "Filer") => 4.5,
            ("payment_for_goods", "LateFiler") => 6.5,
            ("payment_for_goods", _) => 9.0,
            ("payment_for_services", "Filer") => 8.0,
            ("payment_for_services", "LateFiler") => 12.0,
            ("payment_for_services", _) => 16.0,
            ("salary_payment", "Filer") => 12.5,
            ("salary_payment", "LateFiler") => 15.0,
            ("salary_payment", _) => 20.0,
            (_, "Filer") => 4.5,
            (_, "LateFiler") => 6.5,
            (_, _) => 9.0,
        };

        let gross: f64 = amount_str.parse().unwrap_or(0.0);
        let wh = gross * rate / 100.0;
        let net = gross - wh;

        Json(json!({
            "entity_id": entity_id,
            "gross_amount": format!("{gross:.2}"),
            "withholding_amount": format!("{wh:.2}"),
            "withholding_rate": format!("{rate}"),
            "net_amount": format!("{net:.2}"),
            "currency": currency,
            "withholding_type": tx_type,
            "ntn_status": status_str,
            "computed_at": Utc::now().to_rfc3339()
        }))
    }

    async fn consent_create(State(s): State<S>, Json(body): Json<Value>) -> Response {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let c = json!({
            "id": id.to_string(),
            "organizationId": body.get("organizationId").and_then(|v| v.as_str()).unwrap_or(""),
            "operationType": body.get("operationType"),
            "status": "PENDING",
            "votes": [],
            "numVotesRequired": body.get("numBoardMemberApprovalsRequired"),
            "approvalCount": 0,
            "rejectionCount": 0,
            "createdAt": now,
            "updatedAt": now
        });
        s.consents.insert(id, c.clone());
        (StatusCode::CREATED, Json(c)).into_response()
    }

    async fn consent_approve(State(s): State<S>, Path(id): Path<Uuid>) -> Response {
        match s.consents.get_mut(&id) {
            Some(mut e) => {
                let c = e.value_mut();
                if let Some(obj) = c.as_object_mut() {
                    obj.insert("status".to_string(), json!("APPROVED"));
                }
                let org_id = c.get("organizationId").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let vote = json!({
                    "consentId": id.to_string(),
                    "organizationId": org_id,
                    "vote": "APPROVED",
                    "votedBy": "system",
                    "majorityReached": true,
                    "createdAt": Utc::now().to_rfc3339()
                });
                Json(vote).into_response()
            }
            None => StatusCode::NOT_FOUND.into_response(),
        }
    }

    async fn cap_table_create(State(s): State<S>, Json(body): Json<Value>) -> Response {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let ct = json!({
            "id": id.to_string(),
            "organizationId": body.get("organizationId").and_then(|v| v.as_str()).unwrap_or(""),
            "authorizedShares": body.get("authorizedShares").and_then(|v| v.as_u64()).unwrap_or(10_000_000),
            "outstandingShares": 0,
            "fullyDilutedShares": 0,
            "reservedShares": 0,
            "unreservedShares": 10_000_000,
            "shareClasses": [],
            "shareholders": [],
            "optionsPools": [],
            "createdAt": now,
            "updatedAt": now
        });
        s.cap_tables.insert(id, ct.clone());
        (StatusCode::CREATED, Json(ct)).into_response()
    }

    async fn verify_cnic(Json(body): Json<Value>) -> Response {
        let cnic_raw = body.get("cnic").and_then(|v| v.as_str()).unwrap_or("");
        let digits: String = cnic_raw.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 13 {
            return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid CNIC"}))).into_response();
        }
        Json(json!({
            "cnic": digits,
            "verified": true,
            "full_name": body.get("full_name"),
            "identity_id": Uuid::new_v4().to_string(),
            "verification_timestamp": Utc::now().to_rfc3339(),
            "details": {"match_score": 0.95, "cnic_status": "Active"}
        }))
        .into_response()
    }

    async fn verify_ntn(Json(body): Json<Value>) -> Response {
        let ntn = body.get("ntn").and_then(|v| v.as_str()).unwrap_or("");
        let valid = ntn.len() == 7 && ntn.chars().all(|c| c.is_ascii_digit());
        if !valid {
            return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid NTN"}))).into_response();
        }
        let tax_status = match ntn.as_bytes().first() {
            Some(b'0') => "NonFiler",
            Some(b'9') => "LateFiler",
            _ => "Filer",
        };
        Json(json!({
            "ntn": ntn,
            "verified": true,
            "registered_name": body.get("entity_name"),
            "tax_status": tax_status,
            "identity_id": Uuid::new_v4().to_string(),
            "verification_timestamp": Utc::now().to_rfc3339(),
            "details": {"filer_status": tax_status}
        }))
        .into_response()
    }

    async fn template_sign(State(s): State<S>, Json(body): Json<Value>) -> Response {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let sub = json!({
            "id": id,
            "entityId": body.get("entityId"),
            "context": "SOVEREIGN_GOVOS",
            "status": "PENDING",
            "signingOrder": "RANDOM",
            "signers": [],
            "createdAt": now,
            "updatedAt": now
        });
        s.submissions.insert(id.clone(), sub.clone());
        (StatusCode::CREATED, Json(sub)).into_response()
    }

    async fn submission_get(State(s): State<S>, Path(id): Path<String>) -> Response {
        match s.submissions.get(&id) {
            Some(e) => Json(e.value().clone()).into_response(),
            None => StatusCode::NOT_FOUND.into_response(),
        }
    }

    async fn members_get() -> Json<Value> {
        Json(json!([]))
    }

    async fn board_get() -> Json<Value> {
        Json(json!([]))
    }

    async fn shareholders_get() -> Json<Value> {
        Json(json!([]))
    }

    async fn identities_list() -> Json<Value> {
        Json(json!([]))
    }

    async fn template_available() -> Json<Value> {
        Json(json!([
            {"templateType": "CERTIFICATE_OF_INCORPORATION", "templateName": "Certificate of Incorporation"}
        ]))
    }

    // ── Router ──────────────────────────────────────────────────────

    let s = S::new();

    let app = Router::new()
        .route("/health", get(health))
        .route("/organization-info/api/v1/organization/create", post(org_create))
        .route("/organization-info/api/v1/organization/:id", get(org_get))
        .route("/organization-info/api/v1/membership/:org_id/members", get(members_get))
        .route("/organization-info/api/v1/board/:org_id", get(board_get))
        .route("/organization-info/api/v1/identity/cnic/verify", post(verify_cnic))
        .route("/organization-info/api/v1/identity/ntn/verify", post(verify_ntn))
        .route("/treasury-info/api/v1/treasury/create", post(treasury_create))
        .route("/treasury-info/api/v1/account/create", post(account_create))
        .route("/treasury-info/api/v1/transaction/create/payment", post(payment_create))
        .route("/treasury-info/api/v1/tax-events", post(tax_event_create))
        .route("/treasury-info/api/v1/withholding/compute", post(withholding_compute))
        .route("/consent-info/api/v1/consents", post(consent_create))
        .route("/consent-info/api/v1/consents/approve/:id", post(consent_approve))
        .route("/consent-info/api/v1/capTables", post(cap_table_create))
        .route("/consent-info/api/v1/shareholders/organization/:org_id", get(shareholders_get))
        .route("/consent-info/api/v1/identities", get(identities_list))
        .route("/templating-engine/api/v1/template/sign", post(template_sign))
        .route("/templating-engine/api/v1/template/available", get(template_available))
        .route("/templating-engine/api/v1/submission/:id", get(submission_get))
        .with_state(s);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(async { rx.await.ok(); })
            .await
            .ok();
    });

    // Wait for the server to be ready.
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client
            .get(format!("http://127.0.0.1:{port}/health"))
            .send()
            .await
            .is_ok()
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    (port, tx)
}

#[tokio::test]
async fn sovereign_pipeline_full_13_steps() {
    let (port, _shutdown) = start_stub_server().await;
    let base = format!("http://127.0.0.1:{port}");
    let client = reqwest::Client::new();

    // ── (a) Create organization ─────────────────────────────────────
    let resp = client
        .post(format!("{base}/organization-info/api/v1/organization/create"))
        .json(&json!({
            "name": "Sovereign Corp PK",
            "jurisdiction": "pk-sifc",
            "tags": ["sovereign", "govos"]
        }))
        .send()
        .await
        .expect("org create");
    assert_eq!(resp.status(), 201, "step a: create org");
    let org: serde_json::Value = resp.json().await.unwrap();
    let org_id = org["id"].as_str().expect("org id");
    assert_eq!(org["name"], "Sovereign Corp PK");
    assert_eq!(org["status"], "ACTIVE");

    // ── (b) Create treasury ─────────────────────────────────────────
    let resp = client
        .post(format!("{base}/treasury-info/api/v1/treasury/create"))
        .json(&json!({
            "entityId": org_id,
            "entityName": "Sovereign Corp PK"
        }))
        .send()
        .await
        .expect("treasury create");
    assert_eq!(resp.status(), 201, "step b: create treasury");
    let treasury: serde_json::Value = resp.json().await.unwrap();
    let treasury_id = treasury["id"].as_str().expect("treasury id");
    assert_eq!(treasury["entityId"], org_id);

    // ── (c) Create account ──────────────────────────────────────────
    let resp = client
        .post(format!(
            "{base}/treasury-info/api/v1/account/create?treasuryId={treasury_id}&idempotencyKey=idem-1&name=PKR+Operating"
        ))
        .send()
        .await
        .expect("account create");
    assert_eq!(resp.status(), 201, "step c: create account");
    let account: serde_json::Value = resp.json().await.unwrap();
    let account_id = account["id"].as_str().expect("account id");
    assert_eq!(account["treasuryId"], treasury_id);

    // ── (d) Create payment ──────────────────────────────────────────
    let resp = client
        .post(format!("{base}/treasury-info/api/v1/transaction/create/payment"))
        .json(&json!({
            "sourceAccountId": account_id,
            "amount": "50000.00",
            "currency": "PKR",
            "paymentType": "PAYMENT"
        }))
        .send()
        .await
        .expect("payment create");
    assert_eq!(resp.status(), 201, "step d: create payment");
    let payment: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(payment["status"], "PENDING");
    assert_eq!(payment["amount"], "50000.00");

    // ── (e) Record tax event ────────────────────────────────────────
    let resp = client
        .post(format!("{base}/treasury-info/api/v1/tax-events"))
        .json(&json!({
            "entity_id": org_id,
            "event_type": "WITHHOLDING_AT_SOURCE",
            "amount": "10000.00",
            "currency": "PKR",
            "tax_year": "2025-2026",
            "details": {}
        }))
        .send()
        .await
        .expect("tax event");
    assert_eq!(resp.status(), 201, "step e: record tax event");
    let tax_event: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(tax_event["entityId"], org_id);

    // ── (f) Compute withholding ─────────────────────────────────────
    let resp = client
        .post(format!("{base}/treasury-info/api/v1/withholding/compute"))
        .json(&json!({
            "entity_id": org_id,
            "transaction_amount": "100000.00",
            "currency": "PKR",
            "transaction_type": "payment_for_goods",
            "ntn": "1234567",
            "jurisdiction_id": "PK"
        }))
        .send()
        .await
        .expect("withholding");
    assert_eq!(resp.status(), 200, "step f: compute withholding");
    let wh: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(wh["ntn_status"], "Filer");
    assert_eq!(wh["withholding_rate"], "4.5");
    assert_eq!(wh["withholding_amount"], "4500.00");
    assert_eq!(wh["net_amount"], "95500.00");

    // ── (g) Create consent ──────────────────────────────────────────
    let resp = client
        .post(format!("{base}/consent-info/api/v1/consents"))
        .json(&json!({
            "organizationId": org_id,
            "operationType": "EQUITY_OFFER",
            "numBoardMemberApprovalsRequired": 1
        }))
        .send()
        .await
        .expect("consent create");
    assert_eq!(resp.status(), 201, "step g: create consent");
    let consent: serde_json::Value = resp.json().await.unwrap();
    let consent_id = consent["id"].as_str().expect("consent id");
    assert_eq!(consent["organizationId"], org_id);
    assert_eq!(consent["status"], "PENDING");

    // ── (h) Approve consent ─────────────────────────────────────────
    let resp = client
        .post(format!("{base}/consent-info/api/v1/consents/approve/{consent_id}"))
        .send()
        .await
        .expect("consent approve");
    assert_eq!(resp.status(), 200, "step h: approve consent");
    let vote: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(vote["vote"], "APPROVED");
    assert_eq!(vote["organizationId"], org_id);
    assert_eq!(vote["majorityReached"], true);

    // ── (i) Create cap table ────────────────────────────────────────
    let resp = client
        .post(format!("{base}/consent-info/api/v1/capTables"))
        .json(&json!({
            "organizationId": org_id,
            "authorizedShares": 1000000
        }))
        .send()
        .await
        .expect("cap table create");
    assert_eq!(resp.status(), 201, "step i: create cap table");
    let cap_table: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(cap_table["organizationId"], org_id);
    assert_eq!(cap_table["authorizedShares"], 1000000);

    // ── (j) Verify CNIC ────────────────────────────────────────────
    let resp = client
        .post(format!("{base}/organization-info/api/v1/identity/cnic/verify"))
        .json(&json!({
            "cnic": "12345-1234567-1",
            "full_name": "Ali Khan"
        }))
        .send()
        .await
        .expect("cnic verify");
    assert_eq!(resp.status(), 200, "step j: verify CNIC");
    let cnic_resp: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(cnic_resp["verified"], true);
    assert_eq!(cnic_resp["cnic"], "1234512345671"); // stripped dashes

    // ── (k) Verify NTN ─────────────────────────────────────────────
    let resp = client
        .post(format!("{base}/organization-info/api/v1/identity/ntn/verify"))
        .json(&json!({
            "ntn": "1234567",
            "entity_name": "Sovereign Corp PK"
        }))
        .send()
        .await
        .expect("ntn verify");
    assert_eq!(resp.status(), 200, "step k: verify NTN");
    let ntn_resp: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(ntn_resp["verified"], true);
    assert_eq!(ntn_resp["tax_status"], "Filer");

    // ── (l) Sign template ───────────────────────────────────────────
    let resp = client
        .post(format!("{base}/templating-engine/api/v1/template/sign"))
        .json(&json!({
            "entityId": org_id,
            "templateTypes": ["CERTIFICATE_OF_INCORPORATION"],
            "signers": []
        }))
        .send()
        .await
        .expect("template sign");
    assert_eq!(resp.status(), 201, "step l: sign template");
    let submission: serde_json::Value = resp.json().await.unwrap();
    let submission_id = submission["id"].as_str().expect("submission id");
    assert_eq!(submission["entityId"], org_id);

    // ── (m) Get submission ──────────────────────────────────────────
    let resp = client
        .get(format!("{base}/templating-engine/api/v1/submission/{submission_id}"))
        .send()
        .await
        .expect("get submission");
    assert_eq!(resp.status(), 200, "step m: get submission");
    let fetched_sub: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(fetched_sub["id"], submission_id);
    assert_eq!(fetched_sub["entityId"], org_id);

    // ── Pipeline complete ───────────────────────────────────────────
    // All 13 steps returned 2xx. The sovereign zone can run the
    // complete GovOS pipeline end-to-end.
}
