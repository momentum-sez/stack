# Pakistan -- Tax Legal Corpus

This module contains the **tax legal corpus** for the **Islamic Republic of Pakistan** (`pk`), covering the principal federal tax statutes and withholding tax schedules as of fiscal year 2025-26.

## Scope

The corpus encodes provisions from:

- **Income Tax Ordinance, 2001** (Ordinance XLIX of 2001) -- charge of tax, corporate tax rates, super tax, minimum tax on turnover, withholding tax schedules (Sections 148-155, 231A, 236G), and Economic Zone incentives.
- **Sales Tax Act, 1990** (Act VII of 1990) -- standard rate, scope, zero-rating for exports.
- **Federal Excise Act, 2005** (Act IV of 2005) -- referenced for completeness; substantive provisions to be added in a future version.
- **Economic Zones Act, 2012** -- tax exemptions and incentives for zone enterprises.

## Rates and Thresholds

All rates and monetary thresholds are encoded as `<num>` elements in the Akoma Ntoso XML and reflect the **Finance Act 2024** amendments applicable to tax year 2025 (fiscal year 2024-25 onward). Users should verify against the latest Finance Act and FBR circulars for any mid-year amendments.

## Sources

See `sources.yaml` for authoritative references. The primary source is the FBR published text of the Income Tax Ordinance 2001 as amended.

## How to Update

1. Update `sources.yaml` with the latest gazette notification or Finance Act amendment date.
2. Edit `src/akn/main.xml` to reflect amended rates, thresholds, or new provisions.
3. Bump `version` in `module.yaml`.
4. Recompute content digests and update `stack.lock`.

## License

`NOASSERTION` -- Pakistan statutory text is public law. Redistribution of the structured encoding is subject to the repository license (BUSL-1.1). The underlying legal text is not subject to copyright assertion.
