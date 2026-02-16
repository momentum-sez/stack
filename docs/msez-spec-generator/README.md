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

## Known Limitations

### Table of Contents Requires Manual Update

The generated TOC uses a Word field code (`TOC \o "1-1" \h`) that is evaluated client-side when the document is opened. This means:

1. **Microsoft Word**: Right-click the TOC area and select **"Update Field"** → **"Update entire table"** to populate it.
2. **Google Docs / LibreOffice**: The TOC may appear empty or as placeholder text. These renderers do not evaluate Word field codes.
3. **Automated DOCX→PDF pipelines**: Will produce a document with a blank TOC unless a post-processing step updates fields (e.g., via headless LibreOffice: `libreoffice --headless --macro "UpdateFields" file.docx`).

This is a known limitation of the `docx-js` library, not a bug in the generator.

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
  00-toc.js              # Table of contents (field code)
  00-executive-summary.js
  01-mission-vision.js   # Chapter 1 (Part I: Foundation)
  ...                    # 56 chapters + 11 appendices
  K-govos-checklist.js
output/
  MSEZ_Stack_v0.4.44_GENESIS_Specification.docx
```
