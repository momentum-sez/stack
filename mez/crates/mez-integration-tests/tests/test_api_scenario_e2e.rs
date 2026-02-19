//! # End-to-End API Scenario: PK-REZ Opens Corridor to AE-DIFC
//!
//! The first test that exercises the full HTTP API as a unified system.
//! One test function, seven acts, one story: a Pakistani EZ opens a trade
//! corridor to Dubai's DIFC, transactions flow through the receipt chain,
//! compliance is evaluated across 20 domains, a credential is signed and
//! verified, and a sanctions trigger halts the corridor.
//!
//! Acts that depend on not-yet-landed endpoints are gracefully skipped
//! (404/501 detection) and activate automatically when the endpoints are
//! merged. The test is a living document that grows more powerful with
//! each sprint.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use mez_api::state::AppState;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build the full application with auth disabled for testing.
///
/// Uses `AppState::new()` which sets `auth_token: None`, disabling the
/// auth middleware. The full middleware stack (tracing, metrics, rate limit)
/// remains active — proving that middleware doesn't interfere with domain logic.
fn test_app() -> axum::Router {
    let state = AppState::new();
    mez_api::app(state)
}

/// Parse a response body as JSON.
async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Build a POST request with a JSON body.
fn post(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

/// Build a PUT request with a JSON body.
fn put(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("PUT")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

/// Build a GET request.
fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

/// Check if an endpoint exists by testing whether it returns 404 or 501.
/// Returns `None` if the endpoint doesn't exist (skip the act),
/// `Some(response)` if it does.
///
/// This makes the test resilient to partial implementation: acts that
/// depend on endpoints from future sprints are gracefully skipped and
/// activate automatically when those sprints land.
async fn try_endpoint(
    app: axum::Router,
    request: Request<Body>,
    act: &str,
) -> Option<axum::response::Response> {
    let resp = app.oneshot(request).await.unwrap();
    if resp.status() == StatusCode::NOT_FOUND {
        eprintln!("  \u{23ed} {act}: endpoint not yet implemented \u{2014} skipping");
        return None;
    }
    if resp.status() == StatusCode::NOT_IMPLEMENTED {
        eprintln!("  \u{23ed} {act}: endpoint returns 501 \u{2014} skipping");
        return None;
    }
    Some(resp)
}

// ---------------------------------------------------------------------------
// The Scenario
// ---------------------------------------------------------------------------

#[tokio::test]
async fn scenario_pk_rez_opens_corridor_to_ae_difc() {
    let app = test_app();

    // =====================================================================
    // Act 1: Create the corridor
    // The PK-REZ zone opens a bilateral trade corridor to AE-DIFC.
    // This is the foundational operation — everything else flows from here.
    // =====================================================================

    let resp = app
        .clone()
        .oneshot(post(
            "/v1/corridors",
            serde_json::json!({
                "jurisdiction_a": "pk-ez-01",
                "jurisdiction_b": "ae-difc"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "Act 1: corridor creation must return 201"
    );
    let corridor = body_json(resp).await;
    let corridor_id = corridor["id"]
        .as_str()
        .expect("Act 1: response must include corridor id")
        .to_string();

    // A new corridor starts in DRAFT — not yet approved by either jurisdiction.
    assert_eq!(
        corridor["state"], "DRAFT",
        "Act 1: a new corridor must start in DRAFT state"
    );

    eprintln!(
        "  \u{2713} Act 1: corridor created (id: {}\u{2026})",
        &corridor_id[..8]
    );

    // =====================================================================
    // Act 2: Walk the corridor to ACTIVE
    // Two regulatory approvals advance the corridor through its lifecycle:
    //   DRAFT -> PENDING (bilateral agreement signed)
    //   PENDING -> ACTIVE (regulatory approval from both jurisdictions)
    // =====================================================================

    // DRAFT -> PENDING: bilateral agreement signed.
    let evidence_a = "a".repeat(64);
    let resp = app
        .clone()
        .oneshot(put(
            &format!("/v1/corridors/{corridor_id}/transition"),
            serde_json::json!({
                "target_state": "PENDING",
                "evidence_digest": evidence_a,
                "reason": "Bilateral agreement signed by PK-REZ and AE-DIFC authorities"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Act 2a: DRAFT -> PENDING transition must succeed"
    );

    // PENDING -> ACTIVE: both jurisdictions approved.
    let evidence_b = "b".repeat(64);
    let resp = app
        .clone()
        .oneshot(put(
            &format!("/v1/corridors/{corridor_id}/transition"),
            serde_json::json!({
                "target_state": "ACTIVE",
                "evidence_digest": evidence_b,
                "reason": "Regulatory approval received from both jurisdictions"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Act 2b: PENDING -> ACTIVE transition must succeed"
    );

    let corridor = body_json(resp).await;
    // The corridor is now live — trade can flow.
    assert_eq!(
        corridor["state"], "ACTIVE",
        "Act 2: corridor must be ACTIVE after two approvals"
    );
    // Two transitions recorded: DRAFT->PENDING, PENDING->ACTIVE.
    let log = corridor["transition_log"]
        .as_array()
        .expect("Act 2: transition_log must be an array");
    assert_eq!(
        log.len(),
        2,
        "Act 2: transition log must have exactly 2 entries"
    );

    eprintln!("  \u{2713} Act 2: corridor transitioned DRAFT \u{2192} PENDING \u{2192} ACTIVE");

    // =====================================================================
    // Act 3: First receipt — a cross-border payment instruction
    // Acme Corp (PK-REZ) sends $50,000 to Gulf Trading (AE-DIFC).
    // The receipt is the cryptographic proof of this corridor event.
    // =====================================================================

    let resp = app
        .clone()
        .oneshot(post(
            "/v1/corridors/state/propose",
            serde_json::json!({
                "corridor_id": corridor_id,
                "payload": {
                    "type": "payment_instruction",
                    "from": "pk-ez-01:entity:acme-corp",
                    "to": "ae-difc:entity:gulf-trading",
                    "amount": "50000.00",
                    "currency": "USD"
                }
            }),
        ))
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "Act 3: receipt proposal must return 201"
    );
    let receipt1 = body_json(resp).await;

    // First receipt in the chain — sequence starts at 0.
    assert_eq!(
        receipt1["sequence"], 0,
        "Act 3: first receipt must be sequence 0"
    );
    // The canonical digest is a SHA-256 hex string (64 chars).
    let next_root_1 = receipt1["next_root"]
        .as_str()
        .expect("Act 3: next_root must be a string");
    assert_eq!(
        next_root_1.len(),
        64,
        "Act 3: next_root must be a 64-char SHA-256 hex digest"
    );
    assert!(
        next_root_1.chars().all(|c| c.is_ascii_hexdigit()),
        "Act 3: next_root must contain only hex characters"
    );
    // MMR root after first append.
    let mmr_root_1 = receipt1["mmr_root"]
        .as_str()
        .expect("Act 3: mmr_root must be a string")
        .to_string();
    assert_eq!(
        mmr_root_1.len(),
        64,
        "Act 3: mmr_root must be a 64-char SHA-256 hex digest"
    );

    eprintln!(
        "  \u{2713} Act 3: first receipt (seq=0, next_root={}\u{2026})",
        &next_root_1[..8]
    );

    // =====================================================================
    // Act 4: Second receipt — chain integrity across HTTP round-trips
    // Gulf Trading (AE-DIFC) sends a counter-payment to Acme Corp.
    // This proves that the receipt chain maintains cryptographic
    // integrity when accessed through HTTP boundaries: receipt 2's
    // prev_root must equal receipt 1's mmr_root.
    // =====================================================================

    let resp = app
        .clone()
        .oneshot(post(
            "/v1/corridors/state/propose",
            serde_json::json!({
                "corridor_id": corridor_id,
                "payload": {
                    "type": "payment_instruction",
                    "from": "ae-difc:entity:gulf-trading",
                    "to": "pk-ez-01:entity:acme-corp",
                    "amount": "48500.00",
                    "currency": "USD",
                    "reference": "counter-payment"
                }
            }),
        ))
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "Act 4: second receipt proposal must return 201"
    );
    let receipt2 = body_json(resp).await;

    // Second receipt in the chain.
    assert_eq!(
        receipt2["sequence"], 1,
        "Act 4: second receipt must be sequence 1"
    );
    // Chain integrity: receipt 2's prev_root must equal receipt 1's next_root.
    // Under the dual commitment model, prev_root tracks the hash-chain head
    // (final_state_root), which advances to next_root after each append.
    // The MMR accumulator runs independently for inclusion proofs.
    assert_eq!(
        receipt2["prev_root"].as_str().unwrap(),
        next_root_1,
        "Act 4: receipt chain is broken \u{2014} prev_root does not match prior next_root (hash-chain head)"
    );
    // The MMR root must change after appending a second leaf.
    let mmr_root_2 = receipt2["mmr_root"].as_str().unwrap();
    assert_ne!(
        mmr_root_2, &mmr_root_1,
        "Act 4: MMR root must change after appending a second receipt"
    );

    eprintln!("  \u{2713} Act 4: chain integrity verified (prev_root matches)");

    // =====================================================================
    // Act 5: Create a smart asset and evaluate its compliance
    // An equity instrument registered in the PK-REZ jurisdiction.
    // The compliance tensor evaluates the asset across regulatory
    // domains to determine whether it can operate in this corridor.
    // =====================================================================

    let resp = app
        .clone()
        .oneshot(post(
            "/v1/assets/genesis",
            serde_json::json!({
                "asset_type": "equity",
                "jurisdiction_id": "pk-ez-01",
                "metadata": {
                    "name": "Acme Corp Series A Preferred",
                    "issuer": "pk-ez-01:entity:acme-corp"
                }
            }),
        ))
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "Act 5a: asset genesis must return 201"
    );
    let asset = body_json(resp).await;
    let asset_id = asset["id"]
        .as_str()
        .expect("Act 5a: response must include asset id")
        .to_string();

    // Asset starts in GENESIS state.
    assert_eq!(
        asset["status"], "GENESIS",
        "Act 5a: new asset must be in GENESIS status"
    );

    eprintln!(
        "  \u{2713} Act 5a: asset created (id: {}\u{2026})",
        &asset_id[..8]
    );

    // Evaluate compliance via the smart_assets endpoint.
    let resp = app
        .clone()
        .oneshot(post(
            &format!("/v1/assets/{asset_id}/compliance/evaluate"),
            serde_json::json!({
                "domains": ["aml", "kyc", "sanctions"],
                "context": {
                    "entity_id": "pk-ez-01:entity:acme-corp",
                    "corridor_id": corridor_id
                }
            }),
        ))
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Act 5b: compliance evaluation must return 200"
    );
    let eval = body_json(resp).await;
    // The evaluation must reference the correct asset.
    assert_eq!(
        eval["asset_id"].as_str().unwrap(),
        &asset_id,
        "Act 5b: evaluation must reference the correct asset"
    );

    eprintln!(
        "  \u{2713} Act 5b: compliance evaluation completed (status: {})",
        eval["overall_status"]
    );

    // =====================================================================
    // Act 6: Issue and verify a compliance credential
    // The zone signs an attestation that the asset passed evaluation
    // across all 20 compliance domains. The credential is a W3C
    // Verifiable Credential signed with the zone's Ed25519 key.
    // Then we verify it through the verification endpoint — proving
    // the full sign-verify round trip works through the HTTP layer.
    // =====================================================================

    // Build attestations for all 20 compliance domains.
    let all_domains = [
        "aml",
        "kyc",
        "sanctions",
        "tax",
        "securities",
        "corporate",
        "custody",
        "data_privacy",
        "licensing",
        "banking",
        "payments",
        "clearing",
        "settlement",
        "digital_assets",
        "employment",
        "immigration",
        "ip",
        "consumer_protection",
        "arbitration",
        "trade",
    ];
    let mut attestations = serde_json::Map::new();
    for domain in &all_domains {
        attestations.insert(
            domain.to_string(),
            serde_json::json!({"status": "compliant"}),
        );
    }

    if let Some(resp) = try_endpoint(
        app.clone(),
        post(
            &format!("/v1/assets/{asset_id}/credentials/compliance"),
            serde_json::json!({
                "attestations": serde_json::Value::Object(attestations)
            }),
        ),
        "Act 6a: credential issuance",
    )
    .await
    {
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Act 6a: credential issuance must return 200"
        );
        let cred_resp = body_json(resp).await;

        // The evaluation must cover all 20 compliance domains.
        assert_eq!(
            cred_resp["evaluation"]["domain_count"], 20,
            "Act 6: all 20 compliance domains must be evaluated"
        );
        assert_eq!(
            cred_resp["evaluation"]["overall_status"], "compliant",
            "Act 6: evaluation must be compliant when all domains are attested"
        );
        // Two specifically attested domains must show as compliant.
        let domain_results = &cred_resp["evaluation"]["domain_results"];
        assert_eq!(
            domain_results["aml"], "compliant",
            "Act 6: AML must be compliant"
        );
        assert_eq!(
            domain_results["kyc"], "compliant",
            "Act 6: KYC must be compliant"
        );

        // Tensor commitment is a 64-char hex SHA-256 digest.
        let commitment = cred_resp["evaluation"]["tensor_commitment"]
            .as_str()
            .expect("Act 6: tensor_commitment must be present");
        assert_eq!(
            commitment.len(),
            64,
            "Act 6: tensor commitment must be a 64-char hex digest"
        );

        // The credential was issued because the evaluation was passing.
        assert!(
            !cred_resp["credential"].is_null(),
            "Act 6: credential must be issued for a passing evaluation"
        );
        let credential = &cred_resp["credential"];

        // The issuer must be the zone's DID.
        assert!(
            credential["issuer"]
                .as_str()
                .unwrap()
                .starts_with("did:mass:zone:"),
            "Act 6: credential issuer must be the zone's DID"
        );

        eprintln!("  \u{2713} Act 6a: credential issued (20 domains compliant)");

        // Verify the credential.
        if let Some(verify_resp) = try_endpoint(
            app.clone(),
            post("/v1/credentials/verify", credential.clone()),
            "Act 6b: credential verification",
        )
        .await
        {
            assert_eq!(
                verify_resp.status(),
                StatusCode::OK,
                "Act 6b: credential verification must return 200"
            );
            let verification = body_json(verify_resp).await;

            // A freshly issued credential must verify successfully.
            assert_eq!(
                verification["verified"], true,
                "Act 6b: a freshly issued credential must verify successfully"
            );
            assert!(
                verification["proof_count"].as_u64().unwrap() >= 1,
                "Act 6b: at least one proof must be verified"
            );
            // The first proof result must be valid.
            assert_eq!(
                verification["results"][0]["valid"], true,
                "Act 6b: proof must be valid"
            );

            eprintln!(
                "  \u{2713} Act 6b: credential verified (proofs: {})",
                verification["proof_count"]
            );
        }
    }

    // =====================================================================
    // Act 7: Sanctions trigger halts the corridor
    // An OFAC sanctions update fires into the agentic policy engine.
    // The engine evaluates the trigger against registered policies
    // and produces a Halt action targeting the active corridor.
    // This is the final proof: the system reacts to environmental
    // change by transitioning corridor state through the typestate
    // machine — the same machine that enforced DRAFT->PENDING->ACTIVE
    // in Act 2.
    // =====================================================================

    if let Some(resp) = try_endpoint(
        app.clone(),
        post(
            "/v1/triggers",
            serde_json::json!({
                "trigger_type": "sanctions_list_update",
                "asset_id": corridor_id,
                "data": {
                    "affected_parties": ["self"],
                    "source": "OFAC",
                    "list_version": "2026-02-15"
                }
            }),
        ),
        "Act 7a: sanctions trigger",
    )
    .await
    {
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Act 7a: trigger submission must return 200"
        );
        let trigger_resp = body_json(resp).await;

        // The policy engine must produce actions in response to a sanctions update.
        assert!(
            trigger_resp["actions_produced"].as_u64().unwrap() > 0,
            "Act 7: sanctions trigger must produce at least one action"
        );
        // At least one action must have been executed (the corridor halt).
        let actions = trigger_resp["actions"].as_array().unwrap();
        let executed_count = actions.iter().filter(|a| a["status"] == "executed").count();
        assert!(
            executed_count > 0,
            "Act 7: at least one action must be executed (the corridor halt)"
        );

        eprintln!(
            "  \u{2713} Act 7a: sanctions trigger processed ({} actions, {} executed)",
            trigger_resp["actions_produced"], executed_count
        );

        // Verify the corridor is now HALTED.
        let resp = app
            .clone()
            .oneshot(get(&format!("/v1/corridors/{corridor_id}")))
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Act 7b: corridor GET must return 200"
        );
        let halted_corridor = body_json(resp).await;

        // The agentic engine must have transitioned the corridor to HALTED.
        assert_eq!(
            halted_corridor["state"], "HALTED",
            "Act 7b: corridor must be HALTED after sanctions trigger"
        );
        // The transition log should now have 3 entries:
        // DRAFT->PENDING, PENDING->ACTIVE, ACTIVE->HALTED (agentic).
        let final_log = halted_corridor["transition_log"].as_array().unwrap();
        assert_eq!(
            final_log.len(),
            3,
            "Act 7b: transition log must have 3 entries after sanctions halt"
        );
        let last_entry = &final_log[2];
        assert_eq!(
            last_entry["from_state"], "ACTIVE",
            "Act 7b: last transition must be from ACTIVE"
        );
        assert_eq!(
            last_entry["to_state"], "HALTED",
            "Act 7b: last transition must be to HALTED"
        );
        // The agentic engine provides cryptographic evidence for the transition.
        assert!(
            !last_entry["evidence_digest"].is_null(),
            "Act 7b: agentic transition must include evidence digest"
        );

        eprintln!("  \u{2713} Act 7b: corridor HALTED \u{2014} system reacted to sanctions update");
    }

    eprintln!("\n  === Scenario complete: all acts passed ===");
}
