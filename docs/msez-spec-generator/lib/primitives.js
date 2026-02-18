const {
  Paragraph, TextRun, Table, TableRow, TableCell,
  HeadingLevel, AlignmentType, BorderStyle, WidthType,
  ShadingType, PageBreak, BookmarkStart, BookmarkEnd
} = require("docx");

const C = require("./constants");

// --- TOC Heading Registry ---
// Headings register themselves here at creation time.
// After all chapters are built, build.js reads this to generate a static TOC.
const _tocEntries = [];
let _bookmarkCounter = 1;

function _registerHeading(text, level) {
  const id = String(_bookmarkCounter++);
  const name = `_toc_${id}`;
  _tocEntries.push({ text, level, bookmarkName: name, bookmarkId: id });
  return { name, id };
}

/** Return collected heading entries for static TOC generation. */
function getTocEntries() {
  return _tocEntries.slice();
}

// --- Text Primitives ---

/** Body paragraph — Garamond 11.5pt, charcoal, justified, 1.3× line spacing.
 *  Widow/orphan control ensures no single-line stranding at page breaks. */
function p(text, opts = {}) {
  return new Paragraph({
    alignment: AlignmentType.JUSTIFIED,
    spacing: { after: 180, line: 312 },
    widowControl: true,
    children: [new TextRun({ text, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK, ...opts })]
  });
}

