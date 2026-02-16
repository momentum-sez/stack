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
      stylesWithLevels: [
        { styleName: "Heading 1", level: 1 },
        { styleName: "Heading 2", level: 2 },
      ],
    }),
    pageBreak()
  ];
};
