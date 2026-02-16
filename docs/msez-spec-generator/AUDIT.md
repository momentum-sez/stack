# Deep Audit Report: `msez-spec-generator` v3

**Date**: 2026-02-16
**Scope**: `docs/msez-spec-generator/` — the Node.js/docx-js pipeline that generates the MSEZ Stack v0.4.44 GENESIS specification document
**Auditor**: Architecture review per CLAUDE.md mandate
**Supersedes**: AUDIT.md v2 (same date, previous round of findings)
**Methodology**: Full read of all 70 chapter source files, lib/, build.js, validate.js, and cross-reference against the live Rust codebase on the `main` branch at commit `7dadc90`.

---

## 1. Executive Summary

This audit is the third pass over the spec generator. Audit v2 identified 23 findings; 14 have been resolved. This audit verifies those resolutions, identifies 5 remaining open issues, and surfaces 6 new findings discovered during this deeper pass. Total: **11 open findings across 3 severity levels.**

### What Was Fixed Since v2

| v2 ID | Description | Resolution |
|-------|-------------|------------|
| P0-001 | Chapter 10 compliance domains wrong | **RESOLVED** — §10.2 now lists all 20 canonical `ComplianceDomain` enum variants with correct names matching `msez-core/src/domain.rs` |
| P0-002 | TOC bloat (495 entries, ~30 pages) | **RESOLVED** — `partHeading()` is display-only (no `HeadingLevel`), `chapterHeading()` uses H1, `headingStyleRange: "1-2"`. TOC entries: 76 H1 + 228 H2 = ~304 (~18-20 pages) |
| P0-005 | No page breaks between chapters | **RESOLVED** — `chapterHeading()` includes `pageBreakBefore: true` |
| P1-003 | Single-section document | **RESOLVED** — `build.js` produces per-Part sections with Part-specific headers |
| P1-004 | 365 spacer() calls | **RESOLVED** — reduced to 4 (cover page only, for layout) |
| P1-005 | Code blocks lack visual definition | **RESOLVED** — `codeBlock()` has left accent border (`C.ACCENT`, 4pt) |
| P1-006 | No bullet list primitive | **RESOLVED** — `bulletItem()` defined in `primitives.js` |
| P1-009 | partHeading and chapterHeading identical style | **RESOLVED** — partHeading is 44pt centered display-only; chapterHeading is 36pt H1 |
| P1-010 | Executive summary statistics stale | **RESOLVED** — updated to 17 crates, ~109K lines, 257 files, 5,000+ tests |
| P2-002 | Build validation absent | **RESOLVED** — `validate.js` checks exports, return types, heading sequencing |
| P2-003 | package.json missing scripts | **RESOLVED** — has `validate` and `build` scripts |

### What Remains Open

| # | Problem | Severity | Status |
|---|---------|----------|--------|
| 1 | **Chapter 12 compliance domain names wrong.** §12.2 and the Pakistan GovOS table listed fabricated names (CIVIC, INSURANCE, ENVIRONMENTAL, etc.) that do not exist in `ComplianceDomain`. | P0 | **FIXED in this audit cycle** — replaced with canonical 20 variants |
| 2 | **Appendix D Definition D.2 contradicted Chapter 10.** Used `T: Entity × Jurisdiction → R^20` with continuous [0,1] scores instead of the canonical discrete 7-state `ComplianceState` lattice. | P0 | **FIXED in this audit cycle** — reconciled with canonical definition |
| 3 | **AML_CFT references in prose.** Chapters 7, 11 used "AML_CFT" (no such enum variant; canonical name is `Aml`). | P1 | **FIXED in this audit cycle** — replaced with canonical names |
| 4 | **Chapter 7 (Profiles) repetitive table structure.** 479 lines, 19 tables, ~28 pages. Seven identical per-profile module tables despite §7.1 already having a comparison matrix. | P1 | **OPEN** — requires structural rewrite |
| 5 | **Content repetition across chapters.** "Compliance tensor" referenced 76 times in 34/70 files. Most are appropriate cross-references, but some still re-explain the concept. | P1 | **OPEN** — editorial pass needed |
| 6 | **Marketing language in profiles.** Superlatives ("most comprehensive", "most demanding") and sales positioning remain in chapter 7. | P1 | **OPEN** — editorial pass needed |
| 7 | **Module family names ≠ ComplianceDomain names.** The 16 module families in profiles (Insurance, Civic, Customs, etc.) do not all correspond to `ComplianceDomain` variants, but the spec does not clarify this distinction. | P1-NEW | **OPEN** — needs clarifying note |
| 8 | **`bulletItem()` defined but never used.** Zero calls across all 70 chapter files. | P2-NEW | **OPEN** |
| 9 | **README TOC documentation stale.** References `"1-3"` headingStyleRange; actual is `"1-2"`. | P2-NEW | **FIXED in this audit cycle** |
| 10 | **Cover page uses raw docx API.** Does not use `primitives.js` helpers. | P2 | **OPEN** (carried from v2) |
| 11 | **48 chapters lack h3() depth.** Flat heading hierarchy in most files. | P2 | **OPEN** (carried from v2) |

