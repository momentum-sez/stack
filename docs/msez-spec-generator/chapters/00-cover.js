const { Paragraph, TextRun, AlignmentType, PageBreak } = require("docx");
const { spacer, pageBreak } = require("../lib/primitives");
const C = require("../lib/constants");

module.exports = function build_cover() {
  return [
    spacer(2000),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 200 }, children: [
      new TextRun({ text: "MOMENTUM", font: C.BODY_FONT, size: 52, bold: true, color: "0F2B46" })
    ]}),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 120 }, children: [
      new TextRun({ text: "OPEN SOURCE SEZ STACK", font: C.BODY_FONT, size: 36, color: "2E86AB" })
    ]}),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
      new TextRun({ text: "Technical Specification", font: C.BODY_FONT, size: 28, italics: true, color: C.DARK })
    ]}),
    spacer(200),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 80 }, children: [
      new TextRun({ text: "Version 0.4.44 \u2014 GENESIS Release", font: C.BODY_FONT, size: 24, bold: true, color: C.DARK })
    ]}),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
      new TextRun({ text: "Complete SEZ-in-a-Box: Multi-Jurisdiction Composition", font: C.BODY_FONT, size: 22, italics: true, color: C.DARK })
    ]}),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
      new TextRun({ text: "with One-Click Deployment via Mass", font: C.BODY_FONT, size: 22, italics: true, color: C.DARK })
    ]}),
    spacer(400),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
      new TextRun({ text: "Prepared by Momentum", font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK })
    ]}),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
      new TextRun({ text: "https://github.com/momentum-sez/stack", font: C.BODY_FONT, size: 20, color: "2E86AB" })
    ]}),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
      new TextRun({ text: "February 2026", font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK })
    ]}),
    spacer(400),
    new Paragraph({ alignment: AlignmentType.CENTER, children: [
      new TextRun({ text: "CONFIDENTIAL", font: C.BODY_FONT, size: 28, bold: true, color: "0F2B46" })
    ]}),
    pageBreak()
  ];
};
