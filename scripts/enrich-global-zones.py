#!/usr/bin/env python3
"""Enrich remaining non-US scaffold zone.yaml files.

Transforms 28-line scaffolds into enriched manifests for:
- Brazil, Egypt, Honduras Prospera, Indonesia, Ireland, Kenya
- China (federal + 5 cities), Kazakhstan (federal + AIFC + Alatau)
- Portugal, Qatar + QFC, Seychelles
- Tanzania + Zanzibar, BVI, South Africa

Usage:
    python3 scripts/enrich-global-zones.py
"""

import os

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
JURISDICTIONS_DIR = os.path.join(REPO_ROOT, "jurisdictions")

SOVEREIGN_COMPLIANCE = ["aml", "kyc", "sanctions", "tax", "corporate", "licensing"]
EXTENDED_COMPLIANCE = ["aml", "kyc", "sanctions", "tax", "securities", "corporate",
                       "licensing", "data_privacy"]
FC_COMPLIANCE = ["aml", "kyc", "sanctions", "tax", "securities", "corporate",
                 "licensing", "data_privacy", "consumer_protection"]
FC_CORRIDORS = [
    "org.momentum.mez.corridor.swift.iso20022-cross-border",
    "org.momentum.mez.corridor.stablecoin.regulated-stablecoin",
]

ZONES = {}

def z(jid, name, profile, stack, compliance=None, corridors=None,
      adapters=None, regpack_parent=None, lawpacks=None, licensepacks=None,
      comment=""):
    if compliance is None:
        compliance = SOVEREIGN_COMPLIANCE
    if corridors is None:
        corridors = ["org.momentum.mez.corridor.swift.iso20022-cross-border"]
    if adapters is None:
        adapters = {}
    if lawpacks is None:
        lawpacks = ["civil", "financial"]
    ZONES[jid] = dict(jid=jid, name=name, profile=profile, stack=stack,
                      compliance=compliance, corridors=corridors,
                      adapters=adapters, regpack_parent=regpack_parent,
                      lawpacks=lawpacks, licensepacks=licensepacks,
                      comment=comment)

# ── Brazil ──────────────────────────────────────────────────────────────────
z("br", "Brazil", "sovereign-govos", ["br"],
  compliance=EXTENDED_COMPLIANCE,
  adapters={
      "bcb": ("Banco Central do Brasil", "${BCB_URL:-}"),
      "cvm": ("Comissao de Valores Mobiliarios", "${CVM_URL:-}"),
      "receita_federal": ("Receita Federal do Brasil", "${RECEITA_URL:-}"),
  },
  lawpacks=["civil", "financial", "tax"],
  comment="Brazil zone.\n# BCB + CVM framework; crypto asset law (Lei 14.478/2022).\n# PIX instant payment system; 150M+ users.")

# ── China ───────────────────────────────────────────────────────────────────
z("cn", "China", "sovereign-govos", ["cn"],
  compliance=EXTENDED_COMPLIANCE,
  adapters={
      "pboc": ("People's Bank of China", "${PBOC_URL:-}"),
      "csrc": ("China Securities Regulatory Commission", "${CSRC_URL:-}"),
      "sat_cn": ("State Administration of Taxation", "${SAT_CN_URL:-}"),
      "samr": ("State Administration for Market Regulation", "${SAMR_URL:-}"),
  },
  comment="China federal zone.\n# PBOC + CSRC framework; restrictive crypto stance.\n# DCEP/e-CNY CBDC; Cross-Border Interbank Payment System (CIPS).")

z("cn-beijing", "Beijing", "sovereign-govos", ["cn", "cn-beijing"],
  compliance=SOVEREIGN_COMPLIANCE,
  adapters={"pboc": ("PBOC", "${PBOC_URL:-}"), "samr": ("SAMR", "${SAMR_URL:-}")},
  comment="Beijing zone.\n# National capital; regulatory headquarters.\n# Zhongguancun Science Park innovation zone.")

z("cn-hainan", "Hainan", "sovereign-govos", ["cn", "cn-hainan"],
  compliance=EXTENDED_COMPLIANCE,
  adapters={"pboc": ("PBOC", "${PBOC_URL:-}"), "hainan_ftz": ("Hainan FTZ Authority", "${HAINAN_FTZ_URL:-}")},
  comment="Hainan Free Trade Port.\n# China's largest free trade zone; duty-free policies.\n# Hainan Free Trade Port Law (2021).")

z("cn-hangzhou", "Hangzhou", "sovereign-govos", ["cn", "cn-hangzhou"],
  compliance=SOVEREIGN_COMPLIANCE,
  adapters={"pboc": ("PBOC", "${PBOC_URL:-}")},
  comment="Hangzhou zone.\n# Fintech innovation hub; Alibaba/Ant Group headquarters.\n# Blockchain innovation pilot city.")