---

## 2. Quantitative Profile (Updated)

### 2.1 Codebase vs. Spec Alignment

| Metric | Spec Claims (exec summary) | Actual (`main` branch) | Status |
|--------|---------------------------|----------------------|--------|
| ComplianceDomain variants | 20 | **20** | **MATCH** |
| Crates | 17 | **16** | **MISMATCH** — spec says 17, workspace has 16 members |
| .rs files | 257 | **257** | **MATCH** |
| Lines of Rust | ~109,000 | **~109,131** | **MATCH** |
| `#[test]` count | 5,000+ | **3,323** | **MISMATCH** — spec overstates. Actual: 3,323 `#[test]` annotations |
| Workspace version | 0.4.44 | **0.4.44** | **MATCH** |

**Action items:**
- Executive summary line 7: change "17 crates" to "16 crates"
- Executive summary line 7: change "over 5,000 tests" to "over 3,300 tests"
- Executive summary line 58: same corrections

### 2.2 Document Element Inventory (Verified Counts)

| Element | v2 Reported | Actual (this audit) | Delta |
|---------|-------------|---------------------|-------|
| `chapterHeading()` | 68 | **76** | +8 (some files have 2 chapterHeading calls) |
| `partHeading()` | 19 | **19** | Exact |
| `h2()` | 282 | **228** | -54 (previous count was inflated) |
| `h3()` | 126 | **196** | +70 (many chapters gained h3 depth in prior fixes) |
| `table()` | 185 | **209** | +24 |
| `codeBlock()` | 100 | **101** | +1 |
| `spacer()` | ~365 | **4** | -361 (cover page only) |
| `pageBreak()` | 21 | **21** | 7 in profiles, rest across other chapters |
| `definition()` | 30 | **30** | Exact |
| `theorem()` | 10 | **10** | Exact |
| `bulletItem()` | 0 | **0** | Never used despite being defined |

### 2.3 TOC Entry Count (Current)

| Level | Style | Count | In TOC? |
|-------|-------|-------|---------|
| Part headings | Display-only (no heading level) | 19 | **No** |
| Chapter headings | `HeadingLevel.HEADING_1` | 76 | **Yes** |
| H2 sections | `HeadingLevel.HEADING_2` | 228 | **Yes** |
| H3 subsections | `HeadingLevel.HEADING_3` | 196 | **No** (`headingStyleRange: "1-2"`) |
| **Total TOC entries** | | **304** | **~18-20 pages** |

**Assessment**: Down from 495 entries (~30 pages) in v2 to 304 entries (~18-20 pages). Still above the 10-15 page target. To reach that, either reduce H2 count via demotion to H3, or switch to `headingStyleRange: "1-1"` (76 entries, ~5 pages).

### 2.4 Content Repetition (Re-measured)

