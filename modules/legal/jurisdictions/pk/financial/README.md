# Pakistan -- Financial/Banking Legal Corpus

This module contains the **financial and banking legal corpus** for the **Islamic Republic of Pakistan** (`pk`), covering the principal federal banking statutes, payment systems regulations, microfinance rules, electronic money institution regulations, and foreign exchange controls as of 1 January 2024.

## Scope

The corpus encodes provisions from:

- **Banking Companies Ordinance, 1962** (Ordinance LVII of 1962) -- minimum paid-up capital (PKR 10B for commercial banks per SBP BSD Circular 2022), cash reserve requirement (CRR 5%), statutory liquidity requirement (SLR 18%), capital adequacy (Basel III: 11.5% total CAR, 6% CET-1), restrictions on related-party lending, and SBP supervisory powers.
- **Microfinance Institutions Ordinance, 2001** (Ordinance LI of 2001) -- licensing by SBP, minimum capital (PKR 1B for nationwide MFBs per SBP circular 2023), maximum individual loan exposure (PKR 500,000 for micro credit), deposit protection fund participation, and SBP examination powers.
- **State Bank of Pakistan Act, 1956** (Act XXXIII of 1956) -- SBP mandate, monetary policy, supervisory authority over banks and payment systems.
- **Payment Systems and Electronic Fund Transfers Act, 2007** (Act V of 2007) -- SBP authorization for payment system operators, settlement finality (irrevocability of settled transfers).
- **SBP Raast Regulations, 2021** -- real-time settlement via SBP Raast infrastructure, P2P transfer limits (PKR 50,000 per transaction, PKR 200,000 daily as updated 2024), P2M merchant onboarding KYC, IPS participation criteria, ISO 20022 message format requirements.
- **SBP Regulations for Electronic Money Institutions, 2019** -- licensing, minimum paid-up capital (PKR 200M), e-money wallet limits (PKR 500,000 maximum balance, PKR 1M monthly throughput at Level 2 KYC), branchless banking agent requirements, safeguarding of customer funds.
- **Foreign Exchange Regulation Act, 1947** (Act VII of 1947) -- authorized dealer regime, capital account restrictions, export proceeds repatriation (120 days), penalties for unauthorized forex dealing.

## Rates and Thresholds

All monetary thresholds and regulatory ratios are encoded as `<num>` elements in the Akoma Ntoso XML. Users should verify against the latest SBP circulars and gazette notifications for mid-year amendments.

## Sources

See `sources.yaml` for authoritative references. Primary sources are SBP-published texts and gazette notifications.

## How to Update

1. Update `sources.yaml` with the latest gazette notification or SBP circular date.
2. Edit `src/akn/main.xml` to reflect amended rates, thresholds, or new provisions.
3. Bump `version` in `module.yaml`.
4. Recompute content digests and update `stack.lock`.

## License

`NOASSERTION` -- Pakistan statutory text is public law. Redistribution of the structured encoding is subject to the repository license (BUSL-1.1). The underlying legal text is not subject to copyright assertion.
