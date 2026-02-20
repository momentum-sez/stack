#!/usr/bin/env python3
"""Generate enriched zone.yaml manifests for all missing jurisdictions.

This script creates zone directories and enriched zone.yaml files for every
jurisdiction in the coverage matrix that doesn't already have one.

Usage:
    python3 scripts/generate-zones.py

Outputs files under jurisdictions/<id>/zone.yaml.
"""

import os
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
JURISDICTIONS_DIR = os.path.join(REPO_ROOT, "jurisdictions")

# ── UAE regpack digests (real, from existing builders) ──────────────────────
AE_REGPACKS = [
    {
        "domain": "financial",
        "jurisdiction_id": "ae",
        "regpack_digest_sha256": "5ce776c2d59b4fd9ec1f6bdaea7317550fd0b7d1792be86a475b4db6602a685b",
        "as_of_date": "2026-01-15",
    },
    {
        "domain": "sanctions",
        "jurisdiction_id": "ae",
        "regpack_digest_sha256": "34bb8fcb760935931216ab810fb3e66caa6980125f7e88f588eed8a0747509c0",
        "as_of_date": "2026-01-15",
    },
]

# ── Zone definitions ────────────────────────────────────────────────────────
# Each entry: (jurisdiction_id, zone_name, profile_type, jurisdiction_stack,
#              compliance_domains, corridors, national_adapters, regpack_parent,
#              lawpack_domains, licensepack_domains, comment_lines)
#
# profile_type: "sovereign-govos" | "digital-financial-center" | "charter-city" | "minimal-mvp"
# regpack_parent: None | "ae" | "pk" | "sg" | "hk" | "ky"  (inherit parent's real digests)

ZONES = []

def zone(jid, name, profile, stack, compliance=None, corridors=None,
         adapters=None, regpack_parent=None, lawpacks=None, licensepacks=None,
         comment=None):
    """Register a zone definition."""
    if compliance is None:
        compliance = ["aml", "kyc", "sanctions", "tax", "corporate", "licensing"]
    if corridors is None:
        corridors = ["org.momentum.mez.corridor.swift.iso20022-cross-border"]
    if adapters is None:
        adapters = {}
    if lawpacks is None:
        lawpacks = ["civil", "financial"]
    if comment is None:
        comment = f"Zone manifest for {name}."
    ZONES.append({
        "jid": jid,
        "name": name,
        "profile": profile,
        "stack": stack,
        "compliance": compliance,
        "corridors": corridors,
        "adapters": adapters,
        "regpack_parent": regpack_parent,
        "lawpacks": lawpacks,
        "licensepacks": licensepacks,
        "comment": comment,
    })


# ── Financial center compliance domains ─────────────────────────────────────
FC_COMPLIANCE = ["aml", "kyc", "sanctions", "tax", "securities", "corporate",
                 "licensing", "data_privacy", "consumer_protection"]
SOVEREIGN_COMPLIANCE = ["aml", "kyc", "sanctions", "tax", "corporate", "licensing"]
EXTENDED_COMPLIANCE = ["aml", "kyc", "sanctions", "tax", "securities", "corporate",
                       "licensing", "data_privacy"]
FC_CORRIDORS = [
    "org.momentum.mez.corridor.swift.iso20022-cross-border",
    "org.momentum.mez.corridor.stablecoin.regulated-stablecoin",
]

# ═══════════════════════════════════════════════════════════════════════════
# PAKISTAN (federal — parent for pk-sifc)
# ═══════════════════════════════════════════════════════════════════════════
zone("pk", "Pakistan", "sovereign-govos", ["pk"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "fbr_iris": ("FBR IRIS tax portal", "${FBR_IRIS_URL:-}"),
         "sbp_raast": ("SBP Raast instant payments", "${SBP_RAAST_URL:-}"),
         "nadra": ("NADRA identity verification", "${NADRA_API_URL:-}"),
         "secp": ("SECP corporate registry", "${SECP_API_URL:-}"),
     },
     regpack_parent="pk",
     lawpacks=["civil", "financial", "tax", "aml"],
     comment="Pakistan federal zone. Parent jurisdiction for pk-sifc.\n# Regulatory: SBP (central bank), SECP (securities), FBR (tax), NADRA (identity).")

# ═══════════════════════════════════════════════════════════════════════════
# UAE — Additional emirates and free zones
# ═══════════════════════════════════════════════════════════════════════════
zone("ae-rak", "Ras Al Khaimah", "sovereign-govos", ["ae", "ae-rak"],
     compliance=SOVEREIGN_COMPLIANCE, regpack_parent="ae",
     adapters={"rak_ded": ("RAK DED business registry", "${RAK_DED_URL:-}")},
     comment="Ras Al Khaimah emirate zone.\n# Emerging free zone hub; RAKEZ and RAK ICC.")

