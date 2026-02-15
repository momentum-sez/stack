const { chapterHeading, p, table, spacer, h2, codeBlock } = require("../lib/primitives");

module.exports = function build_appendixH() {
  return [
    chapterHeading("Appendix H: CLI Reference"),
    p("The msez binary provides a clap-derived CLI:"),
    table(
      ["Command", "Subcommand", "Description"],
      [
        ["msez init", "--profile <name> --jurisdiction <id>", "Initialize a new jurisdiction deployment"],
        ["msez pack", "import / verify / list / diff", "Pack Trilogy management"],
        ["msez deploy", "--target docker|aws|k8s", "Deploy infrastructure"],
        ["msez verify", "--all | --service <name>", "Verify deployment health"],
        ["msez corridor", "activate / status / sync", "Corridor management"],
        ["msez migrate", "up / down / status", "Database migrations"],
        ["msez artifact", "graph verify / bundle attest", "Artifact graph operations"],
        ["msez tensor", "evaluate / slice / commit", "Compliance tensor operations"],
        ["msez watcher", "register / bond / attest", "Watcher economy operations"],
        ["msez govos", "deploy / status / handover", "GovOS lifecycle management"],
      ],
      [1800, 3200, 4360]
    ),
    spacer(),

    h2("H.1 Example Usage"),
    spacer(),

    p("Initialize a Pakistan GovOS deployment with sovereign profile:"),
    ...codeBlock(
      "msez init --profile sovereign-govos --jurisdiction PAK \\\n" +
      "  --output ./deployments/pakistan \\\n" +
      "  --lawpack ./packs/pakistan/lawpack.json \\\n" +
      "  --regpack ./packs/pakistan/regpack.json \\\n" +
      "  --licensepack ./packs/pakistan/licensepack.json"
    ),
    spacer(),

    p("Import a lawpack from Akoma Ntoso XML source and verify its integrity:"),
    ...codeBlock(
      "# Import Income Tax Ordinance 2001 from Akoma Ntoso XML\n" +
      "msez pack import --type lawpack \\\n" +
      "  --source ./akn/pak/act/ito-2001.xml \\\n" +
      "  --jurisdiction PAK\n" +
      "\n" +
      "# Verify all packs for a jurisdiction\n" +
      "msez pack verify --all --jurisdiction PAK\n" +
      "\n" +
      "# List loaded packs with digest and version\n" +
      "msez pack list --jurisdiction PAK\n" +
      "\n" +
      "# Diff two versions of a regpack\n" +
      "msez pack diff --type regpack \\\n" +
      "  --from v2025.12 --to v2026.02 --jurisdiction PAK"
    ),
    spacer(),

    p("Deploy infrastructure to Kubernetes and verify health:"),
    ...codeBlock(
      "# Deploy to Kubernetes cluster\n" +
      "msez deploy --target k8s \\\n" +
      "  --config ./deployments/pakistan/k8s.yaml \\\n" +
      "  --namespace msez-pakistan\n" +
      "\n" +
      "# Verify all services are healthy\n" +
      "msez verify --all\n" +
      "\n" +
      "# Verify a specific service\n" +
      "msez verify --service treasury-proxy"
    ),
    spacer(),

    p("Manage trade corridors:"),
    ...codeBlock(
      "# Activate the PAK-UAE trade corridor\n" +
      "msez corridor activate --id PAK-UAE \\\n" +
      "  --agreement ./agreements/pak-uae-2026.vc.json\n" +
      "\n" +
      "# Check corridor status and receipt chain head\n" +
      "msez corridor status --id PAK-UAE\n" +
      "\n" +
      "# Synchronize corridor state and trigger netting\n" +
      "msez corridor sync --id PAK-UAE --force-reconcile"
    ),
    spacer(),

    p("Run database migrations:"),
    ...codeBlock(
      "# Apply all pending migrations\n" +
      "msez migrate up --database-url postgres://localhost/msez\n" +
      "\n" +
      "# Rollback the last migration\n" +
      "msez migrate down --database-url postgres://localhost/msez\n" +
      "\n" +
      "# Check migration status\n" +
      "msez migrate status --database-url postgres://localhost/msez"
    ),
    spacer(),

    p("Artifact graph verification and attestation:"),
    ...codeBlock(
      "# Verify the full artifact dependency graph\n" +
      "msez artifact graph verify --root ./deployments/pakistan\n" +
      "\n" +
      "# Bundle and attest a release artifact\n" +
      "msez artifact bundle attest \\\n" +
      "  --signing-key ./keys/release.ed25519 \\\n" +
      "  --output ./releases/v0.4.44.bundle"
    ),
    spacer(),

    p("Evaluate compliance tensor for an entity:"),
    ...codeBlock(
      "# Evaluate all 20 compliance domains for an entity\n" +
      "msez tensor evaluate \\\n" +
      "  --entity-id ent_01HX3K9M7V \\\n" +
      "  --jurisdiction PAK\n" +
      "\n" +
      "# Slice tensor to view a specific domain\n" +
      "msez tensor slice \\\n" +
      "  --entity-id ent_01HX3K9M7V \\\n" +
      "  --domain TaxCompliance\n" +
      "\n" +
      "# Commit tensor snapshot to persistence\n" +
      "msez tensor commit \\\n" +
      "  --entity-id ent_01HX3K9M7V \\\n" +
      "  --jurisdiction PAK --reason \"quarterly-review\""
    ),
    spacer(),

    p("Watcher economy operations:"),
    ...codeBlock(
      "# Register a new watcher node\n" +
      "msez watcher register \\\n" +
      "  --public-key ./keys/watcher.ed25519.pub \\\n" +
      "  --jurisdiction PAK\n" +
      "\n" +
      "# Post a bond for watcher participation\n" +
      "msez watcher bond \\\n" +
      "  --amount 10000 --currency PKR \\\n" +
      "  --watcher-id wtc_01HX4A2B3C\n" +
      "\n" +
      "# Submit a watcher attestation\n" +
      "msez watcher attest \\\n" +
      "  --watcher-id wtc_01HX4A2B3C \\\n" +
      "  --corridor PAK-UAE \\\n" +
      "  --receipt-hash sha256:abc123..."
    ),
    spacer(),

    p("GovOS lifecycle management:"),
    ...codeBlock(
      "# Deploy a full GovOS instance for Pakistan\n" +
      "msez govos deploy \\\n" +
      "  --jurisdiction PAK \\\n" +
      "  --config ./deployments/pakistan/govos.yaml\n" +
      "\n" +
      "# Check GovOS deployment status\n" +
      "msez govos status --jurisdiction PAK\n" +
      "\n" +
      "# Execute operational handover to national team\n" +
      "msez govos handover \\\n" +
      "  --jurisdiction PAK \\\n" +
      "  --recipient-key ./keys/pak-ops-team.ed25519.pub \\\n" +
      "  --sla-document ./agreements/handover-sla.vc.json"
    ),
    spacer(),
  ];
};
