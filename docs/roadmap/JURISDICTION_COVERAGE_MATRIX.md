# Jurisdiction Coverage Matrix — MEZ Stack v0.4.44

This matrix tracks deployment readiness across jurisdictions for the Momentum Economic Zones (MEZ) Stack. Each jurisdiction is classified into a tier based on the completeness of its zone manifest, regulatory pack content, national system adapters, and deployment profile. The matrix serves as the canonical reference for sovereign deployment planning and corridor expansion sequencing.

---

## Tier Definitions

| Tier | Definition | Criteria |
|------|------------|----------|
| Tier 1 | Production-ready | `zone.yaml` + regpack builder + real licensepack content + national adapters + profile |
| Tier 2 | Zone manifest with regpacks | `zone.yaml` + regpack builder + `compliance_domains` + national adapter stubs |
| Tier 3 | Enriched zone manifest | `zone.yaml` with `compliance_domains`, `national_adapters`, `key_management`, and profile reference; no regpack builder yet |
| Tier 4 | Planned | No zone manifest yet; jurisdiction identified for expansion |

---

## Current Coverage Summary

| Metric | Count |
|--------|-------|
| Tier 1 — Production Ready | 5 |
| Tier 2 — Zone Manifest with Regpacks | 17 |
| Tier 3 — Enriched Zone Manifest | 187 |
| Tier 4 — Planned Expansion | 0 |
| **Total Deployed Zones** | **209** |

All 209 zones have enriched `zone.yaml` manifests with `compliance_domains`, `national_adapters`, `key_management`, and `corridors` configuration. Zero scaffold manifests remain. Full corridor mesh: **21,736 autonomous corridors** (209 x 208 / 2).

---

## Coverage Matrix

### Tier 1 — Production Ready (5 zones)

These jurisdictions have complete zone manifests, real regpack and licensepack content, functioning national system adapters, and deployment profiles suitable for sovereign corridor activation.

| Jurisdiction ID | Zone Name | Country | zone.yaml | Regpack Builder | Licensepack | National Adapters | Profile | Notes |
|-----------------|-----------|---------|-----------|-----------------|-------------|-------------------|---------|-------|
| `pk-sifc` | Pakistan SIFC | Pakistan | Yes | Yes | Yes (70+ categories) | Yes (FBR IRIS, SECP, NADRA, SBP Raast) | `sovereign-govos` | Primary pilot zone; full Pack Trilogy (4 lawpack domains) |
| `ae-abudhabi-adgm` | Abu Dhabi Global Market | UAE | Yes | Yes | Yes | Yes | `sovereign-govos` | ADGM Financial Services Regulatory Authority framework |
| `sg` | Singapore | Singapore | Yes | Yes | Yes | Yes | `sovereign-govos` | MAS regulatory framework; Payment Services Act coverage |
| `hk` | Hong Kong | Hong Kong SAR | Yes | Yes | Yes | Yes | `sovereign-govos` | SFC + HKMA regulatory framework |
| `ky` | Cayman Islands | Cayman Islands | Yes | Yes | Yes | Yes | `sovereign-govos` | CIMA regulatory framework; fund administration focus |

### Tier 2 — Zone Manifest with Regpacks (17 zones)

These jurisdictions have zone manifests wired with real regpack digests and compliance domains. National adapter stubs are present but not yet connected to live government systems.

