import json
import os
import pathlib

import pytest


@pytest.fixture()
def tmp_artifact_store(tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch) -> pathlib.Path:
    """Create a temporary artifact store root and point MSEZ artifact resolution at it."""
    store = tmp_path / "artifact-store"
    store.mkdir(parents=True, exist_ok=True)
    monkeypatch.setenv("MSEZ_ARTIFACT_STORE_DIRS", str(store))
    return store


def _write_artifact(store_root: pathlib.Path, artifact_type: str, digest_sha256: str, suffix: str, payload: bytes) -> pathlib.Path:
    ddir = store_root / artifact_type
    ddir.mkdir(parents=True, exist_ok=True)
    path = ddir / f"{digest_sha256}.{suffix}"
    path.write_bytes(payload)
    return path


def _minimal_corridor_module(tmp_path: pathlib.Path, corridor_id: str = "msez.test.corridor") -> pathlib.Path:
    """Create a minimal corridor module directory usable by corridor state verification."""
    module_dir = tmp_path / "corridor-module"
    module_dir.mkdir(parents=True, exist_ok=True)
    (module_dir / "receipts").mkdir(exist_ok=True)

    # Minimal (unsigned) corridor definition VC used for genesis_root binding.
    def_vc = {
        "@context": ["https://www.w3.org/2018/credentials/v1"],
        "type": ["VerifiableCredential", "MSEZCorridorDefinition"],
        "issuer": "did:key:z6Mktestcorridordefinition0000000000000000000000000000000000",
        "issuanceDate": "2026-01-05T00:00:00Z",
        "credentialSubject": {"id": corridor_id},
    }
    (module_dir / "corridor-definition.vc.json").write_text(json.dumps(def_vc, indent=2), encoding="utf-8")

    corridor_yaml = f"""corridor_id: {corridor_id}
definition_vc_path: corridor-definition.vc.json
"""
    (module_dir / "corridor.yaml").write_text(corridor_yaml, encoding="utf-8")
    return module_dir


def _sign_receipt(receipt_obj: dict) -> dict:
    """Sign a corridor receipt with a fresh did:key Ed25519 key (valid VC proof)."""
    from tools import vc

    jwk = vc.generate_ed25519_jwk(kid="key-1")
    priv, did = vc.load_ed25519_private_key_from_jwk(jwk)
    vm = f"{did}#key-1"
    vc.add_ed25519_proof(receipt_obj, priv, vm)
    return receipt_obj


def _build_minimal_receipt_chain(module_dir: pathlib.Path, *, ttr_digest: str) -> pathlib.Path:
    """Write a 1-receipt chain anchored to the corridor genesis root."""
    from tools import msez

    corridor_id = msez.load_yaml(module_dir / "corridor.yaml")["corridor_id"]
    genesis = msez.corridor_state_genesis_root(module_dir)

    expected_lawpacks = sorted(msez.corridor_expected_lawpack_digest_set(module_dir))
    expected_rulesets = sorted(msez.corridor_expected_ruleset_digest_set(module_dir))

    receipt = {
        "type": "MSEZCorridorStateReceipt",
        "corridor_id": corridor_id,
        "sequence": 0,
        "timestamp": "2026-01-05T00:00:01Z",
        "prev_root": genesis,
        "next_root": "0" * 64,
        "lawpack_digest_set": expected_lawpacks,
        "ruleset_digest_set": expected_rulesets,
        "transition_type_registry_digest_sha256": ttr_digest,
    }
    receipt["next_root"] = msez.corridor_state_next_root(receipt)
    _sign_receipt(receipt)

    p = module_dir / "receipts" / "000000.json"
    p.write_text(json.dumps(receipt, indent=2), encoding="utf-8")
    return module_dir / "receipts"


def test_transitive_require_artifacts_fails_when_registry_dep_missing(tmp_path: pathlib.Path, tmp_artifact_store: pathlib.Path):
    """--transitive-require-artifacts should fail if a registry-referenced schema/ruleset/circuit digest is absent."""
    from tools import msez

    # Registry snapshot digest committed by receipts.
    ttr_digest = "a" * 64
    schema_digest = "b" * 64
    ruleset_digest = "c" * 64

    lock_obj = {
        "transition_types_lock_version": 1,
        "generated_at": "2026-01-05T00:00:00Z",
        "snapshot": {
            "tag": "msez.transition-types.registry.snapshot.v1",
            "registry_version": 1,
            "transition_types": [
                {
                    "kind": "noop.v1",
                    "schema_digest_sha256": schema_digest,
                    "ruleset_digest_sha256": ruleset_digest,
                }
            ],
        },
        "snapshot_digest_sha256": ttr_digest,
    }
    _write_artifact(tmp_artifact_store, "transition-types", ttr_digest, "transition-types.lock.json", json.dumps(lock_obj).encode("utf-8"))
    # Intentionally omit schema/ruleset artifacts.

    module_dir = _minimal_corridor_module(tmp_path)
    receipts_path = _build_minimal_receipt_chain(module_dir, ttr_digest=ttr_digest)

    _state, _warn, errors = msez._corridor_state_build_chain(
        module_dir,
        receipts_path,
        require_artifacts=True,
        transitive_require_artifacts=True,
    )
    assert errors, "expected missing artifact errors"
    assert any("transition-types.lock" in e and "missing artifact" in e for e in errors)