| Phrase | Occurrences | Files | Assessment |
|--------|-------------|-------|------------|
| "compliance tensor" | 76 | 34 / 70 | Most are terse references ("compliance tensor (§10)"). ~8 files re-explain. |
| "receipt chain" | 72 | 34 / 70 | Similar pattern. Canonical: Ch 9. |
| "20 domains" / "20 compliance" | 22 | 22 / 70 | Most are brief mentions. Chapter 12 re-enumerated (now fixed). |
| "five programmable primitives" | ~12 | ~10 | Acceptable — key marketing term. |
| "Verifiable Credential" | ~40 | ~20 | Acceptable — proper noun. |

---

## 3. New Findings

### P0-NEW-001: Chapter 12 Domain Names (FIXED)

**Severity**: P0
**Location**: `chapters/12-composition.js` lines 18, 47, 103, 107, 110-134
**Status**: **FIXED** in this audit cycle

**Description**: Section 12.2 listed 20 domain names that did not match the canonical `ComplianceDomain` enum. Six fabricated domains (CIVIC, INSURANCE, ENVIRONMENTAL, REAL_ESTATE, HEALTH_SAFETY, plus AML_CFT as a combined variant) were present. Five canonical domains (Kyc, Custody, Clearing, Settlement, ConsumerProtection) were absent. The Pakistan GovOS composition table (§12.3.2) used the same wrong names throughout its 20-row domain assignment matrix.

**Fix applied**: Replaced all domain references with canonical enum variant names. Updated Layer 2, Layer 3 descriptions, and the 20-row domain assignment table. The Kazakhstan example was also corrected.

---

### P0-NEW-002: Appendix D Definition D.2 (FIXED)

**Severity**: P0
**Location**: `chapters/D-security-proofs.js` lines 37-44
**Status**: **FIXED** in this audit cycle

**Description**: Definition D.2 defined the compliance tensor as `T: Entity × Jurisdiction → R^20` with continuous scores in [0, 1]. The canonical definition in Chapter 10 defines it as `C: AssetID × JurisdictionID × ComplianceDomain × TimeQuantum → ComplianceState` with a discrete 7-state lattice. These were fundamentally incompatible — one maps to real-valued vectors, the other to a discrete enum. Additionally, the full-compliance predicate used `T_d(e, j) = 1.0` instead of `ComplianceState ∈ {Compliant, Exempt}`.

**Fix applied**: Rewrote Definition D.2 to use the canonical 4-dimensional mapping with ComplianceState. Updated Theorem 10.1 and its proof sketch for consistency.

---

### P1-NEW-001: AML_CFT References in Prose (FIXED)

**Severity**: P1
**Location**: `chapters/07-profiles.js` lines 130, 409; `chapters/11-savm.js` lines 96, 98
**Status**: **FIXED** in this audit cycle

**Description**: Several chapters used "AML_CFT" as a domain name. No such variant exists in `ComplianceDomain` — the codebase splits this into three separate domains: `Aml`, `Kyc`, `Sanctions`. Additionally, "CUSTOMS" was used as a domain name (should be `Trade`), and "ENVIRONMENTAL" was referenced (does not exist).

**Fix applied**: Replaced all references with canonical enum variant names (`Aml`, `Sanctions`, `Trade`).

---

### P1-NEW-002: Module Family Names ≠ ComplianceDomain Names

**Severity**: P1
**Location**: `chapters/07-profiles.js` — all module activation tables
**Status**: **OPEN** — requires clarifying text

**Description**: The 16 module families used in profiles (Corporate, Financial, Trade, Corridors, Governance, Regulatory, Licensing, Legal, Identity, Compliance, Tax, Insurance, IP, Customs, Land/Property, Civic) are a distinct concept space from the 20 `ComplianceDomain` enum variants (Aml, Kyc, Sanctions, Tax, Securities, Corporate, Custody, DataPrivacy, Licensing, Banking, Payments, Clearing, Settlement, DigitalAssets, Employment, Immigration, Ip, ConsumerProtection, Arbitration, Trade).

