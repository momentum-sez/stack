import hashlib

from tools.mmr import mmr_root_from_next_roots, build_inclusion_proof, verify_inclusion_proof


def _h(i: int) -> str:
    return hashlib.sha256(f"receipt-{i}".encode("utf-8")).hexdigest()


def test_mmr_inclusion_proofs_roundtrip():
    next_roots = [_h(i) for i in range(1, 18)]
    info = mmr_root_from_next_roots(next_roots)
    assert info["size"] == len(next_roots)
    assert isinstance(info["root"], str) and len(info["root"]) == 64

    for idx in [0, 1, 2, 7, 8, 16]:
        proof = build_inclusion_proof(next_roots, idx)
        assert proof["root"] == info["root"]
        assert proof["size"] == info["size"]
        assert verify_inclusion_proof(proof)


def test_mmr_inclusion_proof_tamper_fails():
    next_roots = [_h(i) for i in range(1, 10)]
    proof = build_inclusion_proof(next_roots, 3)
    assert verify_inclusion_proof(proof)

    # Tamper with one sibling hash
    proof2 = dict(proof)
    proof2["path"] = list(proof["path"])
    proof2["path"][0] = dict(proof["path"][0])
    proof2["path"][0]["hash"] = "00" * 32
    assert not verify_inclusion_proof(proof2)
