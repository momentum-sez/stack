package mez.aml

default allow = false

# Example: require KYC tier >= configured threshold for a transaction
allow {
  input.subject.kyc_tier >= input.policy.kyc_tier_required
  input.transaction.amount <= input.policy.max_amount_without_enhanced_due_diligence
}
