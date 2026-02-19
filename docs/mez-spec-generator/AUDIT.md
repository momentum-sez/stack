# Audit Report: `mez-spec-generator`

**Date**: 2026-02-19 (updated)
**Scope**: `docs/mez-spec-generator/` — the Node.js/docx-js pipeline that generates the MEZ Stack v0.4.44 GENESIS specification document
**Auditor**: Architecture review per CLAUDE.md mandate

---

## 1. Executive Summary

The `mez-spec-generator` is a ~9,100-line Node.js project (70 chapter files, 3 library files, 1 build script) that produces a Word document via `docx` (docx-js). It is well-structured: a clean `build.js` orchestrator, a small primitive library (`lib/primitives.js`, 202 lines) providing composable document elements, consistent styling in `lib/styles.js` and `lib/constants.js`, and 70 chapter files following a uniform pattern.

**Findings status (updated 2026-02-19):**

| Severity | Count | Status |
|----------|-------|--------|
| **P0 — Must fix** | 3 | 1 RESOLVED (P0-001 stubs expanded), P0-002 partially resolved, P0-003 RESOLVED |
| **P1 — Should fix** | 5 | 2 RESOLVED (P1-003 tables, P1-004 numbers), 3 open |
| **P2 — Nice to fix** | 4 | All open |

---

## 2. Quantitative Profile

### 2.1 Codebase Metrics

| Metric | Value |
|--------|-------|
| Total chapter files | 70 |
| Total chapter lines | ~8,600 |
| Library + build lines | 423 |
| **Total project lines** | **~9,025** |

### 2.2 Accuracy Corrections — Pass 1 (2026-02-19)

| Stale Value | Corrected Value | Files Changed |
|-------------|-----------------|---------------|
| ~74,000 lines | 151,000 lines | 00-executive-summary, I-module-directory |
| 3,800+ tests | 4,073 tests | 00-executive-summary, B-test-coverage, 56-current-network |
| 298 modules | 323 modules | 00-executive-summary, 02-architecture, I-module-directory, 56-current-network |
| 136 source files | 154 source files | 00-executive-summary, I-module-directory |
| 107 test files | 113 test files | B-test-coverage, I-module-directory |
| Wrong ComplianceDomain names | Real 20 variants from mez-core | 10-compliance-tensor, 12-composition, 07-profiles, 11-savm |
| 7-state ComplianceState lattice | 5-state lattice (Compliant, NonCompliant, Pending, Exempt, NotApplicable) | 10-compliance-tensor |
| "PHOENIX Module Suite" heading | "Rust Crate Map" | 02-architecture |

### 2.3 Factual, Consistency, and Quality Corrections — Pass 2 (2026-02-19)

~50 targeted edits across ~20 files. Organized by category:

#### Factual Errors Fixed

| Error | Correction | File(s) |
|-------|------------|---------|
| "DIFC-LCIA Arbitration Centre" | "DIFC Courts (Arbitration Division)" — DIFC-LCIA dissolved 2021 | 44-arbitration |
| ITO Section 149/148 conflation | Section 148 only (149 is salary withholding, not imports) | 40-tax-pipeline |
| "sections 148–236Y" | "sections 148–236P" — Section 236Y does not exist in Pakistan ITO | 40-tax-pipeline |
| "tax year 2025" | "tax year 2024–25 (Pakistan fiscal year July–June)" | 40-tax-pipeline |
| "within 4 hours" SRO SLA claim | Removed — no verifiable SLA commitment exists | 40-tax-pipeline |
| Ed25519 circuit constraints "~6,000" | "~50,000–200,000" — requires non-native field arithmetic in BN254 | 46-security |
| Total πpriv circuit "~34,000" | "~78,000–228,000 R1CS constraints" | 46-security |
| BN254 security "~126 bits" | "~128 bits" | 46-security |
| Tax-to-GDP "9.2%" | "10.3% (FY2024-25 estimated)" | 39-sovereign-ai |
| GPU specs "A100 80GB SXM4" | "H100 80GB SXM5" / "H100 PCIe" | 39-sovereign-ai |
| "Kazakhstan (Alatau)" | "Kazakhstan (AIFC, Astana)" — AIFC is in Astana, not Alatau district | G-jurisdiction-templates |
| PRISM "SWIFT FIN / ISO 15022" | "Proprietary messaging (SBP format), migrating to ISO 20022" | 38-govos-layers |
| NADRA "e-Sahulat, SOAP/XML" | "VERISYS, REST/JSON API" | 38-govos-layers |
| ZK backends "Implemented" | "Feature-gated" (both Groth16 and Plonk are behind feature flags) | 03-crypto-primitives |
| `CanonicalBytes` in EvidencePackage struct | `ContentDigest` (CanonicalBytes is computation input, not output) | 44-arbitration |