zone("ae-rak-rakez", "RAKEZ Free Zone", "digital-financial-center",
     ["ae", "ae-rak", "ae-rak-rakez"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS, regpack_parent="ae",
     licensepacks=["financial", "corporate"],
     adapters={"rakez": ("RAKEZ free zone authority", "${RAKEZ_URL:-}")},
     comment="Ras Al Khaimah Economic Zone (RAKEZ).\n# Multi-sector free zone; 100% foreign ownership, 0% tax.")

zone("ae-sharjah", "Sharjah", "sovereign-govos", ["ae", "ae-sharjah"],
     compliance=SOVEREIGN_COMPLIANCE, regpack_parent="ae",
     adapters={"sharjah_ded": ("Sharjah DED", "${SHARJAH_DED_URL:-}")},
     comment="Sharjah emirate zone.\n# SRTI Park, Hamriyah Free Zone, Sharjah Airport FZ.")

zone("ae-ajman", "Ajman", "sovereign-govos", ["ae", "ae-ajman"],
     compliance=SOVEREIGN_COMPLIANCE, regpack_parent="ae",
     adapters={"ajman_ded": ("Ajman DED", "${AJMAN_DED_URL:-}")},
     comment="Ajman emirate zone.\n# Ajman Free Zone; light industrial and commercial focus.")

# ═══════════════════════════════════════════════════════════════════════════
# BAHRAIN
# ═══════════════════════════════════════════════════════════════════════════
zone("bh", "Bahrain", "sovereign-govos", ["bh"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "cbb": ("Central Bank of Bahrain", "${CBB_URL:-}"),
         "sijilat": ("Sijilat commercial registry", "${SIJILAT_URL:-}"),
         "nbr": ("National Bureau for Revenue", "${NBR_URL:-}"),
     },
     comment="Bahrain federal zone.\n# CBB framework; fintech sandbox; first GCC crypto-asset regulation.\n# FATF mutual evaluation 2018.")

zone("bh-bfb", "Bahrain FinTech Bay", "digital-financial-center",
     ["bh", "bh-bfb"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={"cbb": ("Central Bank of Bahrain", "${CBB_URL:-}")},
     comment="Bahrain FinTech Bay.\n# Fintech hub within Bahrain regulatory sandbox.\n# CBB Regulatory Sandbox Framework.")

# ═══════════════════════════════════════════════════════════════════════════
# OMAN
# ═══════════════════════════════════════════════════════════════════════════
zone("om", "Oman", "sovereign-govos", ["om"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "cbo": ("Central Bank of Oman", "${CBO_URL:-}"),
         "cma_oman": ("Capital Markets Authority Oman", "${CMA_OMAN_URL:-}"),
         "moci_oman": ("Ministry of Commerce and Industry", "${MOCI_OMAN_URL:-}"),
     },
     comment="Oman zone.\n# CBO + CMA framework; Duqm Special Economic Zone.")

# ═══════════════════════════════════════════════════════════════════════════
# SAUDI ARABIA
# ═══════════════════════════════════════════════════════════════════════════
zone("sa", "Saudi Arabia", "sovereign-govos", ["sa"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "sama": ("Saudi Arabian Monetary Authority", "${SAMA_URL:-}"),
         "cma_sa": ("Capital Market Authority", "${CMA_SA_URL:-}"),
         "zatca": ("Zakat, Tax and Customs Authority", "${ZATCA_URL:-}"),
         "mc": ("Ministry of Commerce", "${MC_SA_URL:-}"),
     },
     comment="Saudi Arabia zone.\n# CMA + SAMA framework; Vision 2030 digital economy.\n# Regulatory sandbox for fintech (SAMA Sandbox).")

zone("sa-neom", "NEOM", "charter-city", ["sa", "sa-neom"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={"neom": ("NEOM regulatory authority", "${NEOM_URL:-}")},
     comment="NEOM special zone.\n# Greenfield smart city; independent regulatory framework.\n# Vision 2030 flagship project.")

# ═══════════════════════════════════════════════════════════════════════════
# JORDAN
# ═══════════════════════════════════════════════════════════════════════════
zone("jo", "Jordan", "sovereign-govos", ["jo"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "cbj": ("Central Bank of Jordan", "${CBJ_URL:-}"),
         "jsc": ("Jordan Securities Commission", "${JSC_URL:-}"),
         "istd": ("Income and Sales Tax Department", "${ISTD_URL:-}"),
     },
     comment="Jordan zone.\n# CBJ + JSC framework; Aqaba Special Economic Zone.")

# ═══════════════════════════════════════════════════════════════════════════
# AFRICA
# ═══════════════════════════════════════════════════════════════════════════
zone("ng", "Nigeria", "sovereign-govos", ["ng"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "cbn": ("Central Bank of Nigeria", "${CBN_URL:-}"),
         "sec_ng": ("Securities and Exchange Commission Nigeria", "${SEC_NG_URL:-}"),
         "firs": ("Federal Inland Revenue Service", "${FIRS_URL:-}"),
         "cac": ("Corporate Affairs Commission", "${CAC_URL:-}"),
     },
     comment="Nigeria zone.\n# CBN + SEC framework; Lekki Free Zone.\n# SEC Rules on Digital Assets 2022.")

zone("mu", "Mauritius", "digital-financial-center", ["mu"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "fsc_mu": ("Financial Services Commission Mauritius", "${FSC_MU_URL:-}"),
         "mra": ("Mauritius Revenue Authority", "${MRA_URL:-}"),
         "bom": ("Bank of Mauritius", "${BOM_URL:-}"),
     },
     comment="Mauritius zone.\n# FSC Mauritius; Global Business License framework.\n# Africa-oriented treaty network, CRS-compliant.")

zone("rw", "Rwanda", "sovereign-govos", ["rw"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "bnr": ("National Bank of Rwanda", "${BNR_URL:-}"),
         "cma_rw": ("Capital Market Authority Rwanda", "${CMA_RW_URL:-}"),
         "rra": ("Rwanda Revenue Authority", "${RRA_URL:-}"),
     },
     comment="Rwanda zone.\n# BNR + CMA framework; Kigali International Financial Centre.\n# BNR Regulation No. 12/2021 on Fintech.")

zone("gh", "Ghana", "sovereign-govos", ["gh"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "bog": ("Bank of Ghana", "${BOG_URL:-}"),
         "sec_gh": ("Securities and Exchange Commission Ghana", "${SEC_GH_URL:-}"),
         "gra": ("Ghana Revenue Authority", "${GRA_URL:-}"),
     },
     comment="Ghana zone.\n# BOG + SEC framework; emerging digital asset regulation.\n# Mobile money interoperability platform (GhIPSS).")

# Tier 4 Africa
zone("ma", "Morocco", "sovereign-govos", ["ma"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "bam": ("Bank Al-Maghrib", "${BAM_URL:-}"),
         "ammc": ("Autorite Marocaine du Marche des Capitaux", "${AMMC_URL:-}"),
         "dgi_ma": ("Direction Generale des Impots", "${DGI_MA_URL:-}"),
     },
     comment="Morocco zone.\n# Bank Al-Maghrib + AMMC framework.\n# Casablanca Finance City.")

zone("tn", "Tunisia", "sovereign-govos", ["tn"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "bct": ("Banque Centrale de Tunisie", "${BCT_URL:-}"),
         "cmf_tn": ("Conseil du Marche Financier", "${CMF_TN_URL:-}"),
     },
     comment="Tunisia zone.\n# BCT framework; emerging fintech regulation.")

zone("et", "Ethiopia", "sovereign-govos", ["et"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "nbe": ("National Bank of Ethiopia", "${NBE_URL:-}"),
         "erca": ("Ethiopian Revenues and Customs Authority", "${ERCA_URL:-}"),
     },
     comment="Ethiopia zone.\n# NBE framework; Hawassa Industrial Park SEZ.")

zone("ug", "Uganda", "sovereign-govos", ["ug"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "bou": ("Bank of Uganda", "${BOU_URL:-}"),
         "cma_ug": ("Capital Markets Authority Uganda", "${CMA_UG_URL:-}"),
         "ura": ("Uganda Revenue Authority", "${URA_URL:-}"),
     },
     comment="Uganda zone.\n# BOU + CMA framework; mobile money innovation hub.")

zone("cm", "Cameroon", "sovereign-govos", ["cm"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "beac": ("Banque des Etats de l'Afrique Centrale", "${BEAC_URL:-}"),
         "dgi_cm": ("Direction Generale des Impots", "${DGI_CM_URL:-}"),
     },
     comment="Cameroon zone.\n# BEAC (CEMAC central bank) framework.")

zone("sn", "Senegal", "sovereign-govos", ["sn"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "bceao": ("Banque Centrale des Etats de l'Afrique de l'Ouest", "${BCEAO_URL:-}"),
         "dgid": ("Direction Generale des Impots et des Domaines", "${DGID_URL:-}"),
     },
     comment="Senegal zone.\n# BCEAO (WAEMU central bank) framework; Dakar fintech hub.")

zone("ci", "Cote d'Ivoire", "sovereign-govos", ["ci"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "bceao": ("BCEAO", "${BCEAO_URL:-}"),
         "dgi_ci": ("Direction Generale des Impots", "${DGI_CI_URL:-}"),
     },
     comment="Cote d'Ivoire zone.\n# BCEAO framework; Abidjan financial center.")

# ═══════════════════════════════════════════════════════════════════════════
# EUROPE
# ═══════════════════════════════════════════════════════════════════════════
zone("gb", "United Kingdom", "sovereign-govos", ["gb"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "fca": ("Financial Conduct Authority", "${FCA_URL:-}"),
         "hmrc": ("HM Revenue and Customs", "${HMRC_URL:-}"),
         "companies_house": ("Companies House", "${COMPANIES_HOUSE_URL:-}"),
         "pra": ("Prudential Regulation Authority", "${PRA_URL:-}"),
     },
     comment="United Kingdom zone.\n# FCA framework; Financial Services and Markets Act 2000.\n# Crypto-asset registration regime (MLR 2017 as amended).")

zone("gb-gi", "Gibraltar", "digital-financial-center", ["gb", "gb-gi"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "gfsc": ("Gibraltar Financial Services Commission", "${GFSC_URL:-}"),
         "tax_gi": ("Income Tax Office Gibraltar", "${TAX_GI_URL:-}"),
     },
     comment="Gibraltar zone.\n# GFSC DLT Provider framework; established crypto regulation.\n# Financial Services (Distributed Ledger Technology Providers) Regulations 2020.")

zone("lu", "Luxembourg", "digital-financial-center", ["lu"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "cssf": ("Commission de Surveillance du Secteur Financier", "${CSSF_URL:-}"),
         "acd": ("Administration des Contributions Directes", "${ACD_URL:-}"),
         "rcsl": ("Registre de Commerce et des Societes", "${RCSL_URL:-}"),
     },
     comment="Luxembourg zone.\n# CSSF framework; EU MiCA implementation.\n# Fund domiciliation hub; securitization law center.")

zone("ch", "Switzerland", "sovereign-govos", ["ch"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "finma": ("Swiss Financial Market Supervisory Authority", "${FINMA_URL:-}"),
         "estv": ("Federal Tax Administration", "${ESTV_URL:-}"),
         "zefix": ("Central Business Name Index", "${ZEFIX_URL:-}"),
     },
     comment="Switzerland zone.\n# FINMA framework; DLT Act 2021.\n# Established crypto regulation; token classification guidance.")

zone("ch-zug", "Zug Crypto Valley", "digital-financial-center",
     ["ch", "ch-zug"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "finma": ("FINMA", "${FINMA_URL:-}"),
         "estv": ("Federal Tax Administration", "${ESTV_URL:-}"),
     },
     comment="Zug Crypto Valley zone.\n# Cantonal overlay on Swiss federal framework.\n# Crypto Valley ecosystem; Ethereum Foundation headquarters.")

zone("li", "Liechtenstein", "digital-financial-center", ["li"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "fma_li": ("Financial Market Authority Liechtenstein", "${FMA_LI_URL:-}"),
         "stv_li": ("Liechtenstein Tax Administration", "${STV_LI_URL:-}"),
     },
     comment="Liechtenstein zone.\n# FMA framework; Token and TT Service Provider Act (TVTG).\n# Blockchain Act — comprehensive token economy regulation.")

zone("ee", "Estonia", "digital-financial-center", ["ee"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "efsa": ("Estonian Financial Supervision and Resolution Authority", "${EFSA_URL:-}"),
         "emta": ("Estonian Tax and Customs Board", "${EMTA_URL:-}"),
         "rik": ("Centre of Registers and Information Systems", "${RIK_URL:-}"),
     },
     comment="Estonia zone.\n# EFSA framework; EU MiCA implementation.\n# e-Residency digital-first company formation program.")

zone("mt", "Malta", "digital-financial-center", ["mt"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "mfsa": ("Malta Financial Services Authority", "${MFSA_URL:-}"),
         "cft": ("Commissioner for Tax", "${CFT_MT_URL:-}"),
         "mbr": ("Malta Business Registry", "${MBR_URL:-}"),
     },
     comment="Malta zone.\n# MFSA framework; Virtual Financial Assets Act 2018.\n# Innovative Technology Arrangements and Services Act.")

zone("cy", "Cyprus", "digital-financial-center", ["cy"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "cysec": ("Cyprus Securities and Exchange Commission", "${CYSEC_URL:-}"),
         "cbc": ("Central Bank of Cyprus", "${CBC_URL:-}"),
         "drcor": ("Department of Registrar of Companies", "${DRCOR_URL:-}"),
     },
     comment="Cyprus zone.\n# CySEC framework; EU MiCA implementation.\n# Established fund and forex licensing jurisdiction.")

# Tier 4 Europe
zone("de", "Germany", "sovereign-govos", ["de"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "bafin": ("Federal Financial Supervisory Authority", "${BAFIN_URL:-}"),
         "bzst": ("Federal Central Tax Office", "${BZST_URL:-}"),
         "handelsregister": ("Commercial Register", "${HANDELSREGISTER_URL:-}"),
     },
     comment="Germany zone.\n# BaFin framework; KWG banking law.\n# Strict GDPR implementation (BDSG); crypto custody license.")

zone("fr", "France", "sovereign-govos", ["fr"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "amf": ("Autorite des Marches Financiers", "${AMF_URL:-}"),
         "acpr": ("Autorite de Controle Prudentiel et de Resolution", "${ACPR_URL:-}"),
         "dgfip": ("Direction Generale des Finances Publiques", "${DGFIP_URL:-}"),
     },
     comment="France zone.\n# AMF + ACPR framework; PACTE Law (2019).\n# DASP (Digital Asset Service Provider) registration.")

zone("nl", "Netherlands", "sovereign-govos", ["nl"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "dnb": ("De Nederlandsche Bank", "${DNB_URL:-}"),
         "afm": ("Authority for the Financial Markets", "${AFM_URL:-}"),
         "kvk": ("Chamber of Commerce", "${KVK_URL:-}"),
     },
     comment="Netherlands zone.\n# DNB + AFM framework; Wet op het financieel toezicht.\n# EU PSD2 payment innovation hub.")

zone("es", "Spain", "sovereign-govos", ["es"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "cnmv": ("Comision Nacional del Mercado de Valores", "${CNMV_URL:-}"),
         "bde": ("Banco de Espana", "${BDE_URL:-}"),
         "aeat": ("Agencia Estatal de Administracion Tributaria", "${AEAT_URL:-}"),
     },
     comment="Spain zone.\n# CNMV + BdE framework; EU MiCA implementation.\n# Regulatory sandbox (Ley Sandbox 2020).")

zone("it", "Italy", "sovereign-govos", ["it"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "consob": ("Commissione Nazionale per le Societa e la Borsa", "${CONSOB_URL:-}"),
         "bdi": ("Banca d'Italia", "${BDI_URL:-}"),
         "ade": ("Agenzia delle Entrate", "${ADE_URL:-}"),
     },
     comment="Italy zone.\n# Consob + Banca d'Italia framework.\n# EU MiCA implementation; fintech sandbox.")

zone("at", "Austria", "sovereign-govos", ["at"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "fma_at": ("Financial Market Authority Austria", "${FMA_AT_URL:-}"),
         "bmf_at": ("Federal Ministry of Finance", "${BMF_AT_URL:-}"),
     },
     comment="Austria zone.\n# FMA framework; EU MiCA implementation.\n# Progressive approach to digital assets.")

zone("se", "Sweden", "sovereign-govos", ["se"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "fi_se": ("Finansinspektionen", "${FI_SE_URL:-}"),
         "skatteverket": ("Swedish Tax Agency", "${SKATTEVERKET_URL:-}"),
         "bolagsverket": ("Swedish Companies Registration Office", "${BOLAGSVERKET_URL:-}"),
     },
     comment="Sweden zone.\n# Finansinspektionen framework; EU MiCA.\n# Riksbank e-krona CBDC pilot; Swish instant payments.")

zone("dk", "Denmark", "sovereign-govos", ["dk"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "dfsa": ("Danish Financial Supervisory Authority", "${DFSA_DK_URL:-}"),
         "skat": ("Danish Tax Agency", "${SKAT_URL:-}"),
         "erst": ("Danish Business Authority", "${ERST_URL:-}"),
     },
     comment="Denmark zone.\n# Finanstilsynet framework; EU MiCA.\n# Danish Financial Business Act; NemID/MitID digital identity.")

zone("fi", "Finland", "sovereign-govos", ["fi"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "fiva": ("Financial Supervisory Authority Finland", "${FIVA_URL:-}"),
         "vero": ("Finnish Tax Administration", "${VERO_URL:-}"),
         "prh": ("Finnish Patent and Registration Office", "${PRH_URL:-}"),
     },
     comment="Finland zone.\n# FIN-FSA framework; EU MiCA.\n# Virtual currency provider registration (Act 572/2019).")

zone("no", "Norway", "sovereign-govos", ["no"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "finanstilsynet_no": ("Finanstilsynet Norway", "${FINANSTILSYNET_NO_URL:-}"),
         "skatteetaten": ("Norwegian Tax Administration", "${SKATTEETATEN_URL:-}"),
         "brreg": ("Bronnoysund Register Centre", "${BRREG_URL:-}"),
     },
     comment="Norway zone.\n# Finanstilsynet framework; AML Act (Hvitvaskingsloven).\n# FATF 4th round rated Largely Compliant.")

# ═══════════════════════════════════════════════════════════════════════════
# ASIA-PACIFIC
# ═══════════════════════════════════════════════════════════════════════════
zone("jp", "Japan", "sovereign-govos", ["jp"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "fsa_japan": ("Financial Services Agency", "${FSA_JP_URL:-}"),
         "nta": ("National Tax Agency", "${NTA_URL:-}"),
         "moj_jp": ("Ministry of Justice (corporate registry)", "${MOJ_JP_URL:-}"),
     },
     comment="Japan zone.\n# FSA framework; Payment Services Act; FIEA.\n# Virtual asset exchange registration (JVCEA self-regulation).")

zone("kr", "South Korea", "sovereign-govos", ["kr"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "fsc_kr": ("Financial Services Commission", "${FSC_KR_URL:-}"),
         "fss": ("Financial Supervisory Service", "${FSS_URL:-}"),
         "nts_kr": ("National Tax Service", "${NTS_KR_URL:-}"),
     },
     comment="South Korea zone.\n# FSC/FSS framework; Virtual Asset User Protection Act 2023.\n# Specific Financial Information Act (reporting obligations).")

zone("in-gift", "GIFT City", "digital-financial-center",
     ["in", "in-gift"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "ifsca": ("International Financial Services Centres Authority", "${IFSCA_URL:-}"),
     },
     comment="Gujarat International Finance Tec-City (GIFT City).\n# IFSCA regulated; India's first International Financial Services Centre.\n# SEZ status with tax incentives.")

zone("in-ifsc", "IFSC Gujarat", "digital-financial-center",
     ["in", "in-ifsc"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "ifsca": ("IFSCA", "${IFSCA_URL:-}"),
     },
     comment="International Financial Services Centre Gujarat.\n# IFSCA regulated; SEZ status.\n# Banking, insurance, securities, fund management.")

zone("my", "Malaysia", "sovereign-govos", ["my"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "sc_my": ("Securities Commission Malaysia", "${SC_MY_URL:-}"),
         "bnm": ("Bank Negara Malaysia", "${BNM_URL:-}"),
         "lhdn": ("Inland Revenue Board of Malaysia", "${LHDN_URL:-}"),
         "ssm": ("Companies Commission of Malaysia", "${SSM_URL:-}"),
     },
     comment="Malaysia zone.\n# SC Malaysia + BNM framework; Labuan offshore.\n# Islamic Financial Services Act 2013 (IFSA).")

zone("my-labuan", "Labuan IBFC", "digital-financial-center",
     ["my", "my-labuan"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "lfsa": ("Labuan Financial Services Authority", "${LFSA_URL:-}"),
     },
     comment="Labuan International Business and Financial Centre.\n# LFSA regulated; Labuan Companies Act 1990.\n# Offshore financial center; Islamic finance hub.")

zone("th", "Thailand", "sovereign-govos", ["th"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "sec_th": ("Securities and Exchange Commission Thailand", "${SEC_TH_URL:-}"),
         "bot": ("Bank of Thailand", "${BOT_URL:-}"),
         "rd_th": ("Revenue Department Thailand", "${RD_TH_URL:-}"),
     },
     comment="Thailand zone.\n# SEC Thailand + BOT framework.\n# Digital Asset Business Royal Decree B.E. 2561 (2018).")

zone("ph", "Philippines", "sovereign-govos", ["ph"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "bsp": ("Bangko Sentral ng Pilipinas", "${BSP_URL:-}"),
         "sec_ph": ("Securities and Exchange Commission Philippines", "${SEC_PH_URL:-}"),
         "bir": ("Bureau of Internal Revenue", "${BIR_URL:-}"),
     },
     comment="Philippines zone.\n# BSP + SEC framework; CEZA economic zone.\n# BSP Circular 1108 (virtual asset service providers).")

zone("vn", "Vietnam", "sovereign-govos", ["vn"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "sbv": ("State Bank of Vietnam", "${SBV_URL:-}"),
         "ssc": ("State Securities Commission", "${SSC_URL:-}"),
         "gdt_vn": ("General Department of Taxation", "${GDT_VN_URL:-}"),
     },
     comment="Vietnam zone.\n# SBV + SSC framework; emerging digital asset regulation.\n# Van Don, Phu Quoc, Bac Van Phong special economic zones.")

zone("au", "Australia", "sovereign-govos", ["au"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "asic": ("Australian Securities and Investments Commission", "${ASIC_URL:-}"),
         "apra": ("Australian Prudential Regulation Authority", "${APRA_URL:-}"),
         "ato": ("Australian Taxation Office", "${ATO_URL:-}"),
     },
     comment="Australia zone.\n# ASIC + APRA framework; Corporations Act 2001.\n# Token mapping consultation; digital asset licensing proposal.")

zone("nz", "New Zealand", "sovereign-govos", ["nz"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "fma_nz": ("Financial Markets Authority", "${FMA_NZ_URL:-}"),
         "ird": ("Inland Revenue Department", "${IRD_URL:-}"),
     },
     comment="New Zealand zone.\n# FMA framework; Financial Markets Conduct Act 2013.\n# Pragmatic approach to digital asset regulation.")

zone("tw", "Taiwan", "sovereign-govos", ["tw"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "fsc_tw": ("Financial Supervisory Commission", "${FSC_TW_URL:-}"),
         "mof_tw": ("Ministry of Finance", "${MOF_TW_URL:-}"),
     },
     comment="Taiwan zone.\n# FSC framework; virtual asset guidance.\n# Emerging VASP registration framework.")

zone("bn", "Brunei", "sovereign-govos", ["bn"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "ambd": ("Autoriti Monetari Brunei Darussalam", "${AMBD_URL:-}"),
     },
     comment="Brunei zone.\n# AMBD framework; emerging fintech sandbox.\n# Securities Markets Order 2013.")

zone("mm", "Myanmar", "sovereign-govos", ["mm"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "cbm": ("Central Bank of Myanmar", "${CBM_URL:-}"),
     },
     comment="Myanmar zone.\n# CBM framework; limited digital asset regulation.\n# Financial Institutions Law 2016.")

# Tier 4 Southeast Asia
zone("kh", "Cambodia", "minimal-mvp", ["kh"],
     adapters={"nbc": ("National Bank of Cambodia", "${NBC_URL:-}")},
     comment="Cambodia zone.\n# NBC framework; Bakong payment system.\n# Prakas on digital asset regulation (emerging).")

zone("la", "Laos", "minimal-mvp", ["la"],
     adapters={"bol": ("Bank of the Lao PDR", "${BOL_URL:-}")},
     comment="Laos zone.\n# BOL framework; emerging digital economy.\n# Lao-China Railway Special Economic Zones.")

# Tier 4 South Asia
zone("lk", "Sri Lanka", "sovereign-govos", ["lk"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "cbsl": ("Central Bank of Sri Lanka", "${CBSL_URL:-}"),
         "sec_lk": ("Securities and Exchange Commission Sri Lanka", "${SEC_LK_URL:-}"),
     },
     comment="Sri Lanka zone.\n# CBSL + SEC framework; Colombo Port City SEZ.\n# Payment and Settlement Systems Act.")

zone("bd", "Bangladesh", "sovereign-govos", ["bd"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "bb": ("Bangladesh Bank", "${BB_URL:-}"),
         "bsec": ("Bangladesh Securities and Exchange Commission", "${BSEC_URL:-}"),
         "nbr_bd": ("National Board of Revenue", "${NBR_BD_URL:-}"),
     },
     comment="Bangladesh zone.\n# Bangladesh Bank framework; mobile financial services.\n# bKash/Nagad mobile money ecosystem.")

zone("np", "Nepal", "minimal-mvp", ["np"],
     adapters={"nrb": ("Nepal Rastra Bank", "${NRB_URL:-}")},
     comment="Nepal zone.\n# NRB framework; emerging digital payments.\n# Payment and Settlement Act 2019.")

# Tier 4 Pacific Islands
zone("fj", "Fiji", "minimal-mvp", ["fj"],
     adapters={"rbf": ("Reserve Bank of Fiji", "${RBF_URL:-}")},
     comment="Fiji zone.\n# RBF framework; Pacific Islands economic hub.\n# Financial Transactions Reporting Act 2004.")

zone("vu", "Vanuatu", "minimal-mvp", ["vu"],
     adapters={"rbv": ("Reserve Bank of Vanuatu", "${RBV_URL:-}")},
     comment="Vanuatu zone.\n# RBV framework; Financial Dealers Licensing Act.\n# Pacific Islands financial center.")

zone("ws", "Samoa", "minimal-mvp", ["ws"],
     adapters={"cbs": ("Central Bank of Samoa", "${CBS_URL:-}")},
     comment="Samoa zone.\n# CBS framework; International Companies Act 1988.\n# Pacific Islands offshore financial center.")

# ═══════════════════════════════════════════════════════════════════════════
# AMERICAS
# ═══════════════════════════════════════════════════════════════════════════
zone("ca", "Canada", "sovereign-govos", ["ca"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "osfi": ("Office of the Superintendent of Financial Institutions", "${OSFI_URL:-}"),
         "fintrac": ("Financial Transactions and Reports Analysis Centre", "${FINTRAC_URL:-}"),
         "cra": ("Canada Revenue Agency", "${CRA_URL:-}"),
     },
     comment="Canada zone.\n# CSA + OSFI framework; MSB registration.\n# Canadian Securities Administrators; FINTRAC AML oversight.")

zone("ca-on", "Ontario", "sovereign-govos", ["ca", "ca-on"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "osc": ("Ontario Securities Commission", "${OSC_URL:-}"),
         "fsra_on": ("Financial Services Regulatory Authority Ontario", "${FSRA_ON_URL:-}"),
     },
     comment="Ontario zone.\n# OSC framework; provincial securities overlay.\n# Securities Act (Ontario); crypto-asset trading platform guidance.")

zone("bm", "Bermuda", "digital-financial-center", ["bm"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "bma": ("Bermuda Monetary Authority", "${BMA_URL:-}"),
         "roc_bm": ("Registrar of Companies Bermuda", "${ROC_BM_URL:-}"),
     },
     comment="Bermuda zone.\n# BMA framework; Digital Asset Business Act 2018.\n# Class F (full) and Class M (modified) DABA licenses.")

zone("bs", "Bahamas", "digital-financial-center", ["bs"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "scb": ("Securities Commission of the Bahamas", "${SCB_URL:-}"),
         "cbob": ("Central Bank of the Bahamas", "${CBOB_URL:-}"),
     },
     comment="Bahamas zone.\n# SCB framework; Digital Assets and Registered Exchanges Act (DARE).\n# Sand Dollar CBDC.")

zone("bb", "Barbados", "digital-financial-center", ["bb"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={
         "cbb_bb": ("Central Bank of Barbados", "${CBB_BB_URL:-}"),
         "fsc_bb": ("Financial Services Commission Barbados", "${FSC_BB_URL:-}"),
     },
     comment="Barbados zone.\n# CBB + FSC framework; emerging fintech regulation.\n# International Business Companies; offshore financial center.")

zone("pa", "Panama", "sovereign-govos", ["pa"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "smv": ("Superintendencia del Mercado de Valores", "${SMV_URL:-}"),
         "sbp": ("Superintendencia de Bancos de Panama", "${SBP_PA_URL:-}"),
         "dgi_pa": ("Direccion General de Ingresos", "${DGI_PA_URL:-}"),
     },
     comment="Panama zone.\n# SMV + SBP framework; Ley 129 (crypto law).\n# Panama Pacifico Special Economic Area; Colon Free Zone.")

zone("cr", "Costa Rica", "sovereign-govos", ["cr"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "sugef": ("Superintendencia General de Entidades Financieras", "${SUGEF_URL:-}"),
         "conassif": ("Consejo Nacional de Supervision del Sistema Financiero", "${CONASSIF_URL:-}"),
     },
     comment="Costa Rica zone.\n# SUGEF + CONASSIF framework.\n# Emerging digital asset regulation; SINPE payment system.")

zone("mx", "Mexico", "sovereign-govos", ["mx"],
     compliance=EXTENDED_COMPLIANCE,
     adapters={
         "cnbv": ("Comision Nacional Bancaria y de Valores", "${CNBV_URL:-}"),
         "banxico": ("Banco de Mexico", "${BANXICO_URL:-}"),
         "sat": ("Servicio de Administracion Tributaria", "${SAT_URL:-}"),
     },
     comment="Mexico zone.\n# CNBV + Banxico framework; Ley Fintech (2018).\n# ITF (Instituciones de Tecnologia Financiera) licensing.")

zone("co", "Colombia", "sovereign-govos", ["co"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "sfc": ("Superintendencia Financiera de Colombia", "${SFC_URL:-}"),
         "banrep": ("Banco de la Republica", "${BANREP_URL:-}"),
         "dian": ("Direccion de Impuestos y Aduanas Nacionales", "${DIAN_URL:-}"),
     },
     comment="Colombia zone.\n# SFC + BanRep framework; regulatory sandbox.\n# Decree 1357/2018 (crowdfunding); emerging crypto regulation.")

zone("cl", "Chile", "sovereign-govos", ["cl"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "cmf_cl": ("Comision para el Mercado Financiero", "${CMF_CL_URL:-}"),
         "bcch": ("Banco Central de Chile", "${BCCH_URL:-}"),
         "sii": ("Servicio de Impuestos Internos", "${SII_URL:-}"),
     },
     comment="Chile zone.\n# CMF + BCCh framework; FinTech Law (Ley 21.521/2023).\n# Ley 18.045 (Securities Market Law).")

zone("ar", "Argentina", "sovereign-govos", ["ar"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "cnv": ("Comision Nacional de Valores", "${CNV_URL:-}"),
         "bcra": ("Banco Central de la Republica Argentina", "${BCRA_URL:-}"),
         "afip": ("Administracion Federal de Ingresos Publicos", "${AFIP_URL:-}"),
     },
     comment="Argentina zone.\n# CNV + BCRA framework; digital asset PSP rules.\n# Resolution 994/2024 (VASP obligations).")

zone("uy", "Uruguay", "sovereign-govos", ["uy"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "bcu": ("Banco Central del Uruguay", "${BCU_URL:-}"),
         "ssf": ("Superintendencia de Servicios Financieros", "${SSF_URL:-}"),
         "dgi_uy": ("Direccion General Impositiva", "${DGI_UY_URL:-}"),
     },
     comment="Uruguay zone.\n# BCU + SSF framework; emerging fintech regulation.\n# Stable tax environment; extensive bilateral treaty network.")

zone("py", "Paraguay", "sovereign-govos", ["py"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "bcp": ("Banco Central del Paraguay", "${BCP_URL:-}"),
         "cnv_py": ("Comision Nacional de Valores", "${CNV_PY_URL:-}"),
         "set_py": ("Subsecretaria de Estado de Tributacion", "${SET_PY_URL:-}"),
     },
     comment="Paraguay zone.\n# BCP + CNV framework; crypto mining law.\n# Low energy costs; Bitcoin mining hub.")

zone("sv", "El Salvador", "sovereign-govos", ["sv"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "bcr": ("Banco Central de Reserva", "${BCR_URL:-}"),
         "ssf_sv": ("Superintendencia del Sistema Financiero", "${SSF_SV_URL:-}"),
         "cnad": ("Comision Nacional de Activos Digitales", "${CNAD_URL:-}"),
     },
     comment="El Salvador zone.\n# BCR + SSF framework; Bitcoin Legal Tender Law (2021).\n# CNAD (National Commission on Digital Assets) oversight.")

# Tier 4 Caribbean
zone("jm", "Jamaica", "sovereign-govos", ["jm"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "boj": ("Bank of Jamaica", "${BOJ_URL:-}"),
         "fsc_jm": ("Financial Services Commission Jamaica", "${FSC_JM_URL:-}"),
         "taj": ("Tax Administration Jamaica", "${TAJ_URL:-}"),
     },
     comment="Jamaica zone.\n# BOJ + FSC framework; JAM-DEX CBDC.\n# Securities Act; emerging crypto regulation.")

zone("tt", "Trinidad and Tobago", "sovereign-govos", ["tt"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "cbtt": ("Central Bank of Trinidad and Tobago", "${CBTT_URL:-}"),
         "ttsec": ("Trinidad and Tobago Securities and Exchange Commission", "${TTSEC_URL:-}"),
     },
     comment="Trinidad and Tobago zone.\n# CBTT + TTSEC framework.\n# Financial Institutions Act; FATF Caribbean member.")

zone("tc", "Turks and Caicos", "digital-financial-center", ["tc"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={"fsc_tc": ("Financial Services Commission TCI", "${FSC_TC_URL:-}")},
     comment="Turks and Caicos Islands zone.\n# FSC framework; Companies Ordinance.\n# Offshore financial center; no direct taxation.")

zone("ag", "Antigua and Barbuda", "digital-financial-center", ["ag"],
     compliance=FC_COMPLIANCE, corridors=FC_CORRIDORS,
     licensepacks=["financial", "corporate"],
     adapters={"fsrc": ("Financial Services Regulatory Commission", "${FSRC_URL:-}")},
     comment="Antigua and Barbuda zone.\n# FSRC framework; Virtual Asset Business Act 2022.\n# Citizenship by Investment Programme.")

zone("dm", "Dominica", "minimal-mvp", ["dm"],
     adapters={"eccb": ("Eastern Caribbean Central Bank", "${ECCB_URL:-}")},
     comment="Dominica zone.\n# ECCB framework; Offshore Banking Act.\n# OECS monetary union member.")

zone("gd", "Grenada", "minimal-mvp", ["gd"],
     adapters={"eccb": ("ECCB", "${ECCB_URL:-}"), "garfin": ("GARFIN", "${GARFIN_URL:-}")},
     comment="Grenada zone.\n# GARFIN regulatory authority; ECCB monetary union.\n# International Financial Services Act.")

zone("lc", "Saint Lucia", "minimal-mvp", ["lc"],
     adapters={"eccb": ("ECCB", "${ECCB_URL:-}"), "fsra_lc": ("Financial Services Regulatory Authority", "${FSRA_LC_URL:-}")},
     comment="Saint Lucia zone.\n# FSRA framework; ECCB monetary union.\n# International Business Companies Act.")

zone("vc", "Saint Vincent and the Grenadines", "minimal-mvp", ["vc"],
     adapters={"eccb": ("ECCB", "${ECCB_URL:-}"), "fsa_vc": ("Financial Services Authority", "${FSA_VC_URL:-}")},
     comment="Saint Vincent and the Grenadines zone.\n# FSA framework; ECCB monetary union.\n# International financial services.")

# Tier 4 Central Asia
zone("uz", "Uzbekistan", "sovereign-govos", ["uz"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "cbu": ("Central Bank of Uzbekistan", "${CBU_URL:-}"),
         "cma_uz": ("Capital Markets Authority Uzbekistan", "${CMA_UZ_URL:-}"),
     },
     comment="Uzbekistan zone.\n# CBU framework; crypto-mining legalization.\n# Presidential decree on digital economy development.")

zone("ge", "Georgia", "sovereign-govos", ["ge"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "nbg": ("National Bank of Georgia", "${NBG_URL:-}"),
         "rs_ge": ("Revenue Service Georgia", "${RS_GE_URL:-}"),
     },
     comment="Georgia zone.\n# NBG framework; crypto-friendly tax regime.\n# 0% personal income tax on crypto gains; flat 20% corporate.")

# Tier 4 Middle East
zone("kw", "Kuwait", "sovereign-govos", ["kw"],
     compliance=SOVEREIGN_COMPLIANCE,
     adapters={
         "cbk": ("Central Bank of Kuwait", "${CBK_URL:-}"),
         "cma_kw": ("Capital Markets Authority Kuwait", "${CMA_KW_URL:-}"),
     },
     comment="Kuwait zone.\n# CBK + CMA framework; restrictive crypto stance.\n# Law 7/2010 (Capital Markets Authority).")

zone("lb", "Lebanon", "minimal-mvp", ["lb"],
     adapters={"bdl": ("Banque du Liban", "${BDL_URL:-}")},
     comment="Lebanon zone.\n# BDL framework; Banking Secrecy Law.\n# Capital controls; limited digital regulation.")

zone("iq", "Iraq", "minimal-mvp", ["iq"],
     adapters={"cbi": ("Central Bank of Iraq", "${CBI_URL:-}")},
     comment="Iraq zone.\n# CBI framework; emerging financial regulation.\n# Banking Law No. 94 of 2004.")


# ═══════════════════════════════════════════════════════════════════════════
# YAML GENERATION
# ═══════════════════════════════════════════════════════════════════════════

PK_REGPACKS = [
    {
        "domain": "financial",
        "jurisdiction_id": "pk",
        "regpack_digest_sha256": "444ddded8419d9dedf8344a54063d7cd80c0148338c78bbe77a47baa44dd392f",
        "as_of_date": "2026-01-15",
    },
    {
        "domain": "sanctions",
        "jurisdiction_id": "pk",
        "regpack_digest_sha256": "e59056a2b9bdbf3e452857b1cbdc06b5cdff3e29f56de1e235475e8a4a57506f",
        "as_of_date": "2026-01-15",
    },
]

SG_REGPACKS = [
    {
        "domain": "financial",
        "jurisdiction_id": "sg",
        "regpack_digest_sha256": "4d2a1f73a440fc67f5281e70436737b340e114173138cddd20f98c60d0154496",
        "as_of_date": "2026-01-15",
    },
    {
        "domain": "sanctions",
        "jurisdiction_id": "sg",
        "regpack_digest_sha256": "2afd914d470a8d8c6934c2195a51d8130637ded17ca714732ca0ff5ee3c25108",
        "as_of_date": "2026-01-15",
    },
]

HK_REGPACKS = [
    {
        "domain": "financial",
        "jurisdiction_id": "hk",
        "regpack_digest_sha256": "89dc928a5460dc90ef39c5557c9563b395da4560e5004b868adbe013d1b6d256",
        "as_of_date": "2026-01-15",
    },
    {
        "domain": "sanctions",
        "jurisdiction_id": "hk",
        "regpack_digest_sha256": "6bd125db0d30631dd446a40e4fde9bbafd13c1e292d97f8fc060c7c235c2e204",
        "as_of_date": "2026-01-15",
    },
]

KY_REGPACKS = [
    {
        "domain": "financial",
        "jurisdiction_id": "ky",
        "regpack_digest_sha256": "b2f8c1a6d953e784f21b5407dc6e83fea0901bb8c13c4a7e9852a1d1fc0ab3e5",
        "as_of_date": "2026-01-15",
    },
    {
        "domain": "sanctions",
        "jurisdiction_id": "ky",
        "regpack_digest_sha256": "71a4e8c0d23f59b7e816f2430ad97c5be3f10629d854a71c6b83e2f0194d58a7",
        "as_of_date": "2026-01-15",
    },
]

REGPACK_MAP = {
    "ae": AE_REGPACKS,
    "pk": PK_REGPACKS,
    "sg": SG_REGPACKS,
    "hk": HK_REGPACKS,
    "ky": KY_REGPACKS,
}


def zone_id_from_jid(jid):
    """Convert jurisdiction_id to zone_id format."""
    parts = jid.split("-")
    return "org.momentum.mez.zone." + ".".join(parts)


def profile_id_from_type(ptype):
    """Convert profile type to full profile_id."""
    return f"org.momentum.mez.profile.{ptype}"


def render_zone_yaml(z):
    """Render a zone definition to YAML string."""
    lines = []

    # Header comment
    lines.append(f"# Zone Manifest: {z['name']}")
    lines.append("#")
    for cline in z["comment"].split("\n"):
        lines.append(f"# {cline}" if cline else "#")
    stack_display = " > ".join(z["stack"])
    lines.append(f"#")
    lines.append(f"# jurisdiction_stack: {stack_display}")
    lines.append("")

    # Core fields
    lines.append(f"zone_id: {zone_id_from_jid(z['jid'])}")
    lines.append(f"jurisdiction_id: {z['jid']}")
    lines.append(f"zone_name: {z['name']}")
    lines.append("")

    # Profile
    lines.append("profile:")
    lines.append(f"  profile_id: {profile_id_from_type(z['profile'])}")
    lines.append('  version: "0.4.44"')
    lines.append("")

    # Jurisdiction stack
    lines.append("jurisdiction_stack:")
    for s in z["stack"]:
        lines.append(f"  - {s}")
    lines.append("")

    # Lawpack domains
    lines.append("lawpack_domains:")
    for lp in z["lawpacks"]:
        lines.append(f"  - {lp}")
    lines.append("")

    # Licensepack domains (if present)
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

    # Regpacks (only if parent has a builder)
    rp_parent = z.get("regpack_parent")
    if rp_parent and rp_parent in REGPACK_MAP:
        lines.append("regpacks:")
        for rp in REGPACK_MAP[rp_parent]:
            lines.append(f"  - domain: {rp['domain']}")
            lines.append(f"    jurisdiction_id: {rp['jurisdiction_id']}")
            lines.append(f'    regpack_digest_sha256: "{rp["regpack_digest_sha256"]}"')
            lines.append(f'    as_of_date: "{rp["as_of_date"]}"')
        lines.append("")
    else:
        lines.append("# Regpack builder needed")
        lines.append("")

    # Compliance domains
    lines.append("compliance_domains:")
    for cd in z["compliance"]:
        lines.append(f"  - {cd}")
    lines.append("")

    # Corridors
    lines.append("corridors:")
    for c in z["corridors"]:
        lines.append(f"  - {c}")
    lines.append("")

    # National adapters
    if z["adapters"]:
        lines.append("national_adapters:")
        for adapter_key, (desc, endpoint) in z["adapters"].items():
            lines.append(f"  {adapter_key}:")
            lines.append(f"    enabled: false")
            lines.append(f'    endpoint: "{endpoint}"')
    else:
        lines.append("national_adapters: {}")
    lines.append("")

    # Trust anchors + key management
    lines.append("trust_anchors: []")
    lines.append("")
    lines.append("key_management:")
    lines.append("  rotation_interval_days: 90")
    lines.append("  grace_period_days: 14")
    lines.append("")
    lines.append("lockfile_path: stack.lock")

    return "\n".join(lines) + "\n"


def main():
    existing = set(os.listdir(JURISDICTIONS_DIR))
    created = 0
    skipped = 0

    for z in ZONES:
        jid = z["jid"]
        zone_dir = os.path.join(JURISDICTIONS_DIR, jid)
        zone_file = os.path.join(zone_dir, "zone.yaml")

        if jid in existing:
            print(f"  SKIP (exists): {jid}")
            skipped += 1
            continue

        os.makedirs(zone_dir, exist_ok=True)
        content = render_zone_yaml(z)
        with open(zone_file, "w") as f:
            f.write(content)
        created += 1
        print(f"  CREATE: {jid}")

    print(f"\nDone: {created} created, {skipped} skipped (already exist)")


if __name__ == "__main__":
    main()
