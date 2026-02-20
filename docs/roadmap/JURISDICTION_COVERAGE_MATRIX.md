# Jurisdiction Coverage Matrix — MEZ Stack v0.4.44

This matrix tracks deployment readiness across jurisdictions for the Momentum Economic Zones (MEZ) Stack. Each jurisdiction is classified into a tier based on the completeness of its zone manifest, regulatory pack content, national system adapters, and deployment profile. The matrix serves as the canonical reference for sovereign deployment planning and corridor expansion sequencing.

---

## Tier Definitions

| Tier | Definition | Criteria |
|------|------------|----------|
| Tier 1 | Production-ready | `zone.yaml` + regpack builder + real licensepack content + national adapters + profile |
| Tier 2 | Zone manifest with regpacks | `zone.yaml` + regpack builder + `compliance_domains` + national adapter stubs |
| Tier 3 | Zone manifest only | `zone.yaml` scaffold with `jurisdiction_stack` and profile reference |
| Tier 4 | Planned | No zone manifest yet; jurisdiction identified for expansion |

---

## Current Coverage Summary

| Metric | Count |
|--------|-------|
| Tier 1 — Production Ready | 5 |
| Tier 2 — Zone Manifest with Regpacks | 17 |
| Tier 3 — Zone Manifest Scaffold (target) | ~30 |
| Tier 4 — Planned Expansion | ~48 |
| **Total Tracked Jurisdictions** | **~100** |

Current deployed zone count: **22** (5 Tier 1 + 17 Tier 2).
Target: **100** jurisdictions tracked across all tiers.

---

## Coverage Matrix

### Tier 1 — Production Ready (5 zones)

These jurisdictions have complete zone manifests, real regpack and licensepack content, functioning national system adapters, and deployment profiles suitable for sovereign corridor activation.

| Jurisdiction ID | Zone Name | Country | Tier | zone.yaml | Regpack Builder | Licensepack | National Adapters | Profile | Notes |
|-----------------|-----------|---------|------|-----------|-----------------|-------------|-------------------|---------|-------|
| `pk-sifc` | Pakistan SIFC | Pakistan | 1 | Yes | Yes | Yes (70+ categories) | Yes (FBR IRIS, SECP, NADRA, SBP Raast) | `sovereign-govos` | Primary pilot zone; full Pack Trilogy (4 lawpack domains) |
| `ae-abudhabi-adgm` | Abu Dhabi Global Market | UAE | 1 | Yes | Yes | Yes | Yes | `sovereign-govos` | ADGM Financial Services Regulatory Authority framework |
| `sg` | Singapore | Singapore | 1 | Yes | Yes | Yes | Yes | `sovereign-govos` | MAS regulatory framework; Payment Services Act coverage |
| `hk` | Hong Kong | Hong Kong SAR | 1 | Yes | Yes | Yes | Yes | `sovereign-govos` | SFC + HKMA regulatory framework |
| `ky` | Cayman Islands | Cayman Islands | 1 | Yes | Yes | Yes | Yes | `sovereign-govos` | CIMA regulatory framework; fund administration focus |

### Tier 2 — Zone Manifest with Regpacks (17 zones)

These jurisdictions have zone manifests wired with real regpack digests and compliance domains. National adapter stubs are present but not yet connected to live government systems.

| Jurisdiction ID | Zone Name | Country | Tier | zone.yaml | Regpack Builder | Licensepack | National Adapters | Profile | Notes |
|-----------------|-----------|---------|------|-----------|-----------------|-------------|-------------------|---------|-------|
| `ae` | UAE Federal | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Federal-level regulatory baseline |
| `ae-abudhabi` | Abu Dhabi Emirate | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Emirate-level overlay on federal |
| `ae-abudhabi-kezad` | Khalifa Economic Zones | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Multi-sector free zone |
| `ae-abudhabi-kizad` | Khalifa Industrial Zone | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Industrial and logistics focus |
| `ae-abudhabi-masdar` | Masdar City | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Clean energy and sustainability zone |
| `ae-abudhabi-twofour54` | twofour54 Media Zone | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Media and entertainment free zone |
| `ae-dubai` | Dubai Emirate | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Emirate-level overlay on federal |
| `ae-dubai-dhcc` | Dubai Healthcare City | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Healthcare and wellness free zone |
| `ae-dubai-dic` | Dubai Internet City | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Technology and innovation free zone |
| `ae-dubai-difc` | Dubai Intl Financial Centre | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Common law financial centre; DFSA regulated |
| `ae-dubai-dmcc` | Dubai Multi Commodities Centre | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Commodities trading free zone |
| `ae-dubai-dso` | Dubai Silicon Oasis | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Technology park and free zone |
| `ae-dubai-dwtc` | Dubai World Trade Centre | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Virtual assets and crypto regulatory sandbox |
| `ae-dubai-ifza` | Intl Free Zone Authority | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Multi-activity free zone |
| `ae-dubai-jafza` | Jebel Ali Free Zone | UAE | 2 | Yes | Yes | Stub | Stub | Ref | Largest free zone; trade and logistics |
| `synth-atlantic-fintech` | Atlantic FinTech (Synthetic) | Synthetic | 2 | Yes | Yes | Synthetic | N/A | Ref | Synthetic reference zone for corridor testing |
| `synth-pacific-trade` | Pacific Trade (Synthetic) | Synthetic | 2 | Yes | Yes | Synthetic | N/A | Ref | Synthetic reference zone for corridor testing |

