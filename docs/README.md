# Documentation

**Momentum EZ Stack** — 17 crates, ~164K lines, ~4,700 tests, 210 zones

---

## Start here

| If you want to... | Read |
|---|---|
| Build, test, and run the stack | [Getting Started](./getting-started.md) |
| Deploy a zone from scratch | [Zone Bootstrap Guide](./ZONE-BOOTSTRAP-GUIDE.md) |
| Understand the system design | [Architecture Overview](./architecture/OVERVIEW.md) |
| Look up a specific crate's API | [Crate Reference](./architecture/CRATE-REFERENCE.md) |
| Read the protocol specification | [Specification](../spec/) |
| Understand the deployment strategy | [Deployment Roadmap](./PRAGMATIC-DEPLOYMENT-ROADMAP.md) |

---

## Architecture

| Document | Scope |
|----------|-------|
| [Architecture Overview](./architecture/OVERVIEW.md) | Two-system design, data flow, the Mass/EZ boundary |
| [Architecture Summary](./ARCHITECTURE.md) | System layers, compliance tensor, corridors |
| [Crate Reference](./architecture/CRATE-REFERENCE.md) | Per-crate API surface: key types, traits, public functions |
| [Mass Integration](./architecture/MASS-INTEGRATION.md) | How the EZ Stack maps onto the five Mass primitives |
| [Security Model](./architecture/SECURITY-MODEL.md) | Trust boundaries, threat model, verification modes |
| [Smart Asset Integration](./architecture/SMART-ASSET-INTEGRATION.md) | Smart Asset lifecycle and compliance tensor binding |
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

## Deployment

| Resource | Scope |
|----------|-------|
| [Docker Compose](../deploy/docker/) | Single-binary stack with Prometheus and Grafana |
| [Two-Zone Compose](../deploy/docker/docker-compose.two-zone.yaml) | Sovereign corridor testing (PK-SIFC + AE-DIFC) |
| [Kubernetes](../deploy/k8s/) | Production manifests |
| [AWS Terraform](../deploy/aws/terraform/) | EKS + RDS + KMS infrastructure |
| [Deploy Scripts](../deploy/scripts/) | Zone deployment and demo scripts |
| [Zone Bootstrap Guide](./ZONE-BOOTSTRAP-GUIDE.md) | End-to-end zone deployment walkthrough |
| [Deployment Roadmap](./PRAGMATIC-DEPLOYMENT-ROADMAP.md) | Phase gates and priorities |

---

## Specification

24 normative chapters in [`spec/`](../spec/). Implementation decisions defer to spec.

---

## API specifications

OpenAPI 3.x specs in [`apis/`](../apis/).

---

## Tooling

| Tool | Location | Purpose |
|------|----------|---------|
| [Spec Generator](./mez-spec-generator/) | `docs/mez-spec-generator/` | Generates the GENESIS specification `.docx` from chapter files |
