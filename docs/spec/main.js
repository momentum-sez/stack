// main.js — Final assembly: generates the unified MSEZ v0.4.44 specification .docx
const fs = require("fs");
const {
  Document, Packer, Paragraph, TextRun,
  Header, Footer, AlignmentType, LevelFormat,
  HeadingLevel, BorderStyle, WidthType, ShadingType,
  PageNumber, TabStopType, TabStopPosition,
  PositionalTab, PositionalTabAlignment, PositionalTabRelativeTo, PositionalTabLeader,
} = require("docx");

const { buildAllSections, heading1, heading2, heading3, p, bold, italic, code, makeTable, codeBlock, codeParagraph, spacer, pageBreak, definitionBlock, theoremBlock, BODY_FONT, CODE_FONT, DARK, ACCENT, LIGHT_GRAY, CONTENT_W, PAGE_W, PAGE_H, MARGIN, borders } = require("./generate-spec.js");

// We need to reconstruct the middle functions from genscriptB
// but genscriptB is truncated at line 1203. We'll source it and handle the truncation.

// Load genscriptB content and patch it to close properly
let genBContent = fs.readFileSync("/home/claude/generate-parts.js", "utf8");

// The file is truncated mid-codeBlock at line 1202-1203.
// We need to close the incomplete function by removing the dangling code
// and closing buildPartsVI_XVIII() before Ch30.2 State Implementation's codeblock

// Find the last complete statement before the truncation
const truncationPoint = genBContent.lastIndexOf('  c.push(heading3("30.2 State Implementation"));');
if (truncationPoint > 0) {
  // Trim everything from "30.2 State Implementation" onward since genscriptC will handle it
  genBContent = genBContent.substring(0, truncationPoint);
  // Close the function properly
  genBContent += `
  c.push(pageBreak());
  return c;
}

module.exports = { buildPartsII_III, buildPartIV, buildPartV, buildPartsVI_XVIII_partial: buildPartsVI_XVIII };
`;
}

// Write the patched file
fs.writeFileSync("/home/claude/generate-parts-patched.js", genBContent);

// Now load it
const { buildPartsII_III, buildPartIV, buildPartV, buildPartsVI_XVIII_partial } = require("./generate-parts-patched.js");
const { buildRemainingParts } = require("./genscriptC.js");

// ─── ASSEMBLE ALL SECTIONS ────────────────────────────────
console.log("Assembling document sections...");

const partI = buildAllSections();         // Title, TOC, Exec Summary, Part I (Foundation)
const partsII_III = buildPartsII_III();    // Parts II-III (Crypto, Artifacts)
const partIV = buildPartIV();              // Part IV (Modules, Pack Trilogy, Profiles)
const partV = buildPartV();                // Part V (Smart Asset Execution Layer)
const partsVI_XI_partial = buildPartsVI_XVIII_partial(); // Parts VI-XI (L1, Governance, Compliance, Corridors, Watchers, Migration partial)
const remaining = buildRemainingParts();   // Parts XI (completion)-XVIII + Appendices

const allChildren = [
  ...partI,
  ...partsII_III,
  ...partIV,
  ...partV,
  ...partsVI_XI_partial,
  ...remaining,
];

console.log(`Total paragraph/table elements: ${allChildren.length}`);

// ─── CREATE DOCUMENT ──────────────────────────────────────
const thinBorder = { style: BorderStyle.SINGLE, size: 1, color: "CCCCCC" };
const noBorders = {
  top: { style: BorderStyle.NONE, size: 0 },
  bottom: { style: BorderStyle.NONE, size: 0 },
  left: { style: BorderStyle.NONE, size: 0 },
  right: { style: BorderStyle.NONE, size: 0 },
};

const doc = new Document({
  styles: {
    default: {
      document: {
        run: { font: BODY_FONT, size: 22 },
      },
    },
    paragraphStyles: [
      {
        id: "Heading1",
        name: "Heading 1",
        basedOn: "Normal",
        next: "Normal",
        quickFormat: true,
        run: { size: 32, bold: true, font: BODY_FONT, color: DARK },
        paragraph: { spacing: { before: 360, after: 200 }, outlineLevel: 0 },
      },
      {
        id: "Heading2",
        name: "Heading 2",
        basedOn: "Normal",
        next: "Normal",
        quickFormat: true,
        run: { size: 26, bold: true, font: BODY_FONT, color: ACCENT },
        paragraph: { spacing: { before: 280, after: 160 }, outlineLevel: 1 },
      },
      {
        id: "Heading3",
        name: "Heading 3",
        basedOn: "Normal",
        next: "Normal",
        quickFormat: true,
        run: { size: 22, bold: true, font: BODY_FONT, color: DARK },
        paragraph: { spacing: { before: 200, after: 120 }, outlineLevel: 2 },
      },
    ],
  },
  sections: [
    {
      properties: {
        page: {
          size: { width: PAGE_W, height: PAGE_H },
          margin: { top: MARGIN, right: MARGIN, bottom: MARGIN, left: MARGIN },
        },
      },
      headers: {
        default: new Header({
          children: [
            new Paragraph({
              alignment: AlignmentType.CENTER,
              spacing: { after: 0 },
              border: {
                bottom: { style: BorderStyle.SINGLE, size: 6, color: ACCENT, space: 4 },
              },
              children: [
                new TextRun({
                  text: "MOMENTUM OPEN SOURCE SEZ STACK \u00B7 v0.4.44 \u00B7 CONFIDENTIAL",
                  font: BODY_FONT,
                  size: 16,
                  color: "666666",
                }),
              ],
            }),
          ],
        }),
      },
      footers: {
        default: new Footer({
          children: [
            new Paragraph({
              border: {
                top: { style: BorderStyle.SINGLE, size: 4, color: "CCCCCC", space: 4 },
              },
              children: [
                new TextRun({
                  text: "Momentum \u00B7 momentum.inc \u00B7 Page ",
                  font: BODY_FONT,
                  size: 16,
                  color: "888888",
                }),
                new TextRun({
                  children: [PageNumber.CURRENT],
                  font: BODY_FONT,
                  size: 16,
                  color: "888888",
                }),
              ],
            }),
          ],
        }),
      },
      children: allChildren,
    },
  ],
});

// ─── GENERATE DOCX ────────────────────────────────────────
console.log("Generating .docx file...");
const outputPath = "/home/claude/MSEZ_Stack_v0.4.44_GENESIS_Specification.docx";

Packer.toBuffer(doc).then((buffer) => {
  fs.writeFileSync(outputPath, buffer);
  console.log(`Document written to ${outputPath}`);
  console.log(`File size: ${(buffer.length / 1024 / 1024).toFixed(2)} MB`);

  // Copy to outputs
  const outDir = "/mnt/user-data/outputs";
  if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true });
  fs.copyFileSync(outputPath, `${outDir}/MSEZ_Stack_v0.4.44_GENESIS_Specification.docx`);
  console.log("Copied to outputs directory.");
}).catch((err) => {
  console.error("Error generating document:", err);
  process.exit(1);
});