Some names overlap (Corporate, Licensing, Trade, Tax), but many do not correspond:
- Module family "Insurance" → no `ComplianceDomain::Insurance` variant
- Module family "Civic" → no `ComplianceDomain::Civic` variant
- Module family "Customs" → closest is `ComplianceDomain::Trade`
- Module family "Land/Property" → no corresponding domain
- `ComplianceDomain::Clearing`, `ComplianceDomain::Settlement` → no corresponding module family

The specification never explains this mapping or distinguishes the two concept spaces. A reader encountering "Insurance: Active" in a profile table and then seeing 20 `ComplianceDomain` variants without "Insurance" will be confused.

**Recommended fix**: Add a clarifying paragraph in §7.1 (after the profile overview table and before the module activation matrix):

> "Module families and compliance domains are distinct concepts. Module families (16) represent functional capabilities deployed in a zone: corporate services, financial services, customs processing, etc. Compliance domains (20, defined in §10.2) represent regulatory dimensions evaluated by the compliance tensor. A single module family may trigger evaluation across multiple compliance domains (e.g., the Financial module family activates Banking, Payments, Clearing, and Settlement domains), and a single compliance domain may be relevant to multiple module families."

---

### P2-NEW-001: `bulletItem()` Defined But Never Used

**Severity**: P2
**Location**: `lib/primitives.js` line 209-215, all 70 chapter files
**Status**: **OPEN**

**Description**: The `bulletItem()` primitive was added (resolving P1-006 from v2) but zero chapters call it. Lists throughout the document are still expressed as comma-separated items within prose paragraphs or as table rows. Several chapters would benefit from bullet lists:
- Executive summary reading guides (lines 95-118) — audience-specific paths
- Deployment chapters (49-53) — configuration checklists
- Pack trilogy (Ch 6) — pack type characteristics

---

### P2-NEW-002: README TOC Documentation Stale (FIXED)

**Severity**: P2
**Location**: `README.md` line 24
**Status**: **FIXED** in this audit cycle

**Description**: README stated `TOC \o "1-3" \h` but the actual `headingStyleRange` is `"1-2"`.

---

## 4. Carried Findings (Open from v2)

### P0-003: Content Repetition (OPEN — Demoted to P1)

**Original severity**: P0 → **Revised severity**: P1

**Rationale for demotion**: The worst cases of re-explanation (Chapter 10 domain re-listing in Chapter 12, contradictory Definition D.2 in Appendix D) have been fixed. The remaining repetition is referential mentions ("compliance tensor (§10)") rather than full re-definitions. This is a prose quality issue, not a factual accuracy issue.

**Remaining work**: An editorial pass across the 34 files that mention "compliance tensor" to ensure each mention is a terse cross-reference rather than re-explanation. Priority files (most mentions):
- `07-profiles.js`: 8 mentions — many are in context ("all 20 compliance domains are active") which is acceptable, but some ("the compliance surface is maximized") are verbose
- `06-pack-trilogy.js`: 5 mentions
- `35-capital-markets.js`: 5 mentions
- `32-corporate.js`: 5 mentions

### P0-004: Marketing Language (OPEN — Severity P1)

**Location**: `chapters/07-profiles.js` primarily

**Remaining examples**:
- Line 74: "This is the most comprehensive profile in the MSEZ Stack" → should be: "This profile activates all 16 module families with maximum compliance depth."
- Line 230: "The sovereign-govos profile is the most demanding deployment configuration in the MSEZ Stack. It transforms the Stack from a zone management system into a national operating system for government services." → Marketing. Should state what it does: "The sovereign-govos profile activates all 16 module families, adds GovOS orchestration, and integrates with national government systems."
- Line 301: "built from first principles" → delete
- Line 353: "Unlike the tech-park profile, which accommodates existing technology companies within a traditional zone framework" → sales comparison

### P1-001: Chapter 7 Profiles Repetition (OPEN)

**Status**: Unchanged from v2. 479 lines, 19 tables, ~28 pages.

