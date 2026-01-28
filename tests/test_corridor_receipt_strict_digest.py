import json
import pathlib

from tools import artifacts as artifact_cas
from tools import msez


def _make_min_receipt() -> dict:
    # Minimal-but-plausible corridor receipt payload.
    # The strict digest semantics only depend on the JCS payload (excluding proof + next_root).
    return {
        "type": "MSEZCorridorReceipt",
        "corridor_id": "did:web:example.com#corridor-trade-obligation",
        "corridor_version": 1,
        "sequence": 1,
        "prev_root": "0" * 64,
        "prev_receipt_digest": "0" * 64,
        "state_transition": {
            "type": "MSEZCorridorStateTransition",
            "kind": "trade.invoice.v1",
            "id": "urn:uuid:invoice-demo-1",
            "attachments": [],
        },
        "state": {
            "kind": "trade.obligation.state.v1",
            "invoice_id": "INV-0001",
            "amount": {"currency": "USD", "value": "100000.00"},
        },
        "proof": {
            "type": "DataIntegrityProof",
            "cryptosuite": "eddsa-jcs-2022",
            "verificationMethod": "did:web:example.com#key-1",
            "proofPurpose": "assertionMethod",
            "created": "2026-01-01T00:00:00Z",
            "proofValue": "z" + ("a" * 20),
        },
    }


def test_corridor_receipt_strict_digest_semantics_ok(tmp_path: pathlib.Path) -> None:
    store_root = tmp_path / "artifacts"
    store_root.mkdir(parents=True, exist_ok=True)

    rcpt = _make_min_receipt()
    rcpt["next_root"] = msez.corridor_state_next_root(rcpt)

    src = tmp_path / "corridor-receipt.json"
    msez.write_canonical_json_file(src, rcpt)

    digest = rcpt["next_root"]
    artifact_cas.store_artifact_file(
        "corridor-receipt",
        digest,
        src,
        repo_root=msez.REPO_ROOT,
        store_root=store_root,
        dest_name=f"{digest}.corridor-receipt.json",
        overwrite=True,
    )

    report_path = tmp_path / "report.json"
    rc = msez.cmd_artifact_graph_verify(
        type(
            "Args",
            (),
            {
                "type": "corridor-receipt",
                "digest": digest,
                "path": "",
                "store_root": [str(store_root)],
                "strict": True,
                "max_nodes": 100,
                "max_depth": 4,
                "out": str(report_path),
                "json": True,
                "edges": False,
            },
        )()
    )

    assert rc == 0
    rep = json.loads(report_path.read_text(encoding="utf-8"))
    assert rep["stats"]["digest_mismatch_total"] == 0


def test_corridor_receipt_strict_digest_semantics_detects_next_root_mismatch(tmp_path: pathlib.Path) -> None:
    store_root = tmp_path / "artifacts"
    store_root.mkdir(parents=True, exist_ok=True)

    rcpt = _make_min_receipt()
    expected = msez.corridor_state_next_root(rcpt)
    rcpt["next_root"] = "0" * 64  # wrong

    src = tmp_path / "corridor-receipt.bad.json"
    msez.write_canonical_json_file(src, rcpt)

    # Store under the *expected* digest to simulate a corrupted / non-canonical receipt payload.
    artifact_cas.store_artifact_file(
        "corridor-receipt",
        expected,
        src,
        repo_root=msez.REPO_ROOT,
        store_root=store_root,
        dest_name=f"{expected}.corridor-receipt.json",
        overwrite=True,
    )

    report_path = tmp_path / "report.json"
    rc = msez.cmd_artifact_graph_verify(
        type(
            "Args",
            (),
            {
                "type": "corridor-receipt",
                "digest": expected,
                "path": "",
                "store_root": [str(store_root)],
                "strict": True,
                "max_nodes": 100,
                "max_depth": 4,
                "out": str(report_path),
                "json": True,
                "edges": False,
            },
        )()
    )

    assert rc != 0
    rep = json.loads(report_path.read_text(encoding="utf-8"))
    assert rep["stats"]["digest_mismatch_total"] == 1
    mm = rep["digest_mismatches"][0]
    assert mm["artifact_type"] == "corridor-receipt"
    assert "next_root mismatch" in (mm.get("error") or "")
