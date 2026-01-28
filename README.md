<div align="center">

# 🏛️ MSEZ Stack

## The Open Standard for Programmable Jurisdictions

**v0.4.42 "Agentic Ascension"**

[![Tests](https://img.shields.io/badge/tests-395%20passing-brightgreen?style=flat-square)]()
[![Schemas](https://img.shields.io/badge/JSON%20schemas-110-blue?style=flat-square)]()
[![Python](https://img.shields.io/badge/python-3.10%2B-blue?style=flat-square)]()
[![Spec](https://img.shields.io/badge/MASS%20Protocol-v0.2-purple?style=flat-square)]()

---

**The infrastructure layer for trillion-dollar cross-border asset mobility.**

Smart Assets · Programmable Compliance · Autonomous Corridors · Cryptographic Auditability

[**Get Started →**](#-quick-start) · [Architecture](#-architecture) · [Examples](#-examples) · [Specification](#-specification)

</div>

---

## 💡 The Problem We Solve

Today's global financial infrastructure was designed for a world of paper, fax machines, and bilateral trust relationships. The result:

| Pain Point | Current Reality | MSEZ Solution |
|------------|-----------------|---------------|
| **Cross-border compliance** | 3-5 days for AML/KYC checks | Real-time programmatic verification |
| **Regulatory fragmentation** | 195+ jurisdictions, incompatible rules | Modular, composable compliance stacks |
| **Settlement finality** | T+2 to T+5 with reconciliation nightmares | Cryptographic receipt chains with instant finality |
| **Audit trails** | Scattered across siloed systems | Unified, tamper-evident, machine-readable |

MSEZ provides the missing layer: **jurisdiction-as-code** infrastructure that lets assets carry their compliance state across borders.

---

## 🎯 What is MSEZ?

MSEZ (Momentum Special Economic Zone) Stack is an **open specification and reference implementation** for building programmable Special Economic Zones—modular, forkable jurisdiction nodes that enable:

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                                                                                 │
│     ASSETS      ──────────►     CORRIDORS     ──────────►     SETTLEMENT       │
│                                                                                 │
│  ┌───────────┐              ┌───────────────┐              ┌───────────────┐   │
│  │  Smart    │              │  Programmable │              │  Cryptographic│   │
│  │  Assets   │──────────────│  Compliance   │──────────────│  Finality     │   │
│  │  (G,R,M,  │              │  Corridors    │              │  Settlement   │   │
│  │   C,H)    │              │               │              │  Anchors      │   │
│  └───────────┘              └───────────────┘              └───────────────┘   │
│       │                            │                              │            │
│       │         ┌──────────────────┼──────────────────┐          │            │
│       │         │                  │                  │          │            │
│       ▼         ▼                  ▼                  ▼          ▼            │
│  ┌─────────────────────────────────────────────────────────────────────┐      │
│  │                      MSEZ Zone (Profile)                            │      │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐   │      │
│  │  │ LawPack  │  │ RegPack  │  │ Modules  │  │  Trust Anchors   │   │      │
│  │  │ (Legal)  │  │(Complian)│  │(Financial│  │  (DIDs/Certs)    │   │      │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────────────┘   │      │
│  └─────────────────────────────────────────────────────────────────────┘      │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

> ⚠️ **Not legal advice.** This repository contains technical specifications and reference implementations. Production deployments require local legal review, political authorization, and licensed operators.

---

## 🚀 Quick Start

### Prerequisites

```
Python 3.10+  ·  pip  ·  50MB disk space
```

### Installation (60 seconds)

```bash
# Clone
git clone https://github.com/momentum-xyz/msez-stack.git
cd msez-stack

# Install
pip install -r tools/requirements.txt

# Verify (395 tests should pass)
PYTHONPATH=. pytest tests/ -q --tb=no

# Expected output:
# 395 passed, 6 skipped in ~30s
```

### Hello World: Your First Zone

```bash
# 1. Validate a pre-built profile
python -m tools.msez validate profiles/digital-financial-center/profile.yaml

# 2. Build a deployable zone bundle
python -m tools.msez build \
    --zone jurisdictions/_starter/zone.yaml \
    --out dist/

# 3. Explore generated artifacts
ls dist/
```

### Hello World: Sanctions Check

```python
from tools.regpack import SanctionsChecker, SanctionsEntry

# Create a sanctions checker
entries = [
    SanctionsEntry(
        entry_id="ofac:12345",
        entry_type="entity",
        source_lists=["OFAC-SDN"],
        primary_name="ACME Trading Ltd",
    )
]
checker = SanctionsChecker(entries, snapshot_id="ofac-2025-01")

# Check an entity
result = checker.check_entity("ACME Trading")
print(f"Match: {result.matched}, Score: {result.match_score}")
# Output: Match: True, Score: 0.95
```

### Hello World: Agentic Policy

```python
from tools.agentic import PolicyEvaluator, EXTENDED_POLICIES
from tools.mass_primitives import AgenticTrigger, AgenticTriggerType

# Load standard policies
evaluator = PolicyEvaluator()
for pid, policy in EXTENDED_POLICIES.items():
    evaluator.register_policy(policy)

# Simulate sanctions alert
trigger = AgenticTrigger(
    trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
    data={"entity_id": "acme-123", "new_sanctioned": True}
)

# Evaluate → automatic HALT action
results = evaluator.evaluate(trigger, asset_id="asset:trade-001")
for r in results:
    if r.matched:
        print(f"Policy '{r.policy_id}' → {r.action}")
# Output: Policy 'sanctions_freeze' → halt
```

---

## 🏗️ Architecture

### Smart Assets: The Core Primitive

A **Smart Asset** is a five-tuple `A = (G, R, M, C, H)`:

| Component | Name | Immutable? | Description |
|-----------|------|:----------:|-------------|
| **G** | Genesis Document | ✓ | Identity, initial config, creator signature |
| **R** | Registry Credential | | Current jurisdictional bindings |
| **M** | Operational Manifest | | Live configuration, metadata, policies |
| **C** | Receipt Chain | ✓ (append-only) | Cryptographic operation history |
| **H** | State Machine | | Deterministic transition function |

**Key Invariants (formally proven):**

```
I1. Identity Immutability:  ∀t ≥ 0: asset_id(t) = SHA256(JCS(G))
I2. Receipt Chain Integrity: ∀i: receipt[i].prev_root = receipt[i-1].next_root
I3. State Determinism:      H(state, transition) → state' is pure
```

### Module System

Zones are composed from **modules**—self-contained packages of legal text, schemas, and validation logic:

```
modules/
├── legal/                    # LawPack: Akoma Ntoso legal documents
│   ├── enabling-act/         #   Zone enabling legislation
│   ├── commercial-code/      #   UCC-style commercial law
│   ├── dispute-resolution/   #   Arbitration framework
│   └── privacy-law/          #   Data protection rules
├── financial/                # Financial infrastructure
│   ├── banking-license/      #   Banking charter module
│   ├── payment-services/     #   PSP licensing
│   └── securities/           #   Securities regulation
├── corridors/                # Settlement corridors
│   ├── swift-settlement/     #   SWIFT MT103/202
│   ├── stablecoin/          #   USDC/USDT settlement
│   └── correspondent/        #   Nostro/vostro
└── compliance/               # RegPack: Compliance automation
    ├── aml-kyc/              #   AML/KYC rules engine
    ├── sanctions/            #   OFAC/EU/UN screening
    └── reporting/            #   Regulatory reporting
```

### Agentic Execution (v0.4.42)

Assets can **respond autonomously** to environmental changes:

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│   ENVIRONMENT    │     │     POLICY       │     │     ACTION       │
│    MONITORS      │────▶│    EVALUATOR     │────▶│    SCHEDULER     │
└──────────────────┘     └──────────────────┘     └──────────────────┘
         │                        │                        │
         ▼                        ▼                        ▼
   5 Monitor Types         16 Standard            Retry Semantics
   - Sanctions             Policies               Authorization
   - Licenses              - sanctions_freeze     Audit Trail
   - Corridors             - license_suspend
   - Guidance              - corridor_failover
   - Checkpoints           - checkpoint_auto
                           - ruling_enforce
```

**Theorem 17.1 (Agentic Determinism):** Given identical trigger events and environment state, agentic execution produces identical state transitions.

---

## 📦 Repository Structure

```
msez-stack/
├── 📁 apis/                      # OpenAPI 3.0 specifications
│   ├── corridor-state.openapi.yaml
│   ├── smart-assets.openapi.yaml
│   └── regulator-console.openapi.yaml
│
├── 📁 docs/
│   ├── examples/                 # Worked examples with real data
│   │   ├── trade/               #   Complete trade finance flow
│   │   ├── regpack/             #   Sanctions screening examples
│   │   ├── lawpack/             #   Legal document examples
│   │   └── agentic/             #   Policy evaluation examples
│   ├── patchlists/              # Version release notes
│   └── roadmap/                 # Future version plans
│
├── 📁 modules/                   # Modular jurisdiction components
│   ├── legal/                   #   LawPack: Akoma Ntoso legal text
│   ├── financial/               #   Banking, payments, securities
│   ├── corridors/               #   Settlement corridor types
│   └── compliance/              #   RegPack: Compliance rules
│
├── 📁 profiles/                  # Pre-configured zone templates
│   ├── digital-financial-center/
│   ├── charter-city/
│   ├── trade-playbook/
│   └── minimal-mvp/
│
├── 📁 registries/                # Global identifier registries
│   ├── jurisdictions.yaml       #   ISO 3166 + custom zones
│   ├── modules.yaml             #   Module catalog
│   ├── corridors.yaml           #   Corridor definitions
│   └── transition-types.yaml    #   State transition taxonomy
│
├── 📁 schemas/                   # 110 JSON Schemas
│   ├── zone.schema.json
│   ├── profile.schema.json
│   ├── corridor-receipt.schema.json
│   ├── agentic.*.schema.json    #   v0.4.42 agentic schemas
│   └── ...
│
├── 📁 spec/                      # Normative specification (20 chapters)
│   ├── 00-terminology.md
│   ├── 17-agentic.md            #   Agentic execution spec
│   └── ...
│
├── 📁 tests/                     # 395 tests
│   ├── test_agentic.py          #   62 agentic tests
│   ├── test_edge_cases_v042.py  #   36 edge case tests
│   ├── test_arbitration.py      #   Arbitration tests
│   └── ...
│
└── 📁 tools/                     # Reference implementation
    ├── msez.py                  #   Main CLI
    ├── mass_primitives.py       #   Core MASS primitives
    ├── agentic.py               #   Agentic framework
    ├── arbitration.py           #   Dispute resolution
    ├── regpack.py               #   Compliance engine
    └── netting.py               #   Settlement netting
```

---

## 📚 Examples

### Example 1: Complete Trade Finance Flow

A cross-border trade between an exporter (Zone A) and importer (Zone B):

```bash
# Generate complete trade playbook
python -m tools.dev.generate_trade_playbook \
    docs/examples/trade/src \
    docs/examples/trade/dist

# Artifacts generated:
# - Corridor agreements (signed VCs)
# - Receipt chains (3 receipts each corridor)
# - Checkpoints (L1-anchorable)
# - Settlement plan (with netting)
# - Settlement anchor (finality proof)
```

**Generated artifact graph:**

```
                    ┌─────────────────────┐
                    │  Settlement Anchor  │
                    │   (finality proof)  │
                    └──────────┬──────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
     ┌────────────────┐ ┌────────────────┐ ┌────────────────┐
     │ Settlement     │ │ Proof Bindings │ │ Zone Locks     │
     │ Plan (netted)  │ │ (sanctions/LC) │ │ (state commit) │
     └────────────────┘ └────────────────┘ └────────────────┘
              │                │                │
              └────────────────┼────────────────┘
                               ▼
              ┌────────────────────────────────┐
              │      Corridor Agreement        │
              │    (exporter ↔ importer)       │
              └────────────────┬───────────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                                 ▼
     ┌────────────────┐                ┌────────────────┐
     │ Exporter Zone  │                │ Importer Zone  │
     │ Receipt Chain  │                │ Receipt Chain  │
     │ [r0→r1→r2]     │                │ [r0→r1→r2]     │
     └────────────────┘                └────────────────┘
```

### Example 2: Sanctions Screening

Real OFAC-style sanctions data with fuzzy matching:

```python
from tools.regpack import SanctionsChecker, SanctionsEntry

# Load sanctions entries (see docs/examples/regpack/)
entries = [
    SanctionsEntry(
        entry_id="ofac:sdn:12345",
        entry_type="entity",
        source_lists=["OFAC-SDN", "EU-CONSOLIDATED"],
        primary_name="ACME TRADING COMPANY LIMITED",
        aliases=[
            {"alias_type": "AKA", "name": "ACME TRADING"},
            {"alias_type": "FKA", "name": "ACME IMPORT EXPORT CO"}
        ],
        identifiers=[
            {"id_type": "REGISTRATION_NUMBER", "value": "HK-12345678"}
        ],
        programs=["SDGT", "IRGC"],
        listing_date="2023-06-15",
    ),
    # ... 11 more entries in example file
]

checker = SanctionsChecker(entries, snapshot_id="ofac-sdn-2025-01-15")

# Exact match
result = checker.check_entity("ACME TRADING COMPANY LIMITED")
assert result.matched == True
assert result.match_score == 1.0

# Fuzzy match (alias)
result = checker.check_entity("Acme Trading")
assert result.matched == True
assert result.match_score >= 0.8

# No match
result = checker.check_entity("Legitimate Business Corp")
assert result.matched == False
```

### Example 3: Agentic Policy Evaluation

```python
from tools.agentic import (
    PolicyEvaluator, AgenticExecutionEngine,
    create_sanctions_monitor, create_license_monitor,
    EXTENDED_POLICIES
)
from tools.mass_primitives import AgenticTrigger, AgenticTriggerType

# Create execution engine
engine = AgenticExecutionEngine()

# Register all 16 standard policies
for policy_id, policy in EXTENDED_POLICIES.items():
    engine.policy_evaluator.register_policy(policy)

# Simulate license expiry
trigger = AgenticTrigger(
    trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
    data={
        "license_id": "lic:banking:001",
        "old_status": "valid",
        "new_status": "expired",
    }
)

# Process trigger → schedules HALT action
scheduled = engine.process_trigger(trigger, asset_id="asset:bank-001")

for action in scheduled:
    print(f"Scheduled: {action.action_type} for {action.asset_id}")
    print(f"  Policy: {action.policy_id}")
    print(f"  Status: {action.status}")

# Output:
# Scheduled: halt for asset:bank-001
#   Policy: license_expiry_alert
#   Status: pending
```

### Example 4: Arbitration Dispute

```python
from tools.arbitration import ArbitrationManager, DisputeRequest, Party, Claim, Money

# Create arbitration manager (DIFC-LCIA rules)
manager = ArbitrationManager(institution_id="difc-lcia")

# File dispute
dispute = DisputeRequest(
    dispute_id="dispute:trade:2025-001",
    claimant=Party(
        party_id="party:exporter",
        name="ExportCo Ltd",
        did="did:key:z6MkExporter..."
    ),
    respondent=Party(
        party_id="party:importer",
        name="ImportCo Inc",
        did="did:key:z6MkImporter..."
    ),
    claims=[
        Claim(
            claim_id="claim:001",
            claim_type="breach_of_contract",
            description="Non-payment for delivered goods per Invoice INV-2025-001",
            amount=Money(amount=50000, currency="USD")
        )
    ],
    institution="difc-lcia",
    filing_date="2025-01-15T00:00:00Z",
)

# Dispute triggers automatic asset halt via agentic policy
# See: EXTENDED_POLICIES["dispute_filed_halt"]
```

---

## 📋 Specification Compliance

MSEZ implements **MASS Protocol v0.2**:

| Chapter | Title | Status | Implementation |
|:-------:|-------|:------:|----------------|
| 11 | Smart Assets | ✅ | `tools/mass_primitives.py` |
| 12 | Receipt Chains | ✅ | `tools/mass_primitives.py` |
| 14 | Cross-Jurisdiction Transfer | ✅ | Protocol 14.1 |
| 16 | Fork Resolution | ✅ | Protocol 16.1, Theorem 16.1 |
| 17 | Agentic Execution | ✅ | `tools/agentic.py`, `spec/17-agentic.md` |
| 18 | Artifact Graph | ✅ | Protocol 18.1 |
| 20 | RegPack | ✅ | `tools/regpack.py` |
| 26 | Arbitration | ✅ | `tools/arbitration.py` |
| 29 | Cryptographic Proofs | ✅ | Theorems 29.1, 29.2 |

**Formal Theorems Implemented:**

| Theorem | Statement | Verification |
|---------|-----------|--------------|
| 16.1 | Offline Operation | `test_mass_primitives.py::test_theorem_16_1_*` |
| 17.1 | Agentic Determinism | `test_agentic.py::test_theorem_17_1_*` |
| 29.1 | Identity Immutability | `test_mass_primitives.py::test_theorem_29_1_*` |
| 29.2 | Non-Repudiation | `test_mass_primitives.py::test_theorem_29_2_*` |

---

## 🧪 Testing

```bash
# Run all tests (395 tests, ~30 seconds)
PYTHONPATH=. pytest tests/ -v

# Run by category
PYTHONPATH=. pytest tests/test_agentic.py -v           # Agentic (62 tests)
PYTHONPATH=. pytest tests/test_edge_cases_v042.py -v   # Edge cases (36 tests)
PYTHONPATH=. pytest tests/test_arbitration.py -v       # Arbitration
PYTHONPATH=. pytest tests/test_regpack.py -v           # RegPack

# Run with coverage
PYTHONPATH=. pytest tests/ --cov=tools --cov-report=html
```

**Test Categories:**

| Category | Tests | Coverage |
|----------|------:|----------|
| Core Primitives | 89 | Smart Assets, Receipt Chains, MMR |
| Agentic Framework | 62 | Monitors, Policies, Scheduling |
| Arbitration | 45 | Disputes, Rulings, Enforcement |
| RegPack | 38 | Sanctions, Licenses, Compliance |
| Edge Cases | 36 | Version consistency, Determinism |
| Trade Playbook | 24 | Generation, Verification |
| Schema Validation | 104 | All 110 schemas |

---

## 📈 Version History

| Version | Codename | Date | Highlights |
|---------|----------|------|------------|
| **0.4.42** | Agentic Ascension | Jan 2026 | Agentic Framework, 16 policies, 5 monitors |
| 0.4.41 | Radical Yahoo | Jan 2026 | Arbitration, RegPack, πruling circuit |
| 0.4.40 | — | Dec 2025 | Trade instruments, Settlement netting |
| 0.4.39 | — | Nov 2025 | Settlement anchors, Proof bindings |

See [`governance/CHANGELOG.md`](governance/CHANGELOG.md) for complete history.

---

## 🛠️ CLI Reference

```bash
# Validation
msez validate <profile.yaml>              # Validate profile
msez validate --zone <zone.yaml>          # Validate zone
msez validate --all-modules               # Validate all modules

# Building
msez build --zone <zone.yaml> --out <dir> # Build zone bundle

# Inspection
msez inspect <artifact.json>              # Inspect artifact
msez verify <receipt.json>                # Verify receipt chain

# Development
msez fetch-akoma-schemas                  # Fetch AKN schemas
```

---

## 🤝 Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for guidelines.

**Quick checklist:**
1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing`)
3. Ensure tests pass (`PYTHONPATH=. pytest tests/`)
4. Update documentation
5. Submit pull request

---

## 📄 License

Licensed under terms in [`LICENSES/`](LICENSES/). Modules may have additional terms in their `module.yaml`.

---

<div align="center">

**Built with ❤️ by [Momentum Protocol](https://momentum.xyz)**

[Documentation](./docs/) · [Specification](./spec/) · [Examples](./docs/examples/) · [Issues](https://github.com/momentum-xyz/msez-stack/issues)

---

*"Jurisdiction-native infrastructure for the programmable economy."*

</div>
