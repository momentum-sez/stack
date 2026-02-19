package mez.lic.emi.eligibility

default allow = false

# Input model:
# {
#   "application": <application payload>,
#   "policy": {
#       "minimum_capital_amount": 0,
#       "kyc_tier_required": 2
#   }
# }

allow {
  has_required_declarations
  capital_ok
  has_directors
  has_bos
}

has_required_declarations {
  input.application.declarations.truthful == true
  input.application.declarations.consent_to_checks == true
}

capital_ok {
  # if minimum_capital_amount is set > 0, require paid_up_capital >= minimum
  input.policy.minimum_capital_amount <= 0
} else {
  input.application.financials.paid_up_capital >= input.policy.minimum_capital_amount
}

has_directors {
  count(input.application.applicant.directors) >= 1
}

has_bos {
  count(input.application.applicant.beneficial_owners) >= 1
}
