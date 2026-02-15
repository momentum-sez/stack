const {
  chapterHeading, h2,
  p, pageBreak, table,
  spacer
} = require("../lib/primitives");

module.exports = function build_chapter35() {
  return [
    pageBreak(),
    chapterHeading("Chapter 35: Capital Markets Module Family"),

    // --- 35.1 Module Overview ---
    h2("35.1 Module Overview"),
    p("The Capital Markets module family provides end-to-end securities infrastructure from issuance through post-trade settlement. Each module integrates with the compliance tensor for jurisdiction-specific securities regulation and with the corridor system for cross-border transactions."),
    table(
      ["Module", "Function"],
      [
        ["Securities Issuance", "Primary issuance, offering documents, allocation"],
        ["Trading", "Order matching, execution, market data"],
        ["Post-Trade", "Confirmation, affirmation, allocation"],
        ["CSD", "Central securities depository, registry maintenance"],
        ["Clearing", "Netting, novation, margin management"],
        ["DVP-PVP", "Delivery versus payment, payment versus payment"],
        ["Corporate Actions", "Dividends, splits, rights issues, M&A"],
        ["Surveillance", "Market monitoring, insider trading detection, reporting"],
        ["Fund Administration", "NAV calculation, subscription/redemption, reporting"],
      ],
      [2400, 6960]
    ),
    spacer(),

    // --- 35.2 Securities Issuance Module ---
    h2("35.2 Securities Issuance Module"),
    p("The Securities Issuance Module orchestrates primary issuance of equity, debt, and hybrid instruments. It manages offering document generation (via Mass templating-engine), investor qualification verification (via compliance tensor), allocation computation, and settlement. Each issued security is represented as a Smart Asset with full lifecycle tracking through the receipt chain."),

    // --- 35.3 Trading Module ---
    h2("35.3 Trading Module"),
    p("The Trading Module provides order management, matching, and execution services. It supports limit orders, market orders, and negotiated trades with jurisdiction-specific pre-trade compliance checks. All trades generate receipts anchored to the corridor state and are subject to real-time compliance tensor evaluation."),

    // --- 35.4 Post-Trade Module ---
    h2("35.4 Post-Trade Module"),
    p("The Post-Trade Module handles trade confirmation, affirmation, and allocation. It implements T+0 to T+3 settlement cycles depending on jurisdiction and instrument type. The module integrates with the clearing and DVP modules for final settlement and generates post-trade Verifiable Credentials."),

    // --- 35.5 CSD Module ---
    h2("35.5 CSD Module"),
    p("The CSD Module provides central securities depository functionality including securities registry maintenance, ownership transfer recording, and corporate action processing. It interfaces with investment-info through msez-mass-client for cap table updates and maintains an immutable audit trail of all registry changes as receipt chain entries."),

    // --- 35.6 Clearing Module ---
    h2("35.6 Clearing Module"),
    p("The Clearing Module implements multilateral netting, novation, and margin management. It computes net obligations across counterparties and corridors, manages collateral requirements, and produces settlement instructions for the DVP module. The module supports both central counterparty (CCP) and bilateral clearing models."),

    // --- 35.7 DVP-PVP Module ---
    h2("35.7 DVP-PVP Module"),
    p("The DVP-PVP Module ensures atomic delivery-versus-payment and payment-versus-payment settlement. Securities delivery (via CSD) and payment (via treasury-info.api.mass.inc) are locked in an atomic transaction. If either leg fails, both are rolled back. Cross-currency settlements use the PVP mechanism with FX rates sourced from the regpack."),

    // --- 35.8 Corporate Actions Module ---
    h2("35.8 Corporate Actions Module"),
    p("The Corporate Actions Module processes dividends, stock splits, rights issues, mergers, and other corporate events. It computes entitlements from cap table data (via investment-info), generates payment instructions (via treasury-info.api.mass.inc), and updates the securities registry. Corporate action announcements and outcomes are issued as Verifiable Credentials."),

    // --- 35.9 Surveillance Module ---
    h2("35.9 Surveillance Module"),
    p("The Surveillance Module monitors trading activity for market abuse, insider trading, and regulatory violations. It applies jurisdiction-specific surveillance rules from the regpack and generates alerts and suspicious transaction reports. The module integrates with the compliance tensor for real-time evaluation against 20 compliance domains."),
  ];
};
