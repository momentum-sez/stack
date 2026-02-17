const { Paragraph, TextRun, PageBreak, AlignmentType, TabStopType, LeaderType,
        SimpleField, InternalHyperlink } = require("docx");
const C = require("../lib/constants");

/**
 * Build a static TOC from collected heading entries.
 *
 * Each TOC entry is a paragraph containing an InternalHyperlink (anchored to
 * the heading's bookmark) wrapping the heading text, a dot-leader tab, and a
 * PAGEREF simple field. Clicking anywhere on the entry navigates to the
 * heading in the document.
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
    const fontSize = isLevel2 ? 20 : 22;

    elements.push(new Paragraph({
      tabStops: [{ type: TabStopType.RIGHT, position: C.CONTENT_W, leader: LeaderType.DOT }],
      spacing: { after: isLevel2 ? 20 : 80, before: isLevel2 ? 0 : 40 },
      indent: isLevel2 ? { left: 360 } : undefined,
      children: [
        new InternalHyperlink({
          anchor: entry.bookmarkName,
          children: [
            new TextRun({
              text: entry.text,
              font: C.BODY_FONT,
              size: fontSize,
              bold: !isLevel2,
              color: C.DARK,
            }),
            new TextRun({ children: ["\t"], font: C.BODY_FONT, size: fontSize }),
          ],
        }),
        // PAGEREF \h produces a clickable page number that also navigates to the bookmark.
        new SimpleField(` PAGEREF ${entry.bookmarkName} \\h `, "\u2013"),
      ]
    }));
  }

  elements.push(new Paragraph({ children: [new PageBreak()] }));

  return elements;
};