#### Internal Consistency Fixes

| Issue | Fix | File(s) |
|-------|-----|---------|
| Fabricated "P1-009" defect reference | Removed — no such finding exists in CLAUDE.md audit | 34-tax |
| "mez-agentic" (non-existent crate) | "the agentic framework (§45)" | 34-tax |
| Trigger taxonomy overclaim ("full taxonomy") | "representative Smart Asset triggers" + cross-ref to §45 | 08-smart-asset |
| "self-healing behavior" | "automated recovery behavior" | 08-smart-asset |
| "Production-ready milestone" | "Functionally deployable milestone" (CLAUDE.md: Phase 3 still blocked) | A-version-history |
| BBS+/ZK described as current capabilities | Rewritten as "Phase 4, §3.6/§3.7 will enable..." (both are stubs) | 01-mission-vision |
| Rust overclaim "Bugs that survive the compiler are bugs in the specification" | "The type system catches entire classes of defects at compile time" | 01-mission-vision |

#### Dev Environment Leaks Fixed (Heroku)

| Leak | Replacement | File(s) |
|------|-------------|---------|
| "investment-info (Heroku seed)" | "investment-info.api.mass.inc" | 01-mission-vision, 37-mass-bridge |
| "investment-info (Heroku)" | "investment-info.api.mass.inc" | 32-corporate (2 occurrences) |
| "via Heroku" in Templating Engine | Removed | 01-mission-vision |
| herokuapp.com URLs in API reference | Canonical .api.mass.inc domains | F-api-endpoints |

#### Marketing / Whitepaper Language Removed

| Issue | Fix | File(s) |
|-------|-----|---------|
| Cover page "CONFIDENTIAL" | "BUSL-1.1" — contradicts "Open Source" on same page | 00-cover |
| "flagship" GovOS deployment | "reference" | G-jurisdiction-templates |
| GovOS opening marketing paragraph | Replaced with concise technical description | 38-govos-layers |
| Sovereign AI marketing opening | Replaced with technical description | 39-sovereign-ai |
| "not as an afterthought" / "not months after the fact" | Removed marketing phrasing | 40-tax-pipeline |
| "Mass enhances, never replaces" | Neutral phrasing | 38-govos-layers |
| Internal architecture criticism in Docker chapter | Removed "This replaces the prior nine-service Python layout..." | 50-docker |
| "CLAUDE.md" reference in published spec | Removed internal artifact reference | 00-executive-summary |
| Volatile dollar estimates ($0.50, $1.00) for anchor costs | Removed — cryptocurrency prices fluctuate | 16-anchoring |
| Undefined "TLC finality" acronym | Removed | 16-anchoring |

#### Scope / Framing Clarifications Added

| Chapter | Clarification |
|---------|---------------|
| 41-sovereignty | Added "This chapter describes a reference handover framework." |
| 54-adoption | Added "This chapter describes the target market analysis informing the system's design." |
| 39-sovereign-ai | Added fiscal year qualifier; changed "identify 30-40% of tax gap" to "identify revenue gaps via cross-referencing" |

---

## 3. Findings

### P0-001: Stub Chapters — **RESOLVED**

**Status**: RESOLVED (2026-02-18). All six formerly stub chapters have been expanded with tables, code blocks, and proper h3 subsections:

| Chapter | Before | After | Content Added |
|---------|--------|-------|---------------|
| 47-hardening | 39 lines, prose | 73 lines | Validation framework table, thread safety table, crypto hardening table, Rust guarantees table |
| 49-deployment | stub | 88 lines | Infrastructure requirements, deployment profiles, binary deployment, topology table, scaling guidelines |
| 50-docker | stub | 82 lines | Service architecture table, credential security, database schema table, Dockerfile code block |
| 51-terraform | 18 lines | 97 lines | Core infrastructure table, VPC code block, database sizing table, K8s resources table, node group table |
| 52-one-click | 37 lines | 73 lines | Deployment pipeline code block, step details table, deployment targets table, AWS pipeline table |
| 53-operations | 22 lines | 75 lines | Metric families table, alert rules table, incident response table, change management table |

---

### P0-002: Content Repetition — PARTIALLY RESOLVED

**Severity**: P0
**Description**: Core concepts are re-explained in every chapter they appear. The compliance tensor is described 75 times across 34 files. Recommended: use terse cross-references ("See §10.2") instead of re-explanation.
**Status**: Partially addressed — cross-references added where feasible (e.g., trigger taxonomy in ch08 now references §45), but systematic deduplication across all 34 files remains.

---

### P0-003: Marketing Language — **RESOLVED**

