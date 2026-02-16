const { Paragraph, TextRun, AlignmentType } = require("docx");
const C = require("../lib/constants");

/** Returns the "TABLE OF CONTENTS" header paragraph only.
 *  Actual TOC entries are injected by build.js after all chapters
 *  have been built (so the title registry is fully populated). */
function build_toc_header() {
  return [
    new Paragraph({
      alignment: AlignmentType.CENTER,
      spacing: { after: 300 },
      children: [new TextRun({ text: "TABLE OF CONTENTS", font: C.BODY_FONT, size: 28, bold: true, color: C.H1_COLOR })]
    }),
  ];
}

/** Generate static TOC entries from the Part/chapter structure.
 *  @param {Array<{partTitle: string|null, chapterCount: number}>} parts
 *  @param {string[]} titles — flat list of chapter titles in document order
 *  @returns {import("docx").Paragraph[]}
 */
function buildTocEntries(parts, titles) {
  const entries = [];
  let titleIdx = 0;

  for (const part of parts) {
    // Part heading as a TOC separator (bold, no indent)
    if (part.partTitle) {
      entries.push(new Paragraph({
        spacing: { before: 160, after: 40 },
        children: [new TextRun({
          text: part.partTitle,
          bold: true,
          font: C.BODY_FONT,
          size: 20,
          color: C.H1_COLOR
        })]
      }));
    }

    // Chapter entries (indented under their Part)
    for (let i = 0; i < part.chapterCount; i++) {
      const title = titles[titleIdx++];
      if (!title) continue;
      entries.push(new Paragraph({
        spacing: { after: 40, line: 276 },
        indent: { left: part.partTitle ? 360 : 0 },
        children: [new TextRun({
          text: title,
          font: C.BODY_FONT,
          size: 20,
          color: C.DARK
        })]
      }));
    }
  }

  return entries;
}

module.exports = build_toc_header;
module.exports.buildTocEntries = buildTocEntries;
