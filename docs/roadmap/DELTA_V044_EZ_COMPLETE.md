# v0.4.44 DELTA: Completing the Economic Zone in a Box

## The Gap Between v0.4.43 and "Clone → Deploy → Economy"

**Strategic Context:** v0.4.43 PHOENIX delivered the execution layer — compliance tensors, ZK proofs, migration protocol, watcher economy, Smart Asset VM. The *engine* is built. What's missing is the *economy* that runs on it: corporate services, identity, capital markets, trade, tax, and the deployment automation to make it one-click.

**Prime Directive for v0.4.44:** Close every gap required so that `git clone` + `mez deploy --profile digital-financial-center` produces a zone with functioning corporate services, banking, payments, custody, arbitration, licensing, identity, and the beginnings of capital markets and trade finance.

---

## DELTA ANALYSIS: What Exists vs What's Missing

### Category A: SHIPPED AND COMPLETE (no work needed)

These module families are production-ready in v0.4.43:

| Family | Modules | Notes |
|--------|---------|-------|
| Legal Foundation | 9 modules, 60+ jurisdictions | Comprehensive AKN XML templates |
| Regulatory Framework | 8 modules | Full FATF/GDPR/OFAC coverage |
| Corridors & Settlement | 7 modules | SWIFT, stablecoin, correspondent, passporting |
| Governance & Civic | 10 modules | 8 voting mechanisms + liquid democracy + ZK |
| Operations & Observability | 9 modules | Audit, telemetry, regulator console, dashboards |
| Execution Layer (PHOENIX) | 10 modules | VM, tensor, ZK, migration, watcher, anchor, bridge |
| Agentic Framework | 5/6 modules | Policy engine, triggers, schedules (missing: MASS 5 primitives) |

**Total shipped: 87 modules / capabilities — 60% of target**

### Category B: CRITICAL GAPS (must close for v0.4.44)

These are the gaps that prevent the system from functioning as a real economy.

---

#### B1. CORPORATE SERVICES MODULE FAMILY (Priority: P0)

**Why critical:** The user explicitly requires "Corporate service provider like functions." Entity-registry exists as a legal registration, but the full CSP lifecycle — from formation through annual compliance to dissolution — is missing.

**Gap items (7 new modules):**

```
modules/corporate/
├── module.yaml                      # Family manifest
├── formation/                       # Entity formation workflows
│   ├── module.yaml
│   ├── templates/
│   │   ├── articles-of-association.yaml
│   │   ├── memorandum-of-association.yaml
│   │   ├── partnership-agreement.yaml
│   │   └── trust-deed.yaml
│   └── workflows/
│       ├── llc-formation.yaml
│       ├── corporation-formation.yaml
│       ├── partnership-formation.yaml
│       └── trust-formation.yaml
├── registered-agent/                # Registered office/agent services
│   ├── module.yaml
│   └── policy/
├── secretarial/                     # Corporate secretarial
│   ├── module.yaml
│   ├── templates/
│   │   ├── board-resolution.yaml
│   │   ├── annual-return.yaml
│   │   ├── minutes-template.yaml
│   │   └── change-of-directors.yaml
│   └── reporting/
├── beneficial-ownership/            # UBO registry
│   ├── module.yaml
│   ├── schemas/
│   │   └── ubo-declaration.schema.json
│   └── policy/
│       └── verification-requirements.yaml
├── governance-templates/            # Corporate governance
│   ├── module.yaml
│   └── templates/
│       ├── shareholder-agreement.yaml
│       ├── director-service-agreement.yaml
│       └── power-of-attorney.yaml
├── annual-compliance/               # Filing calendars and automation
│   ├── module.yaml
│   ├── calendars/
│   └── policy/
│       └── filing-deadlines.yaml
├── dissolution/                     # Winding up procedures
│   ├── module.yaml
│   └── workflows/
│       ├── voluntary-dissolution.yaml
│       └── involuntary-dissolution.yaml
└── cap-table/                       # Share capital management
    ├── module.yaml
    ├── schemas/
    │   └── cap-table.schema.json
    └── templates/
        ├── share-issuance.yaml
        ├── share-transfer.yaml
        └── vesting-schedule.yaml
```

