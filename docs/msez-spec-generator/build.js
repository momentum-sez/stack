const fs = require("fs");
const path = require("path");
const { Document, Packer, Header, Footer, Paragraph, TextRun,
        PageNumber, AlignmentType } = require("docx");
const styles = require("./lib/styles");
const C = require("./lib/constants");

// Import all chapter builders in order
const chapters = [
  require("./chapters/00-cover"),
  require("./chapters/00-toc"),
  require("./chapters/00-executive-summary"),
  require("./chapters/01-mission-vision"),
  require("./chapters/02-architecture"),
  require("./chapters/03-crypto-primitives"),
  require("./chapters/04-artifact-model"),
  require("./chapters/05-module-specs"),
  require("./chapters/06-pack-trilogy"),
  require("./chapters/07-profiles"),
  require("./chapters/08-smart-asset"),
  require("./chapters/09-receipt-chain"),
  require("./chapters/10-compliance-tensor"),
  require("./chapters/11-savm"),
  require("./chapters/12-composition"),
  require("./chapters/13-l1-architecture"),
  require("./chapters/14-proving-system"),
  require("./chapters/15-privacy"),
  require("./chapters/16-anchoring"),
  require("./chapters/17-constitutional"),
  require("./chapters/18-civic-services"),
  require("./chapters/19-compliance-arch"),
  require("./chapters/20-manifold"),
  require("./chapters/21-zkkyc"),
  require("./chapters/22-corridor-arch"),
  require("./chapters/23-corridor-bridge"),
  require("./chapters/24-multilateral"),
  require("./chapters/25-live-corridors"),
  require("./chapters/26-watcher-arch"),
  require("./chapters/27-bond-slashing"),
  require("./chapters/28-quorum-finality"),
  require("./chapters/29-migration"),
  require("./chapters/30-migration-fsm"),
  require("./chapters/31-compensation"),
  require("./chapters/32-corporate"),
  require("./chapters/33-identity"),
  require("./chapters/34-tax"),
  require("./chapters/35-capital-markets"),
  require("./chapters/36-trade"),
  require("./chapters/37-mass-bridge"),
  require("./chapters/38-govos-layers"),
  require("./chapters/39-sovereign-ai"),
  require("./chapters/40-tax-pipeline"),
  require("./chapters/41-sovereignty"),
  require("./chapters/42-protocol-overview"),
  require("./chapters/43-credentials"),
  require("./chapters/44-arbitration"),
  require("./chapters/45-agentic"),
  require("./chapters/46-security"),
  require("./chapters/47-hardening"),
  require("./chapters/48-zk-circuits"),
  require("./chapters/49-deployment"),
  require("./chapters/50-docker"),
  require("./chapters/51-terraform"),
  require("./chapters/52-one-click"),
  require("./chapters/53-operations"),
  require("./chapters/54-adoption"),
  require("./chapters/55-partners"),
  require("./chapters/56-current-network"),
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
];

// Flatten nested arrays
function flatten(arr) {
  return arr.reduce((acc, el) =>
    Array.isArray(el) ? acc.concat(flatten(el)) : acc.concat(el), []);
}

console.log("Assembling document sections...");

// Build all chapter content
const allElements = flatten(chapters.map(fn => fn()));

console.log(`Total elements: ${allElements.length}`);

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
  sections: [{
    properties: {
      page: {
        size: { width: C.PAGE_W, height: C.PAGE_H },
        margin: { top: C.MARGIN, right: C.MARGIN, bottom: C.MARGIN, left: C.MARGIN }
      }
    },
    headers: {
      default: new Header({
        children: [new Paragraph({
          alignment: AlignmentType.RIGHT,
          children: [new TextRun({
            text: "MSEZ Stack v0.4.44 \u2014 GENESIS",
            font: C.BODY_FONT, size: 16, color: "999999", italics: true
          })]
        })]
      })
    },
    footers: {
      default: new Footer({
        children: [new Paragraph({
          alignment: AlignmentType.CENTER,
          children: [
            new TextRun({ text: "Momentum \u00B7 CONFIDENTIAL \u00B7 Page ", font: C.BODY_FONT, size: 16, color: "999999" }),
            new TextRun({ children: [PageNumber.CURRENT], font: C.BODY_FONT, size: 16, color: "999999" })
          ]
        })]
      })
    },
    children: allElements
  }]
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
