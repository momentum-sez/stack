const {
  chapterHeading, h2, h3,
  p, codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter31() {
  return [
    chapterHeading("Chapter 31: Compensation and Recovery"),

    // --- 31.1 Compensation Actions ---
    h2("31.1 Compensation Actions"),
    p("Each migration phase defines a compensation action that reverses its effects:"),
    table(
      ["Phase", "Forward Action", "Compensation Action"],
      [
        ["COMPLIANCE_CHECK", "Verify source/destination compliance", "Log failure and release holds"],
        ["ATTESTATION_GATHERING", "Collect required attestations", "Revoke partial attestations"],
        ["SOURCE_LOCK", "Lock asset at source jurisdiction", "Unlock asset at source"],
        ["TRANSIT", "Transfer asset state to destination", "Rollback state to source"],
        ["DESTINATION_VERIFICATION", "Verify compliance at destination", "Return asset to source"],
      ],
      [2800, 3200, 3360]
    ),

    // --- 31.2 Saga Pattern ---
    h2("31.2 Saga Pattern"),
    p("The saga pattern maintains a persistent log of completed steps. Each step records its forward action and the corresponding compensation action. The compensation engine processes steps in reverse order, ensuring that partial migrations are fully unwound."),
    ...codeBlock(
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct CompensationStep {\n" +
      "    pub phase: MigrationPhase,\n" +
      "    pub completed_at: DateTime<Utc>,\n" +
      "    pub compensation_action: CompensationAction,\n" +
      "}\n" +
      "\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub enum CompensationAction {\n" +
      "    UnlockSource { asset_id: AssetId, jurisdiction: JurisdictionId },\n" +
      "    RollbackTransit { asset_id: AssetId, snapshot: StateSnapshot },\n" +
      "    RevokeAttestations { attestation_ids: Vec<AttestationId> },\n" +
      "    LogFailure { reason: String },\n" +
      "}\n" +
      "\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct CompensationResult {\n" +
      "    pub steps_compensated: usize,\n" +
      "    pub final_state: MigrationState,\n" +
      "    pub completed_at: DateTime<Utc>,\n" +
      "}"
    ),

    p("Compensation guarantees: every forward step has an inverse, compensation is idempotent (re-running produces the same result), and timeout handling ensures that stuck migrations are eventually compensated. If compensation itself fails, the migration enters the Failed state and requires manual intervention with full audit trail available."),

    // --- 31.3 Timeout Configuration ---
    h2("31.3 Timeout Configuration"),
    table(
      ["Phase", "Default Timeout", "On Timeout", "Retry Policy"],
      [
        ["COMPLIANCE_CHECK", "30 seconds", "Fail migration, log diagnostic", "No retry (compliance state may change)"],
        ["ATTESTATION_GATHERING", "5 minutes", "Fail if quorum not reached", "Retry once with extended watcher set"],
        ["SOURCE_LOCK", "60 seconds", "Fail migration, release any partial locks", "Retry once after 10s backoff"],
        ["TRANSIT", "2 minutes", "Rollback to source, compensate all prior steps", "No retry (state consistency risk)"],
        ["DESTINATION_VERIFICATION", "30 seconds", "Return asset to source, compensate transit", "No retry (compliance state may change)"],
      ],
      [2400, 1600, 2800, 2560]
    ),
  ];
};
