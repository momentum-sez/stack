const {
  chapterHeading, h2,
  p, p_runs, bold,
  codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter40() {
  return [
    chapterHeading("Chapter 40: Tax Collection Pipeline"),

    p("Every economic activity on Mass generates a tax event. The pipeline operates as follows:"),

    table(
      ["Stage", "Action", "System"],
      [
        ["1. Transaction", "Economic activity occurs via Mass Treasury API", "Mass Fiscal"],
        ["2. Tax Identification", "MEZ Pack Trilogy identifies applicable tax rules", "MEZ Pack Trilogy"],
        ["3. Withholding", "Automatic withholding at source per WHT schedule", "Mass Fiscal + MEZ Bridge"],
        ["4. Reporting", "Real-time reporting to FBR IRIS", "National System Integration"],
        ["5. Gap Analysis", "AI-powered gap analysis identifies evasion patterns", "Sovereign AI Spine"],
        ["6. Enforcement", "Automated compliance actions for non-filing entities", "GovOS Console"],
      ],
      [1800, 4200, 3360]
    ),

    h2("Stage-by-Stage Pipeline Detail"),

    p_runs([bold("Stage 1 — Transaction."), " Every economic activity that flows through Mass Fiscal generates a structured transaction event. This includes payments between entities, salary disbursements, vendor payments, import/export settlements, dividend distributions, and service fee collections. Each transaction carries metadata: payer NTN, payee NTN, transaction category (per FBR classification), amount, currency, and timestamp. The transaction is recorded atomically in Mass Fiscal and simultaneously emitted as an event to the tax pipeline. No transaction can complete without generating its corresponding tax event — this is enforced at the API layer, not as an afterthought."]),

    p_runs([bold("Stage 2 — Tax Identification."), " The MEZ Pack Trilogy evaluates each transaction event against the applicable tax rules. The lawpack for Pakistan encodes the Income Tax Ordinance 2001 (including all active SROs), the Sales Tax Act 1990, and the Customs Act 1969. The system determines: (a) which taxes apply (income tax, sales tax, customs duty, federal excise duty), (b) the applicable rate based on entity type, transaction category, and any applicable exemptions or reduced rates, (c) whether withholding at source is required, and (d) filing obligations triggered by the transaction. The identification is deterministic and auditable — every tax determination references the specific legal provision."]),

    p_runs([bold("Stage 3 — Withholding."), " For transactions where withholding at source is mandated, the system computes the withholding amount, deducts it from the payment, and routes the withheld amount to the designated FBR collection account. Pakistan's Income Tax Ordinance prescribes withholding on over 60 categories of payments (sections 148-236Y). The withholding is applied before the payment reaches the payee, ensuring collection at the point of economic activity rather than relying on voluntary compliance. The withheld amount is tagged with the payer NTN, payee NTN, applicable section, and tax period for reconciliation."]),

    p_runs([bold("Stage 4 — Reporting."), " Transaction data and withholding records are reported to FBR IRIS in real-time via the National System Integration layer. Each reporting event includes: the withholding tax statement (equivalent to a digital form 149), the underlying transaction details, and a cryptographic attestation (W3C Verifiable Credential) proving the computation chain from transaction to withholding. This replaces the current manual quarterly/annual filing of withholding statements with continuous, real-time reporting. FBR officers see withholding data as it occurs, not months after the fact."]),

    p_runs([bold("Stage 5 — Gap Analysis."), " The Sovereign AI Spine continuously analyzes the stream of tax events against expected patterns. For each registered entity, the AI maintains a dynamic tax profile: expected transaction volumes by category, seasonal patterns, peer-group benchmarks, and historical filing behavior. Deviations trigger risk scores: an entity whose declared sales tax input credits exceed its observed purchase transactions, an entity whose withholding certificates received far exceed its declared income, or an entity with high-volume transactions but no filed return. These risk scores feed the Tax & Revenue Dashboard as prioritized audit leads."]),

    p_runs([bold("Stage 6 — Enforcement."), " The GovOS Console surfaces enforcement actions for FBR officers. For non-filing entities, the system generates automated notices (with configurable escalation: reminder, warning, penalty notice, account restriction). For entities flagged by gap analysis, the system prepares audit case files with AI-generated evidence packages including transaction summaries, anomaly explanations, and comparable entity benchmarks. All enforcement actions are recorded as consent workflows via Mass Consent, ensuring due process and audit trails. Appeals are routed through the MEZ Arbitration module."]),

    h2("Pakistan Withholding Tax Rate Schedule"),

    p("The following table lists representative withholding tax rates under the Income Tax Ordinance 2001 as applicable to transactions processed through Mass Fiscal. Rates shown are for tax year 2025 and are subject to amendment by Finance Act or SRO. The MEZ Pack Trilogy maintains the authoritative rate table and is updated within 4 hours of any SRO notification."),

    table(
      ["ITO Section", "Payment Category", "Filer Rate", "Non-Filer Rate", "Collection Point"],
      [
        ["149 / 148", "Import of goods", "1-6% (value-dependent)", "2-9% (value-dependent)", "At customs clearance via PSW integration"],
        ["153(1)(a)", "Sale of goods", "4.0%", "8.0%", "At payment via Mass Fiscal"],
        ["153(1)(b)", "Rendering of services", "4.0%", "8.0%", "At payment via Mass Fiscal"],
        ["153(1)(c)", "Execution of contracts", "7.0%", "14.0%", "At payment via Mass Fiscal"],
        ["155", "Income on debt (profit on deposits)", "15.0%", "30.0%", "At credit by financial institution"],
        ["156", "Prizes and winnings", "15.0%", "30.0%", "At payment via Mass Fiscal"],
        ["231A", "Cash withdrawal exceeding PKR 50,000", "0.6%", "1.2%", "At withdrawal via banking integration"],
        ["231AA", "Banking transactions (non-filer)", "N/A", "0.6% of transaction", "At transaction via banking integration"],
        ["236G", "Sale to retailers", "0.5%", "1.0%", "At sale via Mass Fiscal"],
        ["236H", "Sale to distributors/dealers/wholesalers", "0.5%", "1.0%", "At sale via Mass Fiscal"],
        ["236P", "Advance tax on banking transactions", "0.6%", "0.6%", "At transaction exceeding PKR 50,000"],
        ["152(1)", "Payments to non-residents (royalty/fee)", "15-20%", "15-20%", "At remittance via SBP Raast/RTGS"],
      ],
      [1000, 2400, 1600, 1800, 2560]
    ),

    h2("Pipeline Flow"),

    p("The following code block illustrates the core pipeline data flow from transaction event to tax determination, withholding computation, and reporting."),

    ...codeBlock(
`/// A tax event emitted by Mass Fiscal for every economic transaction.
pub struct TaxEvent {
    pub transaction_id: TransactionId,
    pub payer_ntn: Ntn,
    pub payee_ntn: Ntn,
    pub category: TransactionCategory,
    pub amount: Decimal,
    pub currency: CurrencyCode,       // ISO 4217; PKR for domestic
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// The result of Pack Trilogy evaluation against a TaxEvent.
pub struct TaxDetermination {
    pub event: TaxEvent,
    pub applicable_taxes: Vec<ApplicableTax>,
    pub withholding_required: bool,
    pub legal_basis: Vec<LegalReference>,  // Akoma Ntoso act IDs + section
}

pub struct ApplicableTax {
    pub tax_type: TaxType,           // IncomeTax, SalesTax, CustomsDuty, FED
    pub ito_section: String,         // e.g., "153(1)(a)"
    pub rate: Decimal,               // e.g., 0.04 for 4%
    pub filer_status: FilerStatus,   // Filer | NonFiler
    pub computed_amount: Decimal,    // rate * taxable_amount
}

/// Pipeline execution: Transaction -> Identify -> Withhold -> Report
pub async fn process_tax_event(
    event: TaxEvent,
    packs: &PackTrilogy,
    mass_fiscal: &MassFiscalClient,
    fbr_iris: &FbrIrisClient,
) -> Result<TaxPipelineResult, TaxPipelineError> {
    // Stage 2: Identify applicable taxes via Pack Trilogy
    let determination = packs.evaluate_tax_event(&event)?;

    // Stage 3: Withhold at source if required
    let withholding = if determination.withholding_required {
        let wht = mass_fiscal.apply_withholding(
            &event.transaction_id,
            &determination.applicable_taxes,
        ).await?;
        Some(wht)
    } else {
        None
    };

    // Stage 4: Report to FBR IRIS in real-time
    let report = fbr_iris.submit_withholding_statement(
        &determination,
        &withholding,
    ).await?;

    Ok(TaxPipelineResult {
        determination,
        withholding,
        fbr_report_id: report.confirmation_id,
    })
}`
    ),

  ];
};
