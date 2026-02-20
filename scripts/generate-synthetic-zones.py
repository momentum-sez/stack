#!/usr/bin/env python3
"""Generate synthetic zone compositions (zones 3-12 from catalog + 8 new).

Each synthetic zone gets:
  - jurisdictions/synth-{id}/composition.yaml
  - jurisdictions/synth-{id}/zone.yaml
  - profiles/synthetic-synth-{id}/profile.yaml

Usage:
    python3 scripts/generate-synthetic-zones.py
"""

import os

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
JURISDICTIONS_DIR = os.path.join(REPO_ROOT, "jurisdictions")
PROFILES_DIR = os.path.join(REPO_ROOT, "profiles")

# Regpack digests from existing builders
AE_REGPACKS = [
    ("financial", "ae", "5ce776c2d59b4fd9ec1f6bdaea7317550fd0b7d1792be86a475b4db6602a685b"),
    ("sanctions", "ae", "34bb8fcb760935931216ab810fb3e66caa6980125f7e88f588eed8a0747509c0"),
]
SG_REGPACKS = [
    ("financial", "sg", "4d2a1f73a440fc67f5281e70436737b340e114173138cddd20f98c60d0154496"),
    ("sanctions", "sg", "2afd914d470a8d8c6934c2195a51d8130637ded17ca714732ca0ff5ee3c25108"),
]
HK_REGPACKS = [
    ("financial", "hk", "89dc928a5460dc90ef39c5557c9563b395da4560e5004b868adbe013d1b6d256"),
    ("sanctions", "hk", "6bd125db0d30631dd446a40e4fde9bbafd13c1e292d97f8fc060c7c235c2e204"),
]
PK_REGPACKS = [
    ("financial", "pk", "444ddded8419d9dedf8344a54063d7cd80c0148338c78bbe77a47baa44dd392f"),
    ("sanctions", "pk", "e59056a2b9bdbf3e452857b1cbdc06b5cdff3e29f56de1e235475e8a4a57506f"),
]
KY_REGPACKS = [
    ("financial", "ky", "b2f8c1a6d953e784f21b5407dc6e83fea0901bb8c13c4a7e9852a1d1fc0ab3e5"),
    ("sanctions", "ky", "71a4e8c0d23f59b7e816f2430ad97c5be3f10629d854a71c6b83e2f0194d58a7"),
]

REGPACK_MAP = {"ae": AE_REGPACKS, "sg": SG_REGPACKS, "hk": HK_REGPACKS,
               "pk": PK_REGPACKS, "ky": KY_REGPACKS}

COMPLIANCE = ["aml", "consumer_protection", "corporate", "data_privacy",
              "kyc", "licensing", "sanctions", "securities", "tax"]

SYNTH_ZONES = []

def synth(sid, name, primary, layers, use_case, statutory_refs):
    """Register a synthetic zone."""
    SYNTH_ZONES.append(dict(
        sid=sid, name=name, primary=primary, layers=layers,
        use_case=use_case, statutory_refs=statutory_refs,
    ))

# ═══════════════════════════════════════════════════════════════════════════
# CATALOG ZONES 3-12 (from SYNTHETIC_ZONE_CATALOG.md)
# ═══════════════════════════════════════════════════════════════════════════

synth("gulf-trade-bridge", "Gulf Cooperation Council Trade Bridge", "ae", [
    ("corporate_formation", "ae-dubai-dmcc"),
    ("civic_code", "ae"),
    ("digital_assets", "bh"),
    ("arbitration", "ae-dubai-difc"),
    ("tax", "ae"),
    ("aml_cft", "ae"),
    ("payment_rails", "ae"),
    ("licensing", "qa-qfc"),
], "Intra-GCC trade and logistics. Combines UAE free zone infrastructure with Bahrain crypto regulation and Qatar financial licensing.",
[
    "DMCC Company Regulations 2023",
    "UAE Federal Civil Transactions Law (No. 5/1985)",
    "Bahrain CBB Rulebook Volume 6 (Crypto-Asset Module)",
    "DIFC Arbitration Law (DIFC Law No. 1/2008)",
    "UAE Federal Decree-Law No. 47/2022 (Corporate Tax)",
    "UAE Federal Decree-Law No. 20/2018 (AML/CFT)",
    "CBUAE Regulations on Stored Value Facilities",
    "QFC Financial Services Regulations",
])