| Jurisdiction ID | Zone Name | Country | zone.yaml | Regpack Builder | Licensepack | National Adapters | Profile | Notes |
|-----------------|-----------|---------|-----------|-----------------|-------------|-------------------|---------|-------|
| `ae` | UAE Federal | UAE | Yes | Yes | Stub | Stub | Ref | Federal-level regulatory baseline |
| `ae-abudhabi` | Abu Dhabi Emirate | UAE | Yes | Yes | Stub | Stub | Ref | Emirate-level overlay on federal |
| `ae-abudhabi-kezad` | Khalifa Economic Zones | UAE | Yes | Yes | Stub | Stub | Ref | Multi-sector free zone |
| `ae-abudhabi-kizad` | Khalifa Industrial Zone | UAE | Yes | Yes | Stub | Stub | Ref | Industrial and logistics focus |
| `ae-abudhabi-masdar` | Masdar City | UAE | Yes | Yes | Stub | Stub | Ref | Clean energy and sustainability zone |
| `ae-abudhabi-twofour54` | twofour54 Media Zone | UAE | Yes | Yes | Stub | Stub | Ref | Media and entertainment free zone |
| `ae-dubai` | Dubai Emirate | UAE | Yes | Yes | Stub | Stub | Ref | Emirate-level overlay on federal |
| `ae-dubai-dhcc` | Dubai Healthcare City | UAE | Yes | Yes | Stub | Stub | Ref | Healthcare and wellness free zone |
| `ae-dubai-dic` | Dubai Internet City | UAE | Yes | Yes | Stub | Stub | Ref | Technology and innovation free zone |
| `ae-dubai-difc` | Dubai Intl Financial Centre | UAE | Yes | Yes | Stub | Stub | Ref | Common law financial centre; DFSA regulated |
| `ae-dubai-dmcc` | Dubai Multi Commodities Centre | UAE | Yes | Yes | Stub | Stub | Ref | Commodities trading free zone |
| `ae-dubai-dso` | Dubai Silicon Oasis | UAE | Yes | Yes | Stub | Stub | Ref | Technology park and free zone |
| `ae-dubai-dwtc` | Dubai World Trade Centre | UAE | Yes | Yes | Stub | Stub | Ref | Virtual assets and crypto regulatory sandbox |
| `ae-dubai-ifza` | Intl Free Zone Authority | UAE | Yes | Yes | Stub | Stub | Ref | Multi-activity free zone |
| `ae-dubai-jafza` | Jebel Ali Free Zone | UAE | Yes | Yes | Stub | Stub | Ref | Largest free zone; trade and logistics |
| `synth-atlantic-fintech` | Atlantic FinTech (Synthetic) | Synthetic | Yes | Yes | Synthetic | N/A | Ref | Synthetic reference zone for corridor testing |
| `synth-pacific-trade` | Pacific Trade (Synthetic) | Synthetic | Yes | Yes | Synthetic | N/A | Ref | Synthetic reference zone for corridor testing |

### Tier 3 — Enriched Zone Manifest (187 zones)

All Tier 3 zones have enriched `zone.yaml` manifests with `compliance_domains`, `national_adapters` (with jurisdiction-specific regulator endpoints), `key_management`, `corridors`, and profile references. They are ready for regpack builder integration (Tier 3 → Tier 2 promotion).

#### United States — States and Territories (56 zones)

All 56 US state and territory zones include FinCEN and IRS federal adapters plus state-specific banking/financial regulators. Each zone has `jurisdiction_stack: [us, us-<state>]`.

| Region | Zone IDs | Count | Notable |
|--------|----------|-------|---------|
| Crypto-forward states | `us-wy`, `us-tx`, `us-co`, `us-fl` | 4 | Wyoming DAO LLC Act, SPDI charter |
| Financial centers | `us-ny`, `us-de`, `us-ca`, `us-il`, `us-ct`, `us-ma` | 6 | NY BitLicense, DE DGCL, CA DFPI |
| All other states | `us-al` through `us-wi` | 46 | State banking regulators mapped |

<details>
<summary>Full US zone list (56 zones)</summary>

`us-ak`, `us-al`, `us-ar`, `us-as`, `us-az`, `us-ca`, `us-co`, `us-ct`, `us-dc`, `us-de`, `us-fl`, `us-ga`, `us-gu`, `us-hi`, `us-ia`, `us-id`, `us-il`, `us-in`, `us-ks`, `us-ky`, `us-la`, `us-ma`, `us-md`, `us-me`, `us-mi`, `us-mn`, `us-mo`, `us-mp`, `us-ms`, `us-mt`, `us-nc`, `us-nd`, `us-ne`, `us-nh`, `us-nj`, `us-nm`, `us-nv`, `us-ny`, `us-oh`, `us-ok`, `us-or`, `us-pa`, `us-pr`, `us-ri`, `us-sc`, `us-sd`, `us-tn`, `us-tx`, `us-ut`, `us-va`, `us-vi`, `us-vt`, `us-wa`, `us-wi`, `us-wv`, `us-wy`

