const fs = require("fs");
const path = require("path");
const { Document, Packer, Header, Footer, Paragraph, TextRun,
        PageNumber, AlignmentType, PageBreak } = require("docx");
const styles = require("./lib/styles");
const C = require("./lib/constants");
const { getChapterTitles, pageBreak } = require("./lib/primitives");
const { buildTocEntries } = require("./chapters/00-toc");

// ---------- Front matter (handled separately for TOC injection) ----------
const build_cover = require("./chapters/00-cover");
const build_toc_header = require("./chapters/00-toc");
const build_exec_summary = require("./chapters/00-executive-summary");

// ---------- Content sections (Part I through Appendices) ----------
// Each section groups chapters under a Part heading.
const CONTENT_SECTIONS = [
  {
    partTitle: "PART I: FOUNDATION",
    chapters: [
      require("./chapters/01-mission-vision"),
      require("./chapters/02-architecture"),
    ]
  },
  {
    partTitle: "PART II: CRYPTOGRAPHIC PRIMITIVES",
    chapters: [
      require("./chapters/03-crypto-primitives"),
    ]
  },
  {
    partTitle: "PART III: CONTENT-ADDRESSED ARTIFACT MODEL",
    chapters: [
      require("./chapters/04-artifact-model"),
    ]
  },
  {
    partTitle: "PART IV: CORE COMPONENTS \u2014 MODULES, PACK TRILOGY, PROFILES",
    chapters: [
      require("./chapters/05-module-specs"),
      require("./chapters/06-pack-trilogy"),
      require("./chapters/07-profiles"),
    ]
  },
  {
    partTitle: "PART V: SMART ASSET EXECUTION LAYER",
    chapters: [
      require("./chapters/08-smart-asset"),
      require("./chapters/09-receipt-chain"),
      require("./chapters/10-compliance-tensor"),
      require("./chapters/11-savm"),
      require("./chapters/12-composition"),
    ]
  },
  {
    partTitle: "PART VI: MASS L1 SETTLEMENT INFRASTRUCTURE",
    chapters: [
      require("./chapters/13-l1-architecture"),
      require("./chapters/14-proving-system"),
      require("./chapters/15-privacy"),
      require("./chapters/16-anchoring"),
    ]
  },
  {
    partTitle: "PART VII: GOVERNANCE AND CIVIC SYSTEMS",
    chapters: [
      require("./chapters/17-constitutional"),
      require("./chapters/18-civic-services"),
    ]
  },
  {
    partTitle: "PART VIII: COMPLIANCE AND REGULATORY INTEGRATION",
    chapters: [
      require("./chapters/19-compliance-arch"),
      require("./chapters/20-manifold"),
      require("./chapters/21-zkkyc"),
    ]
  },
  {
    partTitle: "PART IX: CRYPTOGRAPHIC CORRIDOR SYSTEMS",
    chapters: [
      require("./chapters/22-corridor-arch"),
      require("./chapters/23-corridor-bridge"),
      require("./chapters/24-multilateral"),
      require("./chapters/25-live-corridors"),
    ]
  },
  {
    partTitle: "PART X: WATCHER ECONOMY",
    chapters: [
      require("./chapters/26-watcher-arch"),
      require("./chapters/27-bond-slashing"),
      require("./chapters/28-quorum-finality"),
    ]
  },
  {
    partTitle: "PART XI: MIGRATION PROTOCOL",
    chapters: [
      require("./chapters/29-migration"),
      require("./chapters/30-migration-fsm"),
      require("./chapters/31-compensation"),
    ]
  },
  {
    partTitle: "PART XII: INSTITUTIONAL INFRASTRUCTURE MODULES (v0.4.44)",
    chapters: [
      require("./chapters/32-corporate"),
      require("./chapters/33-identity"),
      require("./chapters/34-tax"),
      require("./chapters/35-capital-markets"),
      require("./chapters/36-trade"),
    ]
  },
  {
    partTitle: "PART XIII: MASS API INTEGRATION LAYER",
    chapters: [
      require("./chapters/37-mass-bridge"),
    ]
  },
  {
    partTitle: "PART XIV: GovOS ARCHITECTURE",
    chapters: [
      require("./chapters/38-govos-layers"),
      require("./chapters/39-sovereign-ai"),
      require("./chapters/40-tax-pipeline"),
      require("./chapters/41-sovereignty"),
    ]
  },
  {
    partTitle: "PART XV: PROTOCOL REFERENCE \u2014 CREDENTIALS, ARBITRATION, AND AGENTIC SYSTEMS",
    chapters: [
      require("./chapters/42-protocol-overview"),
      require("./chapters/43-credentials"),
      require("./chapters/44-arbitration"),
      require("./chapters/45-agentic"),
    ]
  },
  {
    partTitle: "PART XVI: SECURITY AND HARDENING",
    chapters: [
      require("./chapters/46-security"),
      require("./chapters/47-hardening"),
      require("./chapters/48-zk-circuits"),
    ]
  },
  {
    partTitle: "PART XVII: DEPLOYMENT AND OPERATIONS",
    chapters: [
      require("./chapters/49-deployment"),
      require("./chapters/50-docker"),
      require("./chapters/51-terraform"),
      require("./chapters/52-one-click"),
      require("./chapters/53-operations"),
    ]
  },
  {
    partTitle: "PART XVIII: NETWORK DIFFUSION",
    chapters: [
      require("./chapters/54-adoption"),
      require("./chapters/55-partners"),
      require("./chapters/56-current-network"),
    ]
  },
  {
    partTitle: "APPENDICES",
    chapters: [
      require("./chapters/A-version-history"),
      require("./chapters/B-test-coverage"),
      require("./chapters/C-scalability"),
      require("./chapters/D-security-proofs"),
      require("./chapters/E-crate-deps"),
      require("./chapters/F-api-endpoints"),
      require("./chapters/G-jurisdiction-templates"),
      require("./chapters/H-cli-reference"),
      require("./chapters/I-module-directory"),
      require("./chapters/J-conformance"),
      require("./chapters/K-govos-checklist"),
    ]
  },
];

