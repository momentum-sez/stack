const {
  partHeading, chapterHeading, h2,
  p, table,
  spacer
} = require("../lib/primitives");

module.exports = function build_chapter29() {
  return [
    ...partHeading("PART XI: MIGRATION PROTOCOL"),
    chapterHeading("Chapter 29: Cross-Jurisdictional Migration"),

    // --- 29.1 Migration Request ---
    h2("29.1 Migration Request"),
    p("The Migration Protocol orchestrates Smart Asset movement between jurisdictions while maintaining continuous compliance and operational integrity. A MigrationRequest contains: request_id, asset_id, asset_state_commitment, source_jurisdiction, destination_jurisdiction, preferred_path, requested_at timestamp, deadline, owner_signature, max_fee, and fee_currency."),

    // --- 29.2 Migration Phases ---
    h2("29.2 Migration Phases"),
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
  ];
};
