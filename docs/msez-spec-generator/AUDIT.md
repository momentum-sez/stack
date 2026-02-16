# Deep Audit Report: `msez-spec-generator` v2

**Date**: 2026-02-16
**Scope**: `docs/msez-spec-generator/` — the Node.js/docx-js pipeline that generates the MSEZ Stack v0.4.44 GENESIS specification document
**Auditor**: Architecture review per CLAUDE.md mandate
**Supersedes**: AUDIT.md v1 (same date, initial findings)

---

## 1. Executive Summary

This audit identifies **23 findings across 3 severity levels** after a complete read of all 70 chapter source files and cross-referencing against the live Rust codebase on the `master` branch.

**The five most impactful problems:**

| # | Problem | Severity | Est. Impact |
|---|---------|----------|-------------|
| 1 | **Compliance domains in Chapter 10 do not match the codebase.** The spec lists 20 domain names; 11 of them differ from the actual `ComplianceDomain` enum in `msez-core/src/domain.rs`. A regulator reading the spec and an engineer reading the code see different taxonomies. | P0 | Credibility-destroying for technical reviewers |
| 2 | **TOC is ~30+ pages.** `headingStyleRange: "1-3"` captures all 495 headings (87 H1 + 282 H2 + 126 H3). `partHeading()` uses `HeadingLevel.HEADING_1`, same as `chapterHeading()`, so Parts and Chapters are visually indistinguishable in the TOC. | P0 | ~25 pages of wasted front matter |
| 3 | **Content repetition inflates the document ~20%.** "Compliance tensor" re-explained 75 times across 34 files. "Receipt chain" re-explained 69 times. Every mention restates what Chapters 9-10 already define. | P0 | ~80-100 pages removable |
| 4 | **Chapter 7 (Profiles) generates ~45 pages of near-identical tables.** Seven profiles repeat the same 16-row module table and 5-row resource table with minor cell variations. No comparison matrix exists. | P1 | ~35 pages compressible to ~10 |
| 5 | **No page breaks between chapters.** Only 21 `pageBreak()` calls for 70 files. Most chapters bleed into each other without visual separation. | P1 | Entire document unprintable as-is |

**Estimated page savings from all interventions: 150-200 pages (from ~550 to ~350-400).**

**Severity summary:**

| Severity | Count | Description |
|----------|-------|-------------|
| **P0 — Must fix** | 5 | Factual errors, credibility-destroying issues, or broken document structure |
| **P1 — Should fix** | 10 | Content quality, readability, information density, or navigability |
| **P2 — Nice to fix** | 8 | Code quality, maintainability, polish |

---

## 2. Quantitative Profile

### 2.1 Codebase Metrics

| Metric | Value |
|--------|-------|
| Total chapter files | 70 |
| Total chapter lines | 8,909 |
| Library + build lines | ~423 |
| **Total project lines** | **~9,332** |

### 2.2 Document Element Inventory

| Element | Count | Notes |
|---------|-------|-------|
| `chapterHeading()` | 68 | One per chapter (excluding cover + TOC) |
| `partHeading()` | 19 | 18 Parts + 1 Appendices header |
| `h2()` | 282 | Primary section headings |
| `h3()` | 126 | Subsection headings (concentrated in 22 of 70 files) |
| `table()` | 185 | Data tables |
| `codeBlock()` | 100 | Rust/pseudocode examples |
| `spacer()` | ~365 | Vertical spacing (5.2 per file average) |
| `pageBreak()` | 21 | Explicit page breaks (should be ~70) |
| `definition()` | 30 | Formal definitions |
| `theorem()` | 10 | Formal theorems |

### 2.3 TOC Entry Count (Current)

| Level | Style | Count | In TOC? |
|-------|-------|-------|---------|
| Part headings | `HeadingLevel.HEADING_1` | 19 | **Yes** (same as chapters) |
| Chapter headings | `HeadingLevel.HEADING_1` | 68 | **Yes** |
| H2 sections | `HeadingLevel.HEADING_2` | 282 | **Yes** |
| H3 subsections | `HeadingLevel.HEADING_3` | 126 | **Yes** (`headingStyleRange: "1-3"`) |
| **Total TOC entries** | | **495** | **~30+ pages at 15-17 entries/page** |

### 2.4 Chapter Size Distribution

| Bucket | Count | Files |
|--------|-------|-------|
| **< 45 lines** | 5 | `00-toc` (18), `42-protocol-overview` (32), `24-multilateral` (41), `55-partners` (41), `00-cover` (43) |
| **45–80 lines** | 12 | `54-adoption` (44), `28-quorum-finality` (54), `31-compensation` (57), `A-version-history` (64), and others |
| **80–130 lines** | 27 | Majority of chapters |
| **130–250 lines** | 19 | Including most institutional/corridor chapters |
| **> 250 lines** | 7 | `06-pack-trilogy` (639), `07-profiles` (466), `03-crypto-primitives` (314), `43-credentials` (245), `08-smart-asset` (237), `30-migration-fsm` (215), `45-agentic` (207) |

### 2.5 Content Repetition

| Phrase | Occurrences | Files |
|--------|-------------|-------|
| "compliance tensor" | 75 | 34 / 70 |
| "receipt chain" | 69 | ~30 |
| "watcher" | 77 | ~25 |
| "20 domains" or "20 compliance" | 14 | 14 |
| "five programmable primitives" | ~12 | ~10 |
| "Verifiable Credential" | ~40 | ~20 |

---

## 3. Findings

### P0-001: Compliance Domain Taxonomy Mismatch (CRITICAL)

**Severity**: P0 — Must fix before any distribution
**Location**: `chapters/10-compliance-tensor.js` lines 63-85

