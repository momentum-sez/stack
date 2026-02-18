# Pakistan -- AML/CFT Legal Corpus

This module contains the **anti-money laundering and counter-financing of terrorism (AML/CFT) legal corpus** for the **Islamic Republic of Pakistan** (`pk`), consolidating the principal federal statute and key regulatory directives.

## Scope

The corpus encodes provisions from:

- **Anti-Money Laundering Act, 2010** (Act VII of 2010) -- definitions, offences, penalties, customer due diligence, suspicious transaction reporting, currency transaction reporting, cross-border wire transfers, record keeping, and tipping-off prohibition.
- **Anti-Terrorism Act, 1997** (Act XXVII of 1997) -- CFT-specific provisions referenced by the AML framework, including proscribed organization financing and freezing orders.
- **SECP AML/CFT Regulations, 2018** -- risk-based approach, CDD measures, ongoing monitoring, STR obligations, compliance officer requirements, and record retention for capital market participants and DNFBPs under SECP supervision.
- **SBP AML/CFT Regulations for Banks** -- banking sector thresholds for CTRs, wire transfer requirements, PEP monitoring, and goAML portal filing.

## Key Thresholds

| Requirement | Threshold | Source |
|---|---|---|
| Currency Transaction Report (CTR) | PKR 2,000,000 | AMLA 2010 s.7A / SBP Regs |
| Wire transfer full originator info | USD 1,000 equivalent | SBP Regs (FATF R.16) |
| STR filing deadline | 7 working days to FMU | AMLA 2010 s.7 / SECP R.8 |
| Record retention | 5 years minimum | AMLA 2010 s.8 / SECP R.12 |
| Cash withdrawal daily limit (for reporting) | PKR 50,000 | Cross-ref ITO 2001 s.231A |

## Regulatory Bodies

- **FMU** (Financial Monitoring Unit) -- Pakistan's financial intelligence unit, receives STRs and CTRs.
- **SBP** (State Bank of Pakistan) -- supervises banks and DFIs for AML/CFT compliance.
- **SECP** (Securities and Exchange Commission of Pakistan) -- supervises capital markets, insurance, and DNFBPs.
- **NACTA** (National Counter Terrorism Authority) -- coordinates CFT policy.

## Sources

See `sources.yaml` for authoritative references. The primary source for the AMLA 2010 is the FMU published text.

## How to Update

1. Update `sources.yaml` with the latest gazette notification or amendment date.
2. Edit `src/akn/main.xml` to reflect amended provisions, thresholds, or new regulations.
3. Bump `version` in `module.yaml`.
4. Recompute content digests and update `stack.lock`.

## License

`NOASSERTION` -- Pakistan statutory text is public law. Redistribution of the structured encoding is subject to the repository license (BUSL-1.1). The underlying legal text is not subject to copyright assertion.
