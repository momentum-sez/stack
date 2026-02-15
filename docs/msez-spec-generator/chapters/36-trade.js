const {
  chapterHeading, h2,
  p, p_runs, bold,
  table, spacer
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
    p_runs([bold("LC Lifecycle Stages."), " Each letter of credit progresses through the following stages. State transitions are governed by the corridor lifecycle FSM, and a Verifiable Credential is issued at each stage transition."]),
    table(
      ["Stage", "Participants", "Actions", "Outputs"],
      [
        ["Application", "Applicant (importer), Issuing Bank", "Buyer submits LC application specifying terms, amount, beneficiary, documents required, and expiry date", "LC application record; compliance tensor evaluation against sanctions and jurisdiction rules"],
        ["Issuance", "Issuing Bank, Advising Bank", "Issuing bank creates the LC and transmits via SWIFT MT700 or corridor messaging to the advising bank in beneficiary's jurisdiction", "LC Smart Asset created; LC issuance VC issued; funds earmarked via treasury-info"],
        ["Amendment", "Applicant, Issuing Bank, Beneficiary", "Any party requests changes to LC terms (amount, expiry, documents); all parties must consent via consent.api.mass.inc", "Amendment VC issued; LC Smart Asset state updated; corridor state reflects new terms"],
        ["Presentation", "Beneficiary (exporter), Presenting Bank", "Beneficiary ships goods and presents required documents (bill of lading, invoice, certificates) to the presenting bank", "Document package created; each document issued as VC with selective disclosure"],
        ["Examination", "Issuing Bank, Nominated Bank", "Banks examine presented documents for compliance with LC terms per UCP 600 Articles 14-16; discrepancies noted within 5 banking days", "Examination result VC (compliant or discrepant); discrepancy notices if applicable"],
        ["Settlement", "Issuing Bank, Beneficiary, Treasury", "Upon compliant presentation, payment is effected via treasury-info.api.mass.inc; deferred payment or acceptance as per LC terms", "Payment confirmation VC; treasury settlement record; corridor receipt chain updated"],
        ["Closure", "Issuing Bank, All Parties", "LC expires or all drawings are completed; final reconciliation of amounts drawn versus LC value", "Closure VC issued; LC Smart Asset marked terminal; funds released or returned"],
      ],
      [1400, 1800, 3560, 2600]
    ),
    spacer(),

    // --- 36.3 Trade Documents Module ---
    h2("36.3 Trade Documents Module"),
    p("The Trade Documents Module manages bills of lading, certificates of origin, packing lists, commercial invoices, and inspection certificates. Documents are issued as Verifiable Credentials with selective disclosure support, enabling traders to share specific document fields with customs authorities, banks, or counterparties without revealing the full document."),
    p_runs([bold("Trade Document Types."), " The following document types are supported. Each is issued as a Verifiable Credential with BBS+ selective disclosure, allowing field-level sharing with different parties."]),
    table(
      ["Document Type", "Issuer", "Purpose", "Key Fields"],
      [
        ["Bill of Lading (B/L)", "Carrier / Shipping Line", "Receipt of goods for shipment; document of title; evidence of contract of carriage", "Shipper, consignee, notify party, port of loading/discharge, goods description, container numbers, freight terms"],
        ["Certificate of Origin (CO)", "Chamber of Commerce / Authorized Body", "Certifies country of manufacture or production; required for preferential tariff treatment under trade agreements", "Exporter, producer, HS codes, origin criteria, trade agreement reference (e.g., FTA, GSP)"],
        ["Packing List", "Exporter / Shipper", "Detailed inventory of shipment contents; aids customs inspection and goods verification", "Package count, dimensions, weights (gross/net), marks and numbers, goods description per package"],
        ["Commercial Invoice", "Exporter / Seller", "Primary billing document; declares transaction value for customs duty computation and payment terms", "Buyer, seller, unit price, total value, currency, Incoterms, payment terms, HS codes"],
        ["Insurance Certificate", "Insurance Company / Broker", "Evidence of cargo insurance coverage; required by LC terms and importing jurisdiction regulations", "Insured party, coverage amount, risks covered, policy number, voyage details, claims procedure"],
        ["Inspection Certificate", "Independent Surveyor / Government Inspector", "Third-party verification of goods quality, quantity, or compliance with specifications before shipment", "Inspector credentials, inspection date/location, findings, pass/fail status, standards reference"],
      ],
      [1800, 1800, 2600, 3160]
    ),
    spacer(),

    // --- 36.4 Supply Chain Finance Module ---
    h2("36.4 Supply Chain Finance Module"),
    p("The Supply Chain Finance Module provides invoice financing, reverse factoring, and dynamic discounting. It leverages verified trade documents and corridor state to assess financing eligibility. The module integrates with treasury-info.api.mass.inc for disbursement and collection, and issues financing Verifiable Credentials that serve as proof of receivable assignment."),
    p_runs([bold("SCF Program Types."), " The module supports the following supply chain finance programs. Each program type defines eligibility criteria based on verified trade documents, corridor state, and counterparty creditworthiness."]),
    table(
      ["Program", "Mechanism", "Trigger", "Risk Bearer"],
      [
        ["Invoice Financing", "Supplier sells approved invoices to a financier at a discount; receives immediate cash against receivables", "Supplier uploads verified commercial invoice VC; financier evaluates credit risk against buyer", "Financier bears buyer credit risk; recourse to supplier if buyer defaults (with-recourse variant)"],
        ["Reverse Factoring", "Buyer's bank offers early payment to suppliers at a discount rate based on buyer's credit rating rather than supplier's", "Buyer approves invoice for early payment via consent.api.mass.inc; bank funds supplier at buyer-grade rate", "Buyer's bank bears risk; rate reflects buyer creditworthiness; supplier gets better terms than own credit would allow"],
        ["Dynamic Discounting", "Buyer offers early payment directly to supplier in exchange for a sliding-scale discount; earlier payment yields larger discount", "Supplier accepts early payment terms on per-invoice basis; discount rate computed based on days-early formula", "No third-party financier; buyer uses own cash; supplier reduces DSO; discount rate negotiated per corridor"],
        ["Pre-shipment Finance", "Exporter obtains financing against confirmed purchase order or LC to fund production and procurement before shipment", "Confirmed LC or purchase order VC submitted; compliance tensor verifies LC validity and buyer jurisdiction risk", "Financier bears performance risk (exporter may not ship); LC or purchase order provides secondary security"],
      ],
      [1800, 2800, 2600, 2160]
    ),
    spacer(),

    // --- 36.5 Customs Module ---
    h2("36.5 Customs Module"),
    p("The Customs Module generates customs declarations, manages HS code classification, and tracks duty computation. It consumes regpack data for tariff schedules, trade agreements, and preferential duty rates. The module supports single-window integration for electronic customs filing."),
    p_runs([bold("Corridor Integration."), " The Customs Module is tightly integrated with the corridor system. When goods move through a trade corridor (e.g., PAK-UAE), the module automatically applies the relevant bilateral trade agreement rates, generates the required customs documentation for both jurisdictions, and tracks duty payments through treasury-info.api.mass.inc."]),

    // --- 36.6 Trade Insurance Module ---
    h2("36.6 Trade Insurance Module"),
    p("The Trade Insurance Module manages trade credit insurance, cargo insurance, and political risk insurance. It integrates with the corridor risk assessment to compute premiums, tracks claims and settlements through the receipt chain, and issues insurance Verifiable Credentials. The module supports both single-transaction policies and revolving coverage facilities."),
  ];
};
