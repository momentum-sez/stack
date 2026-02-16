const {
  partHeading, chapterHeading, h2, h3,
  p, p_runs, bold, code, table,
  codeBlock,
  spacer
} = require("../lib/primitives");

module.exports = function build_chapter29() {
  return [
    ...partHeading("PART XI: MIGRATION PROTOCOL"),
    chapterHeading("Chapter 29: Cross-Jurisdictional Migration"),

    // --- 29.1 Migration Request ---
    h2("29.1 Migration Request"),
    p("The Migration Protocol orchestrates Smart Asset movement between jurisdictions while maintaining continuous compliance and operational integrity. Every migration begins with the construction and cryptographic signing of a MigrationRequest, which captures the full intent of the cross-jurisdictional transfer. The request binds the asset's current state commitment to the desired corridor, establishes fee constraints, and carries the owner's Ed25519 signature authorizing the operation."),
    spacer(),

    ...codeBlock(
      "pub struct MigrationRequest {\n" +
      "    /// Unique identifier for this migration request.\n" +
      "    pub request_id: MigrationId,\n" +
      "    /// The Smart Asset being migrated.\n" +
      "    pub asset_id: AssetId,\n" +
      "    /// SHA-256 commitment over the asset's current canonical state,\n" +
      "    /// computed via CanonicalBytes. Ensures no state drift between\n" +
      "    /// request creation and source-lock.\n" +
      "    pub asset_state_commitment: CanonicalDigest,\n" +
      "    /// ISO 3166-1 alpha-3 code of the originating jurisdiction.\n" +
      "    pub source_jurisdiction: JurisdictionCode,\n" +
      "    /// ISO 3166-1 alpha-3 code of the target jurisdiction.\n" +
      "    pub destination_jurisdiction: JurisdictionCode,\n" +
      "    /// Optional corridor path preference (e.g., PAK -> UAE direct\n" +
      "    /// vs. PAK -> SGP -> UAE transit). None lets the engine choose\n" +
      "    /// the optimal path via the Compliance Manifold.\n" +
      "    pub preferred_path: Option<Vec<JurisdictionCode>>,\n" +
      "    /// Timestamp when the request was created.\n" +
      "    pub requested_at: DateTime<Utc>,\n" +
      "    /// Hard deadline after which the migration must not proceed.\n" +
      "    /// Expired requests are automatically compensated.\n" +
      "    pub deadline: DateTime<Utc>,\n" +
      "    /// Ed25519 signature from the asset owner over the canonical\n" +
      "    /// serialization of all preceding fields.\n" +
      "    pub owner_signature: Ed25519Signature,\n" +
      "    /// Maximum fee the owner is willing to pay for the migration.\n" +
      "    pub max_fee: Decimal,\n" +
      "    /// ISO 4217 currency code for fee settlement.\n" +
      "    pub fee_currency: CurrencyCode,\n" +
      "    /// Migration priority level: Normal, Expedited, or Critical.\n" +
      "    /// Affects queue ordering and watcher bond requirements.\n" +
      "    pub priority: MigrationPriority,\n" +
      "}"
    ),
    spacer(),

    h3("29.1.1 Field Semantics"),
    p_runs([bold("request_id"), " — A globally unique identifier (UUID v7 with embedded timestamp) that serves as the correlation key across all saga steps, compensation records, and audit log entries throughout the migration lifecycle."]),
    p_runs([bold("asset_id"), " — References the Smart Asset in its source jurisdiction. The asset must exist, must not already be locked for another migration, and must be owned by the signer of ", code("owner_signature"), "."]),
    p_runs([bold("asset_state_commitment"), " — A SHA-256 digest computed via ", code("CanonicalBytes::new()"), " over the asset's full state. This commitment is re-verified at source-lock time; if the asset has changed since the request was created, the migration is rejected to prevent state drift."]),
    p_runs([bold("source_jurisdiction / destination_jurisdiction"), " — ISO 3166-1 alpha-3 jurisdiction codes. Both must be recognized by the Compliance Tensor and have active lawpacks and regpacks loaded. The source jurisdiction must permit emigration of the asset class; the destination must permit immigration."]),
    p_runs([bold("preferred_path"), " — When provided, the migration engine validates feasibility of the specified corridor chain. When absent, the Compliance Manifold's path optimization algorithm selects the route with the lowest aggregate compliance cost across all 20 domains."]),
    p_runs([bold("requested_at / deadline"), " — Together these define the migration window. The protocol enforces that ", code("deadline > requested_at"), " and that the deadline has not already passed at submission time. If any phase exceeds the deadline, the saga enters compensation automatically."]),
    p_runs([bold("owner_signature"), " — Ed25519 signature computed over the canonical serialization of all fields preceding it (request_id through deadline). Verified against the asset's registered owner public key before the saga begins."]),
    p_runs([bold("max_fee / fee_currency"), " — Fee constraints for the migration. The engine estimates corridor fees before proceeding; if the estimate exceeds ", code("max_fee"), ", the request is rejected without entering the saga. Fee currency must be supported by the treasury primitive in both jurisdictions."]),
    p_runs([bold("priority"), " — Determines queue position and operational parameters. ", code("Critical"), " priority migrations require higher watcher bonds and are processed ahead of the normal queue. ", code("Expedited"), " migrations receive priority but with standard bond requirements."]),
    spacer(),

    // --- 29.2 Migration Phases ---
    h2("29.2 Migration Phases"),
    p("The migration protocol decomposes cross-jurisdictional asset transfer into eight discrete phases. Each phase has a well-defined entry condition, action, exit condition, and compensation action. The saga pattern guarantees that if any phase fails, all previously completed phases are compensated in reverse order, restoring the system to its pre-migration state."),
    spacer(),
    table(
      ["Phase", "Action", "Compensation"],
      [
        ["INITIATED", "Request received and validated", "Log and close"],
        ["COMPLIANCE_CHECK", "Source and destination compliance verified", "Log failure reason"],
        ["ATTESTATION_GATHERING", "Required attestations collected", "Release partial attestations"],
        ["SOURCE_LOCK", "Asset locked at source jurisdiction", "Unlock at source"],
        ["TRANSIT", "Asset state in transit", "Rollback to source"],
        ["DESTINATION_VERIFICATION", "Destination compliance verification", "Return to source"],
        ["DESTINATION_UNLOCK", "Asset unlocked at destination", "N/A"],
        ["COMPLETED", "Migration successfully completed", "N/A"],
      ],
      [2800, 3600, 2960]
    ),
    spacer(),

    h3("29.2.1 Phase Details"),
    p_runs([bold("Phase 1 — INITIATED. "), "The migration request is deserialized, the owner signature is verified against the asset's registered public key, and structural validation ensures all required fields are present and well-formed. The request_id is registered in the migration log to prevent duplicate submissions. If the deadline has already passed, the request is rejected immediately without entering the saga."]),
    p_runs([bold("Phase 2 — COMPLIANCE_CHECK. "), "The Compliance Tensor (§10) is evaluated for both the source and destination jurisdictions across all 20 ComplianceDomain variants (§10.1). The source jurisdiction must permit asset emigration for the asset's class; the destination must permit immigration. Both jurisdictions' regpacks are consulted for active sanctions lists, and the entity's beneficial ownership chain is screened. If any compliance domain returns a blocking violation, the saga compensates with a detailed failure reason attached to the migration log."]),
    p_runs([bold("Phase 3 — ATTESTATION_GATHERING. "), "Required attestations are collected from jurisdiction-specific authorities. This may include tax clearance certificates from the source jurisdiction, import permits from the destination, or regulatory approvals from intermediate corridor jurisdictions. Each attestation is issued as a Verifiable Credential and added to the migration's evidence package. Partial attestation gathering is time-bounded by the request deadline; if not all attestations arrive in time, collected attestations are released and the saga compensates."]),
    p_runs([bold("Phase 4 — SOURCE_LOCK. "), "The asset is locked at the source jurisdiction by writing a lock record to the corridor state. The asset's state commitment is re-verified against the original request; if the asset state has drifted, the lock fails and the saga compensates. Once locked, the asset cannot be modified, transferred, or used as collateral in the source jurisdiction until the migration completes or compensation unlocks it."]),
    p_runs([bold("Phase 5 — TRANSIT. "), "The asset's canonical state is serialized, signed by the source jurisdiction's corridor authority, and transmitted to the destination jurisdiction. The state package includes the asset data, all gathered attestations, the compliance tensor evaluation results, and the source-lock proof. A receipt is appended to the corridor's receipt chain for auditability. During transit, neither jurisdiction considers the asset active."]),
    p_runs([bold("Phase 6 — DESTINATION_VERIFICATION. "), "The destination jurisdiction independently verifies the incoming asset state. This includes re-evaluating compliance against destination-specific rules, verifying all attestation VCs, confirming the source-lock proof, and validating the asset state commitment. If verification fails, the asset state is returned to the source jurisdiction and compensation unlocks the source-locked asset."]),
    p_runs([bold("Phase 7 — DESTINATION_UNLOCK. "), "The asset is materialized in the destination jurisdiction with a new jurisdiction-specific identifier while retaining its canonical asset_id. The destination corridor authority signs an acceptance receipt, which is appended to the receipt chain. The asset becomes active in the destination jurisdiction. This phase has no compensation action because it is immediately followed by completion."]),
    p_runs([bold("Phase 8 — COMPLETED. "), "The migration is marked as successfully completed. The source jurisdiction's lock record is finalized (marked as migrated rather than active), the corridor receipt chain is sealed, and a Migration Completion VC is issued to the asset owner. This VC serves as a portable proof of lawful cross-jurisdictional transfer. The migration log entry is closed."]),
    spacer(),
  ];
};