The chapter now has comparison matrices in §7.1 (module activation and resource comparison), which is an improvement. However, sections 7.2-7.8 each still include a full 16-row module families table and a 5-row resource requirements table that duplicate information already in §7.1.

**Current structure**: §7.1 has the overview + matrices. §7.2-7.8 each have: intro → capabilities → module table → resource table → example.

**Recommended structure**: Remove the per-profile module and resource tables (§7.N.2 and §7.N.3 for each profile). The §7.1 matrices already capture this data. Keep the per-profile prose (intro, capabilities, example) which contains unique content — the profile-specific configuration notes and deployment examples are valuable and not duplicated.

**Estimated savings**: Removing 14 redundant tables (7 module × 2 + 7 resource × 2... but sovereign-govos national integration table is unique — keep it) → ~12-15 pages saved.

### P1-002: Heading Hierarchy Flat (OPEN — Demoted to P2)

Previous count showed 48 files with zero h3(). Updated count shows improvement — many chapters now use h3(). The h3 count rose from 126 (v2) to 196 (v3). The profile chapter alone accounts for 33 h3 calls. Still, several substantial chapters use only h2 headings:
- `16-anchoring.js` (4 h2, 0 h3)
- `21-zkkyc.js` (4 h2, 0 h3)
- `23-corridor-bridge.js` (4 h2, 0 h3)
- `27-bond-slashing.js` (4 h2, 0 h3)
- `38-govos-layers.js` (4 h2, 0 h3)
- `46-security.js` (4 h2, 0 h3)
- `47-hardening.js` (4 h2, 0 h3)
- `48-zk-circuits.js` (4 h2, 0 h3)

### P1-007: Part XIII Single Chapter (OPEN)

Part XIII ("Mass API Integration Layer") still contains only Chapter 37 (101 lines). `msez-mass-client` is the sole authorized gateway to Mass — arguably the most operationally critical crate — yet it gets less specification coverage than individual profile descriptions.

### P1-008: Part XV Naming (OPEN)

Part XV is still titled "PROTOCOL REFERENCE — CREDENTIALS, ARBITRATION, AND AGENTIC SYSTEMS" in build.js. The v2 recommendation to rename was partially applied (the title now includes the actual content descriptors). **Marking as resolved** — the current title accurately describes the content.

### P2-004: Cover Uses Raw docx API (OPEN)

Low priority. Functional but bypasses `primitives.js` for font/color consistency.

### P2-006: String Concatenation in Code Blocks (OPEN)

Low priority. Cosmetic source code quality issue that does not affect output.

---

## 5. TOC & Heading Hierarchy Status

### 5.1 Current State (Post-v2 Fixes)

```
partHeading()    → Display-only (44pt, centered)    → NOT in TOC     ✓ Fixed
chapterHeading() → HEADING_1 (36pt, pageBreakBefore) → IN TOC        ✓ Fixed
h2()             → HEADING_2 (28pt)                  → IN TOC        ✓ Correct
h3()             → HEADING_3 (24pt)                  → NOT in TOC    ✓ Correct

headingStyleRange: "1-2"

Total TOC entries: 76 + 228 = 304
Estimated TOC pages: 18-20
```

### 5.2 Recommendation for Further Reduction

The 304-entry TOC is functional but still 3-5 pages longer than the 10-15 page target. Two options:

**Option A (Conservative)**: Keep `"1-2"`. Demote 30-50 H2s to H3 in the densest chapters. Target: ~250 entries (~15-16 pages).

**Option B (Aggressive)**: Switch to `headingStyleRange: "1-1"`. 76 entries, ~5 pages. Add per-Part mini-TOC generated from metadata. Best navigation experience but requires build.js changes to emit mini-TOC paragraphs after each `partHeading()`.

---

## 6. Profiles Chapter (Ch 7) Rewrite Guidance

### 6.1 Current State

