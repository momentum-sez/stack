//! # SWIFT pacs.008 Adapter
//!
//! Generates SWIFT pacs.008 (FIToFICustomerCreditTransfer) payment
//! instructions for traditional settlement rails.
//!
//! ## Design
//!
//! The [`SettlementRail`] trait defines the interface for settlement
//! instruction generation. The trait is **sealed** per audit §5.5 —
//! only implementations within this crate are permitted.
//!
//! [`SwiftPacs008`] implements `SettlementRail` by constructing ISO 20022
//! pacs.008 message structures. For Phase 1, the adapter builds the
//! message data structure and serializes to XML. Actual SWIFT network
//! integration is deferred to Phase 5.
//!
//! ## pacs.008 Structure
//!
//! A pacs.008 (FIToFICustomerCreditTransfer) message contains:
//! - **GroupHeader**: Message ID, creation timestamp, number of transactions
//! - **CreditTransferTransaction**: Debtor, creditor, amount, currency,
//!   settlement information, and remittance information
//!
//! ## Spec Reference
//!
//! Extends the settlement rail concepts from `spec/40-corridors.md`.

use serde::{Deserialize, Serialize};

/// Escape a string for safe inclusion in XML text content.
///
/// Replaces the five XML special characters with their entity references.
/// This prevents XML injection when user-supplied strings (names, account IDs,
/// remittance info) are interpolated into pacs.008 messages.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Trait for settlement rail implementations.
///
/// Sealed — only implementations within this crate are permitted.
/// This prevents unaudited settlement rail adapters from being used
/// in production, which could result in incorrect payment instructions.
///
/// ## Audit Reference
///
/// Per audit §5.5: settlement rail trait is sealed.
pub trait SettlementRail: private::Sealed {
    /// Generate a settlement instruction for the given parameters.
    fn generate_instruction(
        &self,
        instruction: &SettlementInstruction,
    ) -> Result<String, SettlementRailError>;

    /// Return the rail identifier (e.g., "SWIFT", "RTGS").
    fn rail_id(&self) -> &str;
}

mod private {
    pub trait Sealed {}
    impl Sealed for super::SwiftPacs008 {}
}

/// Error from settlement rail operations.
#[derive(Debug, thiserror::Error)]
pub enum SettlementRailError {
    /// Invalid BIC code.
    #[error("invalid BIC: {0}")]
    InvalidBic(String),

    /// Missing required field.
    #[error("missing required field: {0}")]
    MissingField(String),

    /// Amount validation failed.
    #[error("invalid amount: {0}")]
    InvalidAmount(String),
}

/// A settlement instruction to be sent over a settlement rail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettlementInstruction {
    /// Unique message identifier.
    pub message_id: String,
    /// Debtor (paying) institution BIC.
    pub debtor_bic: String,
    /// Debtor account identifier (e.g., IBAN).
    pub debtor_account: String,
    /// Debtor name.
    pub debtor_name: String,
    /// Creditor (receiving) institution BIC.
    pub creditor_bic: String,
    /// Creditor account identifier.
    pub creditor_account: String,
    /// Creditor name.
    pub creditor_name: String,
    /// Settlement amount in smallest currency unit.
    pub amount: i64,
    /// ISO 4217 currency code.
    pub currency: String,
    /// Remittance information (payment reference).
    pub remittance_info: Option<String>,
}

/// SWIFT pacs.008 (FIToFICustomerCreditTransfer) message adapter.
///
/// Constructs ISO 20022 pacs.008 XML messages from settlement instructions.
/// For Phase 1, generates well-formed XML structures without SWIFT network
/// connectivity. SWIFT integration is Phase 5.
#[derive(Debug, Default)]
pub struct SwiftPacs008 {
    /// Instructing agent BIC (the SEZ settlement node).
    instructing_agent_bic: String,
}

impl SwiftPacs008 {
    /// Create a new SWIFT pacs.008 adapter.
    ///
    /// `instructing_agent_bic` is the BIC of the SEZ settlement node
    /// that originates the payment instruction.
    pub fn new(instructing_agent_bic: impl Into<String>) -> Self {
        Self {
            instructing_agent_bic: instructing_agent_bic.into(),
        }
    }

    /// Validate a BIC code (basic format check).
    fn validate_bic(bic: &str) -> Result<(), SettlementRailError> {
        let trimmed = bic.trim();
        if trimmed.len() != 8 && trimmed.len() != 11 {
            return Err(SettlementRailError::InvalidBic(format!(
                "BIC must be 8 or 11 characters, got {}",
                trimmed.len()
            )));
        }
        if !trimmed.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(SettlementRailError::InvalidBic(
                "BIC must be alphanumeric".to_string(),
            ));
        }
        Ok(())
    }
}

