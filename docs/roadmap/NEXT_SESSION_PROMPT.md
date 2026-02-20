# Next Session Prompt: Complete Network Topology

The following is a Claude Code session prompt designed for Opus 4.6.
Copy everything below the `---` line into a new "Ask Claude to write code..." session.

---

Complete Network Topology — All N Natural Zones + All M Synthetic Zones → Full N×(N-1)/2 Autonomous Corridor Mesh

Read CLAUDE.md for full audit context. Then read these files before writing any code:
- `docs/roadmap/AWS_OF_ECONOMIC_ZONES.md` — strategic framing
- `docs/roadmap/SYNTHETIC_ZONE_CATALOG.md` — 12 cataloged synthetic compositions (zones 3-12 have exact `composition.yaml` specs ready to materialize)
- `docs/roadmap/JURISDICTION_COVERAGE_MATRIX.md` — tiering system and expansion targets
- `jurisdictions/pk-sifc/zone.yaml` — gold standard natural zone manifest (116 lines, full vertical)
- `jurisdictions/ae-dubai-difc/zone.yaml` — gold standard Tier 2 free zone manifest (80 lines)
- `jurisdictions/synth-atlantic-fintech/zone.yaml` — gold standard synthetic zone manifest (106 lines)
- `jurisdictions/br/zone.yaml` — example of what a 28-line scaffold looks like (this is what needs enrichment)
- `mez/crates/mez-corridor/src/composition.rs` — zone composition algebra (10 regulatory domains, validation rules)
- `mez/crates/mez-corridor/src/registry.rs` — corridor registry (N×(N-1)/2 pairwise generation, DOT output, mesh stats, `classify_corridor()`)
- `mez/crates/mez-cli/src/compose.rs` — synthetic zone generator
- `mez/crates/mez-cli/src/main.rs` — CLI command dispatch

## Current State (precise inventory)

102 jurisdiction directories exist. 22 have enriched zone manifests. 80 are 28-line scaffolds.

| Category | Count | Content level |
|---|---|---|
| Tier 1 (pk-sifc, ae-abudhabi-adgm, sg, hk, ky) | 5 | Full: regpack digests, national adapters, compliance domains, profiles |
| Tier 2 UAE (ae, ae-abudhabi, ae-dubai, 12 sub-zones) | 15 | Wired: ae federal regpack digests, compliance domains, adapter stubs |
| Tier 2 Synthetic (synth-atlantic-fintech, synth-pacific-trade) | 2 | Full: composition + merged regpacks + profile |
| Scaffold (57 US states/territories, 5 China, br, eg, hn-prospera, id, ie, ke, kz, kz-aifc, kz-alatau, pt, qa, qa-qfc, sc, tz, tz-zanzibar, vg, za) | ~80 | Bare: 28 lines, jurisdiction_stack only, no compliance_domains, no regpacks, no adapters |
| Missing directories (Tier 3/4 in coverage matrix but no dir exists) | ~60 | Nothing: bh, de, gb, nl, se, dk, fi, no, mu, ng, rw, pa, uy, cl, co, mx, my, sa, jp, kr, in-gift, in-ifsc, etc. |
| Unbuilt synthetic (cataloged zones 3-12 in SYNTHETIC_ZONE_CATALOG.md) | 10 | Specs exist in doc, no directories |

Regpack builders: 6 (pakistan.rs, uae.rs, singapore.rs, hong_kong.rs, cayman.rs in `mez/crates/mez-pack/src/regpack/`).
National adapter implementations: 12 (PK 4, AE 4, SG 4).
Profiles: 12 (including 2 synthetic).

## Target State — The Complete Graph

When done, every zone in the repository is a deployable node in a fully connected corridor mesh. Every pair of zones has an autonomous corridor. The "AWS of Economic Zones" thesis is demonstrated at scale.

Measurable completion criteria:

1. **Zero scaffold manifests remain.** Every `jurisdictions/*/zone.yaml` has: `compliance_domains`, `key_management`, `trust_anchors`, appropriate `national_adapters` stub block, and either real `regpacks` digests (when parent jurisdiction has a builder) or no regpacks block (with `# Regpack builder needed` comment).

