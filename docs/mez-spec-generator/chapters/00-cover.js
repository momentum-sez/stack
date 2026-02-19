const { Paragraph, TextRun, AlignmentType, BorderStyle, PageBreak } = require("docx");
const { spacer, pageBreak } = require("../lib/primitives");
const C = require("../lib/constants");

module.exports = function build_cover() {
  return [
    // Generous top breathing room — prestige document proportions
    spacer(2400),

    // Brand mark — deep navy, commanding, with generous letter-spacing
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 100 }, children: [
      new TextRun({
        text: "MOMENTUM",
        font: C.BODY_FONT, size: 56, bold: true, color: C.H1_COLOR,
        characterSpacing: 120,
      })
    ]}),

    // Gold hairline rule — signature Momentum divider, centered
    new Paragraph({
      alignment: AlignmentType.CENTER,
      border: { bottom: { style: BorderStyle.SINGLE, size: 1, color: C.ACCENT, space: 6 } },
      spacing: { after: 200 },
      indent: { left: 2400, right: 2400 },
      children: []
    }),

    // Subtitle line — Georgia italic, understated
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 120 }, children: [
      new TextRun({ text: "Open Source EZ Stack", font: C.SUBTITLE_FONT, size: 32, italics: true, color: C.SECONDARY_TEXT })
    ]}),

    // Document type — Garamond, steel blue, formal
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
      new TextRun({ text: "Technical Specification", font: C.BODY_FONT, size: 28, color: C.H2_COLOR })
    ]}),

    spacer(300),

    // Version block — bold deep navy
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 80 }, children: [
      new TextRun({ text: "Version 0.4.44 \u2014 GENESIS Release", font: C.BODY_FONT, size: 24, bold: true, color: C.H1_COLOR })
    ]}),

    // Tagline — charcoal italic, two lines
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 40 }, children: [
      new TextRun({ text: "Complete EZ-in-a-Box: Multi-Jurisdiction Composition", font: C.BODY_FONT, size: C.BODY_SIZE, italics: true, color: C.DARK })
    ]}),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 }, children: [
      new TextRun({ text: "with One-Click Deployment via Mass", font: C.BODY_FONT, size: C.BODY_SIZE, italics: true, color: C.DARK })
    ]}),

    spacer(600),

    // Attribution block — restrained metadata
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 40 }, children: [
      new TextRun({ text: "Prepared by Momentum", font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK })
    ]}),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 40 }, children: [
      new TextRun({ text: "https://github.com/momentum-ez/stack", font: C.BODY_FONT, size: 20, color: C.H2_COLOR })
    ]}),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 40 }, children: [
      new TextRun({ text: "February 2026", font: C.BODY_FONT, size: C.BODY_SIZE, color: C.SECONDARY_TEXT })
    ]}),

    spacer(400),

    // Gold accent label — small, champagne, with tracking
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 40 }, children: [
      new TextRun({
        text: "BUSL-1.1",
        font: C.BODY_FONT, size: 20, bold: true, color: C.ACCENT,
        characterSpacing: 60,
      })
    ]}),

    pageBreak()
  ];
};
