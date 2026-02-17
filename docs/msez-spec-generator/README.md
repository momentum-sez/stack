# msez-spec-generator

Generates the **MSEZ Stack v0.4.44 — GENESIS** specification document as a `.docx` file using [docx-js](https://github.com/dolanmw/docx).

## Usage

```bash
npm install
node validate.js   # pre-build validation (optional)
node build.js      # generates output/MSEZ_Stack_v0.4.44_GENESIS_Specification.docx
```

Or via npm scripts:

```bash
npm run validate
npm run build
```

## Table of Contents

The TOC is generated statically at build time using bookmarks and `PAGEREF` fields. Every H1 and H2 heading registers itself in a heading registry (via `primitives.js`), and `build.js` uses a two-phase approach:

1. **Phase 1**: All chapters are built, populating the heading registry with bookmark names.
2. **Phase 2**: The TOC is generated as 315 static paragraph entries, each containing the heading text, a dot-leader tab, and a `PAGEREF` field referencing the heading's bookmark.

This approach replaces the previous `TableOfContents` field code (`TOC \h \t ...`), which relied on Word's client-side style matching and failed across Word versions, producing a 100+ page TOC filled with body text.

**Page numbers**: `PAGEREF` fields resolve to page numbers when the document is opened or printed. In Microsoft Word, select all (Ctrl+A) then press F9 to update all fields. Most Word versions update `PAGEREF` fields automatically on open.

## Project Structure

```
build.js                 # Document assembler — multi-section, per-Part headers
validate.js              # Pre-build validation (exports, types, heading sequencing)
lib/
  primitives.js          # Composable document elements (p, h2, h3, table, codeBlock, etc.)
  styles.js              # Heading and paragraph style definitions
  constants.js           # Page dimensions, fonts, colors
chapters/
  00-cover.js            # Title page
  00-toc.js              # Table of contents (static, bookmark + PAGEREF)
  00-executive-summary.js
  01-mission-vision.js   # Chapter 1 (Part I: Foundation)
  ...                    # 56 chapters + 11 appendices
  K-govos-checklist.js
output/
  MSEZ_Stack_v0.4.44_GENESIS_Specification.docx
```
