# Synthetic Zone Catalog — MEZ Stack v0.4.44

The MEZ Stack's **Compositional Zone Algebra** allows creating synthetic economic zones
by composing regulatory primitives from multiple jurisdictions. A synthetic zone is not a
real jurisdiction -- it is a deployment configuration that selects best-in-class regulatory
frameworks for each compliance domain, producing an optimal regulatory environment for a
specific use case.

Every synthetic zone is a first-class zone: it receives its own `zone.yaml`, merged
regpacks for each sourced domain, a compliance tensor evaluation across all active domains,
and full corridor connectivity to any other zone in the mesh.

AML/CFT is mandatory in every composition. No zone can be deployed without an AML/CFT
domain source.

---

## How Synthetic Zones Work

The composition process follows five steps:

1. **Select regulatory domains needed.** Choose from the 10 available domains based on
   the zone's use case:
   - `CorporateFormation` -- Company incorporation framework
   - `CivicCode` -- Civil/commercial code
   - `DigitalAssets` -- Digital asset/crypto regulation
   - `Arbitration` -- Dispute resolution framework
   - `Tax` -- Tax regime (corporate, withholding, VAT)
   - `AmlCft` -- AML/CFT compliance framework (mandatory)
   - `DataPrivacy` -- Data protection regulation
   - `Licensing` -- Business licensing regime
   - `PaymentRails` -- Payment system access
   - `Securities` -- Securities regulation

2. **Source each domain from a specific jurisdiction.** Each domain is sourced from a
   jurisdiction that provides the regulatory framework for that domain. The jurisdiction
   must have a regpack available for the domain.

3. **Validate composition.** The composition validator enforces:
   - AML/CFT domain must be present (mandatory)
   - No duplicate domains (each domain sourced at most once)
   - All referenced jurisdictions must have regpacks available
   - No circular dependencies between sourced domains

4. **Generate zone.yaml with merged regpacks and compliance domains.** The composition
   engine merges regpacks from all source jurisdictions, generates the compliance tensor
   configuration, and produces a complete zone manifest.

5. **Deploy as a first-class zone with full corridor connectivity.** The synthetic zone
   can participate in corridors, exchange receipts, and interoperate with any other zone
   in the mesh.

```bash
mez zone compose --spec composition.yaml --output jurisdictions/synth-my-zone/
```

---

## Reference Implementations

Two synthetic zones are already implemented and available as reference:

| Zone ID | Name | Path |
|---|---|---|
| `synth-atlantic-fintech` | Atlantic Fintech Hub | `jurisdictions/synth-atlantic-fintech/` |
| `synth-pacific-trade` | Pacific Trade Hub | `jurisdictions/synth-pacific-trade/` |

Both include `composition.yaml` (input spec) and `zone.yaml` (generated output).

---

## Catalog: 12 High-Value Synthetic Zone Compositions

---

### 1. synth-atlantic-fintech (Implemented)

**Atlantic Fintech Hub**

**Use case:** US-domiciled fintech companies seeking regulated digital asset operations
with favorable tax treatment and credible international arbitration. Serves US/EU/ME
markets from a single regulatory configuration.

| Domain | Source | Rationale |
|---|---|---|
| CorporateFormation | `us-de` | Delaware DGCL Title 8 -- gold standard for US corporate formation |
| CivicCode | `us-ny` | New York General Obligations Law (UCC Article 12 for digital assets) |
| DigitalAssets | `ae-abudhabi-adgm` | ADGM FSMR 2015 -- comprehensive digital asset framework |
| Arbitration | `hk` | Hong Kong Cap 609 Arbitration Ordinance (HKIAC) |
| Tax | `sg` | Singapore ITA -- 17% flat corporate, 9% GST, extensive treaty network |
| AmlCft | `ae` | UAE Federal Decree-Law No. 20/2018 (FATF-aligned) |

```yaml
# jurisdictions/synth-atlantic-fintech/composition.yaml
zone_name: Atlantic Fintech Hub
zone_id: org.momentum.mez.zone.synthetic.atlantic-fintech
jurisdiction_id: synth-atlantic-fintech
primary_jurisdiction: us
layers:
  - domain: corporate_formation
    source: us-de
  - domain: civic_code
    source: us-ny
  - domain: digital_assets
    source: ae-abudhabi-adgm
  - domain: arbitration
    source: hk
  - domain: tax
    source: sg
  - domain: aml_cft
    source: ae
```

