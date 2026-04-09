# Security Policy

## Reporting Vulnerabilities

Email security@momentum.inc with:
- Description of the vulnerability
- Steps to reproduce
- Affected zone configuration (redact credentials)

We will acknowledge within 48 hours and provide a timeline for resolution.

## Credential Handling

- **Never commit `.env` files.** The `.gitignore` excludes them.
- **Rotate `ZONE_SIGNING_KEY_HEX` regularly.** See `key_management` in `zone.yaml`.
- **Use strong `AUTH_TOKEN` values.** Minimum 32 characters, random.
- **Set `POSTGRES_PASSWORD` to a unique value.** Do not reuse across environments.

## Sanctions Compliance

The kernel enforces a hard reject on `NonCompliant` sanctions verdicts.
This is a legal requirement with zero override path. If you believe a
sanctions screening result is incorrect, contact Momentum support — do not
attempt to bypass the kernel's sanctions enforcement.