z("cn-shanghai", "Shanghai", "sovereign-govos", ["cn", "cn-shanghai"],
  compliance=EXTENDED_COMPLIANCE,
  adapters={"pboc": ("PBOC", "${PBOC_URL:-}"), "sse": ("Shanghai Stock Exchange", "${SSE_URL:-}")},
  comment="Shanghai zone.\n# China's financial center; Shanghai FTZ.\n# Lujiazui Financial District; Lin-gang Special Area.")

z("cn-shenzhen", "Shenzhen", "sovereign-govos", ["cn", "cn-shenzhen"],
  compliance=SOVEREIGN_COMPLIANCE,
  adapters={"pboc": ("PBOC", "${PBOC_URL:-}"), "szse": ("Shenzhen Stock Exchange", "${SZSE_URL:-}")},
  comment="Shenzhen zone.\n# Tech innovation hub; Shenzhen SEZ.\n# Qianhai cooperation zone; e-CNY pilot city.")

# ── Egypt ───────────────────────────────────────────────────────────────────
z("eg", "Egypt", "sovereign-govos", ["eg"],
  compliance=SOVEREIGN_COMPLIANCE,
  adapters={
      "cbe": ("Central Bank of Egypt", "${CBE_URL:-}"),
      "fra": ("Financial Regulatory Authority", "${FRA_URL:-}"),
      "eta": ("Egyptian Tax Authority", "${ETA_URL:-}"),
  },
  comment="Egypt zone.\n# CBE + FRA framework; Suez Canal Economic Zone.\n# Central Bank and Banking System Law No. 194/2020.")

# ── Honduras Prospera ───────────────────────────────────────────────────────
z("hn-prospera", "Prospera", "charter-city", ["hn", "hn-prospera"],
  compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
  licensepacks=["financial", "corporate"],
  adapters={"prospera_reg": ("Prospera Regulatory Authority", "${PROSPERA_URL:-}")},
  comment="Prospera ZEDE (Honduras).\n# Charter city with independent regulatory framework.\n# Zone for Employment and Economic Development.")

# ── Indonesia ───────────────────────────────────────────────────────────────
z("id", "Indonesia", "sovereign-govos", ["id"],
  compliance=EXTENDED_COMPLIANCE,
  adapters={
      "ojk": ("Otoritas Jasa Keuangan", "${OJK_URL:-}"),
      "bi": ("Bank Indonesia", "${BI_URL:-}"),
      "bappebti": ("Badan Pengawas Perdagangan Berjangka Komoditi", "${BAPPEBTI_URL:-}"),
      "djp": ("Direktorat Jenderal Pajak", "${DJP_URL:-}"),
  },
  comment="Indonesia zone.\n# OJK + BI framework; Bappebti commodity futures (crypto oversight).\n# Nusantara new capital; multiple Special Economic Zones.")

# ── Ireland ─────────────────────────────────────────────────────────────────
z("ie", "Ireland", "digital-financial-center", ["ie"],
  compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
  licensepacks=["financial", "corporate"],
  adapters={
      "cbi_ie": ("Central Bank of Ireland", "${CBI_IE_URL:-}"),
      "revenue_ie": ("Office of the Revenue Commissioners", "${REVENUE_IE_URL:-}"),
      "cro_ie": ("Companies Registration Office", "${CRO_IE_URL:-}"),
  },
  comment="Ireland zone.\n# CBI framework; EU MiCA implementation.\n# 12.5% corporate tax; IFSC Dublin; fund domiciliation hub.")

# ── Kenya ───────────────────────────────────────────────────────────────────
z("ke", "Kenya", "sovereign-govos", ["ke"],
  compliance=EXTENDED_COMPLIANCE,
  adapters={
      "cbk": ("Central Bank of Kenya", "${CBK_URL:-}"),
      "cma_ke": ("Capital Markets Authority Kenya", "${CMA_KE_URL:-}"),
      "kra": ("Kenya Revenue Authority", "${KRA_URL:-}"),
  },
  comment="Kenya zone.\n# CBK + CMA framework; Nairobi IFC initiative.\n# M-PESA mobile money ecosystem; National Payment System Regulations 2014.")

# ── Kazakhstan ──────────────────────────────────────────────────────────────
z("kz", "Kazakhstan", "sovereign-govos", ["kz"],
  compliance=SOVEREIGN_COMPLIANCE,
  adapters={
      "nbk": ("National Bank of Kazakhstan", "${NBK_URL:-}"),
      "ardfm": ("Agency for Regulation and Development of Financial Market", "${ARDFM_URL:-}"),
      "kgd": ("Committee of State Revenue", "${KGD_URL:-}"),
  },
  comment="Kazakhstan zone.\n# NBK + ARDFM framework; digital tenge CBDC pilot.\n# Crypto mining legalization (2022 amendments).")

