const { Paragraph, TextRun, PageBreak, AlignmentType, BorderStyle,
        TabStopType, LeaderType, SimpleField, InternalHyperlink } = require("docx");
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
    // TOC title — deep navy, non-bold, matching Chapter heading style
    new Paragraph({
      alignment: AlignmentType.CENTER,
      spacing: { after: 120 },
      children: [new TextRun({ text: "TABLE OF CONTENTS", font: C.BODY_FONT, size: 28, bold: false, color: C.H1_COLOR })]
    }),
    // Gold rule beneath TOC title
    new Paragraph({
      border: { bottom: { style: BorderStyle.SINGLE, size: 1, color: C.ACCENT, space: 4 } },
      spacing: { after: 300 },
      indent: { left: 2800, right: 2800 },
      children: []
    }),
  ];

  for (const entry of tocEntries) {
    const isLevel2 = entry.level === 2;
    const fontSize = isLevel2 ? 20 : C.BODY_SIZE;

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