def test_transitive_require_artifacts_passes_when_registry_deps_present(tmp_path: pathlib.Path, tmp_artifact_store: pathlib.Path):
    """--transitive-require-artifacts should pass when all lock-referenced artifacts exist in CAS."""
    from tools import msez

    ttr_digest = "d" * 64
    schema_digest = "e" * 64
    ruleset_digest = "f" * 64
    circuit_digest = "1" * 64

    lock_obj = {
        "transition_types_lock_version": 1,
        "generated_at": "2026-01-05T00:00:00Z",
        "snapshot": {
            "tag": "msez.transition-types.registry.snapshot.v1",
            "registry_version": 1,
            "transition_types": [
                {
                    "kind": "noop.v1",
                    "schema_digest_sha256": schema_digest,
                    "ruleset_digest_sha256": ruleset_digest,
                    "zk_circuit_digest_sha256": circuit_digest,
                }
            ],
        },
        "snapshot_digest_sha256": ttr_digest,
    }
    _write_artifact(tmp_artifact_store, "transition-types", ttr_digest, "transition-types.lock.json", json.dumps(lock_obj).encode("utf-8"))
    _write_artifact(tmp_artifact_store, "schema", schema_digest, "schema.json", b"{}")
    _write_artifact(tmp_artifact_store, "ruleset", ruleset_digest, "ruleset.json", b"{}")
    _write_artifact(tmp_artifact_store, "circuit", circuit_digest, "circuit.bin", b"\x00\x01")

    module_dir = _minimal_corridor_module(tmp_path)
    receipts_path = _build_minimal_receipt_chain(module_dir, ttr_digest=ttr_digest)

    _state, _warn, errors = msez._corridor_state_build_chain(
        module_dir,
        receipts_path,
        require_artifacts=True,
        transitive_require_artifacts=True,
    )
    assert errors == []


def test_transitive_require_artifacts_expands_ruleset_nested_artifactrefs(tmp_path: pathlib.Path, tmp_artifact_store: pathlib.Path):
    """In transitive mode, ruleset artifacts referenced by the transition-types lock may embed ArtifactRefs.

    This test ensures that a missing nested ArtifactRef (e.g., a circuit/proof key) is surfaced as a
    commitment completeness failure.
    """
    from tools import msez

    ttr_digest = "2" * 64
    schema_digest = "3" * 64
    ruleset_digest = "4" * 64
    nested_circuit_digest = "5" * 64

    lock_obj = {
        "transition_types_lock_version": 1,
        "generated_at": "2026-01-05T00:00:00Z",
        "snapshot": {
            "tag": "msez.transition-types.registry.snapshot.v1",
            "registry_version": 1,
            "transition_types": [
                {
                    "kind": "example.with-nested-ruleset-ref.v1",
                    "schema_digest_sha256": schema_digest,
                    "ruleset_digest_sha256": ruleset_digest,
                }
            ],
        },
        "snapshot_digest_sha256": ttr_digest,
    }

    _write_artifact(
        tmp_artifact_store,
        "transition-types",
        ttr_digest,
        "transition-types.lock.json",
        json.dumps(lock_obj).encode("utf-8"),
    )
    _write_artifact(tmp_artifact_store, "schema", schema_digest, "schema.json", b"{}")

    # Ruleset embeds an ArtifactRef to a circuit digest that is intentionally missing from CAS.
    ruleset_obj = {
        "type": "MSEZTransitionRuleset",
        "ruleset_id": "msez.test.ruleset.with-nested-ref",
        "version": "0.1.0",
        "transition_kind": "example.with-nested-ruleset-ref.v1",
        "embedded": {
            "artifact_type": "circuit",
            "digest_sha256": nested_circuit_digest,
            "display_name": "example.zk.circuit",
        },
    }
    _write_artifact(tmp_artifact_store, "ruleset", ruleset_digest, "ruleset.json", json.dumps(ruleset_obj).encode("utf-8"))

    module_dir = _minimal_corridor_module(tmp_path)
    receipts_path = _build_minimal_receipt_chain(module_dir, ttr_digest=ttr_digest)

    _state, _warn, errors = msez._corridor_state_build_chain(
        module_dir,
        receipts_path,
        require_artifacts=True,
        transitive_require_artifacts=True,
    )

    assert errors, "expected missing nested artifact errors"
    assert any(nested_circuit_digest in e for e in errors), errors