z("kz-aifc", "Astana International Financial Centre", "digital-financial-center",
  ["kz", "kz-aifc"],
  compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
  licensepacks=["financial", "corporate"],
  adapters={"afsa": ("Astana Financial Services Authority", "${AFSA_URL:-}")},
  comment="Astana International Financial Centre (AIFC).\n# AFSA regulated; English common law jurisdiction.\n# AIFC Framework for Regulation of Digital Assets.")

z("kz-alatau", "Alatau IT City", "digital-financial-center",
  ["kz", "kz-alatau"],
  compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
  licensepacks=["financial", "corporate"],
  adapters={"alatau_admin": ("Alatau IT City Administration", "${ALATAU_URL:-}")},
  comment="Alatau IT City (Almaty).\n# Technology park and innovation zone.\n# Special tax and regulatory regime for IT companies.")

# ── Portugal ────────────────────────────────────────────────────────────────
z("pt", "Portugal", "sovereign-govos", ["pt"],
  compliance=EXTENDED_COMPLIANCE,
  adapters={
      "bde_pt": ("Banco de Portugal", "${BDE_PT_URL:-}"),
      "cmvm": ("Comissao do Mercado de Valores Mobiliarios", "${CMVM_URL:-}"),
      "at_pt": ("Autoridade Tributaria e Aduaneira", "${AT_PT_URL:-}"),
  },
  comment="Portugal zone.\n# BdP + CMVM framework; EU MiCA.\n# Historically crypto-tax-friendly (NHR regime); Lisbon fintech hub.")

# ── Qatar ───────────────────────────────────────────────────────────────────
z("qa", "Qatar", "sovereign-govos", ["qa"],
  compliance=SOVEREIGN_COMPLIANCE,
  adapters={
      "qcb": ("Qatar Central Bank", "${QCB_URL:-}"),
      "qfma": ("Qatar Financial Markets Authority", "${QFMA_URL:-}"),
      "gta_qa": ("General Tax Authority", "${GTA_QA_URL:-}"),
  },
  comment="Qatar zone.\n# QCB + QFMA framework.\n# Qatar National Vision 2030; no personal income tax.")

z("qa-qfc", "Qatar Financial Centre", "digital-financial-center",
  ["qa", "qa-qfc"],
  compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
  licensepacks=["financial", "corporate"],
  adapters={"qfcra": ("QFC Regulatory Authority", "${QFCRA_URL:-}")},
  comment="Qatar Financial Centre.\n# QFCRA regulated; common law financial centre.\n# QFC Financial Services Regulations; 10% corporate tax.")

# ── Seychelles ──────────────────────────────────────────────────────────────
z("sc", "Seychelles", "digital-financial-center", ["sc"],
  compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
  licensepacks=["financial", "corporate"],
  adapters={
      "fsa_sc": ("Financial Services Authority Seychelles", "${FSA_SC_URL:-}"),
      "src": ("Seychelles Revenue Commission", "${SRC_URL:-}"),
  },
  comment="Seychelles zone.\n# FSA framework; International Business Companies Act.\n# Offshore financial center; crypto exchange domiciliation.")

# ── Tanzania + Zanzibar ─────────────────────────────────────────────────────
z("tz", "Tanzania", "sovereign-govos", ["tz"],
  compliance=SOVEREIGN_COMPLIANCE,
  adapters={
      "bot_tz": ("Bank of Tanzania", "${BOT_TZ_URL:-}"),
      "cmsa": ("Capital Markets and Securities Authority", "${CMSA_URL:-}"),
      "tra": ("Tanzania Revenue Authority", "${TRA_URL:-}"),
  },
  comment="Tanzania zone.\n# BOT framework; EPZ program.\n# National Payment Systems Act 2015; mobile money growth.")

z("tz-zanzibar", "Zanzibar", "sovereign-govos", ["tz", "tz-zanzibar"],
  compliance=SOVEREIGN_COMPLIANCE,
  adapters={"zrb": ("Zanzibar Revenue Board", "${ZRB_URL:-}"), "bot_tz": ("Bank of Tanzania", "${BOT_TZ_URL:-}")},
  comment="Zanzibar zone.\n# Semi-autonomous within Tanzania; own revenue authority.\n# Zanzibar Investment Promotion Authority.")

