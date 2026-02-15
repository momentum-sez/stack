const {
  partHeading, chapterHeading, h2,
  p, codeBlock, table,
  spacer
} = require("../lib/primitives");

module.exports = function build_chapter29() {
  return [
    ...partHeading("PART XI: MIGRATION PROTOCOL"),
    chapterHeading("Chapter 29: Cross-Jurisdictional Migration"),

    // --- 29.1 Migration Request ---
    h2("29.1 Migration Request"),
    p("The Migration Protocol orchestrates Smart Asset movement between jurisdictions while maintaining continuous compliance and operational integrity. Every migration begins with a MigrationRequest that captures the full context needed for compliance evaluation, path planning, and fee computation."),
    ...codeBlock(
      "/// A request to migrate a Smart Asset between jurisdictions.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct MigrationRequest {\n" +
      "    pub request_id: MigrationRequestId,\n" +
      "    pub asset_id: AssetId,\n" +
      "    pub asset_state_commitment: Digest,\n" +
      "    pub source_jurisdiction: JurisdictionId,\n" +
      "    pub destination_jurisdiction: JurisdictionId,\n" +
      "    pub preferred_path: Option<Vec<JurisdictionId>>,\n" +
      "    pub requested_at: DateTime<Utc>,\n" +
      "    pub deadline: DateTime<Utc>,\n" +
      "    pub owner_signature: Ed25519Signature,\n" +
      "    pub max_fee: u128,\n" +
      "    pub fee_currency: CurrencyCode,\n" +
      "    pub migration_reason: MigrationReason,\n" +
      "}\n" +
      "\n" +
      "impl MigrationRequest {\n" +
      "    /// Validates the request structure before processing.\n" +
      "    pub fn validate(&self) -> Result<(), MigrationError> {\n" +
      "        if self.source_jurisdiction == self.destination_jurisdiction {\n" +
      "            return Err(MigrationError::SameJurisdiction);\n" +
      "        }\n" +
      "        if self.deadline <= self.requested_at {\n" +
      "            return Err(MigrationError::InvalidDeadline);\n" +
      "        }\n" +
      "        Ok(())\n" +
      "    }\n" +
      "}"
    ),
    spacer(),

    // --- 29.2 Migration Phases ---
    h2("29.2 Migration Phases"),
    p("Migration proceeds through eight phases, each with a defined forward action and a compensation action for rollback. The saga pattern ensures that partial migrations are fully unwound on failure:"),
    table(
      ["Phase", "Action", "Compensation"],
      [
        ["INITIATED", "Request received and validated", "Log and close"],
        ["COMPLIANCE_CHECK", "Source and destination compliance verified", "Log failure reason"],
        ["ATTESTATION_GATHERING", "Required attestations collected from watcher quorum", "Release partial attestations"],
        ["SOURCE_LOCK", "Asset locked at source jurisdiction, lock receipt emitted", "Unlock at source"],
        ["TRANSIT", "Asset state cryptographically transferred to destination", "Rollback state to source"],
        ["DESTINATION_VERIFICATION", "Destination compliance tensor evaluation", "Return to source"],
        ["DESTINATION_UNLOCK", "Asset unlocked at destination jurisdiction", "N/A"],
        ["COMPLETED", "Migration completed, Migration VC issued", "N/A"],
      ],
      [2800, 3600, 2960]
    ),
    spacer(),

    // --- 29.3 State Machine ---
    h2("29.3 Migration State Machine"),
    ...codeBlock(
      "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\n" +
      "pub enum MigrationState {\n" +
      "    Initiated,\n" +
      "    ComplianceCheck,\n" +
      "    AttestationGathering,\n" +
      "    SourceLocked,\n" +
      "    InTransit,\n" +
      "    DestinationVerified,\n" +
      "    Completed,\n" +
      "    Compensating,\n" +
      "    Compensated,\n" +
      "    Failed,\n" +
      "}\n" +
      "\n" +
      "impl MigrationState {\n" +
      "    /// Returns true if this state is terminal (no further transitions).\n" +
      "    pub fn is_terminal(&self) -> bool {\n" +
      "        matches!(self, Self::Completed | Self::Compensated | Self::Failed)\n" +
      "    }\n" +
      "\n" +
      "    /// Returns the set of states reachable from this state.\n" +
      "    pub fn valid_transitions(&self) -> Vec<MigrationState> {\n" +
      "        match self {\n" +
      "            Self::Initiated => vec![Self::ComplianceCheck, Self::Failed],\n" +
      "            Self::ComplianceCheck => vec![Self::AttestationGathering, Self::Compensating],\n" +
      "            Self::AttestationGathering => vec![Self::SourceLocked, Self::Compensating],\n" +
      "            Self::SourceLocked => vec![Self::InTransit, Self::Compensating],\n" +
      "            Self::InTransit => vec![Self::DestinationVerified, Self::Compensating],\n" +
      "            Self::DestinationVerified => vec![Self::Completed, Self::Compensating],\n" +
      "            Self::Completed => vec![],\n" +
      "            Self::Compensating => vec![Self::Compensated, Self::Failed],\n" +
      "            Self::Compensated => vec![],\n" +
      "            Self::Failed => vec![],\n" +
      "        }\n" +
      "    }\n" +
      "}"
    ),
    spacer(),
  ];
};
