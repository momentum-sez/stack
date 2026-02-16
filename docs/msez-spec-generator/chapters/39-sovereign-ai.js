const {
  chapterHeading, h2,
  p, p_runs, bold,
  codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter39() {
  return [
    chapterHeading("Chapter 39: Sovereign AI Spine"),

    p("The Sovereign AI Spine is embedded at every GovOS layer. The foundation model runs on Pakistani data centers with zero data egress. The Spine is not an optional add-on; it is the intelligence layer that transforms the GovOS from a transactional system into an analytical governance platform."),

    table(
      ["Capability", "Function", "Key Metrics"],
      [
        ["Tax Intelligence", "Gap analysis, evasion detection, under-reporting identification, compliance scoring, revenue projection", "Target: identify 30-40% of current tax gap"],
        ["Operational Intelligence", "Spend anomaly detection across 40+ ministries, predictive budgeting, vendor risk scoring", "Anomaly detection within 24h"],
        ["Regulatory Awareness", "Pre-action compliance verification, predictive legal risk, SRO impact modeling", "SRO propagation within 4h"],
        ["Forensic and Audit", "Cross-department pattern detection, procurement irregularity flagging, IMF PIMA alignment, transfer pricing analysis", "Cross-department correlation"],
      ],
      [2000, 4400, 2960]
    ),

    h2("Revenue Recovery Estimates"),

    p("Pakistan's tax-to-GDP ratio stands at approximately 9.2%, significantly below the regional average of 15-18% and the OECD average of 34%. The Federal Board of Revenue's annual collection target consistently falls short due to structural gaps in identification, assessment, and enforcement. The AI Spine targets the three primary revenue leakage categories:"),

    table(
      ["Leakage Category", "Estimated Annual Gap (PKR)", "AI Detection Method", "Recovery Target (Year 1)"],
      [
        ["Under-reporting of income", "PKR 600-900 billion", "Cross-referencing Mass Fiscal transaction volumes against declared income in FBR IRIS returns; lifestyle analysis from NADRA data", "15-20% of identified gap"],
        ["Unreported economic activity", "PKR 400-800 billion", "Pattern detection across entity registration, banking flows, and trade corridor data; identifying entities with high transaction volumes but no NTN", "10-15% of identified gap"],
        ["WHT leakage and misclassification", "PKR 200-500 billion", "Automated withholding verification against prescribed schedules; detecting incorrect tax category claims and exempt-status fraud", "25-35% of identified gap"],
        ["Transfer pricing manipulation", "PKR 150-400 billion", "Arm's-length pricing analysis on cross-border transactions within MSEZ corridors; comparability analysis against market benchmarks", "10-20% of identified gap"],
        ["Sales tax input fraud", "PKR 150-400 billion", "Invoice chain analysis across registered entities; detecting circular invoicing, phantom suppliers, and inflated input claims", "20-30% of identified gap"],
      ],
      [1800, 1800, 3600, 2160]
    ),

    p("The aggregate tax gap is estimated at PKR 1.5-3.0 trillion annually. Conservative recovery projections for Year 1 of AI Spine deployment target PKR 200-400 billion in additional identified revenue, scaling to PKR 500-800 billion by Year 3 as models improve with more training data and broader system integration coverage."),

    p_runs([bold("Data Sovereignty."), " All AI inference runs on-premise within Pakistani data centers. The training cluster comprises 8x NVIDIA A100 80GB GPUs for model training and fine-tuning. The inference cluster comprises 4x NVIDIA A100 GPUs for production serving. No training data, inference requests, or model weights leave Pakistani jurisdiction. The foundation model is fine-tuned on Pakistani tax data, regulatory filings, corporate registrations, and trade data to produce a sovereign model that understands Pakistani law, Urdu language, and local business patterns."]),

    h2("On-Premise GPU Infrastructure"),

    p("The AI Spine requires dedicated on-premise compute infrastructure within Pakistani data centers. The following specification ensures sovereign model training and inference with zero data egress."),

    table(
      ["Cluster", "Hardware", "Configuration", "Purpose", "Throughput"],
      [
        ["Training Cluster", "8x NVIDIA A100 80GB SXM4", "NVLink fully connected, 2x AMD EPYC 7763 (128 cores), 2TB DDR4 ECC, 8x 3.84TB NVMe SSD (RAID-10), 4x ConnectX-6 200Gbps InfiniBand", "Model fine-tuning on Pakistani tax data, regulatory corpus training, entity relationship graph construction, Urdu NLP model training", "~5 PFLOPS FP16; full fine-tuning cycle in 48-72h"],
        ["Inference Cluster", "4x NVIDIA A100 40GB PCIe", "2x AMD EPYC 7543 (64 cores), 512GB DDR4 ECC, 4x 1.92TB NVMe SSD, 2x ConnectX-6 100Gbps Ethernet", "Production serving: tax gap analysis, anomaly detection, compliance verification, real-time forensic queries", "~12,000 inference requests/min at p95 latency <200ms"],
        ["Storage Tier", "NetApp AFF A400 (or equivalent)", "200TB usable NVMe flash, WORM compliance mode for audit data, AES-256 encryption at rest, replication to DR site", "Training data lake (tax returns, entity records, trade data), model checkpoint storage, inference result archival", "Sequential read 15GB/s; random IOPS 500K"],
        ["Network Fabric", "Arista 7280R3 spine-leaf", "100Gbps leaf, 400Gbps spine, dedicated AI VLAN isolated from general traffic, hardware firewall at perimeter", "Cluster interconnect, zero-egress enforcement via hardware ACLs, monitoring and logging of all data movement", "Non-blocking fabric; east-west bandwidth 12.8Tbps"],
      ],
      [1400, 1800, 2400, 2000, 1760]
    ),

    p_runs([bold("Tax Intelligence Pipeline."), " The AI Spine's primary revenue impact comes from tax intelligence. Pakistan's tax-to-GDP ratio is approximately 9.2%, compared to a regional average of 15-18%. The tax gap — the difference between taxes owed and taxes collected — is estimated at PKR 1.5-3.0 trillion annually. The AI Spine analyzes transaction patterns across Mass Fiscal, cross-references entity data from Mass Entities, and identifies under-reporting, unreported transactions, and evasion patterns. This analysis feeds into the GovOS Console as actionable intelligence for FBR officers."]),

    h2("Tax Intelligence Pipeline Detail"),

    p("The Tax Intelligence Pipeline operates as a continuous cycle. Transaction data flows from Mass Fiscal into the AI Spine, where it is enriched with entity metadata, historical filing patterns, and cross-border corridor data. The enriched data feeds into detection models that produce risk scores, which are surfaced to FBR officers through the Tax & Revenue Dashboard. Officer actions (audit outcomes, assessments, dismissals) feed back into the training loop, continuously improving model accuracy."),

    table(
      ["Pipeline Stage", "Input", "Processing", "Output"],
      [
        ["1. Data Ingestion", "Real-time transaction events from Mass Fiscal, batch imports from FBR IRIS (returns, assessments)", "Normalization, deduplication, NTN-keyed entity resolution, temporal alignment across data sources", "Unified taxpayer activity graph per NTN"],
        ["2. Feature Extraction", "Unified activity graph", "Compute 200+ features: transaction velocity, sector concentration, seasonal patterns, declared-vs-observed ratios, peer group benchmarks", "Feature vectors per entity per tax period"],
        ["3. Risk Scoring", "Feature vectors, historical model weights", "Ensemble model (gradient-boosted trees + neural network) producing risk scores across 5 dimensions: under-reporting, non-filing, WHT evasion, transfer pricing, input fraud", "Risk score vector (0.0-1.0 per dimension) with explainability annotations"],
        ["4. Alert Generation", "Risk scores above configurable thresholds", "Alert deduplication, case grouping (related entities), priority ranking by estimated revenue impact, jurisdiction assignment", "Prioritized audit case queue for FBR officers"],
        ["5. Officer Action", "Audit cases via Tax & Revenue Dashboard", "Officer reviews AI-generated evidence package, initiates assessment or dismisses with reason code", "Assessment notices, dismissed cases with feedback labels"],
        ["6. Feedback Loop", "Officer decisions, assessment outcomes, appeal results", "Supervised learning update: positive labels (confirmed evasion), negative labels (false positives), model retraining on expanded dataset", "Updated model weights deployed to inference cluster"],
      ],
      [1400, 2200, 3200, 2560]
    ),

    ...codeBlock(
`pub struct AiInferenceRequest {
    pub request_type: AiRequestType,
    pub jurisdiction: JurisdictionId,
    pub context: serde_json::Value,
    pub classification: SecurityClassification,
}

pub enum AiRequestType {
    TaxGapAnalysis,
    SpendAnomalyDetection,
    ComplianceVerification,
    ForensicPatternDetection,
}

pub enum SecurityClassification {
    Unclassified,
    Restricted,      // Tax return data, entity financials
    Confidential,    // Cross-department correlation results
    Secret,          // Ongoing evasion investigations
}

pub struct TaxRiskScore {
    pub entity_ntn: Ntn,
    pub tax_period: TaxPeriod,
    pub under_reporting: f64,   // 0.0 - 1.0
    pub non_filing: f64,        // 0.0 - 1.0
    pub wht_evasion: f64,       // 0.0 - 1.0
    pub transfer_pricing: f64,  // 0.0 - 1.0
    pub input_fraud: f64,       // 0.0 - 1.0
    pub explainability: Vec<RiskFactor>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}`
    ),

  ];
};