**Description**: Chapter 10 §10.2 lists 20 compliance domain names. **Eleven of these do not match the actual `ComplianceDomain` enum in `msez-core/src/domain.rs`.** The spec documents a taxonomy that does not exist in the code.

**Actual enum** (from `msez/crates/msez-core/src/domain.rs:32-73`):
```
Aml, Kyc, Sanctions, Tax, Securities, Corporate, Custody, DataPrivacy,
Licensing, Banking, Payments, Clearing, Settlement, DigitalAssets,
Employment, Immigration, Ip, ConsumerProtection, Arbitration, Trade
```

**Spec claims** (Chapter 10, §10.2 table):
```
CIVIC, CORPORATE, COMMERCIAL, FINANCIAL, SECURITIES, BANKING, PAYMENTS,
DIGITAL_ASSETS, TAX, AML_CFT, DATA_PROTECTION, ARBITRATION, LICENSING,
INSURANCE, ENVIRONMENTAL, LABOR, INTELLECTUAL_PROPERTY, IMMIGRATION,
REAL_ESTATE, HEALTH_SAFETY
```

**Domain-by-domain delta:**

| Spec Name | Codebase Name | Status |
|-----------|---------------|--------|
| CIVIC | *(none)* | **FABRICATED** — no Civic domain exists |
| COMMERCIAL | Trade | **WRONG NAME** |
| FINANCIAL | *(none)* | **FABRICATED** — codebase has Clearing + Settlement instead |
| AML_CFT | Aml | **WRONG** — codebase splits AML, KYC, Sanctions into 3 separate domains |
| DATA_PROTECTION | DataPrivacy | **WRONG NAME** |
| INSURANCE | *(none)* | **FABRICATED** — no Insurance domain exists |
| ENVIRONMENTAL | *(none)* | **FABRICATED** — no Environmental domain exists |
| LABOR | Employment | **WRONG NAME** |
| INTELLECTUAL_PROPERTY | Ip | **WRONG NAME** |
| REAL_ESTATE | *(none)* | **FABRICATED** — no RealEstate domain exists |
| HEALTH_SAFETY | *(none)* | **FABRICATED** — no HealthSafety domain exists |
| *(not in spec)* | Kyc | **MISSING** from spec |
| *(not in spec)* | Sanctions | **MISSING** from spec |
| *(not in spec)* | Custody | **MISSING** from spec |
| *(not in spec)* | Clearing | **MISSING** from spec |
| *(not in spec)* | Settlement | **MISSING** from spec |
| *(not in spec)* | ConsumerProtection | **MISSING** from spec |

**Impact**: A regulator reading Chapter 10 sees domains like "ENVIRONMENTAL" and "HEALTH_SAFETY." An engineer reading `domain.rs` sees `Clearing` and `ConsumerProtection`. They are reading different systems. This is exactly the "plausible-sounding nonsense" the CLAUDE.md anti-slop protocol warns about.

**Fix**: Replace the Chapter 10 domain table with the exact enum variants from `msez-core/src/domain.rs`, using the doc-comments as descriptions. Every other chapter that references specific domains must also be updated.

---

### P0-002: TOC Bloat — 495 Entries (~30+ Pages)

**Severity**: P0 — Must fix
**Location**: `chapters/00-toc.js` line 14, `lib/primitives.js` lines 52-61

**Root cause (dual):**

1. `partHeading()` uses `HeadingLevel.HEADING_1`, identical to `chapterHeading()`. Parts and Chapters appear at the same indentation in the TOC, making it impossible to distinguish "PART I: FOUNDATION" from "Chapter 1: Mission and Vision."

2. `headingStyleRange: "1-3"` captures H1 + H2 + H3 = 87 + 282 + 126 = **495 TOC entries**. At ~15-17 entries per page, this produces a **29-33 page TOC.**

**Recommended fix (Option C — Hybrid, targeting 5-7 page TOC):**

- `partHeading()` → render as display-only styled text (large, bold, centered, dark blue), **NOT** a heading level. Remove `heading: HeadingLevel.HEADING_1`. Parts do not appear in the TOC.
- `chapterHeading()` → remains `HeadingLevel.HEADING_1`. Each of 68 chapters is a TOC entry.
- `h2()` → remains `HeadingLevel.HEADING_2`. Stays in TOC.
- `h3()` → remains `HeadingLevel.HEADING_3`. Stays OUT of TOC.
- `headingStyleRange: "1-2"` → captures 68 chapters + 282 sections = **350 entries** (~21 pages). Still too many.
- To reach 5-7 pages: `headingStyleRange: "1-1"` → captures 68 chapters only = **68 entries** (~4-5 pages). Each Part opener includes a local section listing for granular navigation.

**Code patch for `partHeading()` in `lib/primitives.js`:**

```javascript
/** Part heading — display-only, NOT in TOC */
function partHeading(text) {
  return [
    new Paragraph({ children: [new PageBreak()] }),
    new Paragraph({
      // NO heading: HeadingLevel property — keeps it out of TOC
      alignment: AlignmentType.CENTER,
      spacing: { before: 2000, after: 300 },
      children: [new TextRun({
        text: text.toUpperCase(),
        bold: true,
        font: C.BODY_FONT,
        size: 44,       // Larger than H1 (36) for visual distinction
        color: C.H1_COLOR
      })]
    })
  ];
}
```

**Code patch for `00-toc.js`:**
```javascript
new TableOfContents("Table of Contents", {
  hyperlink: true,
  headingStyleRange: "1-2",  // Changed from "1-3" — capture chapters + sections only
})
```

---

### P0-003: Content Repetition Inflates Document ~20%

**Severity**: P0 — Must fix
**Location**: 34 of 70 chapter files

**Description**: Core concepts are re-explained in every chapter they appear. The compliance tensor is described or contextualized 75 times across 34 files. Receipt chains appear 69 times. Every mention restates what Chapters 9-10 already define rather than using a terse cross-reference.