impl SettlementRail for SwiftPacs008 {
    fn generate_instruction(
        &self,
        instruction: &SettlementInstruction,
    ) -> Result<String, SettlementRailError> {
        // Validate BICs.
        Self::validate_bic(&instruction.debtor_bic)?;
        Self::validate_bic(&instruction.creditor_bic)?;

        if instruction.amount <= 0 {
            return Err(SettlementRailError::InvalidAmount(format!(
                "amount must be positive, got {}",
                instruction.amount
            )));
        }

        if instruction.message_id.is_empty() {
            return Err(SettlementRailError::MissingField("message_id".to_string()));
        }

        // Format amount with decimal point based on ISO 4217 currency convention.
        // Most currencies use 2 decimal places, but JPY/KRW use 0, BHD/KWD/OMR use 3.
        let minor_units = match instruction.currency.as_str() {
            "JPY" | "KRW" | "VND" | "CLP" | "ISK" | "UGX" | "PYG" => 0,
            "BHD" | "KWD" | "OMR" => 3,
            _ => 2, // USD, EUR, GBP, PKR, AED, SGD, etc.
        };
        let formatted_amount = match minor_units {
            0 => format!("{}", instruction.amount),
            3 => {
                let major = instruction.amount / 1000;
                let minor = (instruction.amount % 1000).unsigned_abs();
                format!("{major}.{minor:03}")
            }
            _ => {
                let major = instruction.amount / 100;
                let minor = (instruction.amount % 100).unsigned_abs();
                format!("{major}.{minor:02}")
            }
        };

        let remittance = instruction
            .remittance_info
            .as_deref()
            .unwrap_or("SEZ Settlement");

        // Build ISO 20022 pacs.008 XML structure.
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.10">
  <FIToFICstmrCdtTrf>
    <GrpHdr>
      <MsgId>{msg_id}</MsgId>
      <NbOfTxs>1</NbOfTxs>
      <SttlmInf>
        <SttlmMtd>INDA</SttlmMtd>
      </SttlmInf>
      <InstgAgt>
        <FinInstnId>
          <BICFI>{instructing_bic}</BICFI>
        </FinInstnId>
      </InstgAgt>
    </GrpHdr>
    <CdtTrfTxInf>
      <PmtId>
        <InstrId>{msg_id}</InstrId>
        <EndToEndId>{msg_id}</EndToEndId>
      </PmtId>
      <IntrBkSttlmAmt Ccy="{currency}">{amount}</IntrBkSttlmAmt>
      <Dbtr>
        <Nm>{debtor_name}</Nm>
      </Dbtr>
      <DbtrAgt>
        <FinInstnId>
          <BICFI>{debtor_bic}</BICFI>
        </FinInstnId>
      </DbtrAgt>
      <DbtrAcct>
        <Id>
          <Othr>
            <Id>{debtor_account}</Id>
          </Othr>
        </Id>
      </DbtrAcct>
      <Cdtr>
        <Nm>{creditor_name}</Nm>
      </Cdtr>
      <CdtrAgt>
        <FinInstnId>
          <BICFI>{creditor_bic}</BICFI>
        </FinInstnId>
      </CdtrAgt>
      <CdtrAcct>
        <Id>
          <Othr>
            <Id>{creditor_account}</Id>
          </Othr>
        </Id>
      </CdtrAcct>
      <RmtInf>
        <Ustrd>{remittance}</Ustrd>
      </RmtInf>
    </CdtTrfTxInf>
  </FIToFICstmrCdtTrf>
</Document>"#,
            msg_id = xml_escape(&instruction.message_id),
            instructing_bic = xml_escape(&self.instructing_agent_bic),
            currency = xml_escape(&instruction.currency),
            amount = formatted_amount,
            debtor_name = xml_escape(&instruction.debtor_name),
            debtor_bic = xml_escape(&instruction.debtor_bic),
            debtor_account = xml_escape(&instruction.debtor_account),
            creditor_name = xml_escape(&instruction.creditor_name),
            creditor_bic = xml_escape(&instruction.creditor_bic),
            creditor_account = xml_escape(&instruction.creditor_account),
            remittance = xml_escape(remittance),
        );

