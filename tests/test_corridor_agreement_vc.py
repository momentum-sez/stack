import json
import pathlib
import shutil
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))


def load_json(path: pathlib.Path):
    return json.loads(path.read_text(encoding="utf-8"))


def test_corridor_agreement_vc_verifies_and_thresholds_met(tmp_path: pathlib.Path):
    from tools.msez import verify_corridor_definition_vc, verify_corridor_agreement_vc  # type: ignore

    fixture = REPO / "tests" / "fixtures" / "corridor_agreement_2of2"

    # Baseline: fixture passes
    assert not verify_corridor_definition_vc(fixture)
    assert not verify_corridor_agreement_vc(fixture)

    # Negative case: remove one signature -> threshold should fail
    dst = tmp_path / "corridor_agreement_2of2"
    shutil.copytree(fixture, dst)

    vc_path = dst / "corridor.agreement.vc.json"
    vcj = load_json(vc_path)
    proof = vcj.get("proof")

    if isinstance(proof, list) and proof:
        vcj["proof"] = proof[:1]

    vc_path.write_text(json.dumps(vcj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    errs = verify_corridor_agreement_vc(dst)
    assert errs, "expected agreement verification to fail when threshold signatures are missing"
    assert any("activation threshold not met" in e for e in errs), f"unexpected errors: {errs}"

def test_corridor_agreement_party_specific_multi_file_verifies(tmp_path: pathlib.Path):
    from tools.msez import verify_corridor_definition_vc, verify_corridor_agreement_vc  # type: ignore

    fixture = REPO / "tests" / "fixtures" / "corridor_agreement_party_specific_2of2"

    # Baseline: fixture passes
    assert not verify_corridor_definition_vc(fixture)
    assert not verify_corridor_agreement_vc(fixture)

    # Negative case: remove one party VC -> threshold should fail
    dst = tmp_path / "corridor_agreement_party_specific_2of2"
    shutil.copytree(fixture, dst)
    (dst / "corridor.agreement.zone-b.vc.json").unlink()

    errs = verify_corridor_agreement_vc(dst)
    assert errs, "expected agreement verification to fail when a party VC is missing"
    assert any("activation threshold not met" in e for e in errs), f"unexpected errors: {errs}"



def test_corridor_agreement_commitment_blocks_activation(tmp_path: pathlib.Path):
    """Even if activation thresholds could be satisfied, any non-affirmative party commitment must block activation."""
    from tools.msez import verify_corridor_definition_vc, verify_corridor_agreement_vc  # type: ignore

    fixture = REPO / "tests" / "fixtures" / "corridor_agreement_party_specific_1of2_withdraw"

    # Definition still valid
    assert not verify_corridor_definition_vc(fixture)

    errs = verify_corridor_agreement_vc(fixture)
    assert errs, "expected agreement verification to fail when a party withdraws"
    assert any("commitment" in e and "blocks activation" in e for e in errs), f"unexpected errors: {errs}"


def test_corridor_agreement_party_status_lock_duplicate_party(tmp_path: pathlib.Path):
    """agreement_vc_path must include at most one current VC per party id (status lock)."""
    from tools.msez import verify_corridor_agreement_vc  # type: ignore

    fixture = REPO / "tests" / "fixtures" / "corridor_agreement_party_specific_duplicate_party"

    errs = verify_corridor_agreement_vc(fixture)
    assert errs, "expected agreement verification to fail when a party appears twice"
    assert any("duplicate party" in e for e in errs), f"unexpected errors: {errs}"
