//! # Integration Tests for National System HTTP Adapters
//!
//! Tests the real HTTP adapter implementations (HttpFbrIrisAdapter,
//! HttpNadraAdapter, HttpSecpAdapter, HttpRaastAdapter) against wiremock
//! mock servers to verify correct request construction, response parsing,
//! and error handling without requiring live government API access.
//!
//! ## Note on `spawn_blocking`
//!
//! The adapter trait methods are synchronous and use `Handle::block_on`
//! internally. This cannot be called from within a Tokio runtime context.
//! All sync adapter calls are wrapped in `tokio::task::spawn_blocking`
//! to run them on a dedicated blocking thread pool.

use mez_mass_client::fbr::{
    ComplianceStatus, FbrIrisAdapter, FilerStatus, TaxEventSubmission, WithholdingRateQuery,
};
use mez_mass_client::http_adapters::{
    FbrIrisConfig, HttpFbrIrisAdapter, HttpNadraAdapter, HttpRaastAdapter, HttpSecpAdapter,
    NadraConfig, RaastConfig, SecpConfig,
};
use mez_mass_client::nadra::{NadraAdapter, NadraVerificationRequest};
use mez_mass_client::raast::{AliasType, RaastAdapter, RaastPaymentInstruction};
use mez_mass_client::secp::SecpAdapter;
use std::sync::Arc;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── FBR IRIS HTTP Adapter Tests ──────────────────────────────────────────