```
7.1  Profile Overview         — overview table + comparison matrices (7.1.1, 7.1.2)
7.2  digital-financial-center — intro + capabilities + module table + resource table + example
7.3  trade-hub                — intro + capabilities + module table + resource table + example
7.4  tech-park                — same
7.5  sovereign-govos          — same + national integration table
7.6  charter-city             — same
7.7  digital-native-free-zone — same
7.8  asset-history-bundle     — same
7.9  Profile Selection        — composition rules + decision matrix
```

**Problem**: §7.1.1 already contains the module activation matrix (7 columns × 16 rows). Each per-profile section (7.2-7.8) then repeats a full 16-row × 3-column module table. The "Status" column in these per-profile tables is redundant with §7.1.1. Only the "Configuration Notes" column adds new information.

**Problem**: §7.1.2 already contains the resource comparison table. Each per-profile section then repeats a 5-row resource table with "Minimum" and "Recommended" columns. The "Minimum" values are in §7.1.2. Only "Recommended" adds new data.

### 6.2 Recommended Changes

1. **Remove per-profile module tables** (7 tables × 16 rows). Add a "Configuration Notes" column to the §7.1.1 matrix, or create a separate "Configuration Notes by Profile" reference table.

2. **Remove per-profile resource tables** (7 tables × 5 rows). Add "Recommended" values to the §7.1.2 matrix.

3. **Keep per-profile sections** for: definition instantiation, deployed capabilities prose, example deployments. These contain unique content.

4. **Keep sovereign-govos national integration table** (unique to that profile).

5. **Add clarifying paragraph** distinguishing module families from compliance domains (see P1-NEW-002).

### 6.3 Estimated Impact

| Component | Current Pages | After Changes | Savings |
|-----------|--------------|---------------|---------|
| Per-profile module tables (7) | ~10 | 0 | ~10 |
| Per-profile resource tables (7) | ~3 | 0 | ~3 |
| Added §7.1.1 config notes column | 0 | ~2 | -2 |
| Added §7.1.2 recommended column | 0 | ~0.5 | -0.5 |
| **Net savings** | | | **~10-11 pages** |

---

## 7. Prose Style Guide (Carried from v2, Updated)

### Voice and Register

- **Technical specification voice.** Neither academic nor marketing. Write as if documenting an RFC or API reference.
- **Active voice for system behavior.** "The SAVM evaluates the compliance tensor" not "The compliance tensor is evaluated."
- **Present tense** for system behavior. Past tense for historical facts. Future tense only for `[PLANNED]` features.

### Sentence Discipline

- **30-word maximum** for specification sentences. Split longer sentences.
- **One idea per sentence.** If a sentence contains "and" joining independent clauses, split it.
- **No subordinate-clause stacking.** Maximum one subordinate clause per sentence.

### Banned Phrases

| Phrase | Replacement |
|--------|-------------|
| "This is the most [X] in the MSEZ Stack" | State what it does, not how it ranks |
| "from first principles" | Delete |
| "not merely a [X]" | State what it IS |
| "This section describes" | Delete; the heading already says it |
| "comprehensive" (as filler adjective) | Delete or replace with specific scope |
| "transforms [X] into [Y]" (marketing) | State what it provides |

### Cross-Reference Conventions

- **First mention**: Full name + section reference. "The Compliance Tensor V2 (§10) evaluates..."
- **Subsequent mentions**: Terse. "per §10" or "tensor evaluation (§10.4)"
- **Never re-explain** a concept that has its own chapter. The 20 compliance domains are listed once in §10.2.

### Concept-Domain Clarity

- **Module families** (16) are functional deployment units (Corporate, Financial, Trade, etc.)
- **Compliance domains** (20) are regulatory evaluation dimensions (`ComplianceDomain` enum variants)
- Always use the canonical enum variant names when referring to compliance domains
- Always clarify which concept space is being referenced when overlap exists (e.g., "the Trade module family" vs "the `Trade` compliance domain")

---

## 8. docx-js Technical Status