**Key statutory references:**
- Delaware General Corporation Law, Title 8 Del. Code
- New York General Obligations Law; UCC Article 12
- ADGM Financial Services and Markets Regulations 2015
- Hong Kong Arbitration Ordinance (Cap 609)
- Singapore Income Tax Act (Cap 134)
- UAE Federal Decree-Law No. 20/2018 on Anti-Money Laundering

---

### 2. synth-pacific-trade (Implemented)

**Pacific Trade Hub**

**Use case:** Pacific Rim trade entities seeking digital-native settlement with Hong Kong's
territorial tax system and Singapore's arbitration and AML framework. Optimized for
cross-Pacific trade facilitation.

| Domain | Source | Rationale |
|---|---|---|
| CorporateFormation | `us-ca` | California Corp. Code -- tech-friendly incorporation |
| CivicCode | `us-ca` | California Civil Code -- well-established commercial law |
| DigitalAssets | `ae-dubai-difc` | DIFC Digital Assets Law No. 2/2024 |
| Arbitration | `sg` | Singapore International Arbitration Act (Cap 143A) -- SIAC |
| Tax | `hk` | Hong Kong IRO -- 16.5% flat corporate, territorial basis |
| AmlCft | `sg` | Singapore MAS Notice 626 + CDSA (Cap 65A) + TSOFA (Cap 325) |

```yaml
# jurisdictions/synth-pacific-trade/composition.yaml
zone_name: Pacific Trade Hub
zone_id: org.momentum.mez.zone.synthetic.pacific-trade
jurisdiction_id: synth-pacific-trade
primary_jurisdiction: us
layers:
  - domain: corporate_formation
    source: us-ca
  - domain: civic_code
    source: us-ca
  - domain: digital_assets
    source: ae-dubai-difc
  - domain: arbitration
    source: sg
  - domain: tax
    source: hk
  - domain: aml_cft
    source: sg
```

**Key statutory references:**
- California Corporations Code, Division 1
- California Civil Code
- DIFC Law No. 2 of 2024 (Digital Assets)
- Singapore International Arbitration Act (Cap 143A)
- Hong Kong Inland Revenue Ordinance (Cap 112)
- Singapore Corruption, Drug Trafficking and Other Serious Crimes Act (Cap 65A)
- Singapore MAS Notice 626

---

### 3. synth-gulf-trade-bridge

**Gulf Cooperation Council Trade Bridge**

**Use case:** Intra-GCC trade and logistics operations. Combines UAE's free zone
infrastructure with Bahrain's progressive digital asset regulation and Qatar's
financial licensing. Designed for regional trade companies operating across all six
GCC member states.

| Domain | Source | Rationale |
|---|---|---|
| CorporateFormation | `ae-dubai-dmcc` | DMCC free zone -- 100% foreign ownership, purpose-built for commodities trade |
| CivicCode | `ae` | UAE Federal Civil Code (Law No. 5/1985) -- harmonized GCC commercial baseline |
| DigitalAssets | `bh` | Bahrain CBB crypto-asset module -- first GCC central bank framework |
| Arbitration | `ae-dubai-difc` | DIFC-LCIA Arbitration Centre -- English common law arbitration in UAE |
| Tax | `ae` | UAE Corporate Tax Law (Federal Decree-Law No. 47/2022) -- 0% personal, 9% corporate |
| AmlCft | `ae` | UAE Federal Decree-Law No. 20/2018 (FATF-aligned, mutual evaluation 2024) |
| PaymentRails | `ae` | UAE Central Bank (CBUAE) -- IPP instant payments, WPS compliance |
| Licensing | `qa-qfc` | Qatar Financial Centre licensing -- streamlined GCC financial services |

```yaml
# jurisdictions/synth-gulf-trade-bridge/composition.yaml
zone_name: Gulf Cooperation Council Trade Bridge
zone_id: org.momentum.mez.zone.synthetic.gulf-trade-bridge
jurisdiction_id: synth-gulf-trade-bridge
primary_jurisdiction: ae
layers:
  - domain: corporate_formation
    source: ae-dubai-dmcc
  - domain: civic_code
    source: ae
  - domain: digital_assets
    source: bh
  - domain: arbitration
    source: ae-dubai-difc
  - domain: tax
    source: ae
  - domain: aml_cft
    source: ae
  - domain: payment_rails
    source: ae
  - domain: licensing
    source: qa-qfc
```