synth("south-asia-remittance", "South Asia Remittance Corridor", "pk", [
    ("corporate_formation", "pk-sifc"),
    ("civic_code", "pk"),
    ("tax", "pk"),
    ("aml_cft", "pk"),
    ("payment_rails", "pk"),
    ("securities", "sg"),
    ("data_privacy", "sg"),
], "Pakistan-India-Bangladesh remittance optimization. Built around SIFC framework and SBP Raast instant payments.",
[
    "Pakistan Special Investment Facilitation Council Act 2023",
    "Pakistan Contract Act 1872",
    "Pakistan Income Tax Ordinance 2001",
    "Pakistan Anti-Money Laundering Act 2010",
    "SBP Raast Regulations (Circular PSD No. 01/2021)",
    "Singapore Securities and Futures Act 2001 (Cap 289)",
    "Singapore Personal Data Protection Act 2012",
])

synth("european-digital-bank", "European Digital Banking Zone", "lu", [
    ("corporate_formation", "ee"),
    ("civic_code", "lu"),
    ("digital_assets", "ch-zug"),
    ("tax", "ie"),
    ("aml_cft", "lu"),
    ("licensing", "gb"),
    ("data_privacy", "de"),
    ("payment_rails", "nl"),
], "Pan-European digital banking charter. Combines Estonia e-Residency with Luxembourg financial center infrastructure.",
[
    "Estonia Commercial Code + e-Residency Regulation",
    "Luxembourg Civil Code (Code civil)",
    "Swiss Federal Act on Financial Services (FinSA) + FinIA",
    "Irish Taxes Consolidation Act 1997",
    "Luxembourg Law of 12 November 2004 (AML/CFT)",
    "UK Financial Services and Markets Act 2000 (FSMA)",
    "German Federal Data Protection Act (BDSG) + EU GDPR",
    "Dutch Financial Supervision Act (Wft)",
])

synth("africa-fintech-gateway", "Africa Fintech Gateway", "mu", [
    ("corporate_formation", "mu"),
    ("civic_code", "ke"),
    ("tax", "mu"),
    ("aml_cft", "za"),
    ("payment_rails", "ke"),
    ("securities", "ng"),
    ("data_privacy", "za"),
    ("licensing", "rw"),
], "Pan-African fintech operations hub. Mauritius holding company + Kenya mobile money + South Africa AML framework.",
[
    "Mauritius Companies Act 2001 + Financial Services Act 2007",
    "Kenya Law of Contract Act (Cap 23)",
    "Mauritius Income Tax Act 1995",
    "South Africa Financial Intelligence Centre Act 38 of 2001 (FICA)",
    "Central Bank of Kenya NPS Regulations 2014",
    "Nigeria ISA 2007 + SEC Rules on Digital Assets 2022",
    "South Africa POPIA Act 4 of 2013",
    "Rwanda BNR Regulation No. 12/2021 on Fintech",
])

synth("latam-trade-zone", "Latin America Trade Zone", "pa", [
    ("corporate_formation", "pa"),
    ("civic_code", "br"),
    ("tax", "uy"),
    ("aml_cft", "br"),
    ("payment_rails", "br"),
    ("securities", "cl"),
    ("arbitration", "mx"),
    ("licensing", "co"),
], "Cross-border trade in South America. Panama incorporation + Brazil PIX + Chile securities.",
[
    "Panama Law 4 of 2009 (SEM)",
    "Brazil Civil Code (Lei 10.406/2002)",
    "Uruguay Tax Code + Law 18.083 (Tax Reform)",
    "Brazil Lei 9.613/1998 (AML) + BCB Resolution 1/2020",
    "Brazil BCB PIX Regulations",
    "Chile Ley 18.045 (Securities Market Law)",
    "Mexico Codigo de Comercio, Title IV (Arbitration)",
    "Colombia Decree 663/1993 (Financial System)",
])