</details>

#### UAE — Additional Emirates and Free Zones (4 zones)

| Jurisdiction ID | Zone Name | National Adapters | Notes |
|-----------------|-----------|-------------------|-------|
| `ae-ajman` | Ajman Free Zone | CBUAE, FTA | Light industrial free zone |
| `ae-rak` | Ras Al Khaimah | CBUAE, FTA | Emirate-level; emerging free zone activity |
| `ae-rak-rakez` | RAKEZ Free Zone | CBUAE, FTA, RAKEZ Authority | Ras Al Khaimah Economic Zone |
| `ae-sharjah` | Sharjah | CBUAE, FTA | Sharjah Emirate; SRTI Park, Hamriyah FZ |

#### Asia-Pacific (26 zones)

| Jurisdiction ID | Zone Name | Country | National Adapters | Notes |
|-----------------|-----------|---------|-------------------|-------|
| `jp` | Japan | Japan | FSA, NTA | FSA regulatory framework; JFSA virtual asset rules |
| `kr` | South Korea | South Korea | FSC, NTS | FSC/FSS framework; Virtual Asset User Protection Act |
| `in-gift` | GIFT City | India | IFSCA, CBDT | Gujarat Intl Finance Tec-City; IFSCA regulated |
| `in-ifsc` | IFSC Gujarat | India | IFSCA, CBDT | International Financial Services Centre; SEZ status |
| `my` | Malaysia | Malaysia | SC, BNM | SC Malaysia + BNM framework; Labuan offshore |
| `my-labuan` | Labuan IBFC | Malaysia | Labuan FSA | Labuan Intl Business and Financial Centre |
| `th` | Thailand | Thailand | SEC, BOT | SEC Thailand + BOT framework |
| `ph` | Philippines | Philippines | BSP, SEC | BSP + SEC framework; CEZA economic zone |
| `vn` | Vietnam | Vietnam | SBV, SSC | Emerging digital asset rules |
| `id` | Indonesia | Indonesia | OJK, BI | OJK + BI framework; Bappebti commodity futures |
| `au` | Australia | Australia | ASIC, APRA | Token mapping consultation; AFSL regime |
| `nz` | New Zealand | New Zealand | FMA, IRD | Financial Markets Conduct Act |
| `tw` | Taiwan | Taiwan | FSC | FSC Taiwan framework; virtual asset guidance |
| `bn` | Brunei | Brunei | AMBD | Emerging fintech sandbox |
| `mm` | Myanmar | Myanmar | CBM | Limited digital asset regulation |
| `cn` | China | China | PBOC, CSRC | PBOC + CSRC framework; DCEP/e-CNY |
| `cn-beijing` | Beijing | China | PBOC Beijing | National fintech regulatory sandbox |
| `cn-hainan` | Hainan FTP | China | Hainan FTP Authority | Free Trade Port; cross-border data flow pilot |
| `cn-hangzhou` | Hangzhou | China | PBOC Hangzhou | Blockchain innovation zone |
| `cn-shanghai` | Shanghai | China | PBOC Shanghai | Shanghai FTZ; fintech center |
| `cn-shenzhen` | Shenzhen | China | PBOC Shenzhen | Digital currency pilot zone; Qianhai FTZ |
| `lk` | Sri Lanka | Sri Lanka | CBSL | Central Bank of Sri Lanka |
| `bd` | Bangladesh | Bangladesh | BB | Bangladesh Bank framework |
| `kh` | Cambodia | Cambodia | NBC | National Bank of Cambodia; Bakong payment |
| `la` | Laos | Laos | BOL | Bank of the Lao PDR |
| `np` | Nepal | Nepal | NRB | Nepal Rastra Bank |