**Key statutory references:**
- DMCC Company Regulations 2023
- UAE Federal Civil Transactions Law (No. 5/1985)
- Bahrain CBB Rulebook Volume 6 (Crypto-Asset Module)
- DIFC Arbitration Law (DIFC Law No. 1/2008)
- UAE Federal Decree-Law No. 47/2022 (Corporate Tax)
- UAE Federal Decree-Law No. 20/2018 (AML/CFT)
- CBUAE Regulations on Stored Value Facilities and Retail Payment Services
- QFC Financial Services Regulations

---

### 4. synth-south-asia-remittance

**South Asia Remittance Corridor**

**Use case:** Pakistan-India-Bangladesh remittance optimization. Built around Pakistan's
SIFC framework and SBP Raast instant payment system. Addresses the South Asian remittance
market (>$100B annually) with compliant digital channels and favorable regulatory treatment.

| Domain | Source | Rationale |
|---|---|---|
| CorporateFormation | `pk-sifc` | Pakistan SIFC -- dedicated framework for strategic investment facilitation |
| CivicCode | `pk` | Pakistan Civil Code and Contract Act 1872 |
| Tax | `pk` | Pakistan FBR framework -- withholding tax on remittances, FTR regime |
| AmlCft | `pk` | Pakistan AML Act 2010 + FATF Action Plan compliance (exited grey list 2025) |
| PaymentRails | `pk` | SBP Raast -- Pakistan's instant payment system (P2P, P2M, B2B) |
| Securities | `sg` | MAS Securities and Futures Act -- credible securities framework for investor confidence |
| DataPrivacy | `sg` | Singapore PDPA 2012 -- mature data protection baseline |

```yaml
# jurisdictions/synth-south-asia-remittance/composition.yaml
zone_name: South Asia Remittance Corridor
zone_id: org.momentum.mez.zone.synthetic.south-asia-remittance
jurisdiction_id: synth-south-asia-remittance
primary_jurisdiction: pk
layers:
  - domain: corporate_formation
    source: pk-sifc
  - domain: civic_code
    source: pk
  - domain: tax
    source: pk
  - domain: aml_cft
    source: pk
  - domain: payment_rails
    source: pk
  - domain: securities
    source: sg
  - domain: data_privacy
    source: sg
```

**Key statutory references:**
- Pakistan Special Investment Facilitation Council Act 2023
- Pakistan Contract Act 1872
- Pakistan Income Tax Ordinance 2001 (as amended by Finance Act)
- Pakistan Anti-Money Laundering Act 2010
- SBP Raast Regulations (Circular PSD No. 01/2021)
- Singapore Securities and Futures Act 2001 (Cap 289)
- Singapore Personal Data Protection Act 2012

---

### 5. synth-european-digital-bank

**European Digital Banking Zone**

**Use case:** Pan-European digital banking charter. Combines Estonia's frictionless
digital company formation with Luxembourg's financial center infrastructure, Swiss
crypto innovation, and strict German data protection. Designed for digital-first
banks serving the EU single market.

| Domain | Source | Rationale |
|---|---|---|
| CorporateFormation | `ee` | Estonia e-Residency -- fully digital company formation, EU jurisdiction |
| CivicCode | `lu` | Luxembourg Civil Code -- continental European baseline, financial center law |
| DigitalAssets | `ch-zug` | Swiss FINMA crypto framework (Crypto Valley) -- mature token classification |
| Tax | `ie` | Ireland -- 12.5% corporate tax, EU-compliant, extensive treaty network |
| AmlCft | `lu` | Luxembourg CSSF AML/CFT framework -- EU 6AMLD compliant |
| Licensing | `gb` | UK FCA licensing -- recognized globally, e-money and banking authorizations |
| DataPrivacy | `de` | German BDSG + EU GDPR -- strictest EU implementation, gold standard |
| PaymentRails | `nl` | Dutch DNB + PSD2 -- Netherlands as EU payment innovation hub |