synth("islamic-finance-hub", "Islamic Finance Digital Hub", "ae", [
    ("corporate_formation", "ae-dubai-difc"),
    ("civic_code", "ae-dubai-difc"),
    ("digital_assets", "bh"),
    ("arbitration", "ae-dubai-difc"),
    ("tax", "ae"),
    ("aml_cft", "ae"),
    ("licensing", "my"),
    ("securities", "sa"),
], "Shariah-compliant digital finance. DIFC common law + Bahrain crypto + Malaysia IFSA + Saudi CMA.",
[
    "DIFC Companies Law (DIFC Law No. 5/2018)",
    "DIFC Contract Law (DIFC Law No. 6/2004)",
    "Bahrain CBB Rulebook Volume 6 (Shariah Standards)",
    "DIFC Arbitration Law + DIFC-LCIA Rules",
    "UAE Federal Decree-Law No. 47/2022 (Corporate Tax)",
    "UAE Federal Decree-Law No. 20/2018 (AML/CFT)",
    "Malaysia Islamic Financial Services Act 2013 (IFSA)",
    "Saudi Arabia Capital Market Law + CMA Shariah Guidelines",
])

synth("crypto-native-zone", "Crypto-Native Regulatory Zone", "ky", [
    ("corporate_formation", "ky"),
    ("digital_assets", "ae-abudhabi-adgm"),
    ("arbitration", "sg"),
    ("tax", "ky"),
    ("aml_cft", "sg"),
    ("securities", "hk"),
    ("licensing", "ae-dubai-dmcc"),
    ("data_privacy", "sg"),
], "Pure digital asset platform. Optimized for token issuance, exchange, and custody across APAC/ME.",
[
    "Cayman Islands Exempted Limited Partnership Act",
    "ADGM FSMR 2015 (Virtual Assets Framework)",
    "Singapore International Arbitration Act (Cap 143A)",
    "Cayman Islands -- no direct taxation",
    "Singapore Payment Services Act 2019 + MAS Notice PSN02",
    "Hong Kong SFO (Cap 571) + SFC VASP Guidelines",
    "DMCC Crypto Centre Regulations 2023",
    "Singapore Personal Data Protection Act 2012",
])

synth("green-finance-corridor", "Green Finance and ESG Corridor", "lu", [
    ("corporate_formation", "lu"),
    ("civic_code", "lu"),
    ("digital_assets", "ch"),
    ("tax", "nl"),
    ("aml_cft", "lu"),
    ("securities", "gb"),
    ("licensing", "sg"),
    ("data_privacy", "de"),
], "Sustainable finance and ESG compliance. Green bonds, carbon credit tokenization, ESG reporting.",
[
    "Luxembourg Law of 10 August 1915 (Commercial Companies)",
    "Luxembourg Civil Code",
    "Swiss DLT Act (2021)",
    "Netherlands Corporate Income Tax Act 1969 (Innovation Box)",
    "Luxembourg Law of 12 November 2004 (AML/CFT)",
    "UK FSMA 2000 + FCA Green Taxonomy + SDR (PS23/16)",
    "Singapore MAS Environmental Risk Management Guidelines",
    "German BDSG + EU GDPR",
])

synth("nordics-payments", "Nordic Payments Innovation Zone", "se", [
    ("corporate_formation", "se"),
    ("civic_code", "dk"),
    ("tax", "fi"),
    ("aml_cft", "no"),
    ("payment_rails", "se"),
    ("licensing", "ee"),
    ("data_privacy", "fi"),
    ("securities", "dk"),
], "Cross-Nordic payment innovation. Sweden instant payments + Estonia digital licensing + Finland stable tax.",
[
    "Swedish Companies Act (SFS 2005:551)",
    "Danish Sale of Goods Act + Commercial Foundations Act",
    "Finnish Income Tax Act (1535/1992)",
    "Norwegian AML Act (LOV-2018-06-01-23)",
    "Swedish Riksbank Act + Payment Services Act (SFS 2010:751)",
    "Estonian Payment Institutions and E-Money Institutions Act",
    "Finnish Data Protection Act (1050/2018)",
    "Danish Financial Business Act",
])

