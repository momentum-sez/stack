"""SWIFT ISO 20022 (pacs.008) adapter stub.

The MSEZ stack is *rail-agnostic*; the corridor protocol only cares about:
  - deterministic transition envelopes,
  - rulesets attached by digest,
  - and verifiable receipts.

This module offers a minimal, dependency-light adapter that maps a structured
transition payload (`settle.swift.pacs008.v1`) to a minimal pacs.008-like XML
document and back.

It is intentionally NOT a full ISO 20022 implementation. Use it as a scaffold
for integration tests and pilots.
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from typing import Any, Dict, List, Optional

import xml.etree.ElementTree as ET


@dataclass(frozen=True)
class Pacs008Payload:
    """A minimal structured representation of a pacs.008 customer credit transfer."""

    message_type: str
    amount: str
    currency: str
    debtor_name: str
    debtor_account: str
    debtor_agent_bic: str
    creditor_name: str
    creditor_account: str
    creditor_agent_bic: str
    message_id: Optional[str] = None
    uetr: Optional[str] = None
    instruction_id: Optional[str] = None
    end_to_end_id: Optional[str] = None
    remittance_info: Optional[str] = None

    @staticmethod
    def from_dict(d: Dict[str, Any]) -> "Pacs008Payload":
        debtor = d.get("debtor") or {}
        creditor = d.get("creditor") or {}
        return Pacs008Payload(
            message_type=str(d.get("message_type") or ""),
            amount=str(d.get("amount") or ""),
            currency=str(d.get("currency") or ""),
            debtor_name=str(debtor.get("name") or ""),
            debtor_account=str(debtor.get("account") or ""),
            debtor_agent_bic=str(debtor.get("agent_bic") or ""),
            creditor_name=str(creditor.get("name") or ""),
            creditor_account=str(creditor.get("account") or ""),
            creditor_agent_bic=str(creditor.get("agent_bic") or ""),
            message_id=(str(d.get("message_id")) if d.get("message_id") is not None else None),
            uetr=(str(d.get("uetr")) if d.get("uetr") is not None else None),
            instruction_id=(str(d.get("instruction_id")) if d.get("instruction_id") is not None else None),
            end_to_end_id=(str(d.get("end_to_end_id")) if d.get("end_to_end_id") is not None else None),
            remittance_info=(str(d.get("remittance_info")) if d.get("remittance_info") is not None else None),
        )

    def to_dict(self) -> Dict[str, Any]:
        out: Dict[str, Any] = {
            "message_type": self.message_type,
            "amount": self.amount,
            "currency": self.currency,
            "debtor": {
                "name": self.debtor_name,
                "account": self.debtor_account,
                "agent_bic": self.debtor_agent_bic,
            },
            "creditor": {
                "name": self.creditor_name,
                "account": self.creditor_account,
                "agent_bic": self.creditor_agent_bic,
            },
        }
        for k in [
            "message_id",
            "uetr",
            "instruction_id",
            "end_to_end_id",
            "remittance_info",
        ]:
            v = getattr(self, k)
            if v is not None:
                out[k] = v
        return out


class SwiftISO20022Adapter:
    """Stub adapter for producing and parsing a minimal pacs.008-like XML."""

    # A fake namespace to avoid accidentally claiming spec compliance.
    NS = "urn:msez:swift:iso20022:stub"

    def validate_payload(self, payload: Dict[str, Any]) -> List[str]:
        """Validate an ISO 20022 pacs.008 payload and return a list of errors.

        BUG FIX #94: Check all required fields, not just message_type/amount/currency.
        BUG FIX #95: Validate amount as a valid decimal to prevent precision loss.
        """
        errors: List[str] = []
        p = Pacs008Payload.from_dict(payload)

        if not p.message_type:
            errors.append("missing required field: message_type")
        if not p.currency:
            errors.append("missing required field: currency")
        if not p.amount:
            errors.append("missing required field: amount")
        else:
            # Validate amount is a valid decimal number (avoids float precision loss)
            try:
                Decimal(p.amount)
            except InvalidOperation:
                errors.append(f"amount is not a valid decimal number: {p.amount!r}")

        # Debtor required fields
        if not p.debtor_name:
            errors.append("missing required field: debtor.name")
        if not p.debtor_account:
            errors.append("missing required field: debtor.account")
        if not p.debtor_agent_bic:
            errors.append("missing required field: debtor.agent_bic")

        # Creditor required fields
        if not p.creditor_name:
            errors.append("missing required field: creditor.name")
        if not p.creditor_account:
            errors.append("missing required field: creditor.account")
        if not p.creditor_agent_bic:
            errors.append("missing required field: creditor.agent_bic")

        return errors

    def payload_to_xml(self, payload: Dict[str, Any]) -> str:
        p = Pacs008Payload.from_dict(payload)
        # BUG FIX #94: Validate all required fields before XML generation
        errors = self.validate_payload(payload)
        if errors:
            raise ValueError(f"invalid pacs.008 payload: {'; '.join(errors)}")

        ET.register_namespace("msez", self.NS)

        doc = ET.Element(f"{{{self.NS}}}Document")
        cct = ET.SubElement(doc, f"{{{self.NS}}}CstmrCdtTrf")

        def _t(tag: str, text: Optional[str]) -> None:
            if text is None:
                return
            el = ET.SubElement(cct, f"{{{self.NS}}}{tag}")
            el.text = text

        _t("MsgTp", p.message_type)
        _t("MsgId", p.message_id)
        _t("UETR", p.uetr)
        _t("InstrId", p.instruction_id)
        _t("EndToEndId", p.end_to_end_id)

        amt = ET.SubElement(cct, f"{{{self.NS}}}Amt")
        amt.set("Ccy", p.currency)
        amt.text = p.amount

        dbtr = ET.SubElement(cct, f"{{{self.NS}}}Dbtr")
        ET.SubElement(dbtr, f"{{{self.NS}}}Nm").text = p.debtor_name
        ET.SubElement(dbtr, f"{{{self.NS}}}Acct").text = p.debtor_account
        ET.SubElement(dbtr, f"{{{self.NS}}}AgtBIC").text = p.debtor_agent_bic

        cdtr = ET.SubElement(cct, f"{{{self.NS}}}Cdtr")
        ET.SubElement(cdtr, f"{{{self.NS}}}Nm").text = p.creditor_name
        ET.SubElement(cdtr, f"{{{self.NS}}}Acct").text = p.creditor_account
        ET.SubElement(cdtr, f"{{{self.NS}}}AgtBIC").text = p.creditor_agent_bic

        if p.remittance_info:
            _t("RmtInf", p.remittance_info)

        # Deterministic serialization: ElementTree's tostring ordering is stable
        # for a fixed build order.
        xml_bytes = ET.tostring(doc, encoding="utf-8", xml_declaration=True)
        return xml_bytes.decode("utf-8")

    def xml_to_payload(self, xml_text: str) -> Dict[str, Any]:
        root = ET.fromstring(xml_text)
        # Navigate with the stub namespace; fallback to ignoring namespaces if needed.
        ns = {"msez": self.NS}

        def _find_text(path: str) -> Optional[str]:
            el = root.find(path, ns)
            if el is None:
                return None
            return el.text

        # Find first CstmrCdtTrf
        cct = root.find("msez:CstmrCdtTrf", ns)
        if cct is None:
            raise ValueError("missing CstmrCdtTrf")

        def _cct_text(tag: str) -> Optional[str]:
            el = cct.find(f"msez:{tag}", ns)
            return None if el is None else el.text

        amt_el = cct.find("msez:Amt", ns)
        currency = None
        amount = None
        if amt_el is not None:
            currency = amt_el.attrib.get("Ccy")
            amount = amt_el.text

        debtor = cct.find("msez:Dbtr", ns)
        creditor = cct.find("msez:Cdtr", ns)

        def _party(p_el: Optional[ET.Element]) -> Dict[str, str]:
            if p_el is None:
                return {"name": "", "account": "", "agent_bic": ""}
            nm = p_el.find("msez:Nm", ns)
            acct = p_el.find("msez:Acct", ns)
            bic = p_el.find("msez:AgtBIC", ns)
            return {
                "name": (nm.text if nm is not None and nm.text is not None else ""),
                "account": (acct.text if acct is not None and acct.text is not None else ""),
                "agent_bic": (bic.text if bic is not None and bic.text is not None else ""),
            }

        payload: Dict[str, Any] = {
            "message_type": _cct_text("MsgTp") or "",
            "amount": amount or "",
            "currency": currency or "",
            "debtor": _party(debtor),
            "creditor": _party(creditor),
        }

        # Optional fields
        for tag, key in [
            ("MsgId", "message_id"),
            ("UETR", "uetr"),
            ("InstrId", "instruction_id"),
            ("EndToEndId", "end_to_end_id"),
            ("RmtInf", "remittance_info"),
        ]:
            v = _cct_text(tag)
            if v is not None:
                payload[key] = v

        return payload
