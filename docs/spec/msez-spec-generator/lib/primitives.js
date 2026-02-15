const {
  Paragraph, TextRun, Table, TableRow, TableCell,
  HeadingLevel, AlignmentType, BorderStyle, WidthType,
  ShadingType, PageBreak
} = require("docx");

const C = require("./constants");

// --- Text Primitives ---

/** Body paragraph */
function p(text, opts = {}) {
  return new Paragraph({
    spacing: { after: 120, line: 276 },
    children: [new TextRun({ text, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK, ...opts })]
  });
}

/** Paragraph with mixed runs: p_runs([bold("Key:"), " value text"]) */
function p_runs(runs) {
  return new Paragraph({
    spacing: { after: 120, line: 276 },
    children: runs.map(r =>
      typeof r === "string"
        ? new TextRun({ text: r, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK })
        : r
    )
  });
}

/** Bold TextRun (for use inside p_runs) */
function bold(text) {
  return new TextRun({ text, bold: true, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK });
}

/** Italic TextRun */
function italic(text) {
  return new TextRun({ text, italics: true, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK });
}

/** Code-styled TextRun (inline code) */
function code(text) {
  return new TextRun({
    text, font: C.CODE_FONT, size: C.CODE_SIZE,
    color: C.CODE_TEXT, shading: { type: ShadingType.CLEAR, fill: C.CODE_BG }
  });
}

// --- Headings ---

/** Part heading (e.g., "PART I: FOUNDATION") - includes page break */
function partHeading(text) {
  return [
    new Paragraph({ children: [new PageBreak()] }),
    new Paragraph({
      heading: HeadingLevel.HEADING_1,
      spacing: { before: 0, after: 300 },
      children: [new TextRun({ text: text.toUpperCase(), bold: true, font: C.BODY_FONT, size: 36, color: C.H1_COLOR })]
    })
  ];
}

/** Chapter heading (e.g., "Chapter 1: Mission and Vision") */
function chapterHeading(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_1,
    children: [new TextRun({ text, bold: true, font: C.BODY_FONT, size: 36, color: C.H1_COLOR })]
  });
}

/** Section heading (e.g., "1.1 The Programmable Institution Thesis") */
function h2(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_2,
    children: [new TextRun({ text, bold: true, font: C.BODY_FONT, size: 28, color: C.H2_COLOR })]
  });
}

/** Subsection heading (e.g., "6.5.1 License Data Model") */
function h3(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_3,
    children: [new TextRun({ text, bold: true, font: C.BODY_FONT, size: 24, color: C.H3_COLOR })]
  });
}

// --- Definitions and Theorems ---

/** Definition block with left blue border */
function definition(label, text) {
  return new Paragraph({
    border: { left: { style: BorderStyle.SINGLE, size: 6, color: C.ACCENT, space: 8 } },
    spacing: { before: 160, after: 160 },
    indent: { left: 360 },
    children: [
      new TextRun({ text: label + " ", bold: true, italics: true, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK }),
      new TextRun({ text, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK })
    ]
  });
}

/** Theorem block with left border */
function theorem(label, text) {
  return new Paragraph({
    border: { left: { style: BorderStyle.SINGLE, size: 6, color: "6B7280", space: 8 } },
    spacing: { before: 160, after: 160 },
    indent: { left: 360 },
    children: [
      new TextRun({ text: label + " ", bold: true, italics: true, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK }),
      new TextRun({ text, italics: true, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK })
    ]
  });
}

// --- Code Blocks ---

/** Multi-line code block. Pass a string; it splits on \n. */
function codeBlock(codeString) {
  const lines = codeString.split("\n");
  return lines.map(line =>
    new Paragraph({
      spacing: { after: 0, line: 240 },
      shading: { type: ShadingType.CLEAR, fill: C.CODE_BG },
      children: [new TextRun({ text: line || " ", font: C.CODE_FONT, size: C.CODE_SIZE, color: C.CODE_TEXT })]
    })
  );
}

// --- Tables ---

/** Standard table with header row + data rows.
 *  @param {string[]} headers - Column header labels
 *  @param {string[][]} rows - 2D array of cell text
 *  @param {number[]} [colWidths] - Optional column widths in DXA (must sum to 9360)
 */
function table(headers, rows, colWidths) {
  const numCols = headers.length;
  const widths = colWidths || evenWidths(numCols);
  const border = { style: BorderStyle.SINGLE, size: 1, color: "D1D5DB" };
  const borders = { top: border, bottom: border, left: border, right: border };

  function makeCell(text, width, isHeader, altRow) {
    let fill = "FFFFFF";
    if (isHeader) fill = C.TABLE_HEADER_BG;
    else if (altRow) fill = C.TABLE_ALT_ROW;

    return new TableCell({
      borders,
      width: { size: width, type: WidthType.DXA },
      shading: { type: ShadingType.CLEAR, fill },
      margins: { top: 80, bottom: 80, left: 120, right: 120 },
      children: [new Paragraph({
        children: [new TextRun({
          text: text || "",
          bold: isHeader,
          font: C.BODY_FONT,
          size: 20,
          color: isHeader ? C.TABLE_HEADER_TEXT : C.DARK
        })]
      })]
    });
  }

  return new Table({
    width: { size: C.CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows: [
      new TableRow({ children: headers.map((h, i) => makeCell(h, widths[i], true, false)) }),
      ...rows.map((row, ri) =>
        new TableRow({
          children: row.map((cell, ci) => makeCell(cell, widths[ci], false, ri % 2 === 1))
        })
      )
    ]
  });
}

/** Compute even column widths that sum to CONTENT_W */
function evenWidths(n) {
  const w = Math.floor(C.CONTENT_W / n);
  const widths = Array(n).fill(w);
  widths[n - 1] += C.CONTENT_W - w * n;
  return widths;
}

/** Vertical spacer */
function spacer(after = 200) {
  return new Paragraph({ spacing: { after }, children: [] });
}

/** Page break */
function pageBreak() {
  return new Paragraph({ children: [new PageBreak()] });
}

module.exports = {
  p, p_runs, bold, italic, code,
  partHeading, chapterHeading, h2, h3,
  definition, theorem,
  codeBlock, table, evenWidths,
  spacer, pageBreak
};