### Tier 3 — Zone Manifest Scaffold (target: ~30 zones)

These jurisdictions are natural next targets for zone manifest creation. They have been identified based on regulatory clarity, economic zone activity, and corridor demand. Zone manifests will be scaffolded with `jurisdiction_stack` and profile references.

#### Asia-Pacific

| Jurisdiction ID | Zone Name | Country | Tier | zone.yaml | Regpack Builder | Licensepack | National Adapters | Profile | Notes |
|-----------------|-----------|---------|------|-----------|-----------------|-------------|-------------------|---------|-------|
| `jp` | Japan | Japan | 3 | Planned | -- | -- | -- | -- | FSA regulatory framework; JFSA virtual asset rules |
| `kr` | South Korea | South Korea | 3 | Planned | -- | -- | -- | -- | FSC/FSS framework; Virtual Asset User Protection Act |
| `in-gift` | GIFT City | India | 3 | Planned | -- | -- | -- | -- | Gujarat International Finance Tec-City; IFSCA regulated |
| `in-ifsc` | IFSC Gujarat | India | 3 | Planned | -- | -- | -- | -- | International Financial Services Centre; SEZ status |
| `my` | Malaysia | Malaysia | 3 | Planned | -- | -- | -- | -- | SC Malaysia + BNM framework; Labuan offshore |
| `my-labuan` | Labuan IBFC | Malaysia | 3 | Planned | -- | -- | -- | -- | Labuan International Business and Financial Centre |
| `th` | Thailand | Thailand | 3 | Planned | -- | -- | -- | -- | SEC Thailand + BOT framework |
| `ph` | Philippines | Philippines | 3 | Planned | -- | -- | -- | -- | BSP + SEC framework; CEZA economic zone |
| `vn` | Vietnam | Vietnam | 3 | Planned | -- | -- | -- | -- | SBV + SSC framework; emerging digital asset rules |
| `id` | Indonesia | Indonesia | 3 | Planned | -- | -- | -- | -- | OJK + BI framework; Bappebti commodity futures |
| `au` | Australia | Australia | 3 | Planned | -- | -- | -- | -- | ASIC + APRA framework; token mapping consultation |
| `nz` | New Zealand | New Zealand | 3 | Planned | -- | -- | -- | -- | FMA framework; Financial Markets Conduct Act |
| `tw` | Taiwan | Taiwan | 3 | Planned | -- | -- | -- | -- | FSC Taiwan framework; virtual asset guidance |
| `bn` | Brunei | Brunei | 3 | Planned | -- | -- | -- | -- | AMBD framework; emerging fintech sandbox |
| `mm` | Myanmar | Myanmar | 3 | Planned | -- | -- | -- | -- | CBM framework; limited digital asset regulation |

#### Middle East and Africa