**Severity**: P0
**Status**: RESOLVED (2026-02-19). Marketing language removed or replaced with technical prose across ~12 files. Key changes:
- Cover page: CONFIDENTIAL → BUSL-1.1
- GovOS chapters (38, 39, 41): marketing openings replaced with technical descriptions
- Tax pipeline (40): subjective claims removed
- Docker (50): internal criticism removed
- Adoption (54): framing clarification added
- Anchoring (16): volatile price estimates removed
- Architecture (01): overclaims corrected to factual statements

---

### P1-001: Heading Hierarchy (h3 Under-Utilized) — OPEN

48 of 70 files use zero `h3()` calls. Recommended: add h3 hierarchy to improve navigation.

### P1-002: Chapter 07 Profiles Repetition — OPEN

466-line chapter with 7 near-identical profile templates. Recommended: add comparison matrix.

### P1-003: Wall-of-Text Prose — **RESOLVED**

Formerly thin chapters now use tables and structured content (see P0-001 resolution above).

### P1-004: Statistics Accuracy — **RESOLVED**

All hardcoded statistics updated to match actual codebase measurements (2026-02-19):
- 151K lines (was ~74K)
- 4,073 tests (was 3,800+)
- 323 modules (was 298)
- 154 source files (was 136)
- 113 integration test files (was 107)

### P1-005: Part XV Naming — OPEN

Part XIII ("Mass API Integration") and Part XV both reference "Mass" — minor naming collision.

---

### P2 Findings (all OPEN)

| ID | Finding | Status |
|----|---------|--------|
| P2-001 | TOC renders client-side only | Open |
| P2-002 | Single-section document architecture | Open |
| P2-003 | spacer() over-reliance | Open |
| P2-004 | No build validation | Open |

---

## 4. Branding and Naming Compliance

All checks pass per CLAUDE.md naming conventions. "PHOENIX" codename appears only in version history appendix (historical context for v0.4.43), not as a current brand name. All Heroku development URLs have been replaced with canonical `.api.mass.inc` domains.

---

## 5. Domain Accuracy

### ComplianceDomain Alignment

The spec generator now uses the canonical 20 ComplianceDomain variants from `mez-core/src/domain.rs`:

```
Aml, Kyc, Sanctions, Tax, Securities, Corporate, Custody, DataPrivacy,
Licensing, Banking, Payments, Clearing, Settlement, DigitalAssets,
Employment, Immigration, Ip, ConsumerProtection, Arbitration, Trade
```

Previously, the spec used fabricated domain names (CIVIC, COMMERCIAL, FINANCIAL, AML_CFT, DATA_PROTECTION, INSURANCE, ENVIRONMENTAL, LABOR, INTELLECTUAL_PROPERTY, REAL_ESTATE, HEALTH_SAFETY) that did not match any code. These have been corrected in chapters 07, 10, 11, 12.

### ComplianceState Alignment

The spec now documents the implemented 5-state lattice:

```
NonCompliant(0) < Pending(1) < {Compliant, Exempt, NotApplicable}(2)
```

Previously documented a speculative 7-state lattice (adding Unknown, Expired, Suspended) that was never implemented.

### Regulatory Accuracy

Verified against source material in this audit pass:
- **Pakistan ITO**: Section numbers (148, not 149; 236P, not 236Y), fiscal year format
- **NADRA**: VERISYS REST/JSON API (not e-Sahulat SOAP/XML)
- **SBP PRISM**: Proprietary SBP format (not SWIFT FIN/ISO 15022)
- **DIFC**: DIFC Courts Arbitration Division (DIFC-LCIA dissolved 2021)
- **AIFC**: Located in Astana (not Alatau district of Almaty)
- **FATF regional bodies**: EAG for Kazakhstan, ESAAMLG for Seychelles (corrected in appendix G)
- **ZK circuit constraints**: Ed25519 in BN254 SNARK requires ~50K-200K constraints (not ~6K)
- **BN254 security**: ~128 bits (not ~126 bits)

---

## 6. Files Modified Summary

### Pass 1 (10 files — commit `64ae994`)

00-executive-summary, 02-architecture, 07-profiles, 10-compliance-tensor, 11-savm, 12-composition, 56-current-network, B-test-coverage, I-module-directory, AUDIT.md

### Pass 2 (~20 files — this session)

00-cover, 00-executive-summary, 01-mission-vision, 03-crypto-primitives, 08-smart-asset, 16-anchoring, 32-corporate, 34-tax, 37-mass-bridge, 38-govos-layers, 39-sovereign-ai, 40-tax-pipeline, 41-sovereignty, 44-arbitration, 46-security, 50-docker, 54-adoption, A-version-history, F-api-endpoints, G-jurisdiction-templates, AUDIT.md

---

*End of audit.*
