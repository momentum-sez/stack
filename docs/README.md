# Documentation

**Momentum EZ Stack** v0.4.44 GENESIS

---

## Start here

| If you want to... | Read |
|---|---|
| Build, test, and run the stack | [Getting Started](./getting-started.md) |
| Understand the system design | [Architecture Overview](./architecture/OVERVIEW.md) |
| Look up a specific crate's API | [Crate Reference](./architecture/CRATE-REFERENCE.md) |
| Read the protocol specification | [Specification](../spec/) |

---

## Architecture

| Document | Scope |
|----------|-------|
| [Architecture Overview](./architecture/OVERVIEW.md) | Two-system design, data flow, the Mass/EZ boundary, cryptographic invariants |
| [Crate Reference](./architecture/CRATE-REFERENCE.md) | Per-crate API surface: key types, traits, public functions |
| [Mass Integration](./architecture/MASS-INTEGRATION.md) | How the EZ Stack maps onto the five Mass primitives |
| [Security Model](./architecture/SECURITY-MODEL.md) | Trust boundaries, threat model, verification modes |
| [Smart Asset Integration](./architecture/SMART-ASSET-INTEGRATION.md) | Smart Asset lifecycle and compliance tensor binding |
| [Smart Asset OS](./architecture/SMART-ASSET-OS.md) | Stateful policy-constrained objects without a chain |
| [Legal Integration](./architecture/LEGAL-INTEGRATION.md) | Lawpacks as a verifiable evidence layer |
| [Traceability Matrix](./traceability-matrix.md) | Spec chapter to Rust crate mapping |

---

## Authoring guides

| Document | Scope |
|----------|-------|
| [Module Authoring](./authoring/modules.md) | YAML descriptors, families, validation rules |
| [Corridor Authoring](./authoring/corridors.md) | Corridor agreements, routing, pack trilogy binding |
| [Akoma Ntoso](./authoring/akoma.md) | Legal text authoring in Akoma Ntoso XML |
| [Legal Corpus](./authoring/legal-corpus.md) | Building the statutory corpus for a jurisdiction |
| [Licensing Pack](./authoring/licensing-pack.md) | License registry and licensepack lifecycle |

---

## Corridors

| Document | Scope |
|----------|-------|
| [Corridor Overview](./corridors/overview.md) | Receipt chains, fork resolution, netting, SWIFT |
| [Trade Playbooks](./corridors/playbooks.md) | End-to-end trade flow generation |

---

## Reference

| Document | Scope |
|----------|-------|
| [Error Taxonomy](./ERRORS.md) | Structured error codes (P-codes) and recovery strategies |
| [Attestation Catalog](./attestations/catalog.md) | Complete catalog of VC and attestation types |

---

## Specification

24 normative chapters in [`spec/`](../spec/). Implementation decisions defer to spec.

| # | Chapter |
|---|---------|
| [00](../spec/00-terminology.md) | Terminology |
| [01](../spec/01-mission.md) | Mission |
| [02](../spec/02-invariants.md) | Protocol invariants |
| [03](../spec/03-standard-structure.md) | Standard document structure |
| [04](../spec/04-design-rubric.md) | Design decision criteria |
| [10](../spec/10-repo-layout.md) | Repository layout |
| [11](../spec/11-architecture-overview.md) | System architecture |
| [12](../spec/12-mass-primitives-mapping.md) | Mass primitives mapping |
| [17](../spec/17-agentic.md) | Agentic policy engine |
| [20](../spec/20-module-system.md) | Module system |
| [22](../spec/22-templating-and-overlays.md) | Templating and overlays |
| [30](../spec/30-profile-system.md) | Zone profile system |
| [40](../spec/40-corridors.md) | Corridor protocol |
| [41](../spec/41-nodes.md) | Node architecture |
| [50](../spec/50-conformance.md) | Conformance testing |
| [60](../spec/60-governance.md) | Governance structures |
| [61](../spec/61-network-diffusion.md) | Network propagation |
| [71](../spec/71-regulator-console.md) | Regulator console |
| [80](../spec/80-security-privacy.md) | Security and privacy |
| [90](../spec/90-provenance.md) | Provenance tracking |
| [95](../spec/95-lockfile.md) | Lockfile format |
| [96](../spec/96-lawpacks.md) | Lawpack system |
| [97](../spec/97-artifacts.md) | Content-addressed artifacts |
| [98](../spec/98-licensepacks.md) | Licensepack system |

---

## API specifications

OpenAPI 3.x specs in [`apis/`](../apis/):

| Spec | Scope |
|------|-------|
| [smart-assets.openapi.yaml](../apis/smart-assets.openapi.yaml) | Smart Asset CRUD, compliance evaluation |
| [corridor-state.openapi.yaml](../apis/corridor-state.openapi.yaml) | Corridor receipts, forks, finality |
| [mass-node.openapi.yaml](../apis/mass-node.openapi.yaml) | Zone-to-Mass integration |
| [regulator-console.openapi.yaml](../apis/regulator-console.openapi.yaml) | Regulator query access |

---

## Examples

Working examples in [`docs/examples/`](./examples/):

| Directory | Contents |
|-----------|----------|
| [Agentic](./examples/agentic/) | Sanctions halt policy scenario |
| [Keys](./examples/keys/) | Development Ed25519 key pairs |
| [Lawpack](./examples/lawpack/) | Akoma Ntoso enabling act |
| [Regpack](./examples/regpack/) | License registry, sanctions snapshots |
| [State](./examples/state/) | State transition examples |
| [Trade](./examples/trade/) | Trade playbook with receipts and settlement |
| [VC](./examples/vc/) | Verifiable Credential examples |

---

## Deployment

| Resource | Scope |
|----------|-------|
| [Docker Compose](../deploy/docker/) | Single-binary stack with Prometheus |
| [Kubernetes](../deploy/k8s/) | Production manifests |
| [AWS Terraform](../deploy/aws/terraform/) | EKS + RDS + KMS infrastructure |
| [Deployment Roadmap](./PRAGMATIC-DEPLOYMENT-ROADMAP.md) | Phase gates and priorities |

---

## Roadmap

| Document | Scope |
|----------|-------|
| [Deployment Roadmap](./PRAGMATIC-DEPLOYMENT-ROADMAP.md) | Pragmatic path to production |
| [AWS of Economic Zones](./roadmap/AWS_OF_ECONOMIC_ZONES.md) | Strategic vision and gap analysis |
| [Production-Grade Spec](./roadmap/PRODUCTION_GRADE_SPEC.md) | North stars for production evolution |
