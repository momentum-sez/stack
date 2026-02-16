const { Paragraph, TextRun, TableOfContents, PageBreak, AlignmentType } = require("docx");
const C = require("../lib/constants");
const { pageBreak } = require("../lib/primitives");

module.exports = function build_toc() {
  return [
    new Paragraph({
      alignment: AlignmentType.CENTER,
      spacing: { after: 200 },
      children: [new TextRun({ text: "TABLE OF CONTENTS", font: C.BODY_FONT, size: 28, bold: true, color: C.H1_COLOR })]
    }),
    new TableOfContents("Table of Contents", {
      hyperlink: true,
      headingStyleRange: "1-1",  // Chapters (H1) only — ~76 entries, ~5 pages. H2/H3 excluded from global TOC. Per-Part mini-TOCs provide granular navigation.
    }),
    pageBreak()
  ];
};