        Ok(xml)
    }

    fn rail_id(&self) -> &str {
        "SWIFT"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_instruction() -> SettlementInstruction {
        SettlementInstruction {
            message_id: "MSEZ-2026-001".to_string(),
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

    #[test]
    fn generates_valid_xml() {
        let adapter = SwiftPacs008::new("MSEZSEXX");
        let xml = adapter.generate_instruction(&sample_instruction()).unwrap();

        assert!(xml.contains("urn:iso:std:iso:20022:tech:xsd:pacs.008.001.10"));
        assert!(xml.contains("<MsgId>MSEZ-2026-001</MsgId>"));
        assert!(xml.contains("<BICFI>MSEZSEXX</BICFI>"));
        assert!(xml.contains("<BICFI>HABORAEK</BICFI>"));
        assert!(xml.contains("<BICFI>EMIRAEAA</BICFI>"));
        assert!(xml.contains("Ccy=\"USD\""));
        assert!(xml.contains("1000.00"));
        assert!(xml.contains("Corridor settlement PK-RSEZ/AE-DIFC"));
    }

    #[test]
    fn rail_id() {
        let adapter = SwiftPacs008::new("MSEZSEXX");
        assert_eq!(adapter.rail_id(), "SWIFT");
    }

    #[test]
    fn rejects_invalid_bic_length() {
        let adapter = SwiftPacs008::new("MSEZSEXX");
        let mut instr = sample_instruction();
        instr.debtor_bic = "ABC".to_string();
        let result = adapter.generate_instruction(&instr);
        assert!(matches!(result, Err(SettlementRailError::InvalidBic(_))));
    }

    #[test]
    fn rejects_non_alphanumeric_bic() {
        let adapter = SwiftPacs008::new("MSEZSEXX");
        let mut instr = sample_instruction();
        instr.creditor_bic = "EMIR@#$A".to_string();
        let result = adapter.generate_instruction(&instr);
        assert!(matches!(result, Err(SettlementRailError::InvalidBic(_))));
    }

    #[test]
    fn rejects_non_positive_amount() {
        let adapter = SwiftPacs008::new("MSEZSEXX");
        let mut instr = sample_instruction();
        instr.amount = 0;
        let result = adapter.generate_instruction(&instr);
        assert!(matches!(result, Err(SettlementRailError::InvalidAmount(_))));

        instr.amount = -100;
        let result = adapter.generate_instruction(&instr);
        assert!(matches!(result, Err(SettlementRailError::InvalidAmount(_))));
    }

    #[test]
    fn rejects_empty_message_id() {
        let adapter = SwiftPacs008::new("MSEZSEXX");
        let mut instr = sample_instruction();
        instr.message_id = String::new();
        let result = adapter.generate_instruction(&instr);
        assert!(matches!(result, Err(SettlementRailError::MissingField(_))));
    }

    #[test]
    fn accepts_11_char_bic() {
        let adapter = SwiftPacs008::new("MSEZSEXXABC");
        let mut instr = sample_instruction();
        instr.debtor_bic = "HABORAEKXXX".to_string();
        instr.creditor_bic = "EMIRAEAAXXX".to_string();
        let result = adapter.generate_instruction(&instr);
        assert!(result.is_ok());
    }

    #[test]
    fn default_remittance_info() {
        let adapter = SwiftPacs008::new("MSEZSEXX");
        let mut instr = sample_instruction();
        instr.remittance_info = None;
        let xml = adapter.generate_instruction(&instr).unwrap();
        assert!(xml.contains("SEZ Settlement"));
    }

    #[test]
    fn xml_injection_prevented_in_names() {
        let adapter = SwiftPacs008::new("MSEZSEXX");
        let mut instr = sample_instruction();
        instr.debtor_name =
            r#"Evil<Nm>INJECTED</Nm></CdtTrfTxInf><script>alert("xss")</script>"#.to_string();
        instr.creditor_name = "O'Brien & Sons \"Ltd\"".to_string();
        let xml = adapter.generate_instruction(&instr).unwrap();
        assert!(xml.contains("Evil&lt;Nm&gt;INJECTED&lt;/Nm&gt;"));
        assert!(xml.contains("O&apos;Brien &amp; Sons &quot;Ltd&quot;"));
        assert!(!xml.contains("<script>"));
    }

    #[test]
    fn xml_injection_prevented_in_accounts() {
        let adapter = SwiftPacs008::new("MSEZSEXX");
        let mut instr = sample_instruction();
        instr.debtor_account = "ACCT&<>\"'".to_string();
        instr.remittance_info = Some("Pay & <settle>".to_string());
        let xml = adapter.generate_instruction(&instr).unwrap();
        assert!(xml.contains("ACCT&amp;&lt;&gt;&quot;&apos;"));
        assert!(xml.contains("Pay &amp; &lt;settle&gt;"));
    }

    #[test]
    fn xml_escape_handles_empty_and_normal() {
        assert_eq!(xml_escape(""), "");
        assert_eq!(xml_escape("normal text"), "normal text");
        assert_eq!(xml_escape("a&b"), "a&amp;b");
        assert_eq!(xml_escape("<tag>"), "&lt;tag&gt;");
    }

    #[test]
    fn jpy_zero_decimal_formatting() {
        let adapter = SwiftPacs008::new("MSEZSEXX");
        let mut instr = sample_instruction();
        instr.currency = "JPY".to_string();
        instr.amount = 50000;
        let xml = adapter.generate_instruction(&instr).unwrap();
        // JPY has 0 decimal places — amount is already in whole yen
        assert!(xml.contains(">50000<"));
    }

    #[test]
    fn bhd_three_decimal_formatting() {
        let adapter = SwiftPacs008::new("MSEZSEXX");
        let mut instr = sample_instruction();
        instr.currency = "BHD".to_string();
        instr.amount = 12345; // 12.345 BHD
        let xml = adapter.generate_instruction(&instr).unwrap();
        assert!(xml.contains("12.345"));
    }

    #[test]
    fn settlement_instruction_serialization() {
        let instr = sample_instruction();
        let json = serde_json::to_string(&instr).unwrap();
        let deserialized: SettlementInstruction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.message_id, instr.message_id);
        assert_eq!(deserialized.amount, instr.amount);
    }
}
