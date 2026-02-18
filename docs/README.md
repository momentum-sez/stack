# Documentation

**Momentum EZ Stack** -- v0.4.44

This is the navigation hub for all EZ Stack documentation. The codebase is a Rust workspace of 16 crates implementing compliance orchestration, corridor operations, and jurisdictional composition above the live Mass APIs.

---

## Start here

| Document | What you'll learn |
|----------|-------------------|
| [Getting Started](./getting-started.md) | Clone, build, test, run the API server, use the CLI |
| [Architecture Overview](./architecture/OVERVIEW.md) | System design, data flow, the Mass/EZ boundary |
| [Crate Reference](./architecture/CRATE-REFERENCE.md) | Every crate's purpose, key types, and public API |
| [Specification](../spec/) | 25 normative protocol chapters |

---

## Architecture

| Document | Scope |
|----------|-------|
| [Architecture Overview](./architecture/OVERVIEW.md) | Layered system design, artifact model, corridors, verification pipeline |
| [Crate Reference](./architecture/CRATE-REFERENCE.md) | Per-crate API surface: structs, traits, functions, error types |
| [Mass Integration](./architecture/MASS-INTEGRATION.md) | How the EZ Stack maps onto the five Mass primitives |
| [Security Model](./architecture/SECURITY-MODEL.md) | Trust boundaries, attack classes, verification modes |
| [Smart Asset Integration](./architecture/SMART-ASSET-INTEGRATION.md) | Smart Asset lifecycle, compliance tensor binding |
| [Traceability Matrix](./traceability-matrix.md) | Spec chapter to Rust crate mapping |

---

## Operator guides

| Document | Scope |
|----------|-------|
| [Zone Deployment](./operators/ZONE-DEPLOYMENT-GUIDE.md) | End-to-end zone deployment procedure |
| [Corridor Formation](./operators/CORRIDOR-FORMATION-GUIDE.md) | Establish and activate cross-border corridors |
| [Incident Response](./operators/INCIDENT-RESPONSE.md) | Fork detection, key rotation, availability incidents |

---

## Authoring guides

| Document | Scope |
|----------|-------|
| [Module Authoring](./authoring/modules.md) | YAML descriptors, families, validation rules |
| [Akoma Ntoso](./authoring/akoma.md) | Legal text authoring in Akoma Ntoso XML |
| [Legal Corpus](./authoring/legal-corpus.md) | Building the statutory corpus for a jurisdiction |
| [Licensing Pack](./authoring/licensing-pack.md) | License registry and licensepack lifecycle |
| [Corridor Authoring](./authoring/corridors.md) | Corridor agreements, routing, pack trilogy binding |

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
| [Error Taxonomy](./ERRORS.md) | RFC 7807 error codes and recovery strategies |
| [Performance](./PERFORMANCE.md) | Performance characteristics and regression guards |
| [Attestation Catalog](./attestations/catalog.md) | Complete catalog of attestation/VC types |

---

## Specification chapters

The `spec/` directory is the normative protocol specification. Implementation decisions defer to it.

| Chapter | Title |
|---------|-------|
| [00](../spec/00-terminology.md) | Terminology |
| [01](../spec/01-mission.md) | Mission |
| [02](../spec/02-invariants.md) | Invariants |
| [03](../spec/03-standard-structure.md) | Standard structure |
| [04](../spec/04-design-rubric.md) | Design rubric |
| [10](../spec/10-repo-layout.md) | Repository layout |
| [11](../spec/11-architecture-overview.md) | Architecture |
| [12](../spec/12-mass-primitives-mapping.md) | Mass primitives |
| [17](../spec/17-agentic.md) | Agentic engine |
| [20](../spec/20-module-system.md) | Module system |
| [22](../spec/22-templating-and-overlays.md) | Templating |
| [30](../spec/30-profile-system.md) | Profile system |
| [40](../spec/40-corridors.md) | Corridors |
| [41](../spec/41-nodes.md) | Nodes |
| [50](../spec/50-conformance.md) | Conformance |
| [60](../spec/60-governance.md) | Governance |
| [61](../spec/61-network-diffusion.md) | Network diffusion |
| [71](../spec/71-regulator-console.md) | Regulator console |
| [80](../spec/80-security-privacy.md) | Security & privacy |
| [90](../spec/90-provenance.md) | Provenance |
| [95](../spec/95-lockfile.md) | Lockfile |
| [96](../spec/96-lawpacks.md) | Lawpacks |
| [97](../spec/97-artifacts.md) | Artifacts |
| [98](../spec/98-licensepacks.md) | Licensepacks |

---

## Security & audit

| Document | Scope |
|----------|-------|
| [Audit v2](./fortification/sez_stack_audit_v2.md) | Seven-pass audit, tiered execution roadmap |
| [Bug Hunt Log](./bughunt/BUGHUNT_LOG.md) | Historical bug tracking |

---

## API specifications

OpenAPI 3.x specs in `apis/`:

| Spec | Scope |
|------|-------|
| [smart-assets.openapi.yaml](../apis/smart-assets.openapi.yaml) | Smart Asset CRUD, compliance evaluation |
| [corridor-state.openapi.yaml](../apis/corridor-state.openapi.yaml) | Corridor receipts, forks, finality |
| [mass-node.openapi.yaml](../apis/mass-node.openapi.yaml) | Zone-to-Mass integration |
| [regulator-console.openapi.yaml](../apis/regulator-console.openapi.yaml) | Regulator query access |

---

## Examples

Working examples in `docs/examples/`:

| Directory | Contents |
|-----------|----------|
| [Agentic](./examples/agentic/) | Sanctions halt policy scenario |
| [Keys](./examples/keys/) | Development Ed25519 key pairs |
| [Lawpack](./examples/lawpack/) | Akoma Ntoso enabling act |
| [Regpack](./examples/regpack/) | License registry, sanctions snapshots |
| [State](./examples/state/) | State transition examples |
| [Trade](./examples/trade/) | Full trade playbook with receipts and settlement |
| [VC](./examples/vc/) | Verifiable Credential examples (signed/unsigned) |

---

## Deployment

| Resource | Scope |
|----------|-------|
| [Docker Compose](../deploy/docker/) | Multi-service stack with Prometheus |
| [Kubernetes](../deploy/k8s/) | Production manifests |
| [AWS Terraform](../deploy/aws/terraform/) | EKS + RDS + KMS infrastructure |

---

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md).
