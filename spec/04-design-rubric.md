# Design rubric

All design decisions in this stack MUST be justified against a capacity-oriented rubric.

Every substantial decision SHOULD produce an ADR under `spec/adrs/`.

## Rubric axes

1. **Deployability**
   - Can a real government or authority adopt this without implausible assumptions?

2. **Bankability**
   - Can a bank/correspondent/PSP partner onboard with defensible risk controls?

3. **Auditability**
   - Can a regulator verify compliance from attestations and audited views?

4. **Modularity**
   - Can modules be swapped/upgraded without breaking the system?

5. **Interoperability**
   - Does it integrate across corridors and external infrastructures?

6. **Security & privacy**
   - Does it minimize data exposure and support key rotation / revocation?

7. **Maintainability**
   - Is it testable, versioned, and diffable?

8. **Licensing & provenance**
   - Is it redistributable? Are sources documented? Are restrictions enforced?