synth("maritime-trade-hub", "Maritime Trade and Logistics Hub", "sg", [
    ("corporate_formation", "sg"),
    ("civic_code", "gb"),
    ("arbitration", "gb"),
    ("tax", "hk"),
    ("aml_cft", "sg"),
    ("payment_rails", "hk"),
    ("licensing", "ae-dubai-jafza"),
    ("securities", "sg"),
], "Shipping, trade finance, and logistics. Singapore maritime + London law + HK shipping tax exemption.",
[
    "Singapore Companies Act 1967 (Cap 50)",
    "UK Carriage of Goods by Sea Act 1992",
    "LMAA Terms (2021) + London Arbitration",
    "Hong Kong IRO Section 23B (Shipping Profits Exemption)",
    "Singapore MAS Notice 626 + CDSA (Cap 65A)",
    "Hong Kong FPS Rules + CHATS Operating Procedures",
    "JAFZA Rules and Regulations (2023)",
    "Singapore Securities and Futures Act 2001 (Cap 289)",
])

# ═══════════════════════════════════════════════════════════════════════════
# 8 NEW SYNTHETIC ZONES (beyond catalog)
# ═══════════════════════════════════════════════════════════════════════════

synth("us-digital-asset", "US Digital Asset Interstate Zone", "us", [
    ("corporate_formation", "us-wy"),
    ("civic_code", "us-de"),
    ("digital_assets", "us-ny"),
    ("tax", "us-tx"),
    ("aml_cft", "us-ny"),
    ("securities", "us-ca"),
], "US domestic interstate digital asset operations. Wyoming DAO-friendly + Delaware corporate + New York BitLicense + Texas tax.",
[
    "Wyoming DAO LLC Act (SF0038) + SPDI Charter",
    "Delaware DGCL Title 8",
    "New York 23 NYCRR 200 (BitLicense)",
    "Texas Virtual Currency Act",
    "New York Banking Law Article 13-B (AML)",
    "California Digital Financial Assets Law (AB 39)",
])

synth("caribbean-digital-cluster", "Caribbean Digital Asset Cluster", "ky", [
    ("corporate_formation", "ky"),
    ("digital_assets", "bm"),
    ("tax", "ky"),
    ("aml_cft", "ky"),
    ("securities", "bs"),
    ("licensing", "vg"),
    ("arbitration", "bb"),
], "Caribbean digital asset hub. Cayman funds + Bermuda DABA + Bahamas DARE + BVI VASP Act.",
[
    "Cayman Islands Exempted Limited Partnership Act",
    "Bermuda Digital Asset Business Act 2018",
    "Cayman Islands -- no direct taxation",
    "Cayman Islands AML Regulations (2020 Revision)",
    "Bahamas DARE Act (Digital Assets and Registered Exchanges)",
    "BVI Virtual Assets Service Providers Act 2022",
    "Barbados Arbitration Act (Cap 110)",
])

synth("central-asian-gateway", "Central Asian Gateway Zone", "kz", [
    ("corporate_formation", "kz-aifc"),
    ("civic_code", "kz"),
    ("digital_assets", "kz-aifc"),
    ("tax", "ge"),
    ("aml_cft", "kz"),
    ("licensing", "uz"),
    ("arbitration", "kz-aifc"),
], "Central Asian corridor / Belt and Road gateway. AIFC common law + Georgia flat tax + Uzbekistan licensing.",
[
    "AIFC Constitutional Statute (2015)",
    "Kazakhstan Civil Code",
    "AIFC Framework for Regulation of Digital Assets",
    "Georgia Tax Code (flat 20% corporate, 0% crypto PIT)",
    "Kazakhstan AML/CFT Law (2009, as amended)",
    "Uzbekistan Presidential Decree on Digital Economy",
    "AIFC Arbitration Regulations + IAC Rules",
])

