"""USDC (Circle) adapter stub.

This module is a scaffold for integrating corridor transitions with a stablecoin
provider. It intentionally avoids bundling heavy HTTP dependencies into the core
MSEZ repo.

The canonical transition kind for this stub is:
    settle.usdc.circle.transfer.v1

Payload schema:
    schemas/transition.payload.settle.usdc.circle.transfer.v1.schema.json

In a production deployment, you would implement a real client (HTTP, message
queue, custody system, etc.) and bind transfer initiation + confirmation into
corridor receipts.
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from typing import Any, Dict, Optional, Protocol


class CircleClient(Protocol):
    """Abstract client interface.

    This keeps the core repo dependency-free. Implementations can use `httpx`,
    `requests`, or any other transport.
    """

    def create_transfer(self, *, amount: str, destination: str, idempotency_key: str) -> Dict[str, Any]:
        ...


@dataclass(frozen=True)
class CircleTransferResult:
    transfer_id: str
    status: str
    tx_hash: Optional[str] = None


class USDCCircleAdapter:
    """Stub adapter that validates + routes payloads to an injected client."""

    def __init__(self, client: CircleClient):
        self._client = client

    def initiate_transfer(self, payload: Dict[str, Any]) -> CircleTransferResult:
        """Initiate a USDC transfer.

        The payload is expected to conform to the JSON schema. This function
        performs a minimal semantic check and delegates to the injected client.
        """

        if str(payload.get("asset")) != "USDC":
            raise ValueError("payload.asset must be USDC")

        amount = str(payload.get("amount") or "")
        destination = str(payload.get("destination_address") or "")
        idem = str(payload.get("idempotency_key") or "")
        if not amount or not destination or not idem:
            raise ValueError("missing amount/destination_address/idempotency_key")

        # BUG FIX #96: USDC uses 6 decimal places. Validate that the amount
        # doesn't exceed 6 decimal places to prevent precision errors.
        try:
            dec_amount = Decimal(amount)
        except InvalidOperation:
            raise ValueError(f"amount is not a valid decimal number: {amount!r}")
        if dec_amount <= 0:
            raise ValueError(f"amount must be positive, got {amount}")
        # Check decimal places: USDC supports exactly 6 decimal places
        if dec_amount.as_tuple().exponent is not None:
            exponent = dec_amount.as_tuple().exponent
            if isinstance(exponent, int) and exponent < -6:
                raise ValueError(
                    f"USDC supports at most 6 decimal places, "
                    f"got {amount!r} ({-exponent} decimal places)"
                )

        resp = self._client.create_transfer(amount=amount, destination=destination, idempotency_key=idem)
        transfer_id = str(resp.get("transfer_id") or resp.get("id") or "")
        status = str(resp.get("status") or "")
        tx_hash = resp.get("tx_hash")
        if not transfer_id:
            raise ValueError("client response missing transfer_id")
        if not status:
            raise ValueError("client response missing status")

        return CircleTransferResult(transfer_id=transfer_id, status=status, tx_hash=(str(tx_hash) if tx_hash else None))