**Examples of re-explanation (should be cross-references):**

- `07-profiles.js` line 42: "The compliance surface is maximized: all 20 compliance domains are active, sanctions screening operates in real-time" → should be "All 20 compliance domains active (§10.2)"
- `22-corridor-arch.js`: "The compliance tensor evaluates across 20 domains for each jurisdiction" → should be "per §10"
- Multiple chapters: "receipt chains with Merkle proofs" → should be "receipt chains (§9)"

**Recommendation**: Define each concept ONCE in its canonical chapter. Thereafter use `(§N)` or `(§N.M)` references. The canonical locations are:

| Concept | Canonical Chapter | Reference Form |
|---------|-------------------|----------------|
| Compliance tensor | Ch. 10 | "(§10)" or "per the Compliance Tensor V2 (§10)" |
| Compliance domains (20) | Ch. 10, §10.2 | "(§10.2)" |
| Receipt chain | Ch. 9 | "(§9)" |
| Corridor lifecycle FSM | Ch. 22 | "(§22)" |
| Pack trilogy | Ch. 6 | "(§6)" |
| Verifiable Credentials | Ch. 43 | "(§43)" |
| Watcher economy | Ch. 26 | "(§26)" |
| Mass/SEZ boundary | Ch. 1, §1.2 | "(§1.2)" |

**Estimated savings**: 80-100 pages of redundant exposition.

---

### P0-004: Marketing Language in a Technical Specification

**Severity**: P0 — Must fix
**Location**: `00-executive-summary.js`, `01-mission-vision.js`, introductory paragraphs of ~15 chapters

**Description**: The specification oscillates between engineering precision and marketing copy. Examples:

- `07-profiles.js` line 39: "This is the most comprehensive profile in the MSEZ Stack" → Specification language: "This profile activates all 16 module families."
- `07-profiles.js` line 204: "The sovereign-govos profile is the most demanding deployment configuration in the MSEZ Stack. It transforms the Stack from a zone management system into a national operating system for government services." → Marketing.
- `07-profiles.js` line 334: "Unlike the tech-park profile, which accommodates existing technology companies within a traditional zone framework, the digital-native-free-zone is built for organizations that may never require physical premises" → Sales copy comparing SKUs.
- Chapter 7 profile descriptions routinely run 5-10 lines of prose before any structured content. Each profile section opens with a paragraph explaining the profile's market positioning rather than its technical configuration.

**Recommendation**: Strip superlatives and comparative positioning. Each profile section should open with its definition tuple `(M, Θ, T, R)` instantiation, followed by the module table. Prose descriptions of market fit belong in a sales document, not a specification.

---

### P0-005: No Page Breaks Between Chapters

**Severity**: P0 — Must fix
**Location**: All 70 chapter files; `lib/primitives.js` lines 64-69

**Description**: Only 21 `pageBreak()` calls exist across 70 files. `partHeading()` includes a page break, but `chapterHeading()` does not. This means:

- The first chapter of each Part gets a page break (from the Part heading's `PageBreak`)
- Subsequent chapters within the same Part flow directly into the previous chapter without any break
- A 500+ page specification where chapters bleed into each other is unprintable and unnavigable

**Fix**: Add `pageBreakBefore: true` to `chapterHeading()`:

```javascript
function chapterHeading(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_1,
    pageBreakBefore: true,
    children: [new TextRun({ text, bold: true, font: C.BODY_FONT, size: 36, color: C.H1_COLOR })]
  });
}
```

This ensures every chapter starts on a new page. The explicit `pageBreak()` calls before chapter headings in existing files can then be removed to avoid double breaks.

---

### P1-001: Chapter 7 (Profiles) is 466 Lines of Repetition

**Severity**: P1 — Should fix
**Location**: `chapters/07-profiles.js`

**Description**: Seven profiles repeat an identical structure: intro paragraph → capabilities prose → 16-row module table → 5-row resource table → example deployment prose. The module tables share 80%+ structure; the differences are which rows say "Active", "Minimal", or "Inactive." No comparison matrix exists.

**Recommended restructure:**

1. **Add comparison matrix** at §7.1 — a single 7-column × 16-row table showing all profiles side-by-side.
2. **Data-drive the per-profile sections** — extract profile definitions into a JS data structure; generate sections programmatically.
3. **Exception-based documentation** — after the comparison matrix, each profile section documents only what differs from the `digital-financial-center` baseline.

**Estimated savings**: 35 pages compressible to ~10 pages.

---

### P1-002: Heading Hierarchy is Flat

**Severity**: P1 — Should fix
**Location**: 48 of 70 files use zero `h3()` calls

**Description**: 282 H2 headings but only 126 H3 headings, concentrated in 22 files. The remaining 48 files present all sections at H2 level, creating a flat, undifferentiated TOC.

**Recommendation**: Audit chapters with 4+ H2 sections. Where a chapter has clear hierarchical structure (e.g., "Operations" → "Monitoring" → "Metrics", "Alerts"), demote the leaf-level items from H2 to H3.

---

### P1-003: Single-Section Document — Cover Header Incorrect

**Severity**: P1 — Should fix
**Location**: `build.js` lines 112-142

**Description**: The entire document is one section. The cover page displays the header "MSEZ Stack v0.4.44 — GENESIS" and footer "Momentum · CONFIDENTIAL · Page N" — both inappropriate for a title page. The cover should have no header/footer.

**Fix**: Split into at least 3 sections (cover, front matter/TOC, body). The cover section uses `titlePage: true` with blank first-page header/footer.

---

### P1-004: `spacer()` Over-Reliance (365 Calls)

**Severity**: P1 — Should fix
**Location**: All chapter files, `lib/primitives.js` line 187-189

**Description**: Vertical spacing is managed by injecting empty paragraphs. This creates real paragraphs visible in Word's paragraph mark view, can cause unwanted page breaks at spacer locations, and inflates element count.

**Fix**: Set `spacing.after` on table and code block elements in `primitives.js`. The `table()` function currently has no spacing; add `spacing: { after: 200 }` to the wrapper. Do the same for `codeBlock()` trailing paragraph, `definition()`, and `theorem()`. Then systematically remove `spacer()` calls.

---

### P1-005: Code Blocks Lack Visual Definition

**Severity**: P1 — Should fix
**Location**: `lib/primitives.js` lines 118-127

**Description**: `codeBlock()` applies `ShadingType.CLEAR` fill but no border. In a document with gray backgrounds on tables too, code blocks are visually ambiguous.

**Fix**: Add a subtle border or increase left indent:

```javascript
border: {
  top: { style: BorderStyle.SINGLE, size: 1, color: "D1D5DB" },
  bottom: { style: BorderStyle.SINGLE, size: 1, color: "D1D5DB" },
  left: { style: BorderStyle.SINGLE, size: 4, color: C.ACCENT },
},
indent: { left: 360 },
```

---

### P1-006: No Bullet/Numbered List Primitive

**Severity**: P1 — Should fix
**Location**: `lib/primitives.js` (absent), `build.js` lines 98-111

**Description**: `build.js` defines a `"bullets"` numbering reference that is never used by any chapter. The primitives library has no `bulletList()` or `numberedList()` helper. Lists are either absent or simulated with em-dashes in prose, or embedded as comma-separated items in paragraphs.

**Fix**: Add `bulletItem()` and `numberedItem()` primitives:

```javascript
function bulletItem(text) {
  return new Paragraph({
    numbering: { reference: "bullets", level: 0 },
    spacing: { after: 60, line: 276 },
    children: [new TextRun({ text, font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK })]
  });
}
```

---

### P1-007: Part XIII is a Single Chapter

**Severity**: P1 — Should fix
**Location**: `37-mass-bridge.js`

**Description**: Part XIII ("Mass API Integration") contains only Chapter 37 (101 lines). This Part documents `msez-mass-client` — arguably the most operationally critical crate. A single 101-line chapter is insufficient for a crate that is the sole authorized path to Mass.

**Recommendation**: Split into 2-3 chapters: (1) Client architecture and configuration, (2) Orchestration pattern (compliance → Mass → VC → attestation), (3) Contract test methodology.

---

### P1-008: Part XV / Part XIII Naming Collision

**Severity**: P1 — Should fix
**Location**: Part headings in `37-mass-bridge.js` and `42-protocol-overview.js`

**Description**: Part XIII is "Mass API Integration Layer" and Part XV is "Mass Protocol Integration." Both reference "Mass" and "Integration." Readers may conflate the HTTP client layer (Ch. 37) with the credential/arbitration/agentic system (Chs. 42-45).

**Fix**: Rename Part XV to "PART XV: CREDENTIALS, ARBITRATION, AND AGENTIC SYSTEMS" — this describes its actual content.

---

### P1-009: `partHeading()` and `chapterHeading()` Use Identical Style

**Severity**: P1 — Should fix (related to P0-002)
**Location**: `lib/primitives.js` lines 52-69

**Description**: Both use `HeadingLevel.HEADING_1`, 36pt, same color. The only visual difference is `partHeading()` uppercases its text. In the body of the document, a Part heading and a Chapter heading are nearly indistinguishable. Parts should be visually dominant — larger font, centered, possibly with a horizontal rule.

**Fix**: See P0-002 patch — `partHeading()` becomes display-only at 44pt, centered, without a heading level.

---

### P1-010: Executive Summary Statistics Need Verification

**Severity**: P1 — Should fix
**Location**: `00-executive-summary.js` lines 7, 59

**Description**: The executive summary claims "101,000 lines of production code with 3,029 tests" and "243 source files, 16 crates." Current codebase verification shows:

| Metric | Spec Claims | Actual (master branch) |
|--------|-------------|----------------------|
| .rs files | 243 | 257 |
| Lines of Rust | ~101,000 | ~108,911 |
| `#[test]` annotations | 3,029 | ~5,066 |
| Crates | 16 | 17 (Cargo.toml count) |

The spec figures are stale. While the line counts are in the right ballpark, the test count is off by 67%.

**Fix**: Update to current figures or add a note that figures are approximate as of a specific commit.

---

### P2-001: TOC Renders Client-Side Only

**Severity**: P2 — Nice to fix
**Location**: `chapters/00-toc.js`

**Description**: `docx` library's `TableOfContents` generates a field code evaluated by Word on open. The TOC is blank in Google Docs, LibreOffice, and automated PDF pipelines.

**Recommendation**: Document this limitation. Consider a LibreOffice headless post-processing step.

---

### P2-002: Build Validation Absent

**Severity**: P2 — Nice to fix
**Location**: `build.js`

**Description**: No validation that chapters export functions, return valid arrays, or produce valid docx elements. A broken chapter produces a silently malformed document.

**Recommendation**: Add a `validate.js` pre-build check.

---

### P2-003: `package.json` Missing Build Script

**Severity**: P2 — Nice to fix
**Location**: `package.json`

**Description**: No `scripts` entries. Users must know to run `node build.js`. Add:
```json
"scripts": {
  "build": "node build.js",
  "validate": "node validate.js"
}
```

---

### P2-004: Cover Page Uses Raw `docx` API Instead of Primitives

**Severity**: P2 — Nice to fix
**Location**: `chapters/00-cover.js`

**Description**: The cover page manually constructs `Paragraph` and `TextRun` objects rather than using the primitives library. This creates a maintenance burden — if fonts, colors, or spacing change in `constants.js`, the cover page won't reflect them.

---

### P2-005: Inconsistent Table Column Width Sums

**Severity**: P2 — Nice to fix
**Location**: Multiple chapters

**Description**: `constants.js` defines `CONTENT_W: 9360` (DXA). Column widths should sum to 9360. Most tables comply, but manual width arrays should be audited for consistency.

---

### P2-006: `codeBlock()` Uses String Concatenation Instead of Template Literals

**Severity**: P2 — Nice to fix
**Location**: Multiple chapters (e.g., `10-compliance-tensor.js` lines 20-56)

**Description**: Code blocks are constructed via string concatenation with `+` and `\n`. Template literals (backtick strings) are cleaner and already used in some chapters (e.g., `51-terraform.js`). The inconsistency makes the source harder to read.

---

### P2-007: 48 Chapters Lack Any `h3()` Usage

**Severity**: P2 — Nice to fix
**Location**: 48 of 70 chapter files

**Description**: Related to P1-002 but distinct: even chapters with 5+ H2 sections use no H3 subdivisions. This makes chapters like `46-security.js` (4 H2s, 0 H3s) flatter than warranted by their content depth.

---

### P2-008: Branding and Naming Compliance

**Severity**: P2 — Verified PASS

| Check | Result |
|-------|--------|
| "Momentum Protocol" misuse | **PASS** — zero occurrences |
| "momentum.xyz/io/com" | **PASS** — zero occurrences |
| "mass.xyz/io" | **PASS** — zero occurrences |
| "MSEZ Protocol" misuse | **PASS** — zero occurrences |
| "Mass Protocol" context-appropriate | **PASS** — appears only in Part VI (L1) and Part XV |
| Header/footer branding | **PASS** |

---

## 4. TOC & Heading Hierarchy Reform

### 4.1 Current State

```
partHeading()    → HEADING_1 (36pt, uppercase)     → IN TOC
chapterHeading() → HEADING_1 (36pt, title case)    → IN TOC
h2()             → HEADING_2 (28pt)                 → IN TOC
h3()             → HEADING_3 (24pt)                 → IN TOC (headingStyleRange "1-3")

Total TOC entries: 19 + 68 + 282 + 126 = 495
Estimated TOC pages: 29-33
```

### 4.2 Proposed State

```
partHeading()    → Display-only (44pt, centered)    → NOT in TOC
chapterHeading() → HEADING_1 (36pt, pageBreakBefore) → IN TOC
h2()             → HEADING_2 (28pt)                  → IN TOC
h3()             → HEADING_3 (24pt)                  → NOT in TOC

headingStyleRange: "1-2"

Total TOC entries: 68 + 282 = 350
Estimated TOC pages: 21-23
```

### 4.3 Further Reduction (Aggressive Option)

To reach a 5-7 page TOC:

```
headingStyleRange: "1-1"  → chapters only = 68 entries ≈ 4-5 pages

Each Part opener includes a local section listing:
  PART IV: CORE COMPONENTS
    Chapter 5: Module Specifications
    Chapter 6: Pack Trilogy
    Chapter 7: Profile System
```

This requires each `partHeading()` to also emit a mini-TOC of its chapters. This can be generated from metadata.

### 4.4 H2 Density Audit (Chapters with 5+ H2s)

These chapters contribute the most to TOC bloat at the H2 level and should be audited for H2→H3 demotion:

| Chapter | H2 Count | Candidates for H3 Demotion |
|---------|----------|---------------------------|
| `07-profiles.js` | 9 | 7.2-7.8 profile sections could each be H2 with subsections as H3 |
| `33-identity.js` | 6 | Progressive KYC tiers could be H3 under a single "KYC Framework" H2 |
| `22-corridor-arch.js` | 5 | Corridor types could be H3 under "Architecture" H2 |
| `26-watcher-arch.js` | 5 | Bond/slashing details could be H3 |
| `43-credentials.js` | 5 | Credential types could be H3 |
| `J-conformance.js` | 5 | Conformance levels could be H3 |

---

## 5. Content Audit: Chapter-by-Chapter

### Legend

- **KEEP**: Chapter is solid, requires only minor edits
- **EDIT**: Chapter needs prose tightening, cross-reference substitution, or factual correction
- **MERGE**: Chapter should be combined with another
- **REWRITE**: Chapter needs structural overhaul
- **CUT**: Chapter should be removed or absorbed into another

### Preamble

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `00-cover.js` | Cover | 43 | KEEP | Functional. Minor: uses raw docx API instead of primitives |
| `00-toc.js` | TOC | 18 | EDIT | Change `headingStyleRange` to `"1-2"` (minimum) or `"1-1"` (aggressive) |
| `00-executive-summary.js` | Exec Summary | 126 | EDIT | Update codebase statistics (257 files, ~109K lines, ~5,066 tests, 17 crates). Strip any remaining marketing language. |

### Part I: Foundation (Chapters 1-2)

| File | Chapter | Lines | H2 | H3 | Tables | Action | Notes |
|------|---------|-------|----|----|--------|--------|-------|
| `01-mission-vision.js` | Ch 1 | 128 | 4 | 0 | 3 | KEEP | Solid. Cross-references correct. Could add H3 depth under §1.4 Design Principles. |
| `02-architecture.js` | Ch 2 | 137 | 5 | 0 | 3 | KEEP | Good architectural overview. |

### Part II: Cryptographic Primitives (Chapter 3)

| File | Chapter | Lines | H2 | H3 | Tables | Code | Action | Notes |
|------|---------|-------|----|----|--------|------|--------|-------|
| `03-crypto-primitives.js` | Ch 3 | 314 | 5 | 5 | 5 | 4 | KEEP | Strong. Deep crypto spec with Rust code examples. |

### Part III: Artifact Model (Chapter 4)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `04-artifact-model.js` | Ch 4 | 114 | KEEP | Adequate. |

### Part IV: Core Components (Chapters 5-7)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `05-module-specs.js` | Ch 5 | 71 | EDIT | Thin for 16 module families. Could expand. |
| `06-pack-trilogy.js` | Ch 6 | 639 | EDIT | Largest file. Dense but well-structured. Minor repetition trim needed. |
| `07-profiles.js` | Ch 7 | 466 | **REWRITE** | See §6 below — needs comparison matrix, exception-based docs, data-driven generation. |

### Part V: Smart Asset Execution (Chapters 8-12)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `08-smart-asset.js` | Ch 8 | 237 | KEEP | Strong. Good code examples. |
| `09-receipt-chain.js` | Ch 9 | 111 | KEEP | Canonical definition for receipt chains. |
| `10-compliance-tensor.js` | Ch 10 | 192 | **REWRITE** | **P0-001**: Domain taxonomy wrong. Must match `msez-core/src/domain.rs` exactly. Meet/join tables are excellent — keep those. |
| `11-savm.js` | Ch 11 | 181 | KEEP | Good SAVM spec. |
| `12-composition.js` | Ch 12 | 146 | KEEP | Solid composition engine spec. |

### Part VI: L1 Settlement (Chapters 13-16)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `13-l1-architecture.js` | Ch 13 | 108 | KEEP | Adequate for `[PLANNED]` status. |
| `14-proving-system.js` | Ch 14 | 96 | KEEP | |
| `15-privacy.js` | Ch 15 | 77 | KEEP | |
| `16-anchoring.js` | Ch 16 | 66 | EDIT | Thin. Could merge with Ch 13. |

### Part VII: Governance and Civic (Chapters 17-18)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `17-constitutional.js` | Ch 17 | 71 | KEEP | |
| `18-civic-services.js` | Ch 18 | 198 | KEEP | Well-structured civic services. |

### Part VIII: Compliance and Regulatory (Chapters 19-21)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `19-compliance-arch.js` | Ch 19 | 170 | EDIT | Good structure but re-explains tensor. Use cross-ref to §10. |
| `20-manifold.js` | Ch 20 | 174 | KEEP | Strong mathematical spec. |
| `21-zkkyc.js` | Ch 21 | 122 | KEEP | |

### Part IX: Corridor Systems (Chapters 22-25)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `22-corridor-arch.js` | Ch 22 | 145 | EDIT | Canonical corridor chapter — good. Minor repetition trim. |
| `23-corridor-bridge.js` | Ch 23 | 82 | KEEP | |
| `24-multilateral.js` | Ch 24 | 41 | EDIT | Thin. Could merge with Ch 23 or expand. |
| `25-live-corridors.js` | Ch 25 | 84 | KEEP | Good corridor specifications. |

### Part X: Watcher Economy (Chapters 26-28)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `26-watcher-arch.js` | Ch 26 | 109 | KEEP | |
| `27-bond-slashing.js` | Ch 27 | 73 | KEEP | |
| `28-quorum-finality.js` | Ch 28 | 54 | EDIT | Thin. Could merge with Ch 27. |

### Part XI: Migration (Chapters 29-31)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `29-migration.js` | Ch 29 | 98 | MERGE | Merge with Ch 30 — these are one topic. |
| `30-migration-fsm.js` | Ch 30 | 215 | MERGE | Strong FSM spec. Absorb Ch 29 as introduction. |
| `31-compensation.js` | Ch 31 | 57 | MERGE | Thin. Merge into combined Ch 29-30. |

### Part XII: Institutional (Chapters 32-36)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `32-corporate.js` | Ch 32 | 148 | KEEP | Well-structured corporate services. |
| `33-identity.js` | Ch 33 | 100 | KEEP | |
| `34-tax.js` | Ch 34 | 92 | KEEP | |
| `35-capital-markets.js` | Ch 35 | 141 | KEEP | Good depth with 7 tables. |
| `36-trade.js` | Ch 36 | 80 | KEEP | |

### Part XIII: Mass API Integration (Chapter 37)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `37-mass-bridge.js` | Ch 37 | 101 | EDIT | Thin for a single-chapter Part. Consider expanding. See P1-007. |

### Part XIV: GovOS (Chapters 38-41)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `38-govos-layers.js` | Ch 38 | 121 | KEEP | |
| `39-sovereign-ai.js` | Ch 39 | 123 | KEEP | |
| `40-tax-pipeline.js` | Ch 40 | 138 | KEEP | |
| `41-sovereignty.js` | Ch 41 | 142 | KEEP | |

### Part XV: Protocol Reference (Chapters 42-45)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `42-protocol-overview.js` | Ch 42 | 32 | EDIT | Very thin for a Part opener. Expand or merge with Ch 43. |
| `43-credentials.js` | Ch 43 | 245 | KEEP | Strong VC spec. |
| `44-arbitration.js` | Ch 44 | 118 | KEEP | |
| `45-agentic.js` | Ch 45 | 207 | KEEP | Good depth with trigger taxonomy. |

### Part XVI: Security (Chapters 46-48)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `46-security.js` | Ch 46 | 80 | EDIT | Thin for security architecture of a sovereign system. |
| `47-hardening.js` | Ch 47 | 76 | MERGE | Could merge with Ch 46 — hardening is part of security. |
| `48-zk-circuits.js` | Ch 48 | 91 | KEEP | Separate topic (ZK), justified as standalone. |

### Part XVII: Deployment and Ops (Chapters 49-53)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `49-deployment.js` | Ch 49 | 93 | KEEP | Good tables. |
| `50-docker.js` | Ch 50 | 93 | KEEP | |
| `51-terraform.js` | Ch 51 | 101 | KEEP | **Improved since v1 audit.** Now has real tables and code. |
| `52-one-click.js` | Ch 52 | 76 | MERGE | Could merge with Ch 49. |
| `53-operations.js` | Ch 53 | 80 | KEEP | **Improved since v1 audit.** Has proper metric and alert tables. |

### Part XVIII: Network Diffusion (Chapters 54-56)

| File | Chapter | Lines | Action | Notes |
|------|---------|-------|--------|-------|
| `54-adoption.js` | Ch 54 | 44 | MERGE | Thin. Merge with Ch 55 or Ch 56. |
| `55-partners.js` | Ch 55 | 41 | MERGE | **Improved since v1 audit.** Has real tables. But thin. Merge with Ch 54. |
| `56-current-network.js` | Ch 56 | 149 | KEEP | Good network topology spec. |

### Appendices

| File | Lines | Action | Notes |
|------|-------|--------|-------|
| `A-version-history.js` | 64 | KEEP | |
| `B-test-coverage.js` | 139 | EDIT | Update test count to ~5,066. |
| `C-scalability.js` | 126 | KEEP | |
| `D-security-proofs.js` | 194 | KEEP | |
| `E-crate-deps.js` | 98 | EDIT | Verify dependency tree matches current Cargo workspace. |
| `F-api-endpoints.js` | 71 | KEEP | |
| `G-jurisdiction-templates.js` | 103 | KEEP | |
| `H-cli-reference.js` | 168 | KEEP | |
| `I-module-directory.js` | 72 | KEEP | |
| `J-conformance.js` | 107 | KEEP | |
| `K-govos-checklist.js` | 92 | KEEP | |

### Summary of Actions

| Action | Count | Chapters |
|--------|-------|----------|
| **KEEP** | 43 | Most chapters are solid |
| **EDIT** | 14 | Cross-ref cleanup, statistics updates, domain fixes |
| **MERGE** | 10 | 29+30+31, 46+47, 52→49, 54+55, 16→13, 24→23, 28→27, 42→43 |
| **REWRITE** | 2 | Ch 7 (profiles), Ch 10 (tensor domains) |
| **CUT** | 0 | No chapters need full removal |

**After merges: 70 → ~62 chapters (or fewer depending on merge scope).**

---

## 6. Profiles Chapter Rewrite Plan (Chapter 7)

### 6.1 Current Structure (466 lines → ~45 pages)

```
7.1 Profile Overview (table: 7 rows × 3 columns)
7.2 digital-financial-center (intro + capabilities + 16-row module table + 5-row resource table + example)
7.3 trade-hub (same template)
7.4 tech-park (same template)
7.5 sovereign-govos (same template + national systems table)
7.6 charter-city (same template)
7.7 digital-native-free-zone (same template)
7.8 asset-history-bundle (same template)
7.9 Profile Selection and Composition
```

### 6.2 Proposed Structure (~12-15 pages)

```
7.1 Profile Overview (keep existing table)
7.2 Comparison Matrix (NEW: 7-column × 16-row table, Active/Minimal/Inactive)
7.3 Resource Requirements Matrix (NEW: 7-column × 5-row table)
7.4 Profile Details (exception-based)
    7.4.1 digital-financial-center (baseline — 2 paragraphs, no table)
    7.4.2 trade-hub (differences from baseline only)
    7.4.3 tech-park (differences from baseline only)
    7.4.4 sovereign-govos (expanded — national integration table is unique)
    7.4.5 charter-city (differences from baseline only)
    7.4.6 digital-native-free-zone (differences from baseline only)
    7.4.7 asset-history-bundle (differences from baseline only)
7.5 Profile Selection and Composition (keep existing)
7.6 Example Deployments (consolidated)
```

### 6.3 Comparison Matrix Mock-up

| Module Family | DFC | Trade | Tech | GovOS | Charter | Digital | Asset |
|---------------|-----|-------|------|-------|---------|---------|-------|
| Corporate | ● | ● | ● | ● | ● | ● | ◐ |
| Financial | ● | ● | ◐ | ● | ● | ● | ◐ |
| Trade | ● | ● | ✗ | ● | ◐ | ✗ | ● |
| Corridors | ● | ● | ◐ | ● | ● | ● | ● |
| Governance | ● | ● | ● | ● | ● | ● | ◐ |
| Regulatory | ● | ● | ● | ● | ● | ● | ● |
| Licensing | ● | ● | ● | ● | ● | ● | ◐ |
| Legal | ● | ● | ● | ● | ● | ● | ● |
| Identity | ● | ● | ● | ● | ● | ● | ● |
| Compliance | ● (20) | ● (14) | ● (10) | ● (20) | ● (16) | ● (12) | ● (6-10) |
| Tax | ● | ● | ● | ● | ● | ● | ◐ |
| Insurance | ● | ◐ | ✗ | ● | ● | ✗ | ● |
| IP | ● | ✗ | ● | ● | ◐ | ● | ◐ |
| Customs | ● | ● | ✗ | ● | ◐ | ✗ | ● |
| Land/Property | ● | ✗ | ◐ | ● | ● | ✗ | ○ |
| Civic | ● | ◐ | ● | ● | ● | ✗ | ✗ |

Legend: ● Active, ◐ Minimal, ✗ Inactive, ○ Conditional

---

## 7. Prose Style Guide

### Voice and Register

- **Technical specification voice.** Neither academic nor marketing. Write as if documenting an RFC or API reference.
- **Active voice for system behavior.** "The SAVM evaluates the compliance tensor" not "The compliance tensor is evaluated."
- **Present tense** for system behavior. Past tense only for historical facts. Future tense only for explicitly `[PLANNED]` features.

### Sentence Discipline

- **30-word maximum** for specification sentences. If a sentence exceeds 30 words, split it.
- **One idea per sentence.** If a sentence contains "and" joining two independent clauses, split it.
- **No subordinate-clause stacking.** Maximum one subordinate clause per sentence.

### Banned Phrases

| Phrase | Replacement |
|--------|-------------|
| "This is the most [X] in the MSEZ Stack" | State what it does, not how it ranks |
| "categorical shift" | State what changed |
| "eliminates that entire process" | State what the new process is |
| "from first principles" | Delete |
| "not merely a [X]" | State what it IS |
| "the following [X] provides" | Delete; let the content speak |
| "This section describes" | Delete; the heading already says it |

### Cross-Reference Conventions

- **First mention of a concept**: Full name + section reference. "The Compliance Tensor V2 (§10) evaluates..."
- **Subsequent mentions**: Terse reference. "...per §10" or "...tensor evaluation (§10.4)"
- **Never re-explain** a concept that has its own chapter. The 20 compliance domains are listed once in §10.2. Every other chapter uses "the 20 compliance domains (§10.2)."

### Numbers and Units

- Spell out numbers one through nine. Use digits for 10 and above.
- Always include units: "64 GB RAM" not "64 GB".
- Use DXA values in code, human-readable units in prose.

---

## 8. docx-js Technical Fixes

### 8.1 Multi-Section Document Architecture

Replace the single section in `build.js` with at least three sections:

```javascript
sections: [
  // Section 1: Cover page — no header/footer
  {
    properties: {
      page: { size: { width: C.PAGE_W, height: C.PAGE_H },
              margin: { top: C.MARGIN, right: C.MARGIN, bottom: C.MARGIN, left: C.MARGIN } },
      titlePage: true,
    },
    headers: {
      default: new Header({ children: [] }),  // blank header for cover
      first: new Header({ children: [] }),
    },
    footers: {
      default: new Footer({ children: [] }),
      first: new Footer({ children: [] }),
    },
    children: coverElements,
  },
  // Section 2: Front matter (TOC, Executive Summary) — roman numeral pages
  {
    properties: {
      page: { size: { width: C.PAGE_W, height: C.PAGE_H },
              margin: { top: C.MARGIN, right: C.MARGIN, bottom: C.MARGIN, left: C.MARGIN } },
    },
    headers: { default: standardHeader },
    footers: { default: standardFooter },
    children: frontMatterElements,
  },
  // Section 3: Body — arabic numeral pages
  {
    properties: {
      page: { size: { width: C.PAGE_W, height: C.PAGE_H },
              margin: { top: C.MARGIN, right: C.MARGIN, bottom: C.MARGIN, left: C.MARGIN } },
    },
    headers: { default: standardHeader },
    footers: { default: standardFooter },
    children: bodyElements,
  }
]
```

### 8.2 Chapter Page Break Discipline

Add `pageBreakBefore: true` to `chapterHeading()`. Remove explicit `pageBreak()` calls that precede chapter headings.

### 8.3 Code Block Visual Enhancement

Add left accent border and indent to code blocks:

```javascript
function codeBlock(codeString) {
  const lines = codeString.split("\n");
  return lines.map((line, i) =>
    new Paragraph({
      spacing: { after: 0, line: 240 },
      shading: { type: ShadingType.CLEAR, fill: C.CODE_BG },
      border: {
        left: { style: BorderStyle.SINGLE, size: 4, color: C.ACCENT, space: 8 },
      },
      indent: { left: 360 },
      children: [new TextRun({
        text: line || " ",
        font: C.CODE_FONT,
        size: C.CODE_SIZE,
        color: C.CODE_TEXT
      })]
    })
  );
}
```

### 8.4 Spacer Elimination Plan

1. Add `spacing: { after: 200 }` to `table()` wrapper (currently has none)
2. Add `spacing: { after: 160 }` to `codeBlock()` last line
3. Ensure `definition()` and `theorem()` have adequate `spacing.after` (currently 160 — adequate)
4. Systematically grep for `spacer()` calls and remove those immediately following tables, code blocks, definitions, or theorems
5. Retain spacers only where intentional large vertical gaps are needed (e.g., cover page layout)

---

## 9. Estimated Impact

| Intervention | Current Pages | After Fix | Savings |
|-------------|--------------|-----------|---------|
| TOC reform (495 → 68-350 entries) | ~30 | ~5-22 | **8-25 pages** |
| Content repetition removal | ~80-100 scattered | — | **~80-100 pages** |
| Chapter 7 profiles rewrite | ~45 | ~12 | **~33 pages** |
| Chapter merges (8 chapters absorbed) | ~30 | ~20 | **~10 pages** |
| Marketing prose removal | ~15 scattered | — | **~15 pages** |
| **Total estimated reduction** | | | **~150-180 pages** |
| **Estimated final page count** | ~550 | ~370-400 | |

---

## 10. Priority Execution Order

| Phase | Actions | Effort |
|-------|---------|--------|
| **Phase 1 (Critical)** | Fix Chapter 10 compliance domains (P0-001). Fix `partHeading()` to display-only (P0-002). Fix `headingStyleRange` (P0-002). Add `pageBreakBefore` to `chapterHeading()` (P0-005). | Low — code patches only |
| **Phase 2 (Structure)** | Rewrite Chapter 7 profiles with comparison matrix. Merge thin chapter pairs. Add bullet list primitive. | Medium |
| **Phase 3 (Content)** | Replace repeated concept explanations with cross-references across 34 files. Strip marketing language. Update statistics. | Medium — editorial work |
| **Phase 4 (Polish)** | Multi-section document architecture. Spacer elimination. Code block borders. Build validation. | Medium |

---

*End of audit v2.*