synth("indo-pacific-trade", "Indo-Pacific Trade Corridor", "sg", [
    ("corporate_formation", "in-gift"),
    ("civic_code", "sg"),
    ("digital_assets", "sg"),
    ("tax", "sg"),
    ("aml_cft", "sg"),
    ("securities", "au"),
    ("payment_rails", "sg"),
    ("data_privacy", "nz"),
], "India GIFT City to ASEAN/Oceania trade. IFSCA + Singapore MAS + Australia ASIC + NZ FMA.",
[
    "IFSCA (International Financial Services Centres Authority) Act 2019",
    "Singapore Companies Act 1967 (Cap 50)",
    "Singapore Payment Services Act 2019",
    "Singapore Income Tax Act (Cap 134)",
    "Singapore MAS Notice 626 + CDSA",
    "Australia Corporations Act 2001 (ASIC oversight)",
    "Singapore FAST/PayNow payment rails",
    "New Zealand Privacy Act 2020",
])

synth("mediterranean-fintech", "Mediterranean Digital Finance Hub", "mt", [
    ("corporate_formation", "mt"),
    ("civic_code", "cy"),
    ("digital_assets", "mt"),
    ("tax", "pt"),
    ("aml_cft", "mt"),
    ("licensing", "ie"),
    ("securities", "cy"),
], "Mediterranean digital finance. Malta VFA Act + Cyprus CySEC + Portugal NHR + Ireland fund licensing.",
[
    "Malta Virtual Financial Assets Act 2018 (Chapter 590)",
    "Cyprus Contract Law (Cap 149)",
    "Malta VFA Act + ITAS Act",
    "Portugal Tax Code (IRS/IRC) + NHR regime",
    "Malta Prevention of Money Laundering Act (Chapter 373)",
    "Ireland Central Bank licensing framework (CBI)",
    "Cyprus Securities and Exchange Commission (CySEC) Law",
])

synth("pacific-islands-development", "Pacific Islands Economic Development Zone", "nz", [
    ("corporate_formation", "nz"),
    ("civic_code", "fj"),
    ("tax", "vu"),
    ("aml_cft", "nz"),
    ("licensing", "fj"),
    ("payment_rails", "nz"),
], "Pacific Islands economic development. NZ FMA framework + Fiji financial hub + Vanuatu tax regime.",
[
    "New Zealand Companies Act 1993",
    "Fiji Companies Act 2015",
    "Vanuatu Value Added Tax Act",
    "New Zealand AML/CFT Act 2009",
    "Fiji Reserve Bank Financial Institutions Act",
    "New Zealand Real-Time Gross Settlement System",
])

synth("east-african-innovation", "East African Innovation Hub", "ke", [
    ("corporate_formation", "ke"),
    ("civic_code", "ke"),
    ("digital_assets", "rw"),
    ("tax", "rw"),
    ("aml_cft", "ke"),
    ("payment_rails", "ke"),
    ("licensing", "tz"),
    ("securities", "ug"),
], "East African fintech innovation. Kenya M-PESA + Rwanda fintech sandbox + Tanzania EPZ + Uganda CMA.",
[
    "Kenya Companies Act 2015",
    "Kenya Law of Contract Act (Cap 23)",
    "Rwanda BNR Regulation No. 12/2021 on Fintech",
    "Rwanda Income Tax Law",
    "Kenya Proceeds of Crime and AML Act 2009",
    "Central Bank of Kenya NPS Regulations 2014 (M-PESA)",
    "Tanzania EPZ Act (Cap 373)",
    "Uganda Capital Markets Authority Act",
])

synth("swiss-asian-bridge", "Swiss-Liechtenstein-Asian Innovation Bridge", "ch", [
    ("corporate_formation", "ch"),
    ("civic_code", "li"),
    ("digital_assets", "ch-zug"),
    ("tax", "li"),
    ("aml_cft", "ch"),
    ("securities", "sg"),
    ("licensing", "hk"),
    ("data_privacy", "ch"),
], "Swiss-Liechtenstein blockchain to Asian markets. FINMA + FMA TVTG + Singapore + Hong Kong.",
[
    "Swiss Code of Obligations (OR)",
    "Liechtenstein Token and TT Service Provider Act (TVTG)",
    "Swiss FINMA Framework (FinSA + FinIA + DLT Act)",
    "Liechtenstein Tax Act (flat 12.5% corporate)",
    "Swiss AML Act (GwG) + FINMA AML Ordinance",
    "Singapore Securities and Futures Act 2001 (Cap 289)",
    "Hong Kong SFC Licensing framework",
    "Swiss Federal Act on Data Protection (FADP)",
])


