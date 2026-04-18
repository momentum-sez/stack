# Changelog

All notable changes to the zone configuration template.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: tracks kernel compatibility (e.g., 0.4.44 = compatible with kernel 0.4.44).

## [0.4.44] - 2026-04-09

### Added
- Initial zone configuration template
- 5 default operations: entity.incorporate, fiscal.payment, identity.verify, ownership.issue-shares, consent.resolution
- 4 example zone profiles: digital-free-zone, financial-center, charter-city, trade-zone
- JSON schemas for zone.yaml and operation YAML validation
- Docker Compose reference deployment (kernel + Java services)
- GitHub Actions CI for validation on PR
