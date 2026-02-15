const {
  chapterHeading, h2,
  p, p_runs, bold
} = require("../lib/primitives");

module.exports = function build_chapter36() {
  return [
    chapterHeading("Chapter 36: Trade and Commerce Module Family"),

    // --- 36.1 Module Overview ---
    h2("36.1 Module Overview"),
    p("The Trade and Commerce module family provides infrastructure for international trade operations including letters of credit, trade document management, supply chain finance, customs integration, and trade insurance. Each module leverages the corridor system for cross-jurisdictional trade flows and the compliance tensor for trade compliance verification."),

    // --- 36.2 Letters of Credit Module ---
    h2("36.2 Letters of Credit Module"),
    p("The Letters of Credit Module implements UCP 600-compliant documentary credit operations. It manages LC issuance, amendment, presentation, examination, and settlement. Each LC is represented as a Smart Asset with state transitions governed by the corridor lifecycle. The module integrates with treasury-info.api.mass.inc for payment operations and generates LC Verifiable Credentials at each stage."),

    // --- 36.3 Trade Documents Module ---
    h2("36.3 Trade Documents Module"),
    p("The Trade Documents Module manages bills of lading, certificates of origin, packing lists, commercial invoices, and inspection certificates. Documents are issued as Verifiable Credentials with selective disclosure support, enabling traders to share specific document fields with customs authorities, banks, or counterparties without revealing the full document."),

    // --- 36.4 Supply Chain Finance Module ---
    h2("36.4 Supply Chain Finance Module"),
    p("The Supply Chain Finance Module provides invoice financing, reverse factoring, and dynamic discounting. It leverages verified trade documents and corridor state to assess financing eligibility. The module integrates with treasury-info.api.mass.inc for disbursement and collection, and issues financing Verifiable Credentials that serve as proof of receivable assignment."),

    // --- 36.5 Customs Module ---
    h2("36.5 Customs Module"),
    p("The Customs Module generates customs declarations, manages HS code classification, and tracks duty computation. It consumes regpack data for tariff schedules, trade agreements, and preferential duty rates. The module supports single-window integration for electronic customs filing."),
    p_runs([bold("Corridor Integration."), " The Customs Module is tightly integrated with the corridor system. When goods move through a trade corridor (e.g., PAK-UAE), the module automatically applies the relevant bilateral trade agreement rates, generates the required customs documentation for both jurisdictions, and tracks duty payments through treasury-info.api.mass.inc."]),

    // --- 36.6 Trade Insurance Module ---
    h2("36.6 Trade Insurance Module"),
    p("The Trade Insurance Module manages trade credit insurance, cargo insurance, and political risk insurance. It integrates with the corridor risk assessment to compute premiums, tracks claims and settlements through the receipt chain, and issues insurance Verifiable Credentials. The module supports both single-transaction policies and revolving coverage facilities."),
  ];
};
