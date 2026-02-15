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
    table(
      ["Issuance Type", "Description", "Regulatory Requirements"],
      [
        ["Initial Public Offering (IPO)", "First public sale of securities with prospectus filing", "Prospectus approval, underwriter appointment, lock-up periods"],
        ["Rights Issue", "Offer to existing shareholders proportional to holdings", "Shareholder circular, record date, subscription period"],
        ["Private Placement", "Sale to qualified institutional buyers without public offering", "Investor accreditation verification, placement memorandum"],
        ["Shelf Registration", "Pre-approved framework for multiple future issuances", "Base prospectus, takedown supplements per tranche"],
      ],
      [2400, 3600, 3360]
    ),
    spacer(),

    // --- 35.3 Trading Module ---
    h2("35.3 Trading Module"),
    p("The Trading Module provides order management, matching, and execution services with jurisdiction-specific pre-trade compliance checks. All trades generate receipts anchored to the corridor state and are subject to real-time compliance tensor evaluation."),
    table(
      ["Market Structure", "Matching Mechanism", "Use Case"],
      [
        ["Order-Driven", "Continuous limit order book with price-time priority", "Liquid markets with high participant count"],
        ["Quote-Driven", "Dealer-provided bid/ask quotes with obligation to trade", "Less liquid markets, OTC securities"],
        ["Hybrid", "Combined order book with designated market makers", "Markets requiring liquidity guarantees"],
      ],
      [2400, 3600, 3360]
    ),
    spacer(),
    p("Six order types are supported across all market structures:"),
    table(
      ["Order Type", "Behavior", "Validity"],
      [
        ["Limit", "Execute at specified price or better", "Until filled, cancelled, or expired"],
        ["Market", "Execute immediately at best available price", "Immediate (fill or partial fill)"],
        ["Stop", "Becomes market order when trigger price reached", "Until triggered or cancelled"],
        ["Iceberg", "Large order with only a portion visible in the book", "Until fully filled or cancelled"],
        ["Fill-or-Kill (FOK)", "Execute entire quantity immediately or cancel", "Immediate (all or nothing)"],
        ["Good-Till-Cancelled (GTC)", "Remains active until explicitly cancelled", "Until cancelled (max 90 days)"],
      ],
      [2400, 3600, 3360]
    ),
    spacer(),

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
    p("The DVP-PVP Module ensures atomic delivery-versus-payment and payment-versus-payment settlement. Four settlement models accommodate different risk profiles and operational requirements:"),
    table(
      ["Model", "Securities Leg", "Cash Leg", "Risk Profile"],
      [
        ["Model 1 (Gross/Gross)", "Gross settlement per trade", "Gross settlement per trade", "Zero principal risk; highest liquidity demand"],
        ["Model 2 (Gross/Net)", "Gross settlement per trade", "Net settlement end-of-day", "Low principal risk; moderate liquidity"],
        ["Model 3 (Net/Net)", "Net settlement end-of-day", "Net settlement end-of-day", "Settlement risk window; lowest liquidity"],
        ["Model 4 (CCP-Cleared)", "CCP-guaranteed net settlement", "CCP-guaranteed net settlement", "CCP absorbs counterparty risk; margin required"],
      ],
      [2000, 2200, 2200, 2960]
    ),
    spacer(),
    p("Cross-currency settlements use the PVP mechanism with FX rates sourced from the regpack. The PVP protocol extends DVP with simultaneous settlement of both currency legs, eliminating Herstatt risk."),

    // --- 35.8 Corporate Actions Module ---
    h2("35.8 Corporate Actions Module"),
    p("The Corporate Actions Module processes corporate events affecting securities holders. Events are classified as mandatory (automatic application to all holders) or voluntary (requiring holder election):"),
    table(
      ["Category", "Action Type", "Processing"],
      [
        ["Mandatory", "Cash Dividend", "Automatic distribution computed from cap table, withholding tax applied per jurisdiction"],
        ["Mandatory", "Stock Split / Reverse Split", "Registry update with ratio adjustment, fractional share handling"],
        ["Mandatory", "Name / Symbol Change", "Registry metadata update, notification to all holders"],
        ["Voluntary", "Rights Issue", "Subscription offer to existing holders, oversubscription handling, rump placement"],
        ["Voluntary", "Tender Offer", "Offer acceptance/rejection, proration if oversubscribed, settlement"],
        ["Voluntary", "Convertible Conversion", "Conversion ratio application, new share issuance, bond retirement"],
      ],
      [1600, 2400, 5360]
    ),
    spacer(),
    p("All corporate action entitlements are computed from cap table data (via investment-info), payment instructions generated (via treasury-info.api.mass.inc), and outcomes recorded as Verifiable Credentials."),

    // --- 35.9 Surveillance Module ---
    h2("35.9 Surveillance Module"),
    p("The Surveillance Module monitors trading activity for market abuse and regulatory violations. It applies jurisdiction-specific surveillance rules from the regpack and generates alerts and suspicious transaction reports."),
    table(
      ["Monitoring Type", "Indicators", "Alert Threshold"],
      [
        ["Price Surveillance", "Abnormal price movements, gap detection, closing price manipulation", "Configurable per instrument class"],
        ["Volume Surveillance", "Unusual volume spikes, wash trading patterns, pre-announcement activity", ">3\u03c3 from 30-day average"],
        ["Timing Surveillance", "Pre-announcement trading, post-hours activity, cross-market timing", "Correlated with material events"],
        ["Cross-Market Surveillance", "Inter-market arbitrage abuse, cross-venue manipulation", "Cross-venue correlation analysis"],
      ],
      [2000, 4200, 3160]
    ),
    spacer(),
    table(
      ["Abuse Pattern", "Detection Method", "Response"],
      [
        ["Insider Trading", "Pre-announcement trading correlation with material non-public information", "Alert + automatic position freeze"],
        ["Front-Running", "Order timing analysis relative to large incoming orders", "Alert + order cancellation"],
        ["Layering", "Non-genuine order detection through placement/cancellation patterns", "Alert + participant review"],
        ["Spoofing", "Intent analysis on orders placed and cancelled before execution", "Alert + market access suspension"],
      ],
      [2000, 4200, 3160]
    ),
    spacer(),
    p("Circuit breakers halt trading when price movements exceed defined thresholds: Level 1 (5% move) triggers a 5-minute cooling period, Level 2 (10% move) triggers a 15-minute halt, and Level 3 (20% move) triggers a market-wide halt with manual restart."),
  ];
};