#### Middle East (13 zones)

| Jurisdiction ID | Zone Name | Country | National Adapters | Notes |
|-----------------|-----------|---------|-------------------|-------|
| `bh` | Bahrain | Bahrain | CBB | CBB framework; fintech sandbox; crypto-friendly |
| `bh-bfb` | Bahrain FinTech Bay | Bahrain | CBB Sandbox | FinTech hub within regulatory sandbox |
| `qa` | Qatar | Qatar | QCB, QFMA | QCB + QFMA framework |
| `qa-qfc` | Qatar Financial Centre | Qatar | QFCRA | QFC Regulatory Authority; common law financial centre |
| `om` | Oman | Oman | CBO, CMA | Duqm SEZ |
| `sa` | Saudi Arabia | Saudi Arabia | CMA, SAMA | Vision 2030 digital economy |
| `sa-neom` | NEOM | Saudi Arabia | NEOM Authority | Greenfield special regulatory framework |
| `jo` | Jordan | Jordan | CBJ, JSC | Aqaba SEZ |
| `eg` | Egypt | Egypt | CBE, FRA | Suez Canal Economic Zone |
| `kw` | Kuwait | Kuwait | CBK, CMA | Kuwait Financial Centre |
| `lb` | Lebanon | Lebanon | BDL | Banque du Liban |
| `iq` | Iraq | Iraq | CBI | Central Bank of Iraq |
| `pk-fed` | Pakistan Federal | Pakistan | SBP, SECP, FBR, NADRA | Federal zone; parent of pk-sifc |

#### Africa (18 zones)

| Jurisdiction ID | Zone Name | Country | National Adapters | Notes |
|-----------------|-----------|---------|-------------------|-------|
| `ke` | Kenya | Kenya | CBK, CMA | M-PESA ecosystem; Nairobi IFC initiative |
| `ng` | Nigeria | Nigeria | CBN, SEC | Lekki Free Zone; SEC digital asset rules 2022 |
| `za` | South Africa | South Africa | SARB, FSCA | IDZ program; POPIA data protection |
| `mu` | Mauritius | Mauritius | FSC | Global Business License framework |
| `rw` | Rwanda | Rwanda | BNR, CMA | Kigali IFC; fintech sandbox |
| `gh` | Ghana | Ghana | BOG, SEC | Emerging digital asset rules |
| `tz` | Tanzania | Tanzania | BOT | EPZ program |
| `tz-zanzibar` | Zanzibar | Tanzania | BOT Zanzibar | Zanzibar Investment Promotion Authority |
| `sc` | Seychelles | Seychelles | CBS, FSA | IBC framework; offshore financial center |
| `ma` | Morocco | Morocco | BAM, AMMC | Bank Al-Maghrib |
| `tn` | Tunisia | Tunisia | BCT | Banque Centrale de Tunisie |
| `et` | Ethiopia | Ethiopia | NBE | National Bank of Ethiopia |
| `ug` | Uganda | Uganda | BOU, CMA | Bank of Uganda; CMA securities |
| `cm` | Cameroon | Cameroon | BEAC | CEMAC zone; Bank of Central African States |
| `sn` | Senegal | Senegal | BCEAO | WAEMU zone; BCEAO central bank |
| `ci` | Cote d'Ivoire | Cote d'Ivoire | BCEAO | WAEMU zone; Abidjan financial center |
| `ag` | Antigua and Barbuda | Antigua and Barbuda | ECCB | ECCU zone; digital asset framework |

#### Europe (21 zones)

