package msez.lic.trust-company.eligibility

default allow = false

allow {
  input.application.declarations.truthful == true
  input.application.declarations.consent_to_checks == true
}
