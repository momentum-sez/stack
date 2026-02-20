#!/usr/bin/env python3
"""Enrich US state/territory scaffold zone.yaml files.

Transforms 28-line scaffolds into enriched manifests with compliance_domains,
national_adapters (real federal + state regulator names), key_management, etc.

All US states share:
- Federal regulators: FinCEN, IRS, SEC, OCC
- State-level regulator: varies by state
- Profile: sovereign-govos (states are sovereign regulatory entities)
- compliance_domains: aml, kyc, sanctions, tax, corporate, licensing
- corridors: SWIFT ISO 20022 cross-border

Usage:
    python3 scripts/enrich-us-zones.py
"""

import os

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
JURISDICTIONS_DIR = os.path.join(REPO_ROOT, "jurisdictions")

# ── US state/territory data ─────────────────────────────────────────────────
# (state_code, full_name, state_regulator_key, state_regulator_desc)
US_STATES = [
    ("us-ak", "Alaska", "dced_ak", "Division of Banking and Securities"),
    ("us-al", "Alabama", "asbc", "Alabama Securities Commission"),
    ("us-ar", "Arkansas", "asbd", "Arkansas Securities Department"),
    ("us-as", "American Samoa", "asg_treasury", "American Samoa Treasury"),
    ("us-az", "Arizona", "azdfi", "Arizona Department of Financial Institutions"),
    ("us-ca", "California", "dfpi", "Department of Financial Protection and Innovation"),
    ("us-co", "Colorado", "dora_co", "Division of Banking"),
    ("us-ct", "Connecticut", "dobi_ct", "Department of Banking"),
    ("us-dc", "District of Columbia", "disb", "Department of Insurance, Securities and Banking"),
    ("us-de", "Delaware", "ode", "Office of the State Bank Commissioner"),
    ("us-fl", "Florida", "ofr_fl", "Office of Financial Regulation"),
    ("us-ga", "Georgia", "dbf_ga", "Department of Banking and Finance"),
    ("us-gu", "Guam", "drt_gu", "Department of Revenue and Taxation"),
    ("us-hi", "Hawaii", "dcca_hi", "Division of Financial Institutions"),
    ("us-ia", "Iowa", "idob", "Iowa Division of Banking"),
    ("us-id", "Idaho", "idof", "Idaho Department of Finance"),
    ("us-il", "Illinois", "idfpr", "Division of Financial Institutions"),
    ("us-in", "Indiana", "dfi_in", "Department of Financial Institutions"),
    ("us-ks", "Kansas", "osbc_ks", "Office of the State Bank Commissioner"),
    ("us-ky", "Kentucky", "dfi_ky", "Department of Financial Institutions"),
    ("us-la", "Louisiana", "ofi_la", "Office of Financial Institutions"),
    ("us-ma", "Massachusetts", "dob_ma", "Division of Banks"),
    ("us-md", "Maryland", "ofr_md", "Office of Financial Regulation"),
    ("us-me", "Maine", "obfr_me", "Office of Securities"),
    ("us-mi", "Michigan", "difs", "Department of Insurance and Financial Services"),
    ("us-mn", "Minnesota", "mn_commerce", "Department of Commerce"),
    ("us-mo", "Missouri", "dof_mo", "Division of Finance"),
    ("us-mp", "Northern Mariana Islands", "dof_mp", "Department of Finance"),
    ("us-ms", "Mississippi", "dbcf_ms", "Department of Banking and Consumer Finance"),
    ("us-mt", "Montana", "doa_mt", "Division of Banking and Financial Institutions"),
    ("us-nc", "North Carolina", "nccob", "Commissioner of Banks"),
    ("us-nd", "North Dakota", "dfi_nd", "Department of Financial Institutions"),
    ("us-ne", "Nebraska", "ndbf", "Department of Banking and Finance"),
    ("us-nh", "New Hampshire", "nhbd", "Banking Department"),
    ("us-nj", "New Jersey", "dobi_nj", "Department of Banking and Insurance"),
    ("us-nm", "New Mexico", "rld_nm", "Financial Institutions Division"),
    ("us-nv", "Nevada", "fid_nv", "Financial Institutions Division"),
    ("us-ny", "New York", "nydfs", "New York Department of Financial Services"),
    ("us-oh", "Ohio", "dfi_oh", "Division of Financial Institutions"),
    ("us-ok", "Oklahoma", "osbd_ok", "State Banking Department"),
    ("us-or", "Oregon", "dcbs_or", "Division of Financial Regulation"),
    ("us-pa", "Pennsylvania", "dobs_pa", "Department of Banking and Securities"),
    ("us-pr", "Puerto Rico", "ocif", "Office of the Commissioner of Financial Institutions"),
    ("us-ri", "Rhode Island", "dbr_ri", "Department of Business Regulation"),
    ("us-sc", "South Carolina", "bofi_sc", "Board of Financial Institutions"),
    ("us-sd", "South Dakota", "dol_sd", "Division of Banking"),
    ("us-tn", "Tennessee", "tdfi", "Department of Financial Institutions"),
    ("us-tx", "Texas", "tdob", "Department of Banking"),
    ("us-ut", "Utah", "dfi_ut", "Department of Financial Institutions"),
    ("us-va", "Virginia", "bfi_va", "Bureau of Financial Institutions"),
    ("us-vi", "US Virgin Islands", "lt_vi", "Office of the Lieutenant Governor"),
    ("us-vt", "Vermont", "dfr_vt", "Department of Financial Regulation"),
    ("us-wa", "Washington", "dfi_wa", "Department of Financial Institutions"),
    ("us-wi", "Wisconsin", "dfi_wi", "Department of Financial Institutions"),
    ("us-wv", "West Virginia", "dfi_wv", "Division of Financial Institutions"),
    ("us-wy", "Wyoming", "doa_wy", "Division of Banking"),
]

