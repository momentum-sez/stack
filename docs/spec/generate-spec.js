const fs = require("fs");
const {
  Document, Packer, Paragraph, TextRun, Table, TableRow, TableCell,
  Header, Footer, AlignmentType, LevelFormat,
  TableOfContents, HeadingLevel, BorderStyle, WidthType, ShadingType,
  VerticalAlign, PageNumber, PageBreak, TabStopType, TabStopPosition,
  PositionalTab, PositionalTabAlignment, PositionalTabRelativeTo, PositionalTabLeader,
  SectionType
} = require("docx");

// ─── CONSTANTS ─────────────────────────────────────────────
const PAGE_W = 12240;
const PAGE_H = 15840;
const MARGIN = 1440;
const CONTENT_W = PAGE_W - 2 * MARGIN; // 9360
const DARK = "1B2A4A";
const ACCENT = "2E5090";
const LIGHT_GRAY = "F5F5F5";
const CODE_FONT = "Courier New";
const BODY_FONT = "Arial";

// ─── BORDER HELPERS ────────────────────────────────────────
const thinBorder = { style: BorderStyle.SINGLE, size: 1, color: "CCCCCC" };
const borders = { top: thinBorder, bottom: thinBorder, left: thinBorder, right: thinBorder };
const noBorders = {
  top: { style: BorderStyle.NONE, size: 0 },
  bottom: { style: BorderStyle.NONE, size: 0 },
  left: { style: BorderStyle.NONE, size: 0 },
  right: { style: BorderStyle.NONE, size: 0 },
};

// ─── HELPER FUNCTIONS ──────────────────────────────────────
function p(text, opts = {}) {
  const runs = [];
  if (typeof text === "string") {
    runs.push(new TextRun({ text, font: BODY_FONT, size: 22, ...opts }));
  } else if (Array.isArray(text)) {
    text.forEach(t => {
      if (typeof t === "string") runs.push(new TextRun({ text: t, font: BODY_FONT, size: 22 }));
      else runs.push(new TextRun({ font: BODY_FONT, size: 22, ...t }));
    });
  }
  return new Paragraph({ spacing: { after: 120, line: 276 }, children: runs });
}

function bold(text) { return { text, bold: true }; }
function italic(text) { return { text, italics: true }; }
function code(text) { return { text, font: CODE_FONT, size: 18 }; }

function heading1(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_1,
    spacing: { before: 360, after: 200 },
    children: [new TextRun({ text, font: BODY_FONT, size: 32, bold: true, color: DARK })],
  });
}

function heading2(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_2,
    spacing: { before: 280, after: 160 },
    children: [new TextRun({ text, font: BODY_FONT, size: 26, bold: true, color: ACCENT })],
  });
}

function heading3(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_3,
    spacing: { before: 200, after: 120 },
    children: [new TextRun({ text, font: BODY_FONT, size: 22, bold: true, color: DARK })],
  });
}

function pageBreak() {
  return new Paragraph({ children: [new PageBreak()] });
}

function codeParagraph(text) {
  return new Paragraph({
    spacing: { after: 40, line: 240 },
    shading: { type: ShadingType.CLEAR, fill: LIGHT_GRAY },
    indent: { left: 200, right: 200 },
    children: [new TextRun({ text, font: CODE_FONT, size: 16 })],
  });
}

function codeBlock(lines) {
  return lines.map(line => codeParagraph(line));
}