**Estimated effort:** Medium. Primarily YAML/schema definitions following existing module patterns. Some Python tooling for cap table computation and filing calendar automation.

**Dependencies:** Extends `modules/legal/entity-registry`. Requires `modules/licensing/csp` for CSP license binding.

---

#### B2. IDENTITY & CREDENTIALING MODULE FAMILY (Priority: P0)

**Why critical:** Every financial transaction requires identity verification. The spec defines a 4-tier identity system (Tier 0-3). Currently, KYC is embedded in the AML/CFT module, but there's no standalone identity infrastructure.

**Gap items (6 new modules):**

```
modules/identity/
├── module.yaml                      # Family manifest
├── core/                            # DID management and key lifecycle
│   ├── module.yaml
│   ├── schemas/
│   │   ├── did-document.schema.json
│   │   └── key-binding.schema.json
│   └── policy/
│       └── key-rotation.yaml
├── credentials/                     # Verifiable credential issuance
│   ├── module.yaml
│   ├── schemas/
│   │   ├── resident-credential.schema.json
│   │   ├── business-credential.schema.json
│   │   └── professional-credential.schema.json
│   └── templates/
├── kyc-tiers/                       # Progressive KYC (Tier 0-3)
│   ├── module.yaml
│   ├── tier-0-pseudonymous.yaml     # DID only
│   ├── tier-1-basic.yaml            # Name + jurisdiction
│   ├── tier-2-enhanced.yaml         # Address + source of funds
│   └── tier-3-institutional.yaml    # Full due diligence
├── professional-licensing/          # Professional credential management
│   ├── module.yaml
│   └── templates/
│       ├── legal-practitioner.yaml
│       ├── auditor.yaml
│       └── financial-advisor.yaml
├── work-authorization/              # Employment authorization
│   ├── module.yaml
│   └── templates/
└── binding/                         # Entity-identity-instrument linkage
    ├── module.yaml
    └── schemas/
        └── identity-binding.schema.json
```

**Estimated effort:** Medium-High. Requires new Python tooling for credential issuance workflows and DID management. Builds on existing `vc.py`.

**Dependencies:** Core dependency for Corporate Services and Financial Infrastructure.

---

#### B3. TAX & REVENUE MODULE FAMILY (Priority: P0)

**Why critical:** No EZ operates without a fiscal framework — even tax-free zones have fee structures, and international operations require tax treaty awareness.

**Gap items (7 new modules):**

```
modules/tax/
├── module.yaml                      # Family manifest
├── framework/                       # Zone tax regime definition
│   ├── module.yaml
│   ├── templates/
│   │   ├── income-tax.yaml
│   │   ├── vat-gst.yaml
│   │   ├── withholding-tax.yaml
│   │   └── capital-gains.yaml
│   └── policy/
│       └── tax-residency-rules.yaml
├── fee-schedules/                   # Zone operating fees
│   ├── module.yaml
│   ├── schedules/
│   │   ├── license-fees.yaml
│   │   ├── filing-fees.yaml
│   │   ├── annual-fees.yaml
│   │   └── transaction-fees.yaml
│   └── policy/
├── revenue-collection/              # Assessment and collection
│   ├── module.yaml
│   └── workflows/
│       ├── assessment.yaml
│       ├── billing.yaml
│       └── collection.yaml
├── transfer-pricing/                # Arm's-length documentation
│   ├── module.yaml
│   └── policy/
├── treaty-management/               # Double taxation agreements
│   ├── module.yaml
│   └── treaties/
│       └── _template.yaml
├── incentive-programs/              # Tax incentives and holidays
│   ├── module.yaml
│   └── programs/
│       ├── new-establishment.yaml
│       ├── innovation-credit.yaml
│       └── employment-incentive.yaml
└── reporting/                       # Tax reporting automation
    ├── module.yaml
    └── templates/
        ├── crs-reporting.yaml       # Common Reporting Standard
        ├── fatca-reporting.yaml     # FATCA
        └── local-filing.yaml
```

**Estimated effort:** Medium. Primarily declarative YAML definitions. Python tooling for fee computation and CRS/FATCA reporting.