# Special notes for notable states
SPECIAL_NOTES = {
    "us-wy": "Wyoming DORA; DAO LLC Act (SF0038); SPDI charter.\n# Most crypto-forward US state; Wyoming Stable Token Act.",
    "us-de": "Delaware Division of Corporations; DGCL Title 8.\n# Gold standard for US corporate formation; blockchain amendments.",
    "us-ny": "NYDFS BitLicense framework (23 NYCRR 200).\n# Most stringent US crypto regulation; money transmitter licensing.",
    "us-tx": "TDOB framework; Texas Virtual Currency Act.\n# Texas DBA (Department of Banking) supervision of digital assets.",
    "us-ca": "DFPI framework; California Digital Financial Assets Law (AB 39).\n# Largest US state economy; DFPI BitLicense-equivalent.",
    "us-fl": "OFR framework; Florida Money Services Business Act.\n# Growing crypto hub; no state income tax.",
    "us-co": "DORA framework; Colorado Digital Token Act.\n# Securities exemption for certain utility tokens.",
    "us-pr": "OCIF framework; Act 60 (formerly Acts 20/22).\n# Tax incentives for export services and individual investors.",
}


def zone_id(jid):
    parts = jid.split("-")
    return "org.momentum.mez.zone." + ".".join(parts)


def render_us_zone(code, name, reg_key, reg_desc):
    special = SPECIAL_NOTES.get(code, f"US state zone for {name}.")
    lines = []

    # Header
    lines.append(f"# Zone Manifest: {name}")
    lines.append("#")
    for ln in special.split("\n"):
        if ln.startswith("#"):
            lines.append(f"# {ln.lstrip('# ')}")
        else:
            lines.append(f"# {ln}")
    lines.append("#")
    lines.append(f"# jurisdiction_stack: us > {code}")
    lines.append("")

    # Core
    lines.append(f"zone_id: {zone_id(code)}")
    lines.append(f"jurisdiction_id: {code}")
    lines.append(f"zone_name: {name}")
    lines.append("")

    # Profile — US states are sovereign regulatory entities
    lines.append("profile:")
    lines.append("  profile_id: org.momentum.mez.profile.sovereign-govos")
    lines.append('  version: "0.4.44"')
    lines.append("")

    # Jurisdiction stack
    lines.append("jurisdiction_stack:")
    lines.append("  - us")
    lines.append(f"  - {code}")
    lines.append("")

    # Lawpacks
    lines.append("lawpack_domains:")
    lines.append("  - civil")
    lines.append("  - financial")
    lines.append("")

    # No regpack builder for US yet
    lines.append("# Regpack builder needed")
    lines.append("")

    # Compliance domains
    lines.append("compliance_domains:")
    lines.append("  - aml")
    lines.append("  - kyc")
    lines.append("  - sanctions")
    lines.append("  - tax")
    lines.append("  - corporate")
    lines.append("  - licensing")
    lines.append("")

    # Corridors
    lines.append("corridors:")
    lines.append("  - org.momentum.mez.corridor.swift.iso20022-cross-border")
    lines.append("")

    # National adapters — US federal + state-specific
    lines.append("national_adapters:")
    lines.append("  fincen:")
    lines.append("    enabled: false")
    lines.append('    endpoint: "${FINCEN_URL:-}"')
    lines.append("  irs:")
    lines.append("    enabled: false")
    lines.append('    endpoint: "${IRS_URL:-}"')
    lines.append(f"  {reg_key}:")
    lines.append(f"    enabled: false")
    lines.append(f'    endpoint: "${{{reg_key.upper()}_URL:-}}"')
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
    enriched = 0
    for code, name, reg_key, reg_desc in US_STATES:
        zone_file = os.path.join(JURISDICTIONS_DIR, code, "zone.yaml")
        if not os.path.exists(zone_file):
            print(f"  SKIP (missing): {code}")
            continue

        content = render_us_zone(code, name, reg_key, reg_desc)
        with open(zone_file, "w") as f:
            f.write(content)
        enriched += 1
        print(f"  ENRICH: {code}")

    print(f"\nDone: {enriched} US zones enriched")


if __name__ == "__main__":
    main()
