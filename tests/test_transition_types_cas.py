from pathlib import Path


def test_transition_types_lock_cas_store_resolves():
    """A transition-types.lock.json snapshot should be resolvable by digest via the CAS store."""
    from tools.msez import (
        REPO_ROOT,
        load_transition_type_registry_lock,
        resolve_transition_type_registry_lock_by_digest,
        transition_types_lock_cas_path,
    )

    lock_path = REPO_ROOT / "registries" / "transition-types.lock.json"
    assert lock_path.exists()

    _obj, _mapping, digest = load_transition_type_registry_lock(lock_path)

    cas_path = transition_types_lock_cas_path(digest)
    # The repo includes the current lock snapshot in the CAS store for verifier convenience.
    assert cas_path.exists()

    resolved = resolve_transition_type_registry_lock_by_digest(digest)
    assert Path(resolved).exists()

    _obj2, _mapping2, digest2 = load_transition_type_registry_lock(Path(resolved))
    assert digest2 == digest
