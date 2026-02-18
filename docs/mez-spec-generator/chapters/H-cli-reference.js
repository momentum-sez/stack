const { chapterHeading, p, table, h2, h3, codeBlock } = require("../lib/primitives");

module.exports = function build_appendixH() {
  return [
    chapterHeading("Appendix H: CLI Reference"),
    p("The mez binary provides a clap-derived CLI with five subcommand groups. Global flags include --verbose (-v, -vv, -vvv for increasing verbosity), --config (path to configuration file), and --output-dir (path for generated artifacts)."),
    table(
      ["Command", "Subcommands / Flags", "Description"],
      [
        ["mez validate", "--all-modules | --zone <path> | --profile <name>", "Validate modules, profiles, and zones against their JSON schemas (Draft 2020-12)"],
        ["mez lock", "--zone <path> --output <path> | --verify <lockfile>", "Generate or verify a deterministic lockfile for a zone configuration"],
        ["mez corridor", "create | submit | activate | halt | terminate | resume | status", "Corridor lifecycle management (full FSM: Draft \u2192 Pending \u2192 Active \u2192 Halted/Suspended \u2192 Terminated)"],
        ["mez artifact", "store | resolve | verify | graph", "Content-addressed storage operations for artifact management"],
        ["mez vc", "keygen | sign | verify", "Ed25519 key generation, Verifiable Credential signing, and signature verification"],
      ],
      [1800, 3200, 4360]
    ),

    h2("H.1 Corridor Lifecycle Commands"),
    p("The corridor subcommand manages the full corridor lifecycle state machine. Each transition accepts evidence files and digests, computes content digests for the evidence, and records them in the transition audit trail."),
    ...codeBlock(
      "# Create a new corridor between two jurisdictions\n" +
      "mez corridor create \\\n" +
      "  --source PAK --target UAE \\\n" +
      "  --agreement ./agreements/pak-uae-2026.json\n" +
      "\n" +
      "# Submit corridor for review\n" +
      "mez corridor submit --corridor-id PAK-UAE-001\n" +
      "\n" +
      "# Activate an approved corridor\n" +
      "mez corridor activate --corridor-id PAK-UAE-001 \\\n" +
      "  --evidence ./evidence/activation-approval.pdf\n" +
      "\n" +
      "# Check corridor status and receipt chain head\n" +
      "mez corridor status --corridor-id PAK-UAE-001\n" +
      "\n" +
      "# Halt a corridor (reversible)\n" +
      "mez corridor halt --corridor-id PAK-UAE-001 \\\n" +
      "  --reason \"regulatory-review\" \\\n" +
      "  --evidence ./evidence/halt-order.pdf\n" +
      "\n" +
      "# Resume a halted corridor\n" +
      "mez corridor resume --corridor-id PAK-UAE-001\n" +
      "\n" +
      "# Permanently terminate a corridor\n" +
      "mez corridor terminate --corridor-id PAK-UAE-001"
    ),

    h2("H.2 Validation and Lockfile Commands"),
    ...codeBlock(
      "# Validate all modules against their schemas\n" +
      "mez validate --all-modules\n" +
      "\n" +
      "# Validate a specific zone configuration\n" +
      "mez validate --zone ./jurisdictions/_starter/zone.yaml\n" +
      "\n" +
      "# Generate a deterministic lockfile for a zone\n" +
      "mez lock --zone ./jurisdictions/_starter/zone.yaml \\\n" +
      "  --output ./jurisdictions/_starter/zone.lock.json\n" +
      "\n" +
      "# Verify an existing lockfile against current module state\n" +
      "mez lock --verify ./jurisdictions/_starter/zone.lock.json"
    ),

    h2("H.3 Artifact and Signing Commands"),
    ...codeBlock(
      "# Store an artifact in CAS and print its content digest\n" +
      "mez artifact store --file ./packs/pakistan/lawpack.json\n" +
      "\n" +
      "# Resolve an artifact by its content digest\n" +
      "mez artifact resolve --digest sha256:abc123...\n" +
      "\n" +
      "# Verify artifact integrity (declared digest vs actual)\n" +
      "mez artifact verify --all\n" +
      "\n" +
      "# Generate an Ed25519 key pair for VC signing\n" +
      "mez vc keygen --output ./keys/issuer.ed25519\n" +
      "\n" +
      "# Sign a Verifiable Credential\n" +
      "mez vc sign \\\n" +
      "  --key ./keys/issuer.ed25519 \\\n" +
      "  --credential ./credentials/compliance-attestation.json\n" +
      "\n" +
      "# Verify a signed VC\n" +
      "mez vc verify --credential ./credentials/compliance-attestation.vc.json"
    ),
  ];
};
