# Pakistan -- civil/commercial legal corpus

This module contains the civil and commercial legal corpus for **Pakistan** (`pk`),
structured as Akoma Ntoso 3.0 XML for consumption by the MEZ compliance tensor
evaluation pipeline.

## Scope

The corpus covers key operative provisions from the following statutes relevant to
commercial activity, entity formation, contract enforcement, and economic
zone operations within Pakistan:

| Statute | Citation | Domain |
|---|---|---|
| Contract Act, 1872 | Act IX of 1872 | Contract formation, validity, breach |
| Companies Act, 2017 | Act XIX of 2017 | Entity formation, governance, disclosure |
| Partnership Act, 1932 | Act IX of 1932 | Partnership definition, registration |
| Economic Zones Act, 2012 | Act XXIX of 2012 | Zone enterprise approval, tax/customs exemptions |
| Arbitration Act, 1940 | Act X of 1940 | Dispute resolution framework |

## Content status

This module contains **real statutory text** (key operative sections) rather than
placeholders. The text is derived from publicly available official sources
(National Assembly of Pakistan, SECP, Board of Investment). See `sources.yaml` for
authoritative URLs and citation details.

Only selected sections material to EZ compliance evaluation are included. This is
not a complete consolidation of each statute. Sections were selected based on their
relevance to:

- contract validity and enforceability within economic zones,
- entity registration and minimum capital requirements,
- annual filing and disclosure obligations,
- insider trading and beneficial ownership rules,
- EZ-specific tax, customs, and approval frameworks.

## Provenance

All text is from official government publications. The `NOASSERTION` license
reflects that Pakistani statutory text is generally not subject to copyright
restriction, but redistribution terms for specific compilations may vary.

## How to update

1. Update `sources.yaml` with new gazette references or amendment dates.
2. Edit `src/akn/main.xml` to reflect amended provisions.
3. Re-run the pack compilation pipeline to regenerate content digests.
