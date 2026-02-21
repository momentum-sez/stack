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
| **P0 — Must fix** | 3 | 1 RESOLVED (P0-001 stubs expanded), 2 open |
| **P1 — Should fix** | 5 | 2 RESOLVED (P1-003 tables added, P1-004 numbers fixed), 3 open |
| **P2 — Nice to fix** | 4 | All open |

**Additional fixes applied 2026-02-19:**
- All codebase statistics updated: 164K lines, 4,683 tests, 323 modules, 210 zones, 322 source files, 17 crates
- ComplianceDomain names corrected across chapters 07, 10, 11, 12 to match mez-core enum (20 real variants)
- ComplianceState lattice corrected: 5 states (not 7) matching mez-tensor implementation
- PHOENIX codename removed from Chapter 02 heading
- Version history appendix retains PHOENIX as historical codename for v0.4.43

---

## 2. Quantitative Profile

### 2.1 Codebase Metrics

| Metric | Value |
|--------|-------|
| Total chapter files | 70 |
| Total chapter lines | ~8,600 |
| Library + build lines | 423 |
| **Total project lines** | **~9,025** |

### 2.2 Accuracy Corrections Applied (2026-02-19)

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

### P0-002: Content Repetition — OPEN

**Severity**: P0
**Description**: Core concepts are re-explained in every chapter they appear. The compliance tensor is described 75 times across 34 files. Recommended: use terse cross-references ("See §10.2") instead of re-explanation.

---

### P0-003: Marketing Language — OPEN

**Severity**: P0
**Description**: Executive summary and ~15 chapter introductions contain whitepaper-style prose rather than specification content. Recommended: strip throat-clearing, lead with definitions and tables.

---

### P1-001: Heading Hierarchy (h3 Under-Utilized) — OPEN

48 of 70 files use zero `h3()` calls. Recommended: add h3 hierarchy to improve navigation.

### P1-002: Chapter 07 Profiles Repetition — OPEN

466-line chapter with 7 near-identical profile templates. Recommended: add comparison matrix.

### P1-003: Wall-of-Text Prose — **RESOLVED**

Formerly thin chapters now use tables and structured content (see P0-001 resolution above).

### P1-004: Statistics Accuracy — **RESOLVED**

All hardcoded statistics updated to match actual codebase measurements (2026-02-21):
- 164K lines (was 151K)
- 4,683 tests (was 4,073)
- 323 modules (was 298)
- 322 source files (was 154)
- 210 zone definitions (was 100)
- 17 crates (was 16)

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

All checks pass per CLAUDE.md naming conventions. "PHOENIX" codename appears only in version history appendix (historical context for v0.4.43), not as a current brand name.

---

## 5. Domain Accuracy (added 2026-02-19)

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

---

*End of audit.*
