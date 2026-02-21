# External Security Review Framework

**Version:** 1.0
**Status:** Phase 3 prerequisite
**Owner:** Security team

---

## I. SCOPE

This framework defines the external penetration testing and security audit
requirements for Phase 3 (Production) deployment of the EZ Stack.

### In Scope

| Component | Description | Priority |
|-----------|-------------|----------|
| mez-api HTTP surface | All Axum routes, auth middleware, input validation | P0 |
| Cryptographic operations | Ed25519 signing, SHA-256 digests, Poseidon2, BBS+ | P0 |
| Key management (HSM/KMS) | KeyProvider trait, software provider, AWS KMS config | P0 |
| Receipt chain integrity | Hash-chain continuity, MMR proofs, fork detection | P0 |
| Compliance tensor | Fail-closed evaluation, domain exhaustiveness | P0 |
| Mass API proxy | mez-mass-client request forgery, credential handling | P0 |
| Deployment configuration | Docker, K8s manifests, Terraform, secret management | P1 |
| Dependency supply chain | Cargo.lock pinning, crate audit, advisory database | P1 |

### Out of Scope

- Mass APIs (Java/Spring Boot) — separate audit track
- L1 settlement layer — not yet implemented
- SAVM (Smart Asset VM) — not yet implemented

## II. REVIEW TRACKS

### Track 1: Application Security (OWASP)

**Objective:** Identify web application vulnerabilities per OWASP Top 10.

| Check | Technique | Target |
|-------|-----------|--------|
| Injection | Fuzzing, manual review | All API endpoints accepting JSON |
| Broken Authentication | Token analysis, timing attacks | Bearer token middleware |
| Sensitive Data Exposure | TLS config, header analysis | Response headers, error messages |
| Security Misconfiguration | Config review | K8s manifests, Terraform, Docker |
| Broken Access Control | Privilege escalation testing | Role-based access (CallerIdentity) |
| Mass Assignment | Parameter pollution | JSON body parsing |
| Rate Limiting | Load testing | All write endpoints |

### Track 2: Cryptographic Review

**Objective:** Verify correctness and security of all cryptographic operations.

| Check | Technique | Target |
|-------|-----------|--------|
| Ed25519 implementation | Known-answer tests, edge cases | mez-crypto/ed25519.rs |
| SHA-256 centralization | Grep audit, dependency analysis | All crates using sha2 |
| Canonical bytes | Differential testing vs JCS spec | mez-core/canonical.rs |
| Poseidon2 correctness | Cross-implementation testing | mez-crypto/poseidon.rs |
| BBS+ selective disclosure | Commitment soundness analysis | mez-crypto/bbs.rs |
| Key zeroization | Memory dump analysis | SigningKey, SecretString |
| Constant-time comparison | Timing analysis | auth.rs constant_time_token_eq |
| Random number generation | CSPRNG source verification | OsRng usage |

### Track 3: Infrastructure Security

**Objective:** Verify production deployment security posture.

| Check | Technique | Target |
|-------|-----------|--------|
| K8s pod security | Policy audit | SecurityContext, NetworkPolicy |
| Secret management | Config review | ExternalSecret, AWS Secrets Manager |
| KMS key policy | IAM review | aws_kms_key policy document |
| TLS configuration | SSL Labs scan | Ingress TLS, ALB SSL policy |
| Container image | Trivy/Grype scan | Dockerfile, base image |
| IRSA (IAM Roles for SA) | Policy review | zone-authority IAM role |
| Network segmentation | VPC flow analysis | Private subnets, security groups |
| Database security | Encryption audit | RDS encryption, access controls |

### Track 4: Supply Chain Security

**Objective:** Verify dependency integrity and minimize supply chain risk.

| Check | Technique | Target |
|-------|-----------|--------|
| Cargo.lock integrity | cargo audit | All workspace dependencies |
| Advisory database | RustSec advisory check | Known vulnerabilities |
| Dependency minimization | cargo deny | Excessive/duplicate deps |
| License compliance | cargo deny | BUSL-1.1 compatibility |
| Build reproducibility | Deterministic build test | Docker multi-stage build |

## III. INVARIANTS TO VERIFY

The following invariants from CLAUDE.md must be independently verified:

1. **mez-core has zero internal crate dependencies.**
2. **mez-mass-client depends only on mez-core.**
3. **No dependency cycles.**
4. **All SHA-256 flows through mez-core::digest.**
   Verify: `grep -rn "use sha2" crates/ --include="*.rs"` — only `mez-core/src/digest.rs` and `mez-crypto/src/mmr.rs`.
5. **ComplianceDomain has exactly 20 variants.**
6. **Zero `unwrap()` in production code.**
7. **Zero `unimplemented!()` or `todo!()` outside tests.**
8. **`serde_json` does not enable `preserve_order`.**
9. **No default credentials in deploy paths.**
10. **Receipt chain invariants** (hash-chain + MMR).
11. **Compliance tensor fail-closed.**
12. **ZK proof policy fail-closed** in release builds.

## IV. ENGAGEMENT REQUIREMENTS

### Vendor Qualifications

- CREST or equivalent certification
- Prior experience auditing Rust codebases
- Cryptographic audit capability (Ed25519, hash functions, ZK primitives)
- Cloud infrastructure (AWS EKS, KMS, Terraform) experience
- Financial services / compliance domain knowledge (preferred)

### Deliverables

| Deliverable | Format | Timeline |
|-------------|--------|----------|
| Threat model | STRIDE / attack tree | Week 1-2 |
| Vulnerability report | CVSS-scored findings | Week 3-4 |
| Cryptographic review report | Formal analysis | Week 3-4 |
| Infrastructure review report | CIS benchmark results | Week 2-3 |
| Executive summary | PDF | Week 5 |
| Retest report (after fixes) | Delta findings | Week 6-7 |

### Severity Classification

| Severity | CVSS | SLA |
|----------|------|-----|
| Critical | 9.0-10.0 | Fix within 24 hours, retest within 72 hours |
| High | 7.0-8.9 | Fix within 7 days |
| Medium | 4.0-6.9 | Fix within 30 days |
| Low | 0.1-3.9 | Fix within 90 days |
| Informational | N/A | Track and prioritize |

## V. PRE-ENGAGEMENT CHECKLIST

Before engaging the external review team, complete the following:

- [ ] All P0 audit findings from internal review resolved
- [ ] HSM/KMS key provider integrated and tested
- [ ] Poseidon2 implementation cross-tested against reference
- [ ] BBS+ selective disclosure formally verified
- [ ] K8s NetworkPolicies deployed and validated
- [ ] ExternalSecrets operator configured for production
- [ ] Container images scanned (zero critical/high CVEs)
- [ ] cargo audit reports zero advisories
- [ ] All 4,683+ tests passing
- [ ] Load testing completed (target: 1000 RPS per zone)
- [ ] Documentation current (API specs, architecture docs)

## VI. ONGOING SECURITY

Post-audit, establish continuous security practices:

1. **Dependency scanning**: cargo audit in CI (block on advisories)
2. **Container scanning**: Trivy in CI pipeline
3. **Secret rotation**: Automated via AWS Secrets Manager (90-day rotation)
4. **Key rotation**: KMS automatic key rotation (annual)
5. **Penetration testing**: Annual external assessment
6. **Bug bounty**: Consider responsible disclosure program post-Phase 3

---

**End of Security Review Framework**