# ═══════════════════════════════════════════════════════════════════════════
# GENERATION
# ═══════════════════════════════════════════════════════════════════════════

def collect_regpacks(layers):
    """Collect applicable regpack digests from source jurisdictions."""
    rps = []
    seen = set()
    for domain, source in layers:
        # Get root country from source jurisdiction
        root = source.split("-")[0]
        if root == "synth":
            continue
        if root in REGPACK_MAP and root not in seen:
            for d, j, digest in REGPACK_MAP[root]:
                rps.append((d, j, digest))
            seen.add(root)
    return rps


def render_composition_yaml(z):
    lines = []
    lines.append(f"# Composition Spec: {z['name']}")
    lines.append("#")
    lines.append(f"# {z['use_case']}")
    lines.append("#")
    for domain, source in z["layers"]:
        lines.append(f"#   - {domain} <- {source}")
    lines.append("")
    lines.append(f"zone_name: {z['name']}")
    lines.append(f"zone_id: org.momentum.mez.zone.synthetic.{z['sid']}")
    lines.append(f"jurisdiction_id: synth-{z['sid']}")
    lines.append(f"primary_jurisdiction: {z['primary']}")
    lines.append("layers:")
    for domain, source in z["layers"]:
        lines.append(f"  - domain: {domain}")
        lines.append(f"    source: {source}")
    return "\n".join(lines) + "\n"


def render_zone_yaml(z):
    lines = []
    lines.append(f"# Zone Manifest: {z['name']} (Synthetic)")
    lines.append("#")
    lines.append(f"# Synthetic zone composed from regulatory primitives across multiple")
    lines.append(f"# jurisdictions. Primary jurisdiction: {z['primary']}.")
    lines.append("# Generated by: mez zone compose")
    lines.append("#")
    lines.append("# Layers:")
    for domain, source in z["layers"]:
        lines.append(f"#   - {domain} <- {source}")
    lines.append("")

    lines.append(f"zone_id: org.momentum.mez.zone.synthetic.{z['sid']}")
    lines.append(f"jurisdiction_id: synth-{z['sid']}")
    lines.append(f"zone_name: {z['name']}")
    lines.append("zone_type: synthetic")
    lines.append("")

    lines.append("profile:")
    lines.append(f"  profile_id: org.momentum.mez.profile.synthetic-synth-{z['sid']}")
    lines.append('  version: "0.4.44"')
    lines.append("")

    lines.append(f"primary_jurisdiction: {z['primary']}")
    lines.append("")

    lines.append("composition:")
    for domain, source in z["layers"]:
        lines.append(f"  - domain: {domain}")
        lines.append(f"    source_jurisdiction: {source}")
    lines.append("")

    lines.append("jurisdiction_stack:")
    lines.append(f"  - synth-{z['sid']}")
    lines.append("")

    lines.append("lawpack_domains:")
    lines.append("  - civil")
    lines.append("  - financial")
    lines.append("")

    lines.append("licensepack_domains:")
    lines.append("  - corporate")
    lines.append("  - financial")
    lines.append("")

    lines.append("licensepack_refresh_policy:")
    lines.append("  default:")
    lines.append("    frequency: daily")
    lines.append("    max_staleness_hours: 24")
    lines.append("  financial:")
    lines.append("    frequency: hourly")
    lines.append("    max_staleness_hours: 4")
    lines.append("")

    rps = collect_regpacks(z["layers"])
    if rps:
        lines.append("regpacks:")
        for domain, jid, digest in rps:
            lines.append(f"  - domain: {domain}")
            lines.append(f"    jurisdiction_id: {jid}")
            lines.append(f'    regpack_digest_sha256: "{digest}"')
            lines.append(f'    as_of_date: "2026-01-15"')
    else:
        lines.append("# Regpack builder needed for source jurisdictions")
    lines.append("")

    lines.append("compliance_domains:")
    for cd in COMPLIANCE:
        lines.append(f"  - {cd}")
    lines.append("")

    lines.append("corridors:")
    lines.append("  - org.momentum.mez.corridor.swift.iso20022-cross-border")
    lines.append("  - org.momentum.mez.corridor.stablecoin.regulated-stablecoin")
    lines.append("")

    lines.append("trust_anchors: []")
    lines.append("")
    lines.append("key_management:")
    lines.append("  rotation_interval_days: 90")
    lines.append("  grace_period_days: 14")
    lines.append("")
    lines.append("lockfile_path: stack.lock")
    return "\n".join(lines) + "\n"