# ── British Virgin Islands ──────────────────────────────────────────────────
z("vg", "British Virgin Islands", "digital-financial-center", ["vg"],
  compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
  licensepacks=["financial", "corporate"],
  adapters={
      "bvi_fsc": ("BVI Financial Services Commission", "${BVI_FSC_URL:-}"),
      "ird_vg": ("Inland Revenue Department BVI", "${IRD_VG_URL:-}"),
  },
  comment="British Virgin Islands zone.\n# BVI FSC framework; Virtual Assets Service Providers Act 2022.\n# Premier offshore financial center; BVI Business Companies Act.")

# ── South Africa ────────────────────────────────────────────────────────────
z("za", "South Africa", "sovereign-govos", ["za"],
  compliance=EXTENDED_COMPLIANCE,
  adapters={
      "sarb": ("South African Reserve Bank", "${SARB_URL:-}"),
      "fsca": ("Financial Sector Conduct Authority", "${FSCA_URL:-}"),
      "sars": ("South African Revenue Service", "${SARS_URL:-}"),
      "cipc": ("Companies and Intellectual Property Commission", "${CIPC_URL:-}"),
  },
  comment="South Africa zone.\n# SARB + FSCA framework; Financial Intelligence Centre Act (FICA).\n# POPIA data protection; IDZ program; declared crypto as financial product.")


# ═══════════════════════════════════════════════════════════════════════════
# YAML GENERATION
# ═══════════════════════════════════════════════════════════════════════════

def zone_id_from_jid(jid):
    parts = jid.split("-")
    return "org.momentum.mez.zone." + ".".join(parts)

def profile_id_from_type(ptype):
    return f"org.momentum.mez.profile.{ptype}"

def render_zone_yaml(z):
    lines = []
    lines.append(f"# Zone Manifest: {z['name']}")
    lines.append("#")
    for cline in z["comment"].split("\n"):
        if cline.startswith("#"):
            lines.append(f"# {cline.lstrip('# ')}")
        elif cline:
            lines.append(f"# {cline}")
        else:
            lines.append("#")
    lines.append("#")
    stack_display = " > ".join(z["stack"])
    lines.append(f"# jurisdiction_stack: {stack_display}")
    lines.append("")

    lines.append(f"zone_id: {zone_id_from_jid(z['jid'])}")
    lines.append(f"jurisdiction_id: {z['jid']}")
    lines.append(f"zone_name: {z['name']}")
    lines.append("")

    lines.append("profile:")
    lines.append(f"  profile_id: {profile_id_from_type(z['profile'])}")
    lines.append('  version: "0.4.44"')
    lines.append("")

    lines.append("jurisdiction_stack:")
    for s in z["stack"]:
        lines.append(f"  - {s}")
    lines.append("")

    lines.append("lawpack_domains:")
    for lp in z["lawpacks"]:
        lines.append(f"  - {lp}")
    lines.append("")

    if z.get("licensepacks"):
        lines.append("licensepack_domains:")
        for lp in z["licensepacks"]:
            lines.append(f"  - {lp}")
        lines.append("")
        lines.append("licensepack_refresh_policy:")
        lines.append("  default:")
        lines.append("    frequency: daily")
        lines.append("    max_staleness_hours: 24")
        lines.append("  financial:")
        lines.append("    frequency: hourly")
        lines.append("    max_staleness_hours: 4")
        lines.append("")

    lines.append("# Regpack builder needed")
    lines.append("")

    lines.append("compliance_domains:")
    for cd in z["compliance"]:
        lines.append(f"  - {cd}")
    lines.append("")

    lines.append("corridors:")
    for c in z["corridors"]:
        lines.append(f"  - {c}")
    lines.append("")

    if z["adapters"]:
        lines.append("national_adapters:")
        for akey, (desc, endpoint) in z["adapters"].items():
            lines.append(f"  {akey}:")
            lines.append(f"    enabled: false")
            lines.append(f'    endpoint: "{endpoint}"')
    else:
        lines.append("national_adapters: {}")
    lines.append("")

    lines.append("trust_anchors: []")
    lines.append("")
    lines.append("key_management:")
    lines.append("  rotation_interval_days: 90")
    lines.append("  grace_period_days: 14")
    lines.append("")
    lines.append("lockfile_path: stack.lock")

    return "\n".join(lines) + "\n"


def main():
    enriched = 0
    for jid, z in sorted(ZONES.items()):
        zone_file = os.path.join(JURISDICTIONS_DIR, jid, "zone.yaml")
        if not os.path.exists(os.path.dirname(zone_file)):
            print(f"  SKIP (no dir): {jid}")
            continue
        content = render_zone_yaml(z)
        with open(zone_file, "w") as f:
            f.write(content)
        enriched += 1
        print(f"  ENRICH: {jid}")
    print(f"\nDone: {enriched} zones enriched")


if __name__ == "__main__":
    main()