```yaml
# jurisdictions/synth-european-digital-bank/composition.yaml
zone_name: European Digital Banking Zone
zone_id: org.momentum.mez.zone.synthetic.european-digital-bank
jurisdiction_id: synth-european-digital-bank
primary_jurisdiction: lu
layers:
  - domain: corporate_formation
    source: ee
  - domain: civic_code
    source: lu
  - domain: digital_assets
    source: ch-zug
  - domain: tax
    source: ie
  - domain: aml_cft
    source: lu
  - domain: licensing
    source: gb
  - domain: data_privacy
    source: de
  - domain: payment_rails
    source: nl
```

**Key statutory references:**
- Estonia Commercial Code (Aringuseadustik) + e-Residency Regulation
- Luxembourg Civil Code (Code civil)
- Swiss Federal Act on Financial Services (FinSA) + Financial Institutions Act (FinIA)
- Irish Taxes Consolidation Act 1997 (as amended)
- Luxembourg Law of 12 November 2004 (AML/CFT, as amended)
- UK Financial Services and Markets Act 2000 (FSMA)
- German Federal Data Protection Act (BDSG) + EU GDPR (Regulation 2016/679)
- Dutch Financial Supervision Act (Wet op het financieel toezicht)

---

### 6. synth-africa-fintech-gateway

**Africa Fintech Gateway**

**Use case:** Pan-African fintech operations hub. Combines Mauritius as a holding
company jurisdiction with Kenya's mobile money infrastructure, South Africa's mature
AML framework, Nigeria's securities regulation, and Rwanda's fintech sandbox.
Addresses the African fintech market's need for regulatory clarity across multiple
high-growth economies.

| Domain | Source | Rationale |
|---|---|---|
| CorporateFormation | `mu` | Mauritius Global Business Company (GBC) -- Africa-oriented treaty network, CRS-compliant |
| CivicCode | `ke` | Kenya common law -- East African commercial baseline |
| Tax | `mu` | Mauritius 15% corporate + partial exemption system, Africa DTAs |
| AmlCft | `za` | South Africa FICA (Financial Intelligence Centre Act) -- FATF mutual evaluation 2021 |
| PaymentRails | `ke` | Kenya M-PESA ecosystem + CBK RTGS -- mobile money leadership |
| Securities | `ng` | Nigeria SEC framework -- ISA 2007, Rules on Digital Assets 2022 |
| DataPrivacy | `za` | South Africa POPIA (Protection of Personal Information Act) |
| Licensing | `rw` | Rwanda fintech sandbox (BNR Regulation No. 12/2021) |

```yaml
# jurisdictions/synth-africa-fintech-gateway/composition.yaml
zone_name: Africa Fintech Gateway
zone_id: org.momentum.mez.zone.synthetic.africa-fintech-gateway
jurisdiction_id: synth-africa-fintech-gateway
primary_jurisdiction: mu
layers:
  - domain: corporate_formation
    source: mu
  - domain: civic_code
    source: ke
  - domain: tax
    source: mu
  - domain: aml_cft
    source: za
  - domain: payment_rails
    source: ke
  - domain: securities
    source: ng
  - domain: data_privacy
    source: za
  - domain: licensing
    source: rw
```

**Key statutory references:**
- Mauritius Companies Act 2001 + Financial Services Act 2007
- Kenya Law of Contract Act (Cap 23)
- Mauritius Income Tax Act 1995
- South Africa Financial Intelligence Centre Act 38 of 2001 (FICA)
- Central Bank of Kenya National Payment System Regulations 2014
- Nigeria Investments and Securities Act 2007 + SEC Rules on Digital Assets 2022
- South Africa Protection of Personal Information Act 4 of 2013 (POPIA)
- Rwanda BNR Regulation No. 12/2021 on Fintech

---

### 7. synth-latam-trade-zone

**Latin America Trade Zone**

**Use case:** Cross-border trade in South America. Panama provides favorable
incorporation, Brazil supplies the civil code and instant payment infrastructure (PIX),
Uruguay offers stable tax treatment, and Chile provides securities regulation. Addresses
intra-LATAM trade corridors with regulatory clarity.

