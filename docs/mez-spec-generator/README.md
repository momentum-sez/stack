# mez-spec-generator

Generates the **MEZ Stack v0.4.44 GENESIS** specification document as a `.docx` file using [docx-js](https://github.com/dolanmw/docx).

The specification is a 56-chapter, 11-appendix document covering the complete technical architecture of the Momentum EZ Stack: compliance tensor, corridor protocol, pack trilogy, verifiable credentials, agentic policy engine, arbitration, zone composition, and deployment infrastructure.

## Quick start

```bash
npm install
node validate.js   # pre-build validation (optional but recommended)
node build.js      # generates output/MEZ_Stack_v0.4.44_GENESIS_Specification.docx
```

Or via npm scripts:

```bash
npm run validate
npm run build
```

## Project structure

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
  MEZ_Stack_v0.4.44_GENESIS_Specification.docx
```

**70 chapter files** organized into 18 Parts plus appendices A-K. Total project: ~9,100 lines of JavaScript.

## Table of contents approach

The TOC is generated statically at build time using bookmarks and `PAGEREF` fields:

1. **Phase 1**: All chapters are built, populating a heading registry with bookmark names.
2. **Phase 2**: The TOC is generated as 315 static paragraph entries, each containing the heading text, a dot-leader tab, and a `PAGEREF` field referencing the heading's bookmark.

This replaces the previous `TableOfContents` field code approach, which relied on Word's client-side style matching and failed across Word versions.

**Page numbers**: `PAGEREF` fields resolve to page numbers when the document is opened or printed. In Microsoft Word, select all (Ctrl+A) then press F9 to update all fields. Most Word versions update `PAGEREF` fields automatically on open.

## Validation

The `validate.js` script performs pre-build checks:

- Verifies all chapter files export a function
- Checks that exported functions return arrays
- Validates heading sequencing (h2 before h3)
- Reports missing or mistyped primitives

Run before building to catch errors early.

## Updating statistics

When the Rust codebase changes, update statistics in these chapter files:

| Statistic | Files to update |
|-----------|----------------|
| Line count (164K) | `00-executive-summary.js`, `02-architecture.js`, `I-module-directory.js` |
| Test count (4,683) | `00-executive-summary.js`, `B-test-coverage.js`, `56-current-network.js` |
| Crate count (17) | `00-executive-summary.js`, `02-architecture.js`, `E-crate-deps.js`, `56-current-network.js` |
| Source file count (322) | `I-module-directory.js` |
| Module count (323) | `00-executive-summary.js`, `I-module-directory.js` |

See [AUDIT.md](./AUDIT.md) for the full accuracy audit and open findings.

## Known issues

See [AUDIT.md](./AUDIT.md) for the complete audit report. Key open items:

- **P0-002**: Core concepts re-explained across chapters (should use cross-references)
- **P0-003**: Some chapter introductions use whitepaper-style prose instead of specification content
- **P1-001**: 48/70 files use zero `h3()` calls (could improve navigation)
- **P1-002**: Chapter 07 has 7 near-identical profile templates (needs comparison matrix)