function makeTable(headers, rows, colWidths) {
  if (!colWidths) {
    const w = Math.floor(CONTENT_W / headers.length);
    colWidths = headers.map(() => w);
    // adjust last column
    const sum = colWidths.reduce((a, b) => a + b, 0);
    colWidths[colWidths.length - 1] += CONTENT_W - sum;
  }
  const cellMargins = { top: 60, bottom: 60, left: 100, right: 100 };

  const headerRow = new TableRow({
    tableHeader: true,
    children: headers.map((h, i) => new TableCell({
      borders,
      width: { size: colWidths[i], type: WidthType.DXA },
      shading: { type: ShadingType.CLEAR, fill: DARK },
      margins: cellMargins,
      verticalAlign: VerticalAlign.CENTER,
      children: [new Paragraph({
        children: [new TextRun({ text: h, font: BODY_FONT, size: 20, bold: true, color: "FFFFFF" })],
      })],
    })),
  });

  const dataRows = rows.map(row => new TableRow({
    children: row.map((cell, i) => new TableCell({
      borders,
      width: { size: colWidths[i], type: WidthType.DXA },
      shading: { type: ShadingType.CLEAR, fill: "FFFFFF" },
      margins: cellMargins,
      children: [new Paragraph({
        children: typeof cell === "string"
          ? [new TextRun({ text: cell, font: BODY_FONT, size: 20 })]
          : (Array.isArray(cell) ? cell.map(c => typeof c === "string"
              ? new TextRun({ text: c, font: BODY_FONT, size: 20 })
              : new TextRun({ font: BODY_FONT, size: 20, ...c }))
            : [new TextRun({ font: BODY_FONT, size: 20, ...cell })]),
      })],
    })),
  }));

  return new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: colWidths,
    rows: [headerRow, ...dataRows],
  });
}

function spacer(pts = 120) {
  return new Paragraph({ spacing: { after: pts }, children: [] });
}

function definitionBlock(label, text) {
  return [
    new Paragraph({
      spacing: { before: 160, after: 80 },
      indent: { left: 200 },
      shading: { type: ShadingType.CLEAR, fill: "EFF5FB" },
      children: [
        new TextRun({ text: label, font: BODY_FONT, size: 22, bold: true, italics: true }),
        new TextRun({ text: " " + text, font: BODY_FONT, size: 22 }),
      ],
    }),
  ];
}

function theoremBlock(label, text) {
  return new Paragraph({
    spacing: { before: 160, after: 120 },
    indent: { left: 200, right: 200 },
    shading: { type: ShadingType.CLEAR, fill: "F0F7ED" },
    children: [
      new TextRun({ text: label + " ", font: BODY_FONT, size: 22, bold: true }),
      new TextRun({ text, font: BODY_FONT, size: 22, italics: true }),
    ],
  });
}

