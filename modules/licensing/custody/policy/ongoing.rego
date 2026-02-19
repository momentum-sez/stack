package mez.lic.custody.ongoing

default compliant = true

# Example ongoing check: if any BO is PEP, require enhanced due diligence flag in periodic return.
compliant {
  not requires_edd
} else {
  requires_edd
  input.periodic_return.enhanced_due_diligence_completed == true
}

requires_edd {
  some i
  input.licensee.beneficial_owners[i].pep == true
}