2. **All 12 cataloged synthetic zones materialized.** Each has `jurisdictions/synth-{id}/composition.yaml`, `jurisdictions/synth-{id}/zone.yaml`, `profiles/synthetic-synth-{id}/profile.yaml`.

3. **8+ new synthetic zone compositions created beyond the catalog.** Each targets a distinct economic corridor or regional use case not already covered by zones 1-12. Design principles below.

4. **All source jurisdictions for all synthetic compositions have directories.** The catalog's zones 3-12 reference ~20 jurisdictions that currently lack directories (bh, de, gb, nl, se, dk, fi, no, mu, ng, rw, pa, uy, cl, co, mx, my, sa, etc.). These must exist before synthetic zones can reference them.

5. **All Tier 3/4 jurisdictions from the coverage matrix have directories with enriched manifests.**

6. **Full corridor mesh integration test.** Registers every zone, generates N×(N-1)/2 corridors, asserts completeness.

7. **`mez corridor mesh --all` CLI command.** Discovers all `jurisdictions/*/zone.yaml`, registers zones, generates mesh, outputs stats + optional DOT/JSON.

8. **Updated docs.** `JURISDICTION_COVERAGE_MATRIX.md` reflects new state. `SYNTHETIC_ZONE_CATALOG.md` includes new compositions. New `docs/roadmap/CORRIDOR_MESH_TOPOLOGY.md` documents the complete mesh with statistics by corridor type.

## Zone Manifest Enrichment Spec

Transform every scaffold from 28 lines to a deployable zone manifest. Use `ae-dubai-difc/zone.yaml` as the structural template for enrichment. Every enriched manifest must contain:

- **profile**: Assign by zone character. `sovereign-govos` for nation-states with regulatory authority. `digital-financial-center` for financial centers and free zones (DIFC, ADGM, GIFT City, Labuan, QFC, etc.). `charter-city` for charter cities and special zones (hn-prospera, sa-neom). `minimal-mvp` for early-stage and emerging jurisdictions.
- **lawpack_domains**: `[civil, financial]` minimum. Add `tax`, `aml` for jurisdictions with real regulatory frameworks.
- **licensepack_domains**: `[financial, corporate]` for financial centers. Omit for pure nation-state scaffolds without licensing frameworks.
- **licensepack_refresh_policy**: Include when licensepack_domains present. Use the ae-dubai-difc pattern.
- **regpacks**: For zones whose root country has a builder (pk→pk, ae→ae, sg→sg, hk→hk, ky→ky), reference the parent's real SHA-256 digests. For all others, omit the block entirely.
- **compliance_domains**: Minimum `[aml, kyc, sanctions, tax, corporate, licensing]`. Financial centers and free zones add `[securities, data_privacy, consumer_protection]`.
- **corridors**: `[org.momentum.mez.corridor.swift.iso20022-cross-border]` for all. Financial centers add `org.momentum.mez.corridor.stablecoin.regulated-stablecoin`.
- **national_adapters**: Stub block with `enabled: false` entries named after the jurisdiction's actual regulators. Use real regulator names from domain knowledge (e.g., Germany: `bafin`, `bzst`; UK: `fca`, `hmrc`, `companies_house`; Japan: `fsa_japan`, `nta`; Brazil: `bcb`, `cvm`, `receita_federal`). If uncertain about a regulator name, add `# TODO: verify regulator` comment.
- **trust_anchors**: `[]`
- **key_management**: `{ rotation_interval_days: 90, grace_period_days: 14 }`

Batch by country family for efficiency. All US states share a common structure (vary only zone_id, jurisdiction_id, zone_name, and state-specific regulator in national_adapters). Same for China zones, GCC zones, European zones, etc.

## Synthetic Zone Design Principles (for the 8+ new compositions)