// Flatten nested arrays
function flatten(arr) {
  return arr.reduce((acc, el) =>
    Array.isArray(el) ? acc.concat(flatten(el)) : acc.concat(el), []);
}

// ---------- Shared helpers ----------

const BASE_HEADER_TEXT = "MSEZ Stack v0.4.44 \u2014 GENESIS";

function makeHeader(partTitle) {
  const headerText = partTitle
    ? `${BASE_HEADER_TEXT}  \u00B7  ${partTitle}`
    : BASE_HEADER_TEXT;
  return new Header({
    children: [new Paragraph({
      alignment: AlignmentType.RIGHT,
      children: [new TextRun({
        text: headerText,
        font: C.BODY_FONT, size: 16, color: "999999", italics: true
      })]
    })]
  });
}

const defaultFooter = new Footer({
  children: [new Paragraph({
    alignment: AlignmentType.CENTER,
    children: [
      new TextRun({ text: "Momentum \u00B7 CONFIDENTIAL \u00B7 Page ", font: C.BODY_FONT, size: 16, color: "999999" }),
      new TextRun({ children: [PageNumber.CURRENT], font: C.BODY_FONT, size: 16, color: "999999" })
    ]
  })]
});

function makeSection(partTitle, children) {
  return {
    properties: {
      page: {
        size: { width: C.PAGE_W, height: C.PAGE_H },
        margin: { top: C.MARGIN, right: C.MARGIN, bottom: C.MARGIN, left: C.MARGIN }
      }
    },
    headers: { default: makeHeader(partTitle) },
    footers: { default: defaultFooter },
    children: children,
  };
}

// ---------- Two-pass build ----------
// Pass 1: Build executive summary + all content sections.
//         This populates the chapter title registry via chapterHeading().
// Pass 2: Generate static TOC from the registry, assemble front matter.

console.log("Assembling document sections...");
let totalElements = 0;

// Pass 1a: Build executive summary (registers "Executive Summary" title first)
const execSummaryElements = flatten(build_exec_summary());

// Pass 1b: Build all content sections
const contentDocSections = CONTENT_SECTIONS.map((sec) => {
  const children = flatten(sec.chapters.map(fn => fn()));
  totalElements += children.length;
  console.log(`  ${sec.partTitle}: ${children.length} elements`);
  return makeSection(sec.partTitle, children);
});

// Pass 2: Generate static TOC from the title registry
const allTitles = getChapterTitles();
console.log(`  Registered ${allTitles.length} chapter titles for TOC`);

// Build the Part structure for TOC generation:
// First entry is front matter (Executive Summary)
const tocParts = [
  { partTitle: null, chapterCount: 1 }, // Executive Summary
  ...CONTENT_SECTIONS.map(sec => ({
    partTitle: sec.partTitle,
    chapterCount: sec.chapters.length
  }))
];

const coverElements = flatten(build_cover());
const tocHeaderElements = flatten(build_toc_header());
const tocEntries = buildTocEntries(tocParts, allTitles);

const frontMatterChildren = [
  ...coverElements,
  ...tocHeaderElements,
  ...tocEntries,
  pageBreak(),
  ...execSummaryElements,
];
totalElements += frontMatterChildren.length;
console.log(`  Front Matter: ${frontMatterChildren.length} elements (incl. ${tocEntries.length} TOC entries)`);

const frontMatterSection = makeSection(null, frontMatterChildren);

// Combine: front matter + all content sections
const docSections = [frontMatterSection, ...contentDocSections];
console.log(`Total elements: ${totalElements}`);

// Assemble document
const doc = new Document({
  styles: styles,
  numbering: {
    config: [
      {
        reference: "bullets",
        levels: [{
          level: 0,
          format: "bullet",
          text: "\u2022",
          alignment: "left",
          style: { paragraph: { indent: { left: 720, hanging: 360 } } }
        }]
      }
    ]
  },
  sections: docSections,
});

// Write output
const outputPath = path.join(__dirname, "output", "MSEZ_Stack_v0.4.44_GENESIS_Specification.docx");
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
Packer.toBuffer(doc).then(buffer => {
  fs.writeFileSync(outputPath, buffer);
  const mb = (buffer.length / 1048576).toFixed(2);
  console.log(`Written: ${outputPath} (${mb} MB)`);
}).catch(err => {
  console.error("Error generating document:", err);
  process.exit(1);
});