/** Paragraph with mixed runs: p_runs([bold("Key:"), " value text"]) */
function p_runs(runs) {
  return new Paragraph({
    alignment: AlignmentType.JUSTIFIED,
    spacing: { after: 180, line: 312 },
    widowControl: true,
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

/** Part heading (e.g., "PART I: FOUNDATION") — includes page break.
 *  18pt bold deep navy with generous letter-spacing on uppercase.
 *  Followed by a centered gold hairline rule — the Momentum signature. */
function partHeading(text) {
  const bm = _registerHeading(text.toUpperCase(), 1);
  return [
    new Paragraph({ children: [new PageBreak()] }),
    new Paragraph({
      heading: HeadingLevel.HEADING_1,
      spacing: { before: 0, after: 120 },
      keepNext: true,
      keepLines: true,
      children: [
        new BookmarkStart(bm.name, bm.id),
        new TextRun({
          text: text.toUpperCase(),
          bold: true,
          font: C.BODY_FONT,
          size: 36,
          color: C.H1_COLOR,
          characterSpacing: 80,
        }),
        new BookmarkEnd(bm.id),
      ]
    }),
    // Centered gold hairline rule — indented for elegance, not full-width
    new Paragraph({
      border: { bottom: { style: BorderStyle.SINGLE, size: 1, color: C.ACCENT, space: 4 } },
      spacing: { after: 300 },
      indent: { left: 720, right: 720 },
      children: []
    })
  ];
}

/** Chapter heading (e.g., "Chapter 1: Mission and Vision")
 *  16pt non-bold deep navy — structural, elegant. */
function chapterHeading(text) {
  const bm = _registerHeading(text, 1);
  return new Paragraph({
    heading: HeadingLevel.HEADING_1,
    spacing: { before: 360, after: 240 },
    keepNext: true,
    keepLines: true,
    children: [
      new BookmarkStart(bm.name, bm.id),
      new TextRun({ text, bold: false, font: C.BODY_FONT, size: 32, color: C.H1_COLOR }),
      new BookmarkEnd(bm.id),
    ]
  });
}

/** Section heading (e.g., "1.1 The Programmable Institution Thesis")
 *  13pt non-bold steel blue — clean subsection hierarchy. */
function h2(text) {
  const bm = _registerHeading(text, 2);
  return new Paragraph({
    heading: HeadingLevel.HEADING_2,
    spacing: { before: 300, after: 180 },
    keepNext: true,
    keepLines: true,
    children: [
      new BookmarkStart(bm.name, bm.id),
      new TextRun({ text, bold: false, font: C.BODY_FONT, size: 26, color: C.H2_COLOR }),
      new BookmarkEnd(bm.id),
    ]
  });
}

/** Subsection heading (e.g., "6.5.1 License Data Model")
 *  12pt bold deep navy — section-head style for labeled subsections. */
function h3(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_3,
    spacing: { before: 240, after: 120 },
    keepNext: true,
    keepLines: true,
    children: [new TextRun({ text, bold: true, font: C.BODY_FONT, size: 24, color: C.H1_COLOR })]
  });
}

// --- Rules (Decorative Dividers) ---

/** Gold hairline rule — 0.5pt bottom border in champagne accent.
 *  The signature Momentum section divider. */
function rule() {
  return new Paragraph({
    border: { bottom: { style: BorderStyle.SINGLE, size: 1, color: C.ACCENT, space: 4 } },
    spacing: { before: 120, after: 200 },
    children: []
  });
}

/** Secondary rule — warm gray, lighter weight. */
function ruleLight() {
  return new Paragraph({
    border: { bottom: { style: BorderStyle.SINGLE, size: 1, color: C.ACCENT_SECONDARY, space: 4 } },
    spacing: { before: 80, after: 160 },
    children: []
  });
}

// --- Definitions and Theorems ---

/** Definition block with left gold border — champagne accent signals formal definition. */
function definition(label, text) {
  return new Paragraph({
    border: { left: { style: BorderStyle.SINGLE, size: 6, color: C.ACCENT, space: 8 } },
    spacing: { before: 160, after: 200, line: 312 },
    indent: { left: 360 },
    keepNext: true,
    children: [
      new TextRun({ text: label + " ", bold: true, italics: true, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.H1_COLOR }),
      new TextRun({ text, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK })
    ]
  });
}

/** Theorem block with left steel blue border — distinct from gold definitions. */
function theorem(label, text) {
  return new Paragraph({
    border: { left: { style: BorderStyle.SINGLE, size: 6, color: C.H2_COLOR, space: 8 } },
    spacing: { before: 160, after: 200, line: 312 },
    indent: { left: 360 },
    keepNext: true,
    children: [
      new TextRun({ text: label + " ", bold: true, italics: true, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.H1_COLOR }),
      new TextRun({ text, italics: true, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK })
    ]
  });
}

// --- Code Blocks ---

/** Multi-line code block with warm gray left border.
 *  The left border ties code blocks into the document's border design language
 *  (gold for definitions, steel blue for theorems, warm gray for code). */
function codeBlock(codeString) {
  const lines = codeString.split("\n");
  return lines.map((line, i) =>
    new Paragraph({
      spacing: { after: i === lines.length - 1 ? 200 : 0, line: 240 },
      shading: { type: ShadingType.CLEAR, fill: C.CODE_BG },
      border: { left: { style: BorderStyle.SINGLE, size: 4, color: C.ACCENT_SECONDARY, space: 6 } },
      children: [new TextRun({ text: line || " ", font: C.CODE_FONT, size: C.CODE_SIZE, color: C.CODE_TEXT })]
    })
  );
}

// --- Tables ---

/** Standard table with header row + data rows.
 *  Deep navy headers with tracking, warm cream alternating rows, refined borders.
 *  Garamond at 10.5pt for cell text (Garamond's small x-height needs the bump).
 *  @param {string[]} headers - Column header labels
 *  @param {string[][]} rows - 2D array of cell text
 *  @param {number[]} [colWidths] - Optional column widths in DXA (must sum to 9360)
 */
function table(headers, rows, colWidths) {
  const numCols = headers.length;
  const widths = colWidths || evenWidths(numCols);
  const border = { style: BorderStyle.SINGLE, size: 1, color: C.ACCENT_SECONDARY };
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
          size: isHeader ? 20 : 21,
          color: isHeader ? C.TABLE_HEADER_TEXT : C.DARK,
          characterSpacing: isHeader ? 20 : undefined,
        })]
      })]
    });
  }

  return [
    new Table({
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
    }),
    new Paragraph({ spacing: { after: 200 }, children: [] })
  ];
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
  rule, ruleLight,
  codeBlock, table, evenWidths,
  spacer, pageBreak,
  getTocEntries
};
