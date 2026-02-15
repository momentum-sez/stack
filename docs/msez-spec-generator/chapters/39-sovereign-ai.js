const {
  chapterHeading,
  p, p_runs, bold,
  codeBlock, table, spacer
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

    spacer(),

    p_runs([bold("Data Sovereignty."), " All AI inference runs on-premise within Pakistani data centers. The training cluster comprises 8x NVIDIA A100 80GB GPUs for model training and fine-tuning. The inference cluster comprises 4x NVIDIA A100 GPUs for production serving. No training data, inference requests, or model weights leave Pakistani jurisdiction. The foundation model is fine-tuned on Pakistani tax data, regulatory filings, corporate registrations, and trade data to produce a sovereign model that understands Pakistani law, Urdu language, and local business patterns."]),

    p_runs([bold("Tax Intelligence Pipeline."), " The AI Spine's primary revenue impact comes from tax intelligence. Pakistan's tax-to-GDP ratio is approximately 9.2%, compared to a regional average of 15-18%. The tax gap — the difference between taxes owed and taxes collected — is estimated at PKR 1.5-3.0 trillion annually. The AI Spine analyzes transaction patterns across Mass Fiscal, cross-references entity data from Mass Entities, and identifies under-reporting, unreported transactions, and evasion patterns. This analysis feeds into the GovOS Console as actionable intelligence for FBR officers."]),

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
}`
    ),

    spacer(),
  ];
};