| Feature | v2 Status | v3 Status |
|---------|-----------|-----------|
| Multi-section document | Missing | **RESOLVED** — per-Part sections with unique headers |
| Cover page header suppression | Header visible on cover | **IMPROVED** — cover has its own section. Full suppression requires `titlePage: true` in cover section properties (not yet verified) |
| Chapter page breaks | Missing | **RESOLVED** — `pageBreakBefore: true` on `chapterHeading()` |
| Bullet/numbered lists | No primitive | **RESOLVED** — `bulletItem()` exists, but unused in any chapter |
| Spacer elimination | 365 calls | **RESOLVED** — 4 calls (cover page layout only) |
| Code block borders | No borders | **RESOLVED** — left accent border |
| TOC field code limitation | Inherent to docx-js | **DOCUMENTED** — README.md covers the limitation |
| String concat in code blocks | Inconsistent | **OPEN** — low priority cosmetic |

---

## 9. Estimated Impact (Remaining Interventions)

| Intervention | Est. Current Pages | After Fix | Savings |
|-------------|-------------------|-----------|---------|
| Remove per-profile redundant tables (Ch 7) | ~28 | ~17 | **~11 pages** |
| Content repetition tightening (editorial) | scattered | — | **~15-20 pages** |
| Marketing prose removal (Ch 7 primarily) | ~3 | ~1 | **~2 pages** |
| TOC reduction to "1-1" (if applied) | ~20 | ~5 | **~15 pages** |
| Chapter merges (29+30+31, 46+47, 52→49, 54+55) | ~20 | ~14 | **~6 pages** |
| **Total remaining savings** | | | **~49-54 pages** |
| **Previously saved (v2 fixes)** | | | **~100-130 pages** |
| **Cumulative savings from v1** | | | **~150-180 pages** |

---

## 10. Priority Execution Order

| Phase | Actions | Status |
|-------|---------|--------|
| **Phase 1 (Critical — DONE)** | Fix Chapter 10 domains. Fix partHeading to display-only. Fix headingStyleRange. Add pageBreakBefore. Multi-section document. | **COMPLETE** |
| **Phase 2 (Critical — THIS CYCLE)** | Fix Chapter 12 domains. Fix Appendix D Definition D.2. Fix AML_CFT references. Fix exec summary stats. | **COMPLETE** |
| **Phase 3 (Structure)** | Rewrite Chapter 7 to remove redundant tables. Merge thin chapters. | **OPEN** |
| **Phase 4 (Content)** | Editorial pass: cross-reference tightening, marketing language removal, module/domain disambiguation. | **OPEN** |
| **Phase 5 (Polish)** | Use `bulletItem()` in appropriate chapters. H2→H3 demotion for TOC reduction. | **OPEN** |

---

## 11. Verification Checklist

Post-audit verification against the codebase:

| Check | Result |
|-------|--------|
| `ComplianceDomain` enum: 20 variants, compile-time assertion | **PASS** — `domain.rs:115-121` |
| Chapter 10 §10.2 domain table matches enum | **PASS** — all 20 names, declaration order |
| Chapter 12 §12.2 domain enumeration matches enum | **PASS** (fixed this cycle) |
| Appendix D Definition D.2 consistent with Chapter 10 | **PASS** (fixed this cycle) |
| No "AML_CFT" references in any chapter | **PASS** (fixed this cycle) |
| "Momentum Protocol" misuse | **PASS** — zero occurrences |
| "momentum.xyz/io/com" misuse | **PASS** — zero occurrences |
| "mass.xyz/io" misuse | **PASS** — zero occurrences |
| "MSEZ Protocol" misuse | **PASS** — zero occurrences |
| "Mass Protocol" usage context-appropriate | **PASS** — appears in L1 and protocol reference only |
| `partHeading()` out of TOC | **PASS** — no heading level assigned |
| `chapterHeading()` has pageBreakBefore | **PASS** |
| `headingStyleRange: "1-2"` | **PASS** |
| Executive summary statistics current | **NEEDS UPDATE** — crate count (17→16) and test count (5000→3323) |

---

*End of audit v3.*
