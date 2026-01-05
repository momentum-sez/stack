from tools.integrations.swift_iso20022 import SwiftISO20022Adapter


def test_swift_iso20022_adapter_roundtrip_minimal():
    adapter = SwiftISO20022Adapter()
    payload = {
        "message_type": "pacs.008.001.08",
        "amount": "10.00",
        "currency": "USD",
        "debtor": {
            "name": "ALICE",
            "account": "US00ALICE0001",
            "agent_bic": "AAAAUS33",
        },
        "creditor": {
            "name": "BOB",
            "account": "GB00BOB0002",
            "agent_bic": "BBBBGB22",
        },
        "message_id": "MSG-1",
        "uetr": "uetr-123",
        "remittance_info": "invoice 42",
    }

    xml = adapter.payload_to_xml(payload)
    assert xml.startswith("<?xml")

    back = adapter.xml_to_payload(xml)
    # The stub is deterministic + preserves core fields.
    assert back["message_type"] == payload["message_type"]
    assert back["amount"] == payload["amount"]
    assert back["currency"] == payload["currency"]
    assert back["debtor"]["name"] == payload["debtor"]["name"]
    assert back["creditor"]["agent_bic"] == payload["creditor"]["agent_bic"]
    assert back["message_id"] == payload["message_id"]
    assert back["uetr"] == payload["uetr"]
    assert back["remittance_info"] == payload["remittance_info"]
