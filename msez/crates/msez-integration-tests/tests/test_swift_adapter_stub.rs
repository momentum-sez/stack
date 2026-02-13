//! # SWIFT pacs.008 Adapter Stub Test
//!
//! Tests the SWIFT pacs.008 payment instruction adapter for traditional
//! settlement rails. Verifies ISO 20022 XML generation, BIC validation,
//! and settlement rail type identification.

use msez_corridor::swift::{SettlementInstruction, SettlementRailError};
use msez_corridor::{SettlementRail, SwiftPacs008};

fn sample_instruction() -> SettlementInstruction {
    SettlementInstruction {
        message_id: "MSEZ-2026-IT-001".to_string(),
        debtor_bic: "HABORAEK".to_string(),
        debtor_account: "PK36HABB0000001123456702".to_string(),
        debtor_name: "Momentum SEZ Operator PKR".to_string(),
        creditor_bic: "EMIRAEAA".to_string(),
        creditor_account: "AE070331234567890123456".to_string(),
        creditor_name: "Momentum SEZ Operator AED".to_string(),
        amount: 100000, // 1000.00
        currency: "USD".to_string(),
        remittance_info: Some("Corridor settlement PK-RSEZ/AE-DIFC".to_string()),
    }
}

// ---------------------------------------------------------------------------
// 1. SWIFT instruction creation
// ---------------------------------------------------------------------------

#[test]
fn swift_instruction_creation() {
    let adapter = SwiftPacs008::new("MSEZSEXX");
    let xml = adapter.generate_instruction(&sample_instruction()).unwrap();

    assert!(!xml.is_empty());
    assert!(xml.contains("urn:iso:std:iso:20022:tech:xsd:pacs.008.001.10"));
    assert!(xml.contains("<MsgId>MSEZ-2026-IT-001</MsgId>"));
    assert!(xml.contains("Momentum SEZ Operator PKR"));
    assert!(xml.contains("Momentum SEZ Operator AED"));
}

// ---------------------------------------------------------------------------
// 2. SWIFT ISO 20022 serialization
// ---------------------------------------------------------------------------

#[test]
fn swift_iso20022_serialization() {
    let adapter = SwiftPacs008::new("MSEZSEXX");
    let xml = adapter.generate_instruction(&sample_instruction()).unwrap();

    // Verify XML structure contains key pacs.008 elements
    assert!(xml.contains("<FIToFICstmrCdtTrf>"));
    assert!(xml.contains("<GrpHdr>"));
    assert!(xml.contains("<CdtTrfTxInf>"));
    assert!(xml.contains("<BICFI>HABORAEK</BICFI>"));
    assert!(xml.contains("<BICFI>EMIRAEAA</BICFI>"));
    assert!(xml.contains("<BICFI>MSEZSEXX</BICFI>"));
    assert!(xml.contains("Ccy=\"USD\""));
    assert!(xml.contains("1000.00"));
    assert!(xml.contains("Corridor settlement PK-RSEZ/AE-DIFC"));
}

// ---------------------------------------------------------------------------
// 3. Settlement rail type identification
// ---------------------------------------------------------------------------

#[test]
fn settlement_rail_types() {
    let adapter = SwiftPacs008::new("MSEZSEXX");
    assert_eq!(adapter.rail_id(), "SWIFT");
}

// ---------------------------------------------------------------------------
// 4. BIC validation
// ---------------------------------------------------------------------------

#[test]
fn swift_rejects_invalid_bic() {
    let adapter = SwiftPacs008::new("MSEZSEXX");
    let mut instr = sample_instruction();
    instr.debtor_bic = "ABC".to_string(); // Too short
    assert!(matches!(
        adapter.generate_instruction(&instr),
        Err(SettlementRailError::InvalidBic(_))
    ));
}

// ---------------------------------------------------------------------------
// 5. Amount validation
// ---------------------------------------------------------------------------

#[test]
fn swift_rejects_non_positive_amount() {
    let adapter = SwiftPacs008::new("MSEZSEXX");
    let mut instr = sample_instruction();
    instr.amount = 0;
    assert!(matches!(
        adapter.generate_instruction(&instr),
        Err(SettlementRailError::InvalidAmount(_))
    ));
}

// ---------------------------------------------------------------------------
// 6. Default remittance info
// ---------------------------------------------------------------------------

#[test]
fn swift_default_remittance() {
    let adapter = SwiftPacs008::new("MSEZSEXX");
    let mut instr = sample_instruction();
    instr.remittance_info = None;
    let xml = adapter.generate_instruction(&instr).unwrap();
    assert!(xml.contains("SEZ Settlement"));
}