---

#### B4. CAPITAL MARKETS MODULE FAMILY (Priority: P1)

**Why critical:** The user requires "custody, digital and traditional." Capital markets infrastructure (CSD, clearing, DVP) is the backbone of traditional custody.

**Gap items (7 new modules):**

```
modules/capital-markets/
├── module.yaml                      # Family manifest
├── securities-issuance/             # Primary market
│   ├── module.yaml
│   ├── templates/
│   │   ├── equity-issuance.yaml
│   │   ├── debt-issuance.yaml
│   │   └── structured-product.yaml
│   └── workflows/
├── trading-infrastructure/          # Order book specification
│   ├── module.yaml
│   ├── schemas/
│   │   ├── order.schema.json
│   │   ├── trade.schema.json
│   │   └── market-data.schema.json
│   └── policy/
│       └── trading-rules.yaml
├── post-trade/                      # Confirmation, allocation, settlement
│   ├── module.yaml
│   ├── workflows/
│   │   ├── trade-confirmation.yaml
│   │   ├── allocation.yaml
│   │   └── settlement-instruction.yaml
│   └── schemas/
├── csd/                             # Central Securities Depository
│   ├── module.yaml
│   ├── schemas/
│   │   ├── securities-account.schema.json
│   │   └── holding-statement.schema.json
│   └── policy/
│       └── segregation-rules.yaml
├── clearing/                        # CCP clearing
│   ├── module.yaml
│   ├── schemas/
│   │   └── clearing-obligation.schema.json
│   └── policy/
│       ├── margin-requirements.yaml
│       └── default-management.yaml
├── dvp-pvp/                         # Delivery vs Payment
│   ├── module.yaml
│   ├── schemas/
│   │   ├── dvp-instruction.schema.json
│   │   └── pvp-instruction.schema.json
│   └── workflows/
├── corporate-actions/               # Dividends, splits, mergers
│   ├── module.yaml
│   └── templates/
│       ├── dividend.yaml
│       ├── stock-split.yaml
│       ├── rights-issue.yaml
│       └── merger.yaml
└── surveillance/                    # Market monitoring
    ├── module.yaml
    └── policy/
        ├── unusual-activity.yaml
        └── circuit-breakers.yaml
```

**Estimated effort:** High. Requires significant schema design and Python tooling. DVP/PVP needs integration with settlement-adapter.

---

#### B5. TRADE & COMMERCE MODULE FAMILY (Priority: P1)

**Why critical:** EZs exist primarily to facilitate trade. Trade playbook schemas exist but aren't formalized as modules.

**Gap items (5 new modules, upgrading 3 partials):**

```
modules/trade/
├── module.yaml
├── letters-of-credit/              # Documentary and standby LC
│   ├── module.yaml
│   ├── schemas/
│   │   ├── lc-application.schema.json
│   │   ├── lc-issuance.schema.json
│   │   └── lc-amendment.schema.json
│   └── workflows/
│       ├── documentary-lc.yaml
│       └── standby-lc.yaml
├── trade-documents/                 # Bills of lading, invoices, certificates
│   ├── module.yaml
│   ├── schemas/
│   │   ├── bill-of-lading.schema.json
│   │   ├── commercial-invoice.schema.json
│   │   ├── packing-list.schema.json
│   │   └── certificate-of-origin.schema.json
│   └── templates/
├── supply-chain-finance/            # Reverse factoring, dynamic discounting
│   ├── module.yaml
│   └── workflows/
├── customs/                         # Tariffs and customs procedures
│   ├── module.yaml
│   ├── tariff-schedules/
│   └── workflows/
│       ├── import-declaration.yaml
│       └── export-declaration.yaml
└── trade-insurance/                 # Trade credit insurance
    ├── module.yaml
    └── policy/
```

---

#### B6. DEPLOYMENT AUTOMATION (Priority: P0)

**Why critical:** The user explicitly requires "as easy as spinning up new AWS instances." Profiles and manifests exist, but there's no actual IaC to deploy them.

**Gap items (6 new capabilities):**

