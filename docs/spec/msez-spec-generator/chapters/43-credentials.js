const {
  chapterHeading, h2,
  table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter43() {
  return [
    chapterHeading("Chapter 43: Verifiable Credentials"),

    // --- 43.1 Credential Types ---
    h2("43.1 Credential Types"),
    table(
      ["Credential Type", "Issuer", "Verifier", "Selective Disclosure"],
      [
        ["KYC Attestation", "Verification Provider", "Service Provider", "Tier level without identity details"],
        ["License Credential", "Licensing Authority", "Counterparties", "Active status without full license details"],
        ["Compliance Certificate", "Compliance Watcher", "Regulators", "Domain-specific state"],
        ["Corridor Authorization", "Corridor Administrator", "Jurisdiction Nodes", "Permitted operations subset"],
        ["Entity Registration", "Corporate Registry", "Third Parties", "Entity type and status"],
        ["Tax Compliance", "Tax Authority", "Financial Institutions", "Good standing without financials"],
      ],
      [2200, 2000, 2000, 3160]
    ),
    spacer(),
  ];
};
