const { Paragraph, TextRun, PageBreak, AlignmentType, TabStopType, LeaderType, SimpleField } = require("docx");
const C = require("../lib/constants");

/**
 * Build a static TOC from collected heading entries.
 *
 * Previous approach used docx's TableOfContents which emits a Word TOC field
 * code (e.g. `TOC \h \t "Heading 1,1,Heading 2,2"`). That field requires
 * Word's client-side evaluation engine to render correctly. In practice, the
 * style-name matching (`\t` switch) fails across Word versions and locales,
 * causing Word to fall back to including ALL paragraphs — producing a 100+
 * page TOC filled with body text.
 *
 * This replacement builds the TOC explicitly at document generation time:
 * each heading gets a bookmark (see primitives.js), and each TOC entry is a
 * regular paragraph containing the heading text, a dot-leader tab, and a
 * PAGEREF simple field referencing the bookmark. Using w:fldSimple (via
 * SimpleField) produces clean, universally-compatible OOXML that Word
 * evaluates correctly — it just resolves a bookmark to its page number,
 * with no style matching involved.
 *
 * @param {Array<{text: string, level: number, bookmarkName: string}>} tocEntries
 */
module.exports = function build_toc(tocEntries) {
  if (!Array.isArray(tocEntries) || tocEntries.length === 0) {
    throw new Error("build_toc requires a non-empty tocEntries array. Ensure chapters are built before TOC.");
  }

  const elements = [
    new Paragraph({
      alignment: AlignmentType.CENTER,
      spacing: { after: 400 },
      children: [new TextRun({ text: "TABLE OF CONTENTS", font: C.BODY_FONT, size: 28, bold: true, color: C.H1_COLOR })]
    }),
  ];

  for (const entry of tocEntries) {
    const isLevel2 = entry.level === 2;
    elements.push(new Paragraph({
      tabStops: [{ type: TabStopType.RIGHT, position: C.CONTENT_W, leader: LeaderType.DOT }],
      spacing: { after: isLevel2 ? 20 : 80, before: isLevel2 ? 0 : 40 },
      indent: isLevel2 ? { left: 360 } : undefined,
      children: [
        new TextRun({
          text: entry.text,
          font: C.BODY_FONT,
          size: isLevel2 ? 20 : 22,
          bold: !isLevel2,
          color: C.DARK,
        }),
        new TextRun({ children: ["\t"], font: C.BODY_FONT, size: isLevel2 ? 20 : 22 }),
        // SimpleField produces <w:fldSimple w:instr="PAGEREF bookmark"> which is
        // universally handled by Word. The cached value "–" displays until Word
        // resolves the field to the actual page number on open/print.
        new SimpleField(` PAGEREF ${entry.bookmarkName} `, "\u2013"),
      ]
    }));
  }

  elements.push(new Paragraph({ children: [new PageBreak()] }));

  return elements;
};