| Domain | Source | Rationale |
|---|---|---|
| CorporateFormation | `pa` | Panama Sociedad Empresarial de Responsabilidad (SEM) -- favorable foreign ownership |
| CivicCode | `br` | Brazilian Civil Code (Lei 10.406/2002) -- largest LATAM economy |
| Tax | `uy` | Uruguay 25% corporate, extensive bilateral treaty network, tax stability |
| AmlCft | `br` | Brazil COAF/BACEN -- Lei 9.613/1998 (AML), FATF member |
| PaymentRails | `br` | Brazil PIX instant payment system (BCB) -- 150M+ users |
| Securities | `cl` | Chile CMF (Comision para el Mercado Financiero) -- Ley 18.045 |
| Arbitration | `mx` | Mexico ICC arbitration -- Codigo de Comercio Title IV (UNCITRAL Model Law) |
| Licensing | `co` | Colombia SFC (Superintendencia Financiera de Colombia) |

```yaml
# jurisdictions/synth-latam-trade-zone/composition.yaml
zone_name: Latin America Trade Zone
zone_id: org.momentum.mez.zone.synthetic.latam-trade-zone
jurisdiction_id: synth-latam-trade-zone
primary_jurisdiction: pa
layers:
  - domain: corporate_formation
    source: pa
  - domain: civic_code
    source: br
  - domain: tax
    source: uy
  - domain: aml_cft
    source: br
  - domain: payment_rails
    source: br
  - domain: securities
    source: cl
  - domain: arbitration
    source: mx
  - domain: licensing
    source: co
```

**Key statutory references:**
- Panama Law 4 of 2009 (SEM) + General Corporation Law (Law 32 of 1927)
- Brazil Civil Code (Lei 10.406/2002)
- Uruguay Tax Code (Codigo Tributario) + Law 18.083 (Tax Reform)
- Brazil Lei 9.613/1998 (Anti-Money Laundering) + BCB Resolution 1/2020
- Brazil BCB PIX Regulations (Resolution BCB 1/2020)
- Chile Ley 18.045 (Securities Market Law) + CMF NCG 502
- Mexico Codigo de Comercio, Title IV (Commercial Arbitration)
- Colombia Organic Statute of the Financial System (Decree 663/1993)

---

### 8. synth-islamic-finance-hub

**Islamic Finance Digital Hub**

**Use case:** Shariah-compliant digital finance operations. Combines DIFC's
English common law environment with Bahrain's Shariah-compliant crypto framework,
Malaysia's Islamic banking expertise, and Saudi Arabia's CMA Shariah securities
framework. Designed for Islamic fintech companies and digital sukuk platforms.

| Domain | Source | Rationale |
|---|---|---|
| CorporateFormation | `ae-dubai-difc` | DIFC -- English common law, 0% tax, 100% foreign ownership |
| CivicCode | `ae-dubai-difc` | DIFC Contract Law (DIFC Law No. 6/2004) -- English common law |
| DigitalAssets | `bh` | Bahrain CBB -- Shariah-compliant crypto-asset rules within CBB Rulebook |
| Arbitration | `ae-dubai-difc` | DIFC-LCIA Arbitration Centre -- international recognition |
| Tax | `ae` | UAE -- 0% personal income tax, 9% corporate (with small business relief) |
| AmlCft | `ae` | UAE Federal Decree-Law No. 20/2018 (FATF-aligned) |
| Licensing | `my` | Malaysia BNM Islamic Financial Services Act 2013 (IFSA) |
| Securities | `sa` | Saudi Arabia CMA -- Shariah-compliant securities framework |

```yaml
# jurisdictions/synth-islamic-finance-hub/composition.yaml
zone_name: Islamic Finance Digital Hub
zone_id: org.momentum.mez.zone.synthetic.islamic-finance-hub
jurisdiction_id: synth-islamic-finance-hub
primary_jurisdiction: ae
layers:
  - domain: corporate_formation
    source: ae-dubai-difc
  - domain: civic_code
    source: ae-dubai-difc
  - domain: digital_assets
    source: bh
  - domain: arbitration
    source: ae-dubai-difc
  - domain: tax
    source: ae
  - domain: aml_cft
    source: ae
  - domain: licensing
    source: my
  - domain: securities
    source: sa
```

