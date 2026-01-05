"""Reference integration adapters.

These modules are intentionally dependency-light. They provide *boundary objects*
for translating between corridor transition payloads and external settlement rails
(e.g., SWIFT ISO 20022, stablecoin providers).

Production deployments SHOULD treat these as stubs or starting points and
substitute their own integration layers.
"""
