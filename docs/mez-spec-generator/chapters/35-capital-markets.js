const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  pageBreak, table
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

    // --- 35.2 Module Specifications ---
    h2("35.2 Module Specifications"),

    // --- 35.2.1 Securities Issuance Module ---
    h3("35.2.1 Securities Issuance Module"),
    p("The Securities Issuance Module orchestrates primary issuance of equity, debt, and hybrid instruments. It manages offering document generation (via Mass templating-engine), investor qualification verification (via compliance tensor), allocation computation, and settlement. Each issued security is represented as a Smart Asset with full lifecycle tracking through the receipt chain."),
    p_runs([bold("Issuance Types."), " The module supports the following primary issuance mechanisms, each with distinct regulatory requirements evaluated through the compliance tensor."]),
    table(
      ["Issuance Type", "Description", "Regulatory Requirements", "Typical Use Case"],
      [
        ["IPO (Initial Public Offering)", "First public offering of securities to the general market via a regulated exchange", "Full prospectus, regulator approval, underwriter due diligence, investor suitability checks", "Company listing on PSX, ADX, or other exchange; requires SECP/SCA approval"],
        ["Rights Issue", "Offering of new shares to existing shareholders in proportion to their current holdings", "Circular to shareholders, board resolution, regulator notification, pre-emptive rights compliance", "Capital raise by listed company; existing shareholders get priority allocation"],
        ["Private Placement", "Direct sale of securities to a limited number of qualified or institutional investors", "Information memorandum, investor accreditation verification, exemption filing", "Pre-IPO funding round, institutional capital raise; fewer disclosure requirements"],
        ["Shelf Registration", "Pre-approved registration allowing securities to be issued in tranches over a defined period", "Base prospectus, regulator pre-approval, supplement for each tranche, ongoing disclosure", "Flexible capital raising; issuer draws down as needed without repeated approval cycles"],
      ],
      [1800, 2600, 2600, 2360]
    ),

    // --- 35.2.2 Trading Module ---
    h3("35.2.2 Trading Module"),
    p("The Trading Module provides order management, matching, and execution services. It supports limit orders, market orders, and negotiated trades with jurisdiction-specific pre-trade compliance checks. All trades generate receipts anchored to the corridor state and are subject to real-time compliance tensor evaluation."),
    p_runs([bold("Market Structures."), " The module supports multiple market structure configurations, selected per venue and jurisdiction via the regpack."]),
    table(
      ["Structure", "Mechanism", "Price Discovery", "Typical Venue"],
      [
        ["Continuous Auction", "Orders matched continuously as they arrive; price/time priority", "Real-time; best bid/ask spread determines market price", "Major exchanges (PSX, NYSE); high-liquidity instruments"],
        ["Call Auction", "Orders accumulated during a collection period, then matched at a single clearing price", "Periodic; single price maximizes matched volume at auction close", "Opening/closing auctions; illiquid instruments; IPO price discovery"],
        ["Dealer Market", "Dealers quote bid/ask prices and trade from their own inventory as counterparty", "Dealer-driven; spread reflects dealer risk and inventory costs", "OTC markets, bond trading, FX corridors; lower-liquidity instruments"],
      ],
      [1800, 2800, 2400, 2360]
    ),
    p_runs([bold("Order Types."), " The following order types are supported, each with specific execution semantics and validity constraints."]),
    table(
      ["Order Type", "Execution Behavior", "Validity"],
      [
        ["Market", "Executes immediately at the best available price; no price constraint", "Immediate; fills at current market or rejects if no liquidity"],
        ["Limit", "Executes only at the specified price or better; rests in the order book if not immediately matchable", "Until filled, cancelled, or expiry (day/GTC)"],
        ["Stop", "Becomes a market order when the stop price is reached; used for loss protection or breakout entry", "Dormant until trigger; then immediate execution"],
        ["IOC (Immediate or Cancel)", "Executes any available quantity immediately at the limit price; unfilled portion is cancelled", "Instantaneous; partial fills allowed, remainder cancelled"],
        ["FOK (Fill or Kill)", "Executes only if the entire quantity can be filled immediately at the limit price; otherwise fully cancelled", "Instantaneous; all-or-nothing execution"],
        ["GTC (Good Till Cancelled)", "Limit order that remains active in the order book until explicitly cancelled or filled", "Indefinite; persists across trading sessions until cancelled"],
      ],
      [2200, 4960, 2200]
    ),

    // --- 35.2.3 Post-Trade Module ---
    h3("35.2.3 Post-Trade Module"),
    p("The Post-Trade Module handles trade confirmation, affirmation, and allocation. It implements T+0 to T+3 settlement cycles depending on jurisdiction and instrument type. The module integrates with the clearing and DVP modules for final settlement and generates post-trade Verifiable Credentials."),

    // --- 35.2.4 CSD Module ---
    h3("35.2.4 CSD Module"),
    p("The CSD Module provides central securities depository functionality including securities registry maintenance, ownership transfer recording, and corporate action processing. It interfaces with investment-info through mez-mass-client for cap table updates and maintains an immutable audit trail of all registry changes as receipt chain entries."),

    // --- 35.2.5 Clearing Module ---
    h3("35.2.5 Clearing Module"),
    p("The Clearing Module implements multilateral netting, novation, and margin management. It computes net obligations across counterparties and corridors, manages collateral requirements, and produces settlement instructions for the DVP module. The module supports both central counterparty (CCP) and bilateral clearing models."),

    // --- 35.2.6 DVP-PVP Module ---
    h3("35.2.6 DVP-PVP Module"),
    p("The DVP-PVP Module ensures atomic delivery-versus-payment and payment-versus-payment settlement. Securities delivery (via CSD) and payment (via treasury-info.api.mass.inc) are locked in an atomic transaction. If either leg fails, both are rolled back. Cross-currency settlements use the PVP mechanism with FX rates sourced from the regpack."),
    p_runs([bold("Settlement Models."), " The module supports four settlement models as defined by BIS/CPMI standards. The applicable model is determined by jurisdiction, instrument type, and clearing configuration."]),
    table(
      ["Model", "Securities Leg", "Cash Leg", "Risk Profile"],
      [
        ["Model 1: Gross/Gross", "Settled gross (trade-by-trade) with simultaneous cash transfer", "Settled gross (trade-by-trade) in real-time", "Lowest settlement risk; highest liquidity demand; each trade settled individually"],
        ["Model 2: Gross/Net", "Settled gross (trade-by-trade) throughout the settlement cycle", "Settled net at end of cycle; single cash transfer per participant", "Securities move individually; cash is netted, reducing liquidity needs"],
        ["Model 3: Net/Net", "Settled net at end of cycle; single securities transfer per participant", "Settled net at end of cycle; single cash transfer per participant", "Lowest liquidity demand; highest counterparty risk; requires netting engine"],
        ["Model 4: DvP with CCP", "Central counterparty interposes between buyer and seller; novation of obligations", "CCP manages net cash obligations with margin requirements", "CCP absorbs counterparty risk; margin and default fund provide loss mutualization"],
      ],
      [1800, 2600, 2600, 2360]
    ),

    // --- 35.2.7 Corporate Actions Module ---
    h3("35.2.7 Corporate Actions Module"),
    p("The Corporate Actions Module processes dividends, stock splits, rights issues, mergers, and other corporate events. It computes entitlements from cap table data (via investment-info), generates payment instructions (via treasury-info.api.mass.inc), and updates the securities registry. Corporate action announcements and outcomes are issued as Verifiable Credentials."),
    p_runs([bold("Mandatory Corporate Actions."), " These actions apply automatically to all holders without requiring a decision. The module processes them on the effective date using cap table data from investment-info."]),
    table(
      ["Action", "Effect on Holdings", "Cash Flow", "Processing"],
      [
        ["Cash Dividend", "No change to share count; entitlement based on record date holdings", "Payment to shareholders via treasury-info; withholding tax applied per jurisdiction", "Automatic on payment date; tax certificates issued as VCs"],
        ["Stock Dividend", "Additional shares issued proportionally; no cash movement", "None; share count increases, price adjusts on ex-date", "Registry update via CSD; new shares reflected in cap table"],
        ["Stock Split", "Share count multiplied by split ratio; par value adjusted inversely", "None; total market capitalization unchanged", "Registry restatement; all open orders and derivatives adjusted"],
        ["Merger/Acquisition", "Shares converted to acquirer shares or cash per merger terms", "Cash component (if any) paid via treasury-info", "Registry swap on effective date; fractional shares paid in cash"],
      ],
      [1800, 2600, 2600, 2360]
    ),
    p_runs([bold("Voluntary Corporate Actions."), " These actions require a holder election within a specified deadline. The module tracks elections, applies defaults for non-respondents, and settles based on chosen options."]),
    table(
      ["Action", "Holder Decision", "Deadline Handling", "Processing"],
      [
        ["Tender Offer", "Accept or reject offer to sell shares at specified price", "Default: no action (shares retained); deadline enforced via consent.api.mass.inc", "Accepted shares transferred; payment settled via treasury-info"],
        ["Rights Issue", "Exercise rights to purchase new shares at discounted price, or let rights lapse", "Default: rights lapse; tradeable rights may be sold before deadline", "Exercised rights convert to shares; payment collected via treasury-info"],
        ["Conversion", "Convert convertible instruments (bonds, preferred) to common shares per terms", "Default: no conversion; holder retains original instrument", "Converted instruments retired; new shares issued and registered"],
        ["Dividend Reinvestment", "Elect to receive dividend as additional shares instead of cash", "Default: cash payment; election registered before record date", "Reinvested amount used to purchase shares at plan price; fractional shares accumulated"],
      ],
      [1800, 2600, 2600, 2360]
    ),

    // --- 35.2.8 Surveillance Module ---
    h3("35.2.8 Surveillance Module"),
    p("The Surveillance Module monitors trading activity for market abuse, insider trading, and regulatory violations. It applies jurisdiction-specific surveillance rules from the regpack and generates alerts and suspicious transaction reports. The module integrates with the compliance tensor for real-time evaluation against 20 compliance domains."),
  ];
};