```
deploy/
├── docker/
│   ├── Dockerfile.zone-authority     # Zone authority service
│   ├── Dockerfile.corridor-node      # Corridor node service
│   ├── Dockerfile.watcher            # Watcher service
│   ├── Dockerfile.regulator-console  # Regulator UI
│   └── docker-compose.yaml           # Local development stack
├── kubernetes/
│   ├── helm/
│   │   └── mez-zone/
│   │       ├── Chart.yaml
│   │       ├── values.yaml
│   │       └── templates/
│   └── kustomize/
│       ├── base/
│       └── overlays/
│           ├── dev/
│           ├── staging/
│           └── production/
├── terraform/
│   ├── modules/
│   │   ├── vpc/
│   │   ├── eks/
│   │   ├── rds/
│   │   └── monitoring/
│   ├── environments/
│   │   ├── dev/
│   │   ├── staging/
│   │   └── production/
│   └── main.tf
├── scripts/
│   ├── deploy-zone.sh               # One-command zone deployment
│   ├── provision-corridor.sh         # Corridor setup
│   └── onboard-entity.sh            # Entity onboarding workflow
└── monitoring/
    ├── prometheus/
    │   └── alerts.yaml
    ├── grafana/
    │   └── dashboards/
    └── logging/
        └── fluentd.conf
```

---

#### B7. MASS FIVE PRIMITIVES (Priority: P1)

**Why critical:** Spec defines Entities, Ownership, Financial Instruments, Identity, and Consent as the five programmable primitives. Only `mass_primitives.py` exists with basic MASS operations, not the full five-primitive model.

**Gap items (5 new tools):**

```
tools/
├── mass_entities.py               # Entity formation, governance, migration
├── mass_ownership.py              # Direct, beneficial, fractional, conditional ownership
├── mass_instruments.py            # Equity, debt, derivatives, structured products
├── mass_identity.py               # Tier 0-3 identity with progressive verification
└── mass_consent.py                # Transaction, governance, delegation, regulatory consent
```

---

#### B8. ADDITIONAL LICENSING MODULES (Priority: P1)

**Gap items:**

```
modules/licensing/
├── insurance/                     # Insurance carrier/broker/captive
│   ├── module.yaml
│   ├── forms/
│   ├── policy/
│   └── reporting/
├── professional-services/         # Legal, accounting, audit
│   ├── module.yaml
│   ├── forms/
│   └── policy/
├── trade-license/                 # General commercial activity
│   ├── module.yaml
│   └── forms/
├── import-export/                 # Trade licensing
│   ├── module.yaml
│   └── forms/
└── sandbox/                       # Regulatory sandbox
    ├── module.yaml
    ├── policy/
    │   ├── eligibility-criteria.yaml
    │   ├── graduated-requirements.yaml
    │   └── exit-criteria.yaml
    └── reporting/
```

---

#### B9. ADDITIONAL FINANCIAL MODULES (Priority: P1)

**Gap items:**

```
modules/financial/
├── lending/                       # Loan origination and servicing
│   ├── module.yaml
│   ├── schemas/
│   │   ├── loan-application.schema.json
│   │   ├── credit-assessment.schema.json
│   │   └── loan-agreement.schema.json
│   └── workflows/
├── deposit-insurance/             # Depositor protection scheme
│   ├── module.yaml
│   └── policy/
└── rtgs/                          # Real-time gross settlement
    ├── module.yaml
    ├── schemas/
    └── spec/
```

---

#### B10. ARBITRATION EXPANSION (Priority: P2)

**Gap items:**

```
modules/legal/dispute-resolution/
├── small-claims/                  # Fast-track low-value disputes
│   └── policy/
├── mediation/                     # Pre-arbitration mediation
│   └── workflows/
└── expert-determination/          # Technical disputes
    └── policy/
```

---

## v0.4.44 RELEASE PLAN

### Release Codename: GENESIS
*"The zone is born."*

### Priority Tiers

#### Tier 0 — Must Ship (v0.4.44-alpha)
These close the critical gap between "infrastructure" and "economy":