| Jurisdiction ID | Zone Name | Country | National Adapters | Notes |
|-----------------|-----------|---------|-------------------|-------|
| `gb` | United Kingdom | UK | FCA, HMRC | Financial Services and Markets Act; FCA crypto regime |
| `gb-gi` | Gibraltar | Gibraltar | GFSC | DLT framework; established crypto regulation |
| `ie` | Ireland | Ireland | CBI, Revenue | EU MiCA implementation; 12.5% corporate tax |
| `lu` | Luxembourg | Luxembourg | CSSF | EU MiCA; fund domiciliation hub |
| `ch` | Switzerland | Switzerland | FINMA | DLT Act; established crypto rules |
| `ch-zug` | Zug Crypto Valley | Switzerland | FINMA Zug | Cantonal overlay; Crypto Valley ecosystem |
| `li` | Liechtenstein | Liechtenstein | FMA | Token and TT Service Provider Act (TVTG) |
| `ee` | Estonia | Estonia | EFSA | EU MiCA; e-Residency program |
| `mt` | Malta | Malta | MFSA | Virtual Financial Assets Act |
| `cy` | Cyprus | Cyprus | CySEC | EU MiCA implementation |
| `de` | Germany | Germany | BaFin, BBk | BaFin crypto custody license; EU MiCA |
| `fr` | France | France | AMF, ACPR | PSAN registration; EU MiCA |
| `nl` | Netherlands | Netherlands | DNB, AFM | PSD2 innovation hub; EU MiCA |
| `es` | Spain | Spain | CNMV, BdE | EU MiCA; sandbox framework |
| `pt` | Portugal | Portugal | BdP, CMVM | NHR tax regime; emerging crypto framework |
| `it` | Italy | Italy | CONSOB, BdI | EU MiCA; OAM crypto registration |
| `at` | Austria | Austria | FMA | EU MiCA; FMA crypto supervision |
| `se` | Sweden | Sweden | FI, Riksbank | e-krona CBDC pilot; EU MiCA |
| `dk` | Denmark | Denmark | DFSA | Finanstilsynet; EU MiCA |
| `fi` | Finland | Finland | FIN-FSA | 20% corporate tax; EU MiCA |
| `no` | Norway | Norway | Finanstilsynet | EEA/EFTA; robust AML framework |

#### Americas — Non-US (22 zones)

| Jurisdiction ID | Zone Name | Country | National Adapters | Notes |
|-----------------|-----------|---------|-------------------|-------|
| `ca` | Canada | Canada | CSA, OSFI | MSB registration; federal framework |
| `ca-on` | Ontario | Canada | OSC | Provincial securities overlay |
| `bm` | Bermuda | Bermuda | BMA | Digital Asset Business Act |
| `bs` | Bahamas | Bahamas | SCB | DARE Act; Sand Dollar CBDC |
| `vg` | British Virgin Islands | BVI | BVI FSC | Virtual Assets Service Providers Act |
| `bb` | Barbados | Barbados | CBB, FSC | Emerging fintech rules |
| `pa` | Panama | Panama | SMV, SBP | Crypto law (Ley 129) |
| `cr` | Costa Rica | Costa Rica | SUGEF | Emerging digital asset rules |
| `br` | Brazil | Brazil | BCB, CVM | Crypto asset law (14.478/2022); PIX payments |
| `mx` | Mexico | Mexico | CNBV, Banxico | FinTech Law (Ley Fintech) |
| `co` | Colombia | Colombia | SFC, BanRep | Regulatory sandbox |
| `cl` | Chile | Chile | CMF, BCCh | FinTech Law |
| `ar` | Argentina | Argentina | CNV, BCRA | Digital asset PSP rules |
| `uy` | Uruguay | Uruguay | BCU, SSF | Emerging fintech regulation |
| `py` | Paraguay | Paraguay | BCP, CNV | Crypto mining law |
| `sv` | El Salvador | El Salvador | BCR, SSF | Bitcoin legal tender; CNAD oversight |
| `hn-prospera` | Prospera ZEDE | Honduras | Prospera Authority | Charter city; special regulatory framework |
| `tc` | Turks and Caicos | Turks and Caicos | TCIFSC | Financial Services Commission |
| `dm` | Dominica | Dominica | ECCB | ECCU zone; offshore financial center |
| `gd` | Grenada | Grenada | ECCB | ECCU zone |
| `lc` | Saint Lucia | Saint Lucia | ECCB | ECCU zone |
| `vc` | Saint Vincent | Saint Vincent | ECCB, FSA | ECCU zone; emerging fintech |
| `jm` | Jamaica | Jamaica | BOJ, FSC | Bank of Jamaica; JAM-DEX CBDC |
| `tt` | Trinidad and Tobago | Trinidad and Tobago | CBTT, TTSEC | Twin-island financial center |