def render_profile_yaml(z):
    lines = []
    lines.append(f"# Profile: {z['name']} (Synthetic)")
    lines.append("#")
    lines.append("# Synthetic zone profile composed from regulatory primitives.")
    lines.append("# Each regulatory domain is sourced from a different jurisdiction.")
    lines.append("#")
    lines.append("# Layers:")
    for domain, source in z["layers"]:
        lines.append(f"#   {domain} <- {source}")
    lines.append("")

    lines.append(f"profile_id: org.momentum.mez.profile.synthetic-synth-{z['sid']}")
    lines.append(f"profile_name: {z['name']}")
    lines.append('version: "0.4.44"')
    lines.append("zone_type: synthetic")
    lines.append("")

    lines.append("composition:")
    for domain, source in z["layers"]:
        lines.append(f"  - domain: {domain}")
        lines.append(f"    source: {source}")
    lines.append("")

    lines.append("modules:")
    lines.append("  - id: org.momentum.mez.legal.core")
    lines.append("    variant: synthetic-composed")

    # Add domain-specific module entries
    for domain, source in z["layers"]:
        if domain == "aml_cft":
            lines.append("  - id: org.momentum.mez.reg.aml-cft")
            lines.append("    variant: risk-based")
            lines.append("    parameters:")
            lines.append("      kyc_tier: 3")
            lines.append("      edd_threshold_usd: 10000")
            lines.append(f'      source_jurisdiction: "{source}"')
        elif domain == "tax":
            lines.append("  - id: org.momentum.mez.tax")
            lines.append(f"    variant: {source}-tax")
            lines.append("    parameters:")
            lines.append(f'      source_jurisdiction: "{source}"')
        elif domain == "digital_assets":
            lines.append("  - id: org.momentum.mez.reg.digital-assets")
            lines.append(f"    variant: {source}-framework")
            lines.append("    parameters:")
            lines.append(f'      source_jurisdiction: "{source}"')
        elif domain == "corporate_formation":
            lines.append("  - id: org.momentum.mez.legal.commercial-code")
            lines.append(f"    variant: {source}-corp")
        elif domain == "arbitration":
            lines.append("  - id: org.momentum.mez.legal.dispute-resolution")
            lines.append(f"    variant: {source}-arbitration")
    lines.append("  - id: org.momentum.mez.fin.payments-adapter")
    lines.append("    variant: iso20022-mapping")
    lines.append("  - id: org.momentum.mez.corridor.swift")
    lines.append("    variant: iso20022-cross-border")
    lines.append("")

    lines.append("corridors:")
    lines.append("  - org.momentum.mez.corridor.swift.iso20022-cross-border")
    lines.append("  - org.momentum.mez.corridor.stablecoin.regulated-stablecoin")
    return "\n".join(lines) + "\n"


def main():
    created = 0
    skipped = 0
    for z in SYNTH_ZONES:
        sid = z["sid"]
        jdir = os.path.join(JURISDICTIONS_DIR, f"synth-{sid}")
        pdir = os.path.join(PROFILES_DIR, f"synthetic-synth-{sid}")

        if os.path.exists(jdir):
            print(f"  SKIP (exists): synth-{sid}")
            skipped += 1
            continue

        os.makedirs(jdir, exist_ok=True)
        os.makedirs(pdir, exist_ok=True)

        with open(os.path.join(jdir, "composition.yaml"), "w") as f:
            f.write(render_composition_yaml(z))
        with open(os.path.join(jdir, "zone.yaml"), "w") as f:
            f.write(render_zone_yaml(z))
        with open(os.path.join(pdir, "profile.yaml"), "w") as f:
            f.write(render_profile_yaml(z))

        created += 1
        print(f"  CREATE: synth-{sid}")

    print(f"\nDone: {created} synthetic zones created, {skipped} skipped")


if __name__ == "__main__":
    main()