| Jurisdiction ID | Zone Name | Country | Tier | zone.yaml | Regpack Builder | Licensepack | National Adapters | Profile | Notes |
|-----------------|-----------|---------|------|-----------|-----------------|-------------|-------------------|---------|-------|
| `ae-rak` | Ras Al Khaimah | UAE | 3 | Planned | -- | -- | -- | -- | Emirate-level; emerging free zone activity |
| `ae-rak-rakez` | RAKEZ Free Zone | UAE | 3 | Planned | -- | -- | -- | -- | Ras Al Khaimah Economic Zone |
| `ae-sharjah` | Sharjah | UAE | 3 | Planned | -- | -- | -- | -- | Sharjah Emirate; SRTI Park, Hamriyah FZ |
| `ae-ajman` | Ajman | UAE | 3 | Planned | -- | -- | -- | -- | Ajman Free Zone; light industrial focus |
| `bh` | Bahrain | Bahrain | 3 | Planned | -- | -- | -- | -- | CBB framework; fintech sandbox; crypto-friendly |
| `bh-bfb` | Bahrain FinTech Bay | Bahrain | 3 | Planned | -- | -- | -- | -- | FinTech hub within Bahrain regulatory sandbox |
| `qa` | Qatar | Qatar | 3 | Planned | -- | -- | -- | -- | QCB + QFMA framework |
| `qa-qfc` | Qatar Financial Centre | Qatar | 3 | Planned | -- | -- | -- | -- | QFC Regulatory Authority; common law financial centre |
| `om` | Oman | Oman | 3 | Planned | -- | -- | -- | -- | CBO + CMA framework; Duqm SEZ |
| `sa` | Saudi Arabia | Saudi Arabia | 3 | Planned | -- | -- | -- | -- | CMA + SAMA framework; Vision 2030 digital economy |
| `sa-neom` | NEOM | Saudi Arabia | 3 | Planned | -- | -- | -- | -- | NEOM special regulatory framework; greenfield zone |
| `jo` | Jordan | Jordan | 3 | Planned | -- | -- | -- | -- | CBJ + JSC framework; Aqaba SEZ |
| `eg` | Egypt | Egypt | 3 | Planned | -- | -- | -- | -- | CBE + FRA framework; Suez Canal Economic Zone |
| `ke` | Kenya | Kenya | 3 | Planned | -- | -- | -- | -- | CBK + CMA framework; Nairobi IFC initiative |
| `ng` | Nigeria | Nigeria | 3 | Planned | -- | -- | -- | -- | CBN + SEC framework; Lekki Free Zone |
| `za` | South Africa | South Africa | 3 | Planned | -- | -- | -- | -- | SARB + FSCA framework; IDZ program |
| `mu` | Mauritius | Mauritius | 3 | Planned | -- | -- | -- | -- | FSC Mauritius; Global Business License framework |
| `rw` | Rwanda | Rwanda | 3 | Planned | -- | -- | -- | -- | BNR + CMA framework; Kigali IFC |
| `gh` | Ghana | Ghana | 3 | Planned | -- | -- | -- | -- | BOG + SEC framework; emerging digital asset rules |
| `tz` | Tanzania | Tanzania | 3 | Planned | -- | -- | -- | -- | BOT framework; EPZ program |

#### Europe

| Jurisdiction ID | Zone Name | Country | Tier | zone.yaml | Regpack Builder | Licensepack | National Adapters | Profile | Notes |
|-----------------|-----------|---------|------|-----------|-----------------|-------------|-------------------|---------|-------|
| `gb` | United Kingdom | UK | 3 | Planned | -- | -- | -- | -- | FCA framework; Financial Services and Markets Act |
| `gb-gi` | Gibraltar | Gibraltar | 3 | Planned | -- | -- | -- | -- | GFSC DLT framework; established crypto regulation |
| `ie` | Ireland | Ireland | 3 | Planned | -- | -- | -- | -- | CBI framework; EU MiCA implementation |
| `lu` | Luxembourg | Luxembourg | 3 | Planned | -- | -- | -- | -- | CSSF framework; EU MiCA; fund domiciliation hub |
| `ch` | Switzerland | Switzerland | 3 | Planned | -- | -- | -- | -- | FINMA framework; DLT Act; established crypto rules |
| `ch-zug` | Zug Crypto Valley | Switzerland | 3 | Planned | -- | -- | -- | -- | Cantonal overlay; Crypto Valley ecosystem |
| `li` | Liechtenstein | Liechtenstein | 3 | Planned | -- | -- | -- | -- | FMA framework; Token and TT Service Provider Act |
| `ee` | Estonia | Estonia | 3 | Planned | -- | -- | -- | -- | EFSA framework; EU MiCA; e-Residency program |
| `mt` | Malta | Malta | 3 | Planned | -- | -- | -- | -- | MFSA framework; Virtual Financial Assets Act |
| `cy` | Cyprus | Cyprus | 3 | Planned | -- | -- | -- | -- | CySEC framework; EU MiCA implementation |

