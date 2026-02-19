package mez.lic.psp-acquirer.eligibility

default allow = false

allow {
  input.application.declarations.truthful == true
  input.application.declarations.consent_to_checks == true
}
