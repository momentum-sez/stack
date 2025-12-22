package msez.regulator.access

default allow = false

# Example policy: only roles with "regulator:read" can query attestations.
allow {
  input.user.roles[_] == "regulator:read"
}

# Implementations MUST log every request attempt as an audit event regardless of allow/deny.