#### Americas

| Jurisdiction ID | Zone Name | Country | Tier | zone.yaml | Regpack Builder | Licensepack | National Adapters | Profile | Notes |
|-----------------|-----------|---------|------|-----------|-----------------|-------------|-------------------|---------|-------|
| `us-wy` | Wyoming | USA | 3 | Planned | -- | -- | -- | -- | Wyoming DORA; DAO LLC Act; SPDI charter |
| `us-de` | Delaware | USA | 3 | Planned | -- | -- | -- | -- | Delaware Division of Corporations; blockchain amendments |
| `us-ny` | New York | USA | 3 | Planned | -- | -- | -- | -- | NYDFS BitLicense framework |
| `us-tx` | Texas | USA | 3 | Planned | -- | -- | -- | -- | TDOB framework; Virtual Currency Act |
| `ca` | Canada | Canada | 3 | Planned | -- | -- | -- | -- | CSA + OSFI framework; MSB registration |
| `ca-on` | Ontario | Ontario, Canada | 3 | Planned | -- | -- | -- | -- | OSC framework; provincial securities overlay |
| `bm` | Bermuda | Bermuda | 3 | Planned | -- | -- | -- | -- | BMA framework; Digital Asset Business Act |
| `bs` | Bahamas | Bahamas | 3 | Planned | -- | -- | -- | -- | SCB framework; DARE Act; Sand Dollar CBDC |
| `vg` | British Virgin Islands | BVI | 3 | Planned | -- | -- | -- | -- | BVI FSC framework; Virtual Assets Service Providers Act |
| `bb` | Barbados | Barbados | 3 | Planned | -- | -- | -- | -- | CBB + FSC framework; emerging fintech rules |
| `pa` | Panama | Panama | 3 | Planned | -- | -- | -- | -- | SMV + SBP framework; crypto law (Ley 129) |
| `cr` | Costa Rica | Costa Rica | 3 | Planned | -- | -- | -- | -- | SUGEF + CONASSIF framework; emerging digital asset rules |
| `br` | Brazil | Brazil | 3 | Planned | -- | -- | -- | -- | BCB + CVM framework; crypto asset law (14.478/2022) |
| `mx` | Mexico | Mexico | 3 | Planned | -- | -- | -- | -- | CNBV + Banxico framework; FinTech Law (Ley Fintech) |
| `co` | Colombia | Colombia | 3 | Planned | -- | -- | -- | -- | SFC + BanRep framework; regulatory sandbox |
| `cl` | Chile | Chile | 3 | Planned | -- | -- | -- | -- | CMF + BCCh framework; FinTech Law |
| `ar` | Argentina | Argentina | 3 | Planned | -- | -- | -- | -- | CNV + BCRA framework; digital asset PSP rules |
| `uy` | Uruguay | Uruguay | 3 | Planned | -- | -- | -- | -- | BCU + SSF framework; emerging fintech regulation |
| `py` | Paraguay | Paraguay | 3 | Planned | -- | -- | -- | -- | BCP + CNV framework; crypto mining law |
| `sv` | El Salvador | El Salvador | 3 | Planned | -- | -- | -- | -- | BCR + SSF framework; Bitcoin legal tender; CNAD oversight |

### Tier 4 — Planned Expansion

The following jurisdictions are under evaluation for future zone manifest creation. Prioritization is driven by corridor demand (bilateral trade volume, remittance flows, FDI activity), regulatory clarity, and sovereign partnership interest.