**Key statutory references:**
- DIFC Companies Law (DIFC Law No. 5/2018)
- DIFC Contract Law (DIFC Law No. 6/2004)
- Bahrain CBB Rulebook Volume 6 (Crypto-Asset Module, Shariah Standards)
- DIFC Arbitration Law (DIFC Law No. 1/2008) + DIFC-LCIA Rules
- UAE Federal Decree-Law No. 47/2022 (Corporate Tax)
- UAE Federal Decree-Law No. 20/2018 (AML/CFT)
- Malaysia Islamic Financial Services Act 2013 (IFSA, Act 759)
- Saudi Arabia Capital Market Law (Royal Decree M/30) + CMA Shariah Guidelines

---

### 9. synth-crypto-native-zone

**Crypto-Native Regulatory Zone**

**Use case:** Pure digital asset platform operating across Asia-Pacific and Middle East
markets. Optimized for token issuance, exchange operations, and custody services.
Combines the most crypto-forward regulatory frameworks available with credible
arbitration and AML compliance.

| Domain | Source | Rationale |
|---|---|---|
| CorporateFormation | `ky` | Cayman Islands -- Exempted Limited Partnership, crypto-fund standard |
| DigitalAssets | `ae-abudhabi-adgm` | ADGM FSRA -- comprehensive virtual asset framework, MiCA-interoperable |
| Arbitration | `sg` | SIAC -- internationally recognized, tech-savvy arbitration |
| Tax | `ky` | Cayman Islands -- 0% corporate, capital gains, withholding |
| AmlCft | `sg` | Singapore MAS framework + FATF Travel Rule compliance (PS Act 2019) |
| Securities | `hk` | Hong Kong SFC Virtual Asset Trading Platform regime (VASP licensing) |
| Licensing | `ae-dubai-dmcc` | DMCC Crypto Centre -- purpose-built crypto business license |
| DataPrivacy | `sg` | Singapore PDPA 2012 -- balanced data protection for digital platforms |

```yaml
# jurisdictions/synth-crypto-native-zone/composition.yaml
zone_name: Crypto-Native Regulatory Zone
zone_id: org.momentum.mez.zone.synthetic.crypto-native-zone
jurisdiction_id: synth-crypto-native-zone
primary_jurisdiction: ky
layers:
  - domain: corporate_formation
    source: ky
  - domain: digital_assets
    source: ae-abudhabi-adgm
  - domain: arbitration
    source: sg
  - domain: tax
    source: ky
  - domain: aml_cft
    source: sg
  - domain: securities
    source: hk
  - domain: licensing
    source: ae-dubai-dmcc
  - domain: data_privacy
    source: sg
```

**Key statutory references:**
- Cayman Islands Exempted Limited Partnership Act (2021 Revision)
- ADGM Financial Services and Markets Regulations 2015 (Virtual Assets Framework)
- Singapore International Arbitration Act (Cap 143A) + SIAC Rules
- Cayman Islands -- no direct taxation legislation (tax-neutral jurisdiction)
- Singapore Payment Services Act 2019 (PS Act) + MAS Notice PSN02
- Hong Kong Securities and Futures Ordinance (Cap 571) + SFC VASP Guidelines
- DMCC Crypto Centre Regulations 2023
- Singapore Personal Data Protection Act 2012

---

### 10. synth-green-finance-corridor

**Green Finance and ESG Corridor**

**Use case:** Sustainable finance and ESG compliance platform. Designed for green bond
issuance, carbon credit tokenization, and ESG reporting. Combines Luxembourg's
green finance center with Swiss tokenization capabilities, UK's Green Taxonomy,
and Germany's strict GDPR implementation.

| Domain | Source | Rationale |
|---|---|---|
| CorporateFormation | `lu` | Luxembourg Societe -- EU Green Bond Standard early adopter |
| CivicCode | `lu` | Luxembourg Civil Code -- established securitization law |
| DigitalAssets | `ch` | Swiss FINMA -- tokenized green bonds (DLT Act 2021) |
| Tax | `nl` | Netherlands Innovation Box (9% effective rate on qualifying IP) |
| AmlCft | `lu` | Luxembourg CSSF -- EU 6AMLD compliant, sustainable finance AML |
| Securities | `gb` | UK FCA Green Taxonomy + SDR (Sustainability Disclosure Requirements) |
| Licensing | `sg` | MAS Green Fintech taxonomy + Project Greenprint |
| DataPrivacy | `de` | German BDSG + GDPR -- strictest EU data protection implementation |