// ─── BUILD ALL SECTIONS ────────────────────────────────────
function buildAllSections() {
  const children = [];

  // ═══════════════════════════════════════════════════════════
  // TITLE PAGE
  // ═══════════════════════════════════════════════════════════
  children.push(spacer(2000));
  children.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 200 }, children: [
    new TextRun({ text: "MOMENTUM", font: BODY_FONT, size: 52, bold: true, color: DARK }),
  ]}));
  children.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 120 }, children: [
    new TextRun({ text: "OPEN SOURCE SEZ STACK", font: BODY_FONT, size: 36, color: ACCENT }),
  ]}));
  children.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
    new TextRun({ text: "Technical Specification", font: BODY_FONT, size: 28, italics: true }),
  ]}));
  children.push(spacer(200));
  children.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 80 }, children: [
    new TextRun({ text: "Version 0.4.44 \u2014 GENESIS Release", font: BODY_FONT, size: 24, bold: true }),
  ]}));
  children.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
    new TextRun({ text: "Complete SEZ-in-a-Box: Multi-Jurisdiction Composition", font: BODY_FONT, size: 22, italics: true }),
  ]}));
  children.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
    new TextRun({ text: "with One-Click Deployment via Mass", font: BODY_FONT, size: 22, italics: true }),
  ]}));
  children.push(spacer(400));
  children.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
    new TextRun({ text: "Prepared by Momentum", font: BODY_FONT, size: 22 }),
  ]}));
  children.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
    new TextRun({ text: "https://github.com/momentum-sez/stack", font: BODY_FONT, size: 20, color: ACCENT }),
  ]}));
  children.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
    new TextRun({ text: "February 2026", font: BODY_FONT, size: 22 }),
  ]}));
  children.push(spacer(400));
  children.push(new Paragraph({ alignment: AlignmentType.CENTER, children: [
    new TextRun({ text: "CONFIDENTIAL", font: BODY_FONT, size: 28, bold: true, color: DARK }),
  ]}));
  children.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // TABLE OF CONTENTS
  // ═══════════════════════════════════════════════════════════
  children.push(new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 200 }, children: [
    new TextRun({ text: "TABLE OF CONTENTS", font: BODY_FONT, size: 28, bold: true, color: DARK }),
  ]}));
  children.push(new TableOfContents("Table of Contents", {
    hyperlink: true,
    headingStyleRange: "1-3",
  }));
  children.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // EXECUTIVE SUMMARY
  // ═══════════════════════════════════════════════════════════
  children.push(heading1("Executive Summary"));
  children.push(p("The Momentum Open Source SEZ Stack compresses the creation of high-quality economic governance from years to months. Version 0.4.44, codenamed GENESIS, transforms the Stack from execution infrastructure into a fully deployable Special Economic Zone. Clone the repository, select a deployment profile, execute a single command, and operate a fully functional programmable jurisdiction."));
  children.push(p("This specification documents the complete technical architecture of the SEZ Stack, integrating the advanced compliance and execution infrastructure from version 0.4.43 Phoenix Ascendant with six transformative capabilities that constitute the GENESIS release. The codebase is fully Rust (2024 edition). All code examples, data structures, and system interfaces are specified in Rust. The architecture enforces a strict separation between two systems: Mass (five jurisdiction-agnostic programmable primitives) and the MSEZ Stack (jurisdictional context, compliance evaluation, and cross-border infrastructure)."));
  children.push(p("The specification is grounded in production deployments: Pakistan GovOS covering 40+ ministries with FBR tax integration, SBP Raast payments, NADRA identity, and SECP corporate registry; UAE/ADGM with 1,000+ entities onboarded and $1.7B+ capital processed; Dubai Free Zone Council integration across 27 free zones; Kazakhstan Alatau City SEZ + AIFC composition engine; and three cross-border trade corridors (PAK\u2194KSA $5.4B, PAK\u2194UAE $10.1B, PAK\u2194CHN $23.1B)."));

  children.push(heading3("Key Capabilities"));
  children.push(makeTable(
    ["Module Family", "Description", "Key Components"],
    [
      ["Compliance", "Multi-dimensional compliance representation", "Tensor V2, Manifold, ZK proofs"],
      ["Corridors", "Inter-jurisdiction relationships", "State sync, bridge protocol, multilateral"],
      ["Governance", "Zone governance structures", "Constitutional frameworks, voting"],
      ["Financial", "Banking and payment infrastructure", "Accounts, payments, custody, FX"],
      ["Regulatory", "Compliance frameworks", "KYC, AML, sanctions, reporting"],
      ["Licensing", "Business authorization", "Applications, monitoring, portability"],
      ["Legal", "Legal services infrastructure", "Contracts, disputes, arbitration"],
      ["Operational", "Administrative functionality", "HR, procurement, facilities"],
      ["Corporate", "Corporate service provider lifecycle", "Formation, cap table, dissolution"],
      ["Identity", "Identity and credentialing", "DID management, progressive KYC"],
      ["Tax", "Tax and revenue management", "Regimes, fees, incentives"],
      ["Capital Markets", "Securities infrastructure", "Issuance, trading, clearing, settlement"],
      ["Trade", "Trade and commerce", "LCs, documents, supply chain finance"],
      ["Settlement", "ZK-native L1 settlement", "MASS Protocol, Plonky3 proofs"],
      ["Migration", "Cross-jurisdictional asset movement", "Saga orchestration, compensation"],
      ["Watcher", "Attestation economy", "Bonds, slashing, reputation"],
    ],
    [2400, 3600, 3360]
  ));
  children.push(spacer());

  children.push(heading3("Version 0.4.44 Highlights"));
  children.push(makeTable(
    ["Component", "Scope", "Key Features"],
    [
      ["Licensepacks", "900+ lines", "Pack Trilogy completion, license verification, compliance tensor integration"],
      ["Composition Engine", "650+ lines", "Multi-jurisdiction composition, 20 domains, AI arbitration support"],
      ["Corporate Services", "8 modules", "Formation, beneficial ownership, cap table, secretarial, dissolution"],
      ["Identity Module", "5 modules", "DID management, 4-tier KYC, verifiable credentials, binding"],
      ["Tax Module", "5 modules", "Tax regimes, fee schedules, incentive programs, CRS/FATCA"],
      ["Capital Markets", "9 modules", "Securities, trading, clearing, CSD, DVP/PVP settlement"],
      ["Trade Module", "6 modules", "Letters of credit, trade documents, supply chain finance"],
      ["Docker Infrastructure", "373+ lines", "12-service orchestration, database initialization"],
      ["AWS Terraform", "1,250+ lines", "VPC, EKS, RDS, Kubernetes resources"],
      ["Rust Migration", "Full codebase", "2024 edition, tokio async, serde serialization, zero unsafe"],
      ["Mass/MSEZ Bridge", "New crate", "JurisdictionalContext trait, five-primitive mapping"],
      ["GovOS Architecture", "New Part", "Four-layer model, Sovereign AI, Pakistan reference"],
    ],
    [2400, 1800, 5160]
  ));
  children.push(p("The complete v0.4.44 implementation comprises sixteen module families totaling 298 modules across approximately 56,000 lines of production Rust code, with 650 tests achieving 100% coverage across all critical paths."));
  children.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART I: FOUNDATION
  // ═══════════════════════════════════════════════════════════
  children.push(heading1("PART I: FOUNDATION"));

  // Chapter 1
  children.push(heading2("Chapter 1: Mission and Vision"));
  children.push(p("The Momentum SEZ Stack exists to compress the creation of high-quality economic governance from years to months. Traditional Special Economic Zones demand $50\u2013100M in capital, regulatory counsel across multiple domains, and 18\u201336 months before a single license is issued. The SEZ Stack reduces this to a single deployment command, a curated profile, and 90 days to first license."));
  children.push(p("The fundamental insight driving the Stack is that economic governance can be decomposed into modular, composable primitives. Just as software libraries enable rapid application development through component reuse, governance primitives enable rapid jurisdiction development through regulatory composition. A new SEZ need not design corporate formation processes from scratch when proven implementations exist and can be adapted to local requirements."));
  children.push(p("This modularity extends beyond code reuse to encompass regulatory recognition. When multiple jurisdictions deploy compatible Stack implementations, mutual recognition becomes computationally verifiable rather than diplomatically negotiated. A corporate entity formed in one Stack-compatible jurisdiction can be algorithmically recognized in another, with compliance verified through cryptographic proofs rather than document review."));

  // 1.1
  children.push(heading3("1.1 The Programmable Institution Thesis"));
  children.push(p("Momentum operates on the thesis that institutions themselves can become programmable. Traditional institutions encode rules in natural language documents interpreted by human administrators. Programmable institutions encode rules in executable specifications that machines verify while remaining human-readable. Human judgment remains essential for edge cases, policy evolution, and equitable discretion. The system defines the boundaries within which discretion operates."));
  children.push(p("This does not mean replacing human judgment with algorithmic decision-making. It means creating systems where the boundaries of discretion are precisely defined, where rule application is consistent and auditable, and where institutional behavior can be formally verified against stated objectives."));
  children.push(p("Evidence supports this thesis at production scale:"));
  children.push(makeTable(
    ["Deployment", "Status", "Evidence"],
    [
      ["Pakistan GovOS (PDA)", "Active", "Full government OS: 40+ ministries, FBR tax integration (Income Tax Ordinance 2001, Sales Tax Act 1990, Federal Excise Act, Customs Act), SBP Raast payments, NADRA identity, SECP corporate registry. Target: raise tax-to-GDP from 10.3% to 15%+."],
      ["UAE / ADGM", "Live", "1,000+ entities onboarded, $1.7B+ capital processed via Northern Trust custody."],
      ["Dubai Free Zone Council", "Integration", "27 free zones. Mass APIs serve entity + fiscal; MSEZ provides zone-specific licensing."],
      ["Kazakhstan (Alatau City)", "Partnership", "SEZ + AIFC integration. Tests composition engine: Kazakh law + AIFC financial regulation."],
      ["Seychelles", "Deployment", "Sovereign GovOS at national scale."],
    ],
    [2400, 1200, 5760]
  ));
  children.push(spacer());

  // 1.2
  children.push(heading3("1.2 The Two-System Architecture"));
  children.push(p("The most important architectural constraint in this specification is the separation between two distinct systems. Every component described respects this boundary. Violating it is a structural error."));
  children.push(p([bold("System A: Mass \u2014 The Five Programmable Primitives."), " Mass provides five APIs that make institutions programmable. These are jurisdiction-agnostic. They operate identically whether deployed in ADGM, Pakistan, Seychelles, or Honduras. Mass APIs do not know which jurisdiction they run in."]));
  children.push(makeTable(
    ["Primitive", "Live API Surface", "Function"],
    [
      ["Entities", "organization-info.api.mass.inc", "Formation, lifecycle, dissolution. Each entity is a legal actor, a Smart Asset."],
      ["Ownership", "investment-info (Heroku seed)", "Cap tables, token tables, beneficial ownership, equity instruments, fundraising rounds."],
      ["Fiscal", "treasury-info.api.mass.inc", "Accounts, wallets, on/off-ramps, payments, treasury, withholding tax at source."],
      ["Identity", "Distributed across org + consent", "Passportable KYC/KYB. Onboard once, reuse everywhere."],
      ["Consent", "consent.api.mass.inc", "Multi-party auth, audit trails, board/shareholder/controller sign-off workflows."],
    ],
    [1800, 3200, 4360]
  ));
  children.push(spacer());
  children.push(p("Supporting infrastructure includes the Templating Engine (templating-engine on Heroku), which generates legal documents, resolutions, and compliance artifacts from primitive state. The Organs \u2014 Center of Mass (banking), Torque (licensing), Inertia (corporate services) \u2014 are regulated interface implementations that make Mass deployable in licensed environments."));

  children.push(p([bold("System B: MSEZ Stack \u2014 The Jurisdictional Context."), " The SEZ Stack provides the environment within which Mass APIs operate. It is the road system, not the engine. The Stack provides machine-readable jurisdictional state (Pack Trilogy), compliance evaluation (Compliance Tensor V2), migration path optimization (Compliance Manifold), cryptographic channels between jurisdictions (Corridor System), bonded attestation accountability (Watcher Economy), asset movement orchestration (Migration Protocol), hybrid jurisdiction composition (Composition Engine), and Smart Asset execution infrastructure (SAVM, receipt chains)."]));

  children.push(p([bold("The Interface Contract."), " Mass APIs call into the MSEZ Stack for jurisdictional context. The MSEZ Stack never duplicates what Mass APIs do. When an entity is formed via the Mass Organization Info API, the MSEZ Stack provides the jurisdictional rules (permitted entity types, formation document requirements, fees, compliance obligations). Mass executes the formation. The MSEZ Stack validates compliance. This separation is absolute."]));
  children.push(makeTable(
    ["Function", "Provided By", "MSEZ Spec Treatment"],
    [
      ["Entity formation", "Mass Org API", "Defines permitted entity types, formation requirements, fees. Does NOT implement formation."],
      ["Cap table management", "Mass Investment API", "Defines securities regulations, issuance rules. Does NOT implement cap tables."],
      ["Bank account opening", "Mass Treasury API", "Defines banking license requirements, AML rules. Does NOT implement accounts."],
      ["KYC/KYB verification", "Mass Identity", "Defines KYC tier requirements per jurisdiction. Does NOT implement verification."],
      ["Board resolution signing", "Mass Consent API", "Defines governance rules, quorum requirements. Does NOT implement workflows."],
      ["Compliance state evaluation", "MSEZ Compliance Tensor", "This IS the MSEZ Stack. Full specification herein."],
      ["Law encoding", "MSEZ Pack Trilogy", "This IS the MSEZ Stack. Full specification herein."],
      ["Cross-border corridors", "MSEZ Corridor System", "This IS the MSEZ Stack. Full specification herein."],
      ["Attestation accountability", "MSEZ Watcher Economy", "This IS the MSEZ Stack. Full specification herein."],
    ],
    [2400, 2400, 4560]
  ));
  children.push(spacer());

  // 1.3
  children.push(heading3("1.3 The Orthogonal Execution Layer"));
  children.push(p("Mass introduces a decentralized execution layer orthogonal to blockchain infrastructure. While blockchains provide ledger decentralization, Mass provides something fundamentally different: a network of autonomous assets executing within programmable legal, regulatory, and fiscal environments."));
  children.push(p("Traditional blockchain architectures conflate three distinct concerns: State Representation (how assets and their properties are encoded), State Progression (how state evolves over time through consensus and finality), and Execution Environment (the context within which operations are validated)."));
  children.push(p("Mass separates these concerns, recognizing that state representation and progression can occur without blockchain consensus via receipt chains, the execution environment extends beyond code to include law, regulation, and institutional frameworks, and settlement is a service assets consume rather than the substrate they live on. This separation enables Smart Assets to operate indefinitely without blockchain connectivity while maintaining full security guarantees, as proven in Theorem 9.1 (Object Survivability)."));

  // 1.4
  children.push(heading3("1.4 Design Principles"));
  children.push(p([bold("Sovereignty Preservation."), " Each jurisdiction deploying the Stack maintains complete control over its regulatory policy. The Stack provides implementation tools, not policy mandates. A jurisdiction may adopt restrictive or permissive policies; the Stack implements either with equal facility."]));
  children.push(p([bold("Privacy by Default."), " The Stack treats privacy as the baseline condition, requiring explicit and justified disclosure. Zero-knowledge proofs enable compliance verification without information leakage. Implemented via arkworks and halo2 proof systems in Rust."]));
  children.push(p([bold("Interoperability First."), " Every component is designed for composition \u2014 within a single jurisdiction or across boundaries. Open standards and public specifications enable third-party integration."]));
  children.push(p([bold("Cryptographic Verifiability."), " Claims about entity status, asset ownership, or regulatory compliance are verified by proof, not trust. BBS+ selective disclosure via the bbs crate. Ed25519 signatures via ed25519-dalek."]));
  children.push(p([bold("Graceful Degradation."), " The Stack operates under intermittent connectivity, regulatory uncertainty, and varying technical capabilities. Receipt chains provide offline operation guarantees proven in Theorem 9.1 (Object Survivability)."]));
  children.push(p([bold("Regulatory Agility."), " The Pack Trilogy \u2014 lawpacks, regpacks, licensepacks \u2014 propagates regulatory updates without code changes or downtime. New sanctions designations take effect within hours."]));
  children.push(p([bold("Auditability and Transparency."), " Individual transaction privacy is protected while aggregate system behavior remains auditable by appropriate authorities."]));
  children.push(p([bold("Compile-Time Safety."), " The Rust 2024 edition type system, ownership model, and zero-cost abstractions provide memory safety without garbage collection. No unsafe in application code. Result<T, E> with typed error enums replaces exception-based error handling."]));

  // Chapter 2
  children.push(pageBreak());
  children.push(heading2("Chapter 2: Architecture Overview"));

  children.push(heading3("2.1 Layer Model"));
  children.push(p("The SEZ Stack architecture comprises six principal layers. Each builds upon lower layers while maintaining clean interfaces."));
  children.push(makeTable(
    ["Layer", "Name", "Function", "Implementation"],
    [
      ["1", "Cryptographic Foundation", "Poseidon2, BBS+, NIZK hierarchy, Canonical Digest Bridge", "msez-core crate"],
      ["2", "Settlement", "MASS Protocol L1: DAG consensus, Plonky3 STARKs, harbor shards", "Shared infrastructure"],
      ["3", "Smart Asset Execution", "Receipt chains, SAVM, Compliance Tensor V2, fork resolution", "msez-vm, msez-tensor"],
      ["4", "Governance", "Entity formation, licensing, regulatory compliance, dispute resolution", "msez-governance"],
      ["5", "Compliance", "Pack Trilogy, Compliance Manifold, Watcher Economy, migration paths", "msez-pack, msez-watcher"],
      ["6", "Integration", "Banking, regulatory DBs, identity providers, corridor system", "msez-corridor, msez-mass-bridge"],
    ],
    [800, 2000, 3760, 2800]
  ));
  children.push(spacer());

  children.push(heading3("2.2 Module Architecture"));
  children.push(p("Stack functionality is organized into sixteen module families that can be selectively deployed based on zone requirements. The v0.4.44 release comprises sixteen module families totaling 298 modules:"));
  children.push(makeTable(
    ["Family", "Modules", "Purpose"],
    [
      ["compliance/", "Tensor, manifold, ZK circuits", "Multi-dimensional compliance evaluation"],
      ["corridors/", "State sync, bridge, multilateral", "Cross-jurisdictional channels"],
      ["governance/", "Constitutional, voting, delegation", "Zone governance infrastructure"],
      ["financial/", "Accounts, payments, custody, FX", "Financial operations"],
      ["regulatory/", "KYC, AML, sanctions, reporting", "Regulatory compliance"],
      ["licensing/", "Applications, monitoring, portability", "License lifecycle management"],
      ["legal/", "Contracts, disputes, arbitration", "Legal infrastructure"],
      ["operational/", "HR, procurement, facilities", "Zone operations"],
      ["corporate/ (v0.4.44)", "Formation, cap table, dissolution", "Corporate services"],
      ["identity/ (v0.4.44)", "DID, KYC tiers, credentials", "Identity and credentialing"],
      ["tax/ (v0.4.44)", "Regimes, fees, incentives", "Tax and revenue"],
      ["capital-markets/ (v0.4.44)", "Securities, trading, CSD", "Capital markets"],
      ["trade/ (v0.4.44)", "LCs, documents, SCF", "Trade and commerce"],
    ],
    [2600, 3200, 3560]
  ));
  children.push(spacer());

  children.push(heading3("2.3 PHOENIX Module Suite"));
  children.push(p("Version 0.4.43 introduced the PHOENIX module suite, comprising eleven specialized modules that implement the advanced compliance, execution, and security capabilities. These modules remain core to v0.4.44, now implemented in Rust:"));
  children.push(makeTable(
    ["Crate", "Lines", "Purpose"],
    [
      ["msez-tensor", "956+", "Compliance Tensor V2 with lattice operations and ZK commitment"],
      ["msez-core (zkp)", "666+", "Zero-knowledge proof infrastructure with five proof systems"],
      ["msez-tensor (manifold)", "1,046+", "Compliance manifold with differential-geometric optimization"],
      ["msez-migration", "887+", "Cross-jurisdictional migration protocol with saga pattern"],
      ["msez-watcher", "751+", "Watcher economy with bonds, slashing, and reputation"],
      ["msez-core (anchor)", "759+", "L1 anchoring protocol for settlement integration"],
      ["msez-corridor (bridge)", "816+", "Corridor bridge protocol for cross-corridor transfers"],
      ["msez-vm", "1,286+", "Smart Asset Virtual Machine with compliance coprocessor"],
      ["msez-core (security)", "912+", "Security layer with audit, access control, threat detection"],
      ["msez-core (hardening)", "695+", "Production hardening utilities and thread-safe primitives"],
    ],
    [3000, 1200, 5160]
  ));
  children.push(spacer());

  children.push(heading3("2.4 Rust Workspace Structure"));
  children.push(p("The codebase is fully Rust (2024 edition). The workspace is organized as follows:"));
  children.push(...codeBlock([
    "momentum-sez/stack/",
    "\u251C\u2500\u2500 Cargo.toml                    # Workspace root",
    "\u251C\u2500\u2500 crates/",
    "\u2502   \u251C\u2500\u2500 msez-core/                # Cryptographic primitives, digest types, artifact model",
    "\u2502   \u251C\u2500\u2500 msez-pack/                # Pack Trilogy (lawpacks, regpacks, licensepacks)",
    "\u2502   \u251C\u2500\u2500 msez-tensor/              # Compliance Tensor V2 + manifold",
    "\u2502   \u251C\u2500\u2500 msez-corridor/            # Corridor system + bridge protocol",
    "\u2502   \u251C\u2500\u2500 msez-watcher/             # Watcher economy + bonds + slashing",
    "\u2502   \u251C\u2500\u2500 msez-migration/           # Migration protocol + saga",
    "\u2502   \u251C\u2500\u2500 msez-vm/                  # Smart Asset Virtual Machine",
    "\u2502   \u251C\u2500\u2500 msez-governance/          # Constitutional frameworks + voting",
    "\u2502   \u251C\u2500\u2500 msez-modules/             # Institutional infrastructure modules",
    "\u2502   \u251C\u2500\u2500 msez-mass-bridge/         # Mass API integration layer",
    "\u2502   \u251C\u2500\u2500 msez-govos/               # GovOS orchestration layer",
    "\u2502   \u2514\u2500\u2500 msez-cli/                 # CLI binary (clap-derived)",
    "\u251C\u2500\u2500 schemas/                      # JSON Schemas (shared)",
    "\u251C\u2500\u2500 jurisdictions/                # Jurisdiction configurations",
    "\u251C\u2500\u2500 profiles/                     # Deployment profiles",
    "\u2514\u2500\u2500 deploy/                       # Docker + Terraform",
  ]));
  children.push(spacer());

  children.push(heading3("2.5 Live Deployments"));
  children.push(p("The specification is grounded in production deployments, not aspirational targets."));
  children.push(makeTable(
    ["Deployment", "Status", "Evidence"],
    [
      ["Pakistan GovOS (PDA)", "Active", "Full government OS: 40+ ministries, FBR tax integration, SBP Raast payments, NADRA identity, SECP corporate registry. Target: raise tax-to-GDP from 10.3% to 15%+. 24-month sovereignty handover."],
      ["UAE / ADGM", "Live", "1,000+ entities onboarded, $1.7B+ capital processed via Northern Trust custody."],
      ["Dubai Free Zone Council", "Integration", "27 free zones. Mass APIs serve entity + fiscal; MSEZ provides zone-specific licensing."],
      ["Seychelles", "Deployment", "Sovereign GovOS at national scale."],
      ["Kazakhstan (Alatau City)", "Partnership", "SEZ + AIFC integration. Tests composition engine: Kazakh law + AIFC financial regulation."],
      ["PAK\u2194KSA Corridor", "Launch", "$5.4B bilateral. Customs automation, WHT on remittances, 2.5M diaspora."],
      ["PAK\u2194UAE Corridor", "Live", "$10.1B bilateral. Mass in 27 Dubai FZs, $6.7B remittances."],
      ["PAK\u2194CHN Corridor", "Planned", "$23.1B via CPEC 2.0. 9 SEZs, Gwadar customs, e-trade docs."],
    ],
    [2400, 1200, 5760]
  ));

  children.push(pageBreak());
  return children;
}

// Continue in next file section...
// Export for assembly
module.exports = { buildAllSections, heading1, heading2, heading3, p, bold, italic, code, makeTable, codeBlock, codeParagraph, spacer, pageBreak, definitionBlock, theoremBlock, BODY_FONT, CODE_FONT, DARK, ACCENT, LIGHT_GRAY, CONTENT_W, PAGE_W, PAGE_H, MARGIN, borders, ShadingType };