| # | Deliverable | New Modules | New Lines (est.) |
|---|------------|-------------|-----------------|
| 1 | Corporate Services family | 8 modules | ~2,000 YAML + ~800 Python |
| 2 | Identity & Credentialing family | 6 modules | ~1,500 YAML + ~1,200 Python |
| 3 | Tax & Revenue family | 7 modules | ~2,500 YAML + ~600 Python |
| 4 | Deployment automation (Docker + compose) | 5 Dockerfiles + compose | ~500 Docker + ~800 scripts |
| 5 | Zone provisioning CLI (`mez zone create`) | 1 command | ~600 Python |

**Tier 0 total: ~26 new modules/files, ~10,000 new lines**

#### Tier 1 — Should Ship (v0.4.44-beta)
These complete the financial center story:

| # | Deliverable | New Modules | New Lines (est.) |
|---|------------|-------------|-----------------|
| 6 | Capital Markets family | 8 modules | ~3,000 YAML + ~1,500 Python |
| 7 | Trade & Commerce family | 5 modules | ~2,000 YAML + ~800 Python |
| 8 | MASS Five Primitives | 5 Python tools | ~5,000 Python |
| 9 | Additional licensing (insurance, professional, sandbox) | 5 modules | ~1,500 YAML |
| 10 | Additional financial (lending, deposit insurance, RTGS) | 3 modules | ~1,000 YAML + ~500 Python |

**Tier 1 total: ~26 new modules/files, ~15,300 new lines**

#### Tier 2 — Nice to Have (v0.4.44-rc)
These polish the complete picture:

| # | Deliverable | New Modules | New Lines (est.) |
|---|------------|-------------|-----------------|
| 11 | Kubernetes manifests + Helm charts | K8s configs | ~2,000 YAML |
| 12 | Terraform IaC (AWS) | TF modules | ~1,500 HCL |
| 13 | Monitoring stack (Prometheus + Grafana) | Dashboards + alerts | ~800 YAML |
| 14 | Arbitration expansion (small claims, mediation) | 3 sub-modules | ~500 YAML |
| 15 | Production ZK backend stubs (Gnark prep) | Integration points | ~1,000 Python |

**Tier 2 total: ~5,800 new lines**

---

### Module Count Projection

| Version | Shipped | Partial | Missing | Coverage |
|---------|---------|---------|---------|----------|
| v0.4.43 | 87 | 11 | 48 | 60% |
| v0.4.44-alpha (Tier 0) | 108 | 9 | 29 | 74% |
| v0.4.44-beta (Tier 1) | 134 | 4 | 8 | 92% |
| v0.4.44-rc (Tier 2) | 142 | 2 | 2 | 97% |

---

### Updated Slide 09 After v0.4.44

If all Tier 0 + Tier 1 ship, the slide becomes:

```
SLIDE 09 — What You Can Do for a Zone

┌─────────────────────────────────────────────────────────────┐
│                 SPECIAL ECONOMIC ZONE IN A BOX              │
│              git clone → mez deploy → economy              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  LEGAL           CORPORATE        REGULATORY                │
│  ─────           ──────────       ──────────                │
│  Enabling act    Formation        AML/CFT                   │
│  Charter         Reg. agent       Sanctions                 │
│  Civil code      Secretarial      Data protection           │
│  Commercial      UBO registry     Cybersecurity             │
│  Dispute res.    Governance       Market conduct            │
│  Security int.   Compliance       Consumer prot.            │
│                  Cap table         Fin. supervision          │
│                  Dissolution       Anti-corruption           │
│                                                             │
│  LICENSING       IDENTITY         TAX & REVENUE             │
│  ─────────       ────────         ─────────────             │
│  CSP, EMI        Digital ID       Tax framework             │
│  CASP, custody   Progressive KYC  Fee schedules             │
│  Exchange        Credentials      Revenue collection        │
│  Token issuer    Professional     Transfer pricing          │
│  Bank sponsor    Work permits     Treaty mgmt               │
│  PSP, cards      Entity binding   Incentive programs        │
│  Insurance                        CRS/FATCA                 │
│  Sandbox                                                    │
│                                                             │
│  FINANCIAL       CAPITAL MKTS     TRADE                     │
│  ─────────       ────────────     ─────                     │
│  Banking         Issuance         Letters of credit         │
│  Payments        Trading infra    Trade documents           │
│  Settlement      Post-trade       Supply chain fin.         │
│  Treasury        CSD/clearing     Customs/tariffs           │
│  FX, cards       DVP/PVP          Trade insurance           │
│  Safeguarding    Corp. actions                              │
│  Open banking    Surveillance                               │
│  Lending                                                    │
│                                                             │
│  CORRIDORS       GOVERNANCE       ARBITRATION               │
│  ─────────       ──────────       ───────────               │
│  Correspondent   8 voting types   DIFC-LCIA, SIAC          │
│  SWIFT 20022     Liquid democracy ICC, AIFC-IAC             │
│  Stablecoin      ZK participation Small claims              │
│  Open banking    Constitutional   Mediation                 │
│  Passporting     framework        Enforcement               │
│  Multi-hop                                                  │
│                                                             │
│  EXECUTION       AGENTIC          OPERATIONS                │
│  ─────────       ───────          ──────────                │
│  Smart Asset VM  Policy engine    Audit logging             │
│  Compliance      20 trigger types Regulator console         │
│  tensor          MASS 5           Telemetry                 │
│  ZK proofs       primitives       Transparency              │
│  Migration       Scheduling       Incident response         │
│  Watcher econ.   Env. monitors    A/B testing               │
│  L1 anchoring                                               │
│                                                             │
│  DEPLOYMENT                                                 │
│  ──────────                                                 │
│  6 profiles │ Docker │ K8s │ Terraform │ One-click deploy   │
│                                                             │
└─────────────────────────────────────────────────────────────┘

146 modules │ 16 families │ 60+ jurisdictions │ 5 corridor types
```

---

## CRITICAL PATH DEPENDENCIES

```
Identity ──→ Corporate Services ──→ Licensing (insurance, professional)
   │              │
   │              ├──→ Cap Table ──→ Capital Markets (securities issuance)
   │              │
   ▼              ▼
Tax & Revenue    Financial (lending, deposit insurance)
   │
   ▼
Trade & Commerce (customs requires tax; LC requires banking + identity)
   │
   ▼
Deployment Automation (needs all modules to containerize)
```

**Start with Identity → Corporate Services in parallel with Tax & Revenue.**
**Then Capital Markets + Trade in parallel.**
**Deployment automation last (once services are defined).**

---

## WHAT v0.4.44 DOES NOT NEED

To keep scope manageable, the following are explicitly **out of scope** for v0.4.44:

1. **Employment & Labor law** — zones typically use host country labor law
2. **Environmental/ESG** — important but not blocking economic function
3. **Real estate beyond land-registry** — building permits are physical-zone specific
4. **Intellectual property** — can be added as overlay, not core economy
5. **Full MASS L1 consensus** — execution layer (PHOENIX) already works without it
6. **Production ZK backends** — mock implementations are sufficient for v0.4.44
7. **Production L1 anchoring** — mock adapters continue to work

These are v0.4.45+ items.

---

## SUCCESS CRITERIA FOR v0.4.44

### Functional
- [ ] A new entity can be formed in the zone via `mez entity create`
- [ ] Entity receives DID and Tier-1 KYC credential
- [ ] Entity can apply for and receive a license (any of 16 types)
- [ ] Entity can open a bank account and make a payment
- [ ] Entity can custody assets (digital and traditional via CSD)
- [ ] Entity can issue a security and trade it
- [ ] Entity can file a dispute and receive a ruling
- [ ] Entity can compute its tax obligations and pay fees
- [ ] All of the above produces a verifiable audit trail

### Deployment
- [ ] `docker compose up` starts a local zone with all services
- [ ] `mez zone create --profile digital-financial-center` generates a deployable zone repo
- [ ] Zone connects to MASS network via corridor configuration

### Coverage
- [ ] Module count: 134+ (from 87)
- [ ] Module family count: 16 (from 8 on slide, 8 in repo)
- [ ] All 146 items from Slide 09 taxonomy have at least a module.yaml
- [ ] Test coverage maintained at 90%+ for new Python code

---

*Document Version: 1.0*
*Date: February 2026*
*Classification: Internal - Strategic*