```yaml
# jurisdictions/synth-green-finance-corridor/composition.yaml
zone_name: Green Finance and ESG Corridor
zone_id: org.momentum.mez.zone.synthetic.green-finance-corridor
jurisdiction_id: synth-green-finance-corridor
primary_jurisdiction: lu
layers:
  - domain: corporate_formation
    source: lu
  - domain: civic_code
    source: lu
  - domain: digital_assets
    source: ch
  - domain: tax
    source: nl
  - domain: aml_cft
    source: lu
  - domain: securities
    source: gb
  - domain: licensing
    source: sg
  - domain: data_privacy
    source: de
```

**Key statutory references:**
- Luxembourg Law of 10 August 1915 on Commercial Companies (as amended)
- Luxembourg Civil Code (Code civil luxembourgeois)
- Swiss Federal Act on the Adaptation of Federal Law to Developments in DLT (2021)
- Netherlands Corporate Income Tax Act 1969 (Innovation Box, Art. 12b)
- Luxembourg Law of 12 November 2004 (AML/CFT)
- UK Financial Services and Markets Act 2000 + FCA Green Taxonomy + SDR (PS23/16)
- Singapore MAS Guidelines on Environmental Risk Management
- German Federal Data Protection Act (Bundesdatenschutzgesetz, BDSG)

---

### 11. synth-nordics-payments

**Nordic Payments Innovation Zone**

**Use case:** Cross-Nordic payment innovation platform. Combines Sweden's instant
payment infrastructure with Estonia's digital-first licensing, Finland's stable tax
environment, and Norway's robust AML framework. Designed for payment companies
serving the Nordic/Baltic region.

| Domain | Source | Rationale |
|---|---|---|
| CorporateFormation | `se` | Sweden Aktiebolag (AB) -- transparent corporate governance |
| CivicCode | `dk` | Danish Commercial Foundations Act + Sale of Goods Act |
| Tax | `fi` | Finland 20% corporate tax -- stable, predictable Nordic rate |
| AmlCft | `no` | Norway Finanstilsynet (FSA) -- FATF 4th round rated Largely Compliant |
| PaymentRails | `se` | Sweden RIX (Riksbank) + Swish instant payments |
| Licensing | `ee` | Estonia e-Money Institution license -- EU-passportable, digital-native |
| DataPrivacy | `fi` | Finnish Data Protection Ombudsman -- pragmatic GDPR implementation |
| Securities | `dk` | Danish Financial Supervisory Authority (Finanstilsynet) |

```yaml
# jurisdictions/synth-nordics-payments/composition.yaml
zone_name: Nordic Payments Innovation Zone
zone_id: org.momentum.mez.zone.synthetic.nordics-payments
jurisdiction_id: synth-nordics-payments
primary_jurisdiction: se
layers:
  - domain: corporate_formation
    source: se
  - domain: civic_code
    source: dk
  - domain: tax
    source: fi
  - domain: aml_cft
    source: no
  - domain: payment_rails
    source: se
  - domain: licensing
    source: ee
  - domain: data_privacy
    source: fi
  - domain: securities
    source: dk
```

**Key statutory references:**
- Swedish Companies Act (Aktiebolagslagen, SFS 2005:551)
- Danish Sale of Goods Act (Kobeloven) + Commercial Foundations Act
- Finnish Income Tax Act (Tuloverolaki 1535/1992)
- Norwegian Anti-Money Laundering Act (Hvitvaskingsloven, LOV-2018-06-01-23)
- Swedish Riksbank Act + Payment Services Act (Betaltjanstlagen, SFS 2010:751)
- Estonian Payment Institutions and E-Money Institutions Act
- Finnish Data Protection Act (Tietosuojalaki 1050/2018)
- Danish Financial Business Act (Lov om finansiel virksomhed)

---

### 12. synth-maritime-trade-hub

**Maritime Trade and Logistics Hub**

**Use case:** Shipping, trade finance, and logistics operations. Combines Singapore's
maritime incorporation with English maritime law, London arbitration, Hong Kong's
shipping tax exemption, and Dubai's logistics licensing. Designed for shipping
companies, trade finance platforms, and logistics operators.