Each new synthetic zone must:
- Source regulatory domains from at least 3 different countries.
- Include `aml_cft` domain (mandatory per composition algebra).
- Target a distinct economic corridor, trade route, or regulatory use case not covered by the existing 12.
- Use real statutory references for each layer's source jurisdiction. Mark uncertain citations with `# TODO: verify`.
- Pass `validate_composition()` — no duplicate domains, AML/CFT present, all source jurisdictions have zone.yaml.

Geographic coverage targets for the 8+ new compositions — ensure at least one composition for each underserved region/use case:
- US domestic interstate digital asset operations (e.g., Wyoming DAO-friendly corporate + Delaware civic + New York securities + Texas tax)
- Caribbean digital asset cluster (Cayman + BVI + Bermuda + Bahamas)
- Central Asian gateway / Belt and Road corridor
- Indo-Pacific trade (India GIFT City + ASEAN + Oceania)
- Mediterranean / Southern European digital finance
- Pacific Islands economic development
- East African innovation hub
- Swiss-Liechtenstein-Asian innovation bridge

These are suggestions. Use your judgment on the most strategically valuable compositions. Each must produce a valid `composition.yaml` consumable by `mez zone compose`.

## Execution Order (dependency-driven)

1. Create jurisdiction directories for ALL missing jurisdictions: Tier 3/4 from coverage matrix + all source jurisdictions referenced by synthetic zone compositions that don't yet have directories. Each gets an enriched zone.yaml (not a scaffold).

2. Enrich all existing US state scaffold manifests (57 zones). Batch — they share common structure.

3. Enrich all remaining non-US scaffold manifests (China, existing GCC/ME/AF/EU/LATAM/APAC directories). Batch by region.

4. Materialize all 12 cataloged synthetic zones from SYNTHETIC_ZONE_CATALOG.md specs + create 8+ new synthetic compositions.

5. Add `mez corridor mesh --all` CLI command. Add full-mesh integration test.

6. Update all three roadmap docs + create CORRIDOR_MESH_TOPOLOGY.md.

## Constraints

- All existing tests must pass. Run `cargo test --workspace` before each commit.
- Do NOT modify existing enriched zone manifests (the 22 already-complete zones). Extend the network, don't alter existing nodes.
- Regpack digests: Only use real SHA-256 values from existing builders. Never invent hashes. Zones without a parent builder get no regpacks block.
- Corridor taxonomy: 4 types only (CrossBorder, IntraFederal, FreeZoneToHost, FreeZoneToFreeZone). Synthetic-to-anything = CrossBorder. Do not add a 5th type.
- Do not modify `composition.rs` or `registry.rs` core logic. You may add CLI wiring and tests that USE these modules.
- National adapter stubs must name real regulators. Do not invent regulator names.
- Profile assignment must match zone character. A free zone gets `digital-financial-center`, not `sovereign-govos`.
- Follow existing YAML formatting exactly. No trailing whitespace. Use the existing zone manifests as formatting references.
- For the corridor mesh test: parse zone.yaml files to construct ZoneEntry structs. Determine `is_free_zone` from `jurisdiction_stack` length (>1 with free zone parent = true). Determine `zone_type` from presence of `composition:` block.

## Commit Structure (6 commits, logical units)

1. **New jurisdiction directories** — all missing Tier 3/4 jurisdictions + synthetic source jurisdictions. Each with enriched zone.yaml.
2. **US state enrichment** — 57 US zone manifests promoted from scaffold to enriched.
3. **Global zone enrichment** — all remaining non-US scaffolds enriched, batched by region.
4. **Synthetic zone materialization** — all 12 catalog zones + 8+ new compositions. Each with composition.yaml, zone.yaml, profile.yaml.
5. **Corridor mesh tooling** — `mez corridor mesh --all` CLI + full-mesh integration test asserting N×(N-1)/2 completeness.
6. **Documentation update** — JURISDICTION_COVERAGE_MATRIX.md, SYNTHETIC_ZONE_CATALOG.md, new CORRIDOR_MESH_TOPOLOGY.md with complete mesh statistics and DOT visualization of the full graph.

Push to the designated branch when complete.