| Region | Jurisdictions Under Evaluation |
|--------|-------------------------------|
| Europe (additional) | `de` (Germany), `fr` (France), `nl` (Netherlands), `es` (Spain), `pt` (Portugal), `it` (Italy), `at` (Austria), `se` (Sweden), `dk` (Denmark), `fi` (Finland), `no` (Norway) |
| Central Asia | `kz` (Kazakhstan — AIFC), `uz` (Uzbekistan), `ge` (Georgia) |
| South Asia | `lk` (Sri Lanka), `bd` (Bangladesh), `np` (Nepal) |
| Southeast Asia | `kh` (Cambodia), `la` (Laos), `sg-jwp` (Singapore — Jurong West Port) |
| Pacific Islands | `fj` (Fiji), `vu` (Vanuatu), `ws` (Samoa) |
| Caribbean (additional) | `tc` (Turks and Caicos), `ag` (Antigua and Barbuda), `dm` (Dominica), `gd` (Grenada), `lc` (Saint Lucia), `vc` (Saint Vincent), `jm` (Jamaica), `tt` (Trinidad and Tobago) |
| Africa (additional) | `ma` (Morocco), `tn` (Tunisia), `et` (Ethiopia), `ug` (Uganda), `cm` (Cameroon), `sn` (Senegal), `ci` (Cote d'Ivoire) |
| Middle East (additional) | `kw` (Kuwait), `lb` (Lebanon), `iq` (Iraq) |

---

## Expansion Roadmap

New jurisdictions progress through tiers via the following pipeline:

### Step 1: Identify Regulatory Framework
- Map the jurisdiction's financial regulator(s), licensing authority, AML/CFT framework, tax authority, and data protection regime.
- Determine whether the jurisdiction has economic zone or free zone legislation that creates sub-jurisdictional regulatory envelopes.
- Assess corridor demand: bilateral trade volume, remittance corridors, and FDI flows with existing Tier 1/2 zones.

### Step 2: Create Zone Manifest Scaffold (Tier 4 to Tier 3)
- Generate `zone.yaml` with `jurisdiction_stack`, `compliance_domains`, and profile reference.
- Command: `mez zone init --jurisdiction <id> --template starter`
- Map compliance domains to the jurisdiction's regulatory structure.

### Step 3: Build Regpack Content (Tier 3 to Tier 2)
- Populate lawpacks with jurisdiction-specific statutes, regulations, and rules.
- Build regpacks with domain-specific compliance rules (TAX, AML_CFT, LICENSING, SECURITIES, etc.).
- Compute CAS digests: `mez regpack build --jurisdiction <id> --all-domains --store`
- Wire regpack digests into `zone.yaml`.

### Step 4: Implement National Adapters (Tier 2 to Tier 1)
- Implement trait contracts for the jurisdiction's national systems (tax authority, corporate registry, identity authority, central bank/payment system).
- Begin with mock/stub adapters; promote to live HTTP adapters after sovereign partnership agreement.
- Run contract tests against adapter stubs.

### Step 5: Promote to Production (Tier 1)
- Generate `stack.lock` with real module and regpack digests.
- Complete licensepack content (all license categories for the jurisdiction's economic zone framework).
- Deploy sovereign profile with operator-controlled key custody.
- Execute end-to-end corridor test with at least one existing Tier 1 zone.
- Pass independent security review.

---

## Synthetic Zone Composition

Any combination of Tier 1-3 regulatory primitives can be composed into synthetic zones using `mez zone compose`. Synthetic zones enable:

- **Corridor testing**: Create paired synthetic zones to test receipt chain exchange, fork resolution, and checkpoint verification without sovereign dependencies.
- **Regulatory sandbox simulation**: Compose a synthetic zone that inherits compliance domains from multiple jurisdictions for sandbox experimentation.
- **Pre-deployment validation**: Before a jurisdiction reaches Tier 1, compose a synthetic zone using its partial regpack content alongside known-good primitives from Tier 1 zones to validate the compliance pipeline.

Existing synthetic zones (`synth-atlantic-fintech`, `synth-pacific-trade`) demonstrate this capability and are used in the two-zone demo script (`deploy/scripts/demo-two-zone.sh`).

---

## Appendix: Compliance Domain Coverage by Tier

| Compliance Domain | Tier 1 | Tier 2 | Tier 3 | Notes |
|-------------------|--------|--------|--------|-------|
| TAX | Full | Partial | -- | Withholding rules, rates, treaty networks |
| AML_CFT | Full | Partial | -- | CDD/EDD, STR, sanctions screening |
| LICENSING | Full | Stub | -- | License categories, fees, renewal rules |
| SECURITIES | Full | Stub | -- | Offering rules, exemptions, reporting |
| BANKING | Full | Stub | -- | Capital requirements, reserve ratios |
| DATA_PRIVACY | Full | Stub | -- | Data localization, consent, retention |
| SANCTIONS | Full | Partial | -- | SDN/consolidated lists, screening rules |
| CONSUMER_PROTECTION | Partial | -- | -- | Disclosure, cooling-off, redress |
| INSURANCE | Partial | -- | -- | Capital, solvency, policyholder protection |
| ENVIRONMENTAL | Partial | -- | -- | Carbon reporting, ESG disclosure |

---

*Last updated: 2026-02-20*
*Source: MEZ Stack v0.4.44-GENESIS audit and deployment tracking*