| Domain | Source | Rationale |
|---|---|---|
| CorporateFormation | `sg` | Singapore -- world's busiest port, established maritime corporate regime |
| CivicCode | `gb` | English maritime law -- Carriage of Goods by Sea Act, global standard |
| Arbitration | `gb` | London Maritime Arbitrators Association (LMAA) -- dominant maritime forum |
| Tax | `hk` | Hong Kong shipping income exemption (Section 23B IRO) |
| AmlCft | `sg` | Singapore MAS -- FATF compliant, trade-based money laundering focus |
| PaymentRails | `hk` | Hong Kong FPS (Faster Payment System) + CHATS (USD/EUR/RMB RTGS) |
| Licensing | `ae-dubai-jafza` | JAFZA (Jebel Ali Free Zone Authority) -- logistics and trading license |
| Securities | `sg` | MAS trade finance framework -- regulated supply chain finance |

```yaml
# jurisdictions/synth-maritime-trade-hub/composition.yaml
zone_name: Maritime Trade and Logistics Hub
zone_id: org.momentum.mez.zone.synthetic.maritime-trade-hub
jurisdiction_id: synth-maritime-trade-hub
primary_jurisdiction: sg
layers:
  - domain: corporate_formation
    source: sg
  - domain: civic_code
    source: gb
  - domain: arbitration
    source: gb
  - domain: tax
    source: hk
  - domain: aml_cft
    source: sg
  - domain: payment_rails
    source: hk
  - domain: licensing
    source: ae-dubai-jafza
  - domain: securities
    source: sg
```

**Key statutory references:**
- Singapore Companies Act 1967 (Cap 50)
- UK Carriage of Goods by Sea Act 1992 + Marine Insurance Act 1906
- LMAA Terms (2021) + London Arbitration
- Hong Kong Inland Revenue Ordinance Section 23B (Shipping Profits Exemption)
- Singapore MAS Notice 626 + Corruption, Drug Trafficking and Other Serious Crimes Act
- Hong Kong FPS Rules + CHATS Operating Procedures (HKMA)
- JAFZA Rules and Regulations (as amended 2023)
- Singapore Securities and Futures Act 2001 (Cap 289)

---

## Generating Synthetic Zones

### From a composition spec

```bash
# Generate zone from composition spec
mez zone compose --spec jurisdictions/synth-my-zone/composition.yaml \
    --output jurisdictions/synth-my-zone/

# Validate the generated zone
mez zone validate jurisdictions/synth-my-zone/zone.yaml
```

### Corridor connectivity

```bash
# Verify corridor connectivity between zones
mez corridor mesh --zones pk-sifc,synth-my-zone,sg --format dot

# Test receipt exchange between synthetic zone and real jurisdiction
mez corridor test --source pk-sifc --dest synth-atlantic-fintech --receipts 10
```

### Multi-zone deployment

```bash
# Deploy in N-zone mesh
./deploy/scripts/demo-n-zone.sh pk-sifc synth-atlantic-fintech sg hk

# Deploy specific synthetic zone pair
./deploy/scripts/demo-two-zone.sh synth-atlantic-fintech synth-pacific-trade
```

### Compliance verification

```bash
# Check compliance tensor for a synthetic zone
mez tensor evaluate --zone synth-atlantic-fintech --entity test-entity-001

# Verify all domains are covered (no NotApplicable gaps)
mez tensor audit --zone synth-atlantic-fintech --strict
```

---

## Composition Constraints

The following constraints are enforced by the composition validator:

1. **AML/CFT mandatory.** Every synthetic zone must include an `aml_cft` domain source.
   Compositions without AML/CFT are rejected at validation time.

2. **No duplicate domains.** Each of the 10 regulatory domains may appear at most once
   in a composition. Sourcing `tax` from two jurisdictions is not permitted.

3. **Source jurisdiction availability.** Each domain source must reference a jurisdiction
   that has a regpack available for that domain. The validator checks against the
   regpack registry.

4. **Compliance tensor coverage.** In production mode (fail-closed), all domains present
   in the composition must be evaluable by the compliance tensor. Domains that would
   return `NotApplicable` in the absence of a regpack are rejected.

5. **Corridor compatibility.** Synthetic zones must be corridor-compatible with at least
   one real jurisdiction to be deployable. The corridor protocol does not distinguish
   between synthetic and real zones.
