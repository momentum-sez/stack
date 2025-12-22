# Payments Adapter Interface (v1)

This module defines a canonical interface for integrating local payment rails.

Implementations SHOULD provide:
- `send_payment(request)` -> `payment_id`
- `get_payment(payment_id)` -> status
- `list_accounts(entity_id)` -> accounts
- `initiate_kyc(entity_id)` -> workflow id (if applicable)

Messaging SHOULD be mappable to ISO 20022 where possible (pain/pacs/camt families).