fn fbr_adapter(server: &MockServer) -> Arc<HttpFbrIrisAdapter> {
    let config = FbrIrisConfig::new(server.uri(), "test-api-key");
    Arc::new(HttpFbrIrisAdapter::new(config).expect("adapter build"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fbr_verify_ntn_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/ntn/verify"))
        .and(header("Authorization", "Bearer test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "verified": true,
            "ntn": "1234567",
            "registered_name": "Acme Corp",
            "filer_status": "Filer",
            "verification_timestamp": "2026-02-22T10:00:00Z",
            "reference": "FBR-REF-001"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = fbr_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        let ntn = mez_core::Ntn::new("1234567").expect("valid ntn");
        adapter.verify_ntn(&ntn, "Acme Corp")
    })
    .await
    .expect("task")
    .expect("verify");

    assert!(result.verified);
    assert_eq!(result.ntn, "1234567");
    assert_eq!(result.filer_status, FilerStatus::Filer);
    assert_eq!(result.reference, "FBR-REF-001");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fbr_verify_ntn_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/ntn/verify"))
        .respond_with(ResponseTemplate::new(404).set_body_string("NTN not found"))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = fbr_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        let ntn = mez_core::Ntn::new("9999999").expect("valid ntn");
        adapter.verify_ntn(&ntn, "Ghost Corp")
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fbr_submit_tax_event_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tax-events"))
        .and(header("Authorization", "Bearer test-api-key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "accepted": true,
            "fbr_reference": "FBR-EVT-2026-001",
            "recorded_at": "2026-02-22T10:05:00Z",
            "idempotency_key": "idem-001"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = fbr_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        let event = TaxEventSubmission {
            entity_id: "ent-001".to_string(),
            ntn: "1234567".to_string(),
            event_type: "payment_for_goods".to_string(),
            amount: "100000.00".to_string(),
            currency: "PKR".to_string(),
            jurisdiction: "PK".to_string(),
            idempotency_key: "idem-001".to_string(),
            tax_year: Some("2025-2026".to_string()),
            statutory_section: Some("S153(1)(a)".to_string()),
        };
        adapter.submit_tax_event(&event)
    })
    .await
    .expect("task")
    .expect("submit");

    assert!(result.accepted);
    assert_eq!(result.fbr_reference, "FBR-EVT-2026-001");
    assert_eq!(result.idempotency_key, "idem-001");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fbr_submit_tax_event_rejects_empty_idempotency_key() {
    let server = MockServer::start().await;
    let adapter = fbr_adapter(&server);

    let result = tokio::task::spawn_blocking(move || {
        let event = TaxEventSubmission {
            entity_id: "ent-001".to_string(),
            ntn: "1234567".to_string(),
            event_type: "salary_payment".to_string(),
            amount: "50000.00".to_string(),
            currency: "PKR".to_string(),
            jurisdiction: "PK".to_string(),
            idempotency_key: String::new(),
            tax_year: None,
            statutory_section: None,
        };
        adapter.submit_tax_event(&event)
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fbr_query_withholding_rate_success() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/withholding-rates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "rate_percent": "4.5",
            "statutory_section": "S153(1)(a)",
            "is_final_tax": false,
            "description": "Payment for goods to a filer"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = fbr_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        let params = WithholdingRateQuery {
            transaction_type: "payment_for_goods".to_string(),
            filer_status: FilerStatus::Filer,
            jurisdiction: "PK".to_string(),
            tax_year: None,
        };
        adapter.query_withholding_rate(&params)
    })
    .await
    .expect("task")
    .expect("rate");

    assert_eq!(result.rate_percent, "4.5");
    assert_eq!(result.statutory_section, "S153(1)(a)");
    assert!(!result.is_final_tax);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fbr_get_taxpayer_profile_success() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/taxpayers/1234567"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ntn": "1234567",
            "filer_status": "Filer",
            "compliance_status": "Compliant",
            "active_since": "2020-01-15",
            "registered_name": "Acme Corp"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = fbr_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        let ntn = mez_core::Ntn::new("1234567").expect("valid ntn");
        adapter.get_taxpayer_profile(&ntn)
    })
    .await
    .expect("task")
    .expect("profile");

    assert_eq!(result.ntn, "1234567");
    assert_eq!(result.filer_status, FilerStatus::Filer);
    assert_eq!(result.compliance_status, ComplianceStatus::Compliant);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fbr_get_taxpayer_profile_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/taxpayers/9999999"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = fbr_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        let ntn = mez_core::Ntn::new("9999999").expect("valid ntn");
        adapter.get_taxpayer_profile(&ntn)
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fbr_server_error_maps_to_service_unavailable() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/taxpayers/1234567"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = fbr_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        let ntn = mez_core::Ntn::new("1234567").expect("valid ntn");
        adapter.get_taxpayer_profile(&ntn)
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

// ── NADRA HTTP Adapter Tests ─────────────────────────────────────────────

fn nadra_adapter(server: &MockServer) -> Arc<HttpNadraAdapter> {
    let config = NadraConfig::new(server.uri(), "test-api-key");
    Arc::new(HttpNadraAdapter::new(config).expect("adapter build"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nadra_verify_identity_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/verify"))
        .and(header("Authorization", "Bearer test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "verified": true,
            "match_score": 0.97,
            "cnic_status": "Active",
            "verification_timestamp": "2026-02-22T10:00:00Z",
            "reference": "NADRA-REF-001"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = nadra_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        let request = NadraVerificationRequest {
            cnic: "3520112345671".to_string(),
            full_name: "Ali Khan".to_string(),
            father_name: Some("Ahmed Khan".to_string()),
            date_of_birth: Some("1990-05-15".to_string()),
            request_reference: "ref-001".to_string(),
        };
        adapter.verify_identity(&request)
    })
    .await
    .expect("task")
    .expect("verify");

    assert!(result.verified);
    assert_eq!(result.match_score, Some(0.97));
    assert_eq!(result.reference, "NADRA-REF-001");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nadra_verify_identity_invalid_cnic() {
    let server = MockServer::start().await;
    let adapter = nadra_adapter(&server);

    let result = tokio::task::spawn_blocking(move || {
        let request = NadraVerificationRequest {
            cnic: "123".to_string(),
            full_name: "Test".to_string(),
            father_name: None,
            date_of_birth: None,
            request_reference: "ref-002".to_string(),
        };
        adapter.verify_identity(&request)
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nadra_check_cnic_status_success() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/cnic/3520112345671/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!("Active")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let adapter = nadra_adapter(&server);
    let status = tokio::task::spawn_blocking(move || {
        adapter.check_cnic_status("35201-1234567-1")
    })
    .await
    .expect("task")
    .expect("status");
    assert_eq!(status, mez_mass_client::nadra::CnicStatus::Active);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nadra_check_cnic_status_invalid_cnic() {
    let server = MockServer::start().await;
    let adapter = nadra_adapter(&server);

    let result = tokio::task::spawn_blocking(move || {
        adapter.check_cnic_status("invalid")
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nadra_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/cnic/3520112345671/status"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = nadra_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        adapter.check_cnic_status("3520112345671")
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

// ── SECP HTTP Adapter Tests ──────────────────────────────────────────────

fn secp_adapter(server: &MockServer) -> Arc<HttpSecpAdapter> {
    let config = SecpConfig::new(server.uri(), "test-api-key");
    Arc::new(HttpSecpAdapter::new(config).expect("adapter build"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn secp_lookup_company_success() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/companies/0000001"))
        .and(header("Authorization", "Bearer test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "registration_no": "0000001",
            "name": "Acme Private Ltd",
            "company_type": "Private",
            "incorporation_date": "2020-01-15",
            "status": "Active",
            "registered_address": "123 Business Avenue, Karachi",
            "authorized_capital": "10000000"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = secp_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        adapter.lookup_company("0000001")
    })
    .await
    .expect("task")
    .expect("lookup");

    assert_eq!(result.registration_no, "0000001");
    assert_eq!(result.name, "Acme Private Ltd");
    assert_eq!(result.company_type, mez_mass_client::secp::CompanyType::Private);
    assert_eq!(result.status, mez_mass_client::secp::CompanyStatus::Active);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn secp_lookup_company_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/companies/9999999"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = secp_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        adapter.lookup_company("9999999")
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn secp_verify_license_success() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/companies/0000001/licenses/EMI"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "license_ref": "LIC-2026-001",
            "license_type": "EMI",
            "valid": true,
            "expiry_date": "2027-12-31",
            "issued_to": "Acme Private Ltd"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = secp_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        adapter.verify_license("0000001", "EMI")
    })
    .await
    .expect("task")
    .expect("license");

    assert!(result.valid);
    assert_eq!(result.license_type, "EMI");
    assert_eq!(result.expiry_date.as_deref(), Some("2027-12-31"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn secp_check_filing_status_success() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/companies/0000001/filing-status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "registration_no": "0000001",
            "compliance_status": "Current",
            "last_filing_date": "2026-01-30",
            "next_deadline": "2026-04-30"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = secp_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        adapter.check_filing_status("0000001")
    })
    .await
    .expect("task")
    .expect("filing");

    assert_eq!(result.registration_no, "0000001");
    assert_eq!(
        result.compliance_status,
        mez_mass_client::secp::FilingComplianceStatus::Current
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn secp_get_directors_success() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/companies/0000001/directors"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "name": "Ali Khan",
                "cnic": "3520112345671",
                "designation": "CEO",
                "appointment_date": "2020-01-15"
            },
            {
                "name": "Fatima Ahmed",
                "designation": "Director",
                "appointment_date": "2021-06-01"
            }
        ])))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = secp_adapter(&server);
    let directors = tokio::task::spawn_blocking(move || {
        adapter.get_directors("0000001")
    })
    .await
    .expect("task")
    .expect("directors");

    assert_eq!(directors.len(), 2);
    assert_eq!(directors[0].name, "Ali Khan");
    assert_eq!(directors[0].designation, "CEO");
    assert_eq!(directors[1].name, "Fatima Ahmed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn secp_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/companies/0000001"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal error"))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = secp_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        adapter.lookup_company("0000001")
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

// ── SBP Raast HTTP Adapter Tests ─────────────────────────────────────────

fn raast_adapter(server: &MockServer) -> Arc<HttpRaastAdapter> {
    let config = RaastConfig::new(server.uri(), "test-api-key", "HABB");
    Arc::new(HttpRaastAdapter::new(config).expect("adapter build"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raast_initiate_payment_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/payments"))
        .and(header("Authorization", "Bearer test-api-key"))
        .and(header("X-Bank-Code", "HABB"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "raast_reference": "RAAST-2026-001",
            "status": "Pending",
            "timestamp": "2026-02-22T10:00:00Z",
            "fee": 50,
            "settlement_id": "STL-001"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = raast_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        let instruction = RaastPaymentInstruction {
            amount: 100_000,
            from_iban: "PK36SCBL0000001123456702".to_string(),
            to_iban: "PK06HABB0000001234567895".to_string(),
            reference: "PAY-001".to_string(),
            purpose_code: Some("P2P".to_string()),
            remittance_info: Some("Test payment".to_string()),
            idempotency_key: "idem-pay-001".to_string(),
        };
        adapter.initiate_payment(&instruction)
    })
    .await
    .expect("task")
    .expect("payment");

    assert_eq!(result.raast_reference, "RAAST-2026-001");
    assert_eq!(result.status, mez_mass_client::raast::PaymentStatus::Pending);
    assert_eq!(result.fee, Some(50));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raast_initiate_payment_invalid_iban() {
    let server = MockServer::start().await;
    let adapter = raast_adapter(&server);

    let result = tokio::task::spawn_blocking(move || {
        let instruction = RaastPaymentInstruction {
            amount: 100_000,
            from_iban: "INVALID".to_string(),
            to_iban: "PK06HABB0000001234567895".to_string(),
            reference: "PAY-002".to_string(),
            purpose_code: Some("P2P".to_string()),
            remittance_info: None,
            idempotency_key: "idem-pay-002".to_string(),
        };
        adapter.initiate_payment(&instruction)
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raast_initiate_payment_zero_amount() {
    let server = MockServer::start().await;
    let adapter = raast_adapter(&server);

    let result = tokio::task::spawn_blocking(move || {
        let instruction = RaastPaymentInstruction {
            amount: 0,
            from_iban: "PK36SCBL0000001123456702".to_string(),
            to_iban: "PK06HABB0000001234567895".to_string(),
            reference: "PAY-003".to_string(),
            purpose_code: Some("P2P".to_string()),
            remittance_info: None,
            idempotency_key: "idem-pay-003".to_string(),
        };
        adapter.initiate_payment(&instruction)
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raast_check_payment_status_success() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/payments/RAAST-2026-001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "raast_reference": "RAAST-2026-001",
            "status": "Completed",
            "timestamp": "2026-02-22T10:01:00Z",
            "fee": 50,
            "settlement_id": "STL-001"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = raast_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        adapter.check_payment_status("RAAST-2026-001")
    })
    .await
    .expect("task")
    .expect("status");
    assert_eq!(result.status, mez_mass_client::raast::PaymentStatus::Completed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raast_check_payment_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/payments/NONEXISTENT"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = raast_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        adapter.check_payment_status("NONEXISTENT")
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raast_verify_account_success() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/accounts/PK06HABB0000001234567895/verify"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "iban": "PK06HABB0000001234567895",
            "active": true,
            "account_title": "Ali Khan",
            "bank_name": "Habib Bank Limited",
            "verification_timestamp": "2026-02-22T10:00:00Z"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = raast_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        adapter.verify_account("PK06HABB0000001234567895")
    })
    .await
    .expect("task")
    .expect("verify");

    assert!(result.active);
    assert_eq!(result.account_title.as_deref(), Some("Ali Khan"));
    assert_eq!(result.bank_name.as_deref(), Some("Habib Bank Limited"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raast_verify_account_invalid_iban() {
    let server = MockServer::start().await;
    let adapter = raast_adapter(&server);

    let result = tokio::task::spawn_blocking(move || {
        adapter.verify_account("NOT-AN-IBAN")
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raast_lookup_by_alias_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/aliases/lookup"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "alias": "03001234567",
            "alias_type": "MobileNumber",
            "iban": "PK06HABB0000001234567895",
            "account_title": "Ali Khan",
            "bank_name": "Habib Bank Limited"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = raast_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        adapter.lookup_by_alias("03001234567", AliasType::MobileNumber)
    })
    .await
    .expect("task")
    .expect("lookup");

    assert_eq!(result.alias, "03001234567");
    assert_eq!(result.iban, "PK06HABB0000001234567895");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raast_lookup_alias_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/aliases/lookup"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = raast_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        adapter.lookup_by_alias("03009999999", AliasType::MobileNumber)
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raast_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/payments"))
        .respond_with(ResponseTemplate::new(502).set_body_string("Bad Gateway"))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = raast_adapter(&server);
    let result = tokio::task::spawn_blocking(move || {
        let instruction = RaastPaymentInstruction {
            amount: 50_000,
            from_iban: "PK36SCBL0000001123456702".to_string(),
            to_iban: "PK06HABB0000001234567895".to_string(),
            reference: "PAY-ERR".to_string(),
            purpose_code: Some("P2P".to_string()),
            remittance_info: None,
            idempotency_key: "idem-err-001".to_string(),
        };
        adapter.initiate_payment(&instruction)
    })
    .await
    .expect("task");
    assert!(result.is_err());
}

// ── Cross-Adapter Tests ──────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn all_adapters_report_correct_names() {
    let server = MockServer::start().await;

    let fbr = fbr_adapter(&server);
    let nadra = nadra_adapter(&server);
    let secp = secp_adapter(&server);
    let raast = raast_adapter(&server);

    assert_eq!(fbr.adapter_name(), "HttpFbrIrisAdapter");
    assert_eq!(nadra.adapter_name(), "HttpNadraAdapter");
    assert_eq!(secp.adapter_name(), "HttpSecpAdapter");
    assert_eq!(raast.adapter_name(), "HttpRaastAdapter");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raast_bank_code_accessible() {
    let server = MockServer::start().await;
    let adapter = raast_adapter(&server);
    assert_eq!(adapter.bank_code(), "HABB");
}