#### Central Asia (6 zones)

| Jurisdiction ID | Zone Name | Country | National Adapters | Notes |
|-----------------|-----------|---------|-------------------|-------|
| `kz` | Kazakhstan | Kazakhstan | NBK, AFSA | National Bank + AIFC parent |
| `kz-aifc` | Astana IFC | Kazakhstan | AFSA | Common law financial center; AIFC Fintech Lab |
| `kz-alatau` | Alatau IT City | Kazakhstan | Alatau Authority | Technology park; IT special zone |
| `uz` | Uzbekistan | Uzbekistan | CBU | NAPM crypto framework |
| `ge` | Georgia | Georgia | NBG | National Bank of Georgia; flat tax regime |

#### Synthetic Zones — New (18 zones)

18 new synthetic zones created via compositional zone algebra. Each sources regulatory domains from multiple jurisdictions. See `docs/roadmap/SYNTHETIC_ZONE_CATALOG.md` for full composition specs.

| Jurisdiction ID | Zone Name | Primary | Domains | Use Case |
|-----------------|-----------|---------|---------|----------|
| `synth-gulf-trade-bridge` | GCC Trade Bridge | `ae` | 8 | Intra-GCC trade and logistics |
| `synth-south-asia-remittance` | South Asia Remittance | `pk` | 7 | Pakistan-India-Bangladesh remittance |
| `synth-european-digital-bank` | European Digital Bank | `lu` | 8 | Pan-EU digital banking charter |
| `synth-africa-fintech-gateway` | Africa Fintech Gateway | `mu` | 8 | Pan-African fintech hub |
| `synth-latam-trade-zone` | LATAM Trade Zone | `pa` | 8 | Cross-border South American trade |
| `synth-islamic-finance-hub` | Islamic Finance Hub | `ae` | 8 | Shariah-compliant digital finance |
| `synth-crypto-native-zone` | Crypto-Native Zone | `ky` | 8 | Pure digital asset platform |
| `synth-green-finance-corridor` | Green Finance Corridor | `lu` | 8 | Sustainable finance and ESG |
| `synth-nordics-payments` | Nordic Payments Zone | `se` | 8 | Cross-Nordic payment innovation |
| `synth-maritime-trade-hub` | Maritime Trade Hub | `sg` | 8 | Shipping and trade finance |
| `synth-us-digital-asset` | US Digital Asset Zone | `us` | 6 | US interstate digital asset operations |
| `synth-caribbean-digital-cluster` | Caribbean Digital Cluster | `ky` | 7 | Caribbean digital asset operations |
| `synth-central-asian-gateway` | Central Asian Gateway | `kz` | 7 | Central Asian trade corridor |
| `synth-indo-pacific-trade` | Indo-Pacific Trade | `sg` | 8 | India-ASEAN-Pacific trade |
| `synth-mediterranean-fintech` | Mediterranean Fintech | `mt` | 7 | Mediterranean digital finance |
| `synth-pacific-islands-development` | Pacific Islands Dev | `nz` | 6 | Pacific Islands economic development |
| `synth-east-african-innovation` | East African Innovation | `ke` | 8 | East African fintech hub |
| `synth-swiss-asian-bridge` | Swiss-Asian Bridge | `ch` | 8 | Swiss-Liechtenstein-Asian innovation |

### Tier 4 — Planned Expansion (0 zones)

All previously planned jurisdictions have been promoted to Tier 3 with enriched zone manifests. No jurisdictions remain at Tier 4.

---

## Expansion Roadmap

New jurisdictions progress through tiers via the following pipeline:

### Step 1: Identify Regulatory Framework
- Map the jurisdiction's financial regulator(s), licensing authority, AML/CFT framework, tax authority, and data protection regime.
- Determine whether the jurisdiction has economic zone or free zone legislation that creates sub-jurisdictional regulatory envelopes.
- Assess corridor demand: bilateral trade volume, remittance corridors, and FDI flows with existing Tier 1/2 zones.

### Step 2: Create Enriched Zone Manifest (New → Tier 3)
- Generate `zone.yaml` with `jurisdiction_stack`, `compliance_domains`, `national_adapters`, `key_management`, and profile reference.
- Map compliance domains to the jurisdiction's regulatory structure.
- Configure national adapter endpoints for the jurisdiction's regulators.
- Command: `mez zone init --jurisdiction <id> --template starter` then enrich manually.

### Step 3: Build Regpack Content (Tier 3 → Tier 2)
- Populate lawpacks with jurisdiction-specific statutes, regulations, and rules.
- Build regpacks with domain-specific compliance rules (TAX, AML_CFT, LICENSING, SECURITIES, etc.).
- Compute CAS digests: `mez regpack build --jurisdiction <id> --all-domains --store`
- Wire regpack digests into `zone.yaml`.

### Step 4: Implement National Adapters (Tier 2 → Tier 1)
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

Any combination of Tier 1-3 regulatory primitives can be composed into synthetic zones using `mez zone compose`. The stack now contains **20 synthetic zones** (2 Tier 2 reference zones + 18 Tier 3 compositions). Synthetic zones enable:

- **Corridor testing**: Create paired synthetic zones to test receipt chain exchange, fork resolution, and checkpoint verification without sovereign dependencies.
- **Regulatory sandbox simulation**: Compose a synthetic zone that inherits compliance domains from multiple jurisdictions for sandbox experimentation.
- **Pre-deployment validation**: Before a jurisdiction reaches Tier 1, compose a synthetic zone using its partial regpack content alongside known-good primitives from Tier 1 zones to validate the compliance pipeline.
- **Regional trade optimization**: Compose synthetic zones that combine best-in-class regulatory frameworks across regions (e.g., GCC Trade Bridge, LATAM Trade Zone, Nordic Payments).

See `docs/roadmap/SYNTHETIC_ZONE_CATALOG.md` for the full catalog of 20 synthetic zone compositions.

---

## Full Corridor Mesh

With 209 zones, the full autonomous corridor mesh comprises **21,736 corridors**:

```
mez corridor mesh --all --format dot    # Generate full mesh DOT graph
mez corridor mesh --all --format json   # Generate adjacency list
```

See `docs/roadmap/CORRIDOR_MESH_TOPOLOGY.md` for mesh topology documentation, corridor type breakdown, and visualization instructions.

---

## Appendix: Compliance Domain Coverage by Tier

| Compliance Domain | Tier 1 | Tier 2 | Tier 3 | Notes |
|-------------------|--------|--------|--------|-------|
| TAX | Full | Partial | Declared | Withholding rules, rates, treaty networks |
| AML_CFT | Full | Partial | Declared | CDD/EDD, STR, sanctions screening |
| LICENSING | Full | Stub | Declared | License categories, fees, renewal rules |
| SECURITIES | Full | Stub | Declared | Offering rules, exemptions, reporting |
| BANKING | Full | Stub | Declared | Capital requirements, reserve ratios |
| DATA_PRIVACY | Full | Stub | Declared | Data localization, consent, retention |
| SANCTIONS | Full | Partial | Declared | SDN/consolidated lists, screening rules |
| CONSUMER_PROTECTION | Partial | -- | Declared | Disclosure, cooling-off, redress |
| INSURANCE | Partial | -- | -- | Capital, solvency, policyholder protection |
| ENVIRONMENTAL | Partial | -- | -- | Carbon reporting, ESG disclosure |

"Declared" means the domain is listed in `compliance_domains` and mapped to the jurisdiction's regulator, but no regpack content exists yet. Promotion to Tier 2 requires populating regpack rules for each declared domain.

---

*Last updated: 2026-02-20*
*Source: MEZ Stack v0.4.44-GENESIS — full network topology deployment*
