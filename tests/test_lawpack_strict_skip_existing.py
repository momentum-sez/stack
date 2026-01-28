import pathlib
import shutil
import time

import pytest

from tools.lawpack import ingest_lawpack


def test_lawpack_strict_rejects_yaml_implicit_date_types(tmp_path: pathlib.Path):
    """Strict mode should fail if YAML relies on implicit timestamp/date typing.

    This enforces cross-language determinism: non-Python YAML parsers may interpret
    these scalars differently.
    """

    src_module = pathlib.Path("modules/legal/jurisdictions/ex/civil")
    module_dir = tmp_path / "module"
    shutil.copytree(src_module, module_dir)

    # Append an unquoted ISO date, which PyYAML parses as datetime.date.
    sources_path = module_dir / "sources.yaml"
    sources_path.write_text(sources_path.read_text(encoding="utf-8") + "\nimplicit_date: 2025-01-01\n", encoding="utf-8")

    out_dir = tmp_path / "dist" / "lawpacks"

    with pytest.raises(ValueError) as ei:
        ingest_lawpack(
            module_dir=module_dir,
            out_dir=out_dir,
            as_of_date="2025-01-01",
            repo_root=tmp_path,
            strict=True,
        )

    assert "implicit" in str(ei.value).lower() or "timestamp" in str(ei.value).lower()


def test_lawpack_skip_existing_is_side_effect_free(tmp_path: pathlib.Path):
    """With strict+skip_existing, ingest_lawpack should not rewrite files."""

    src_module = pathlib.Path("modules/legal/jurisdictions/ex/civil")
    module_dir = tmp_path / "module"
    shutil.copytree(src_module, module_dir)

    out_dir = tmp_path / "dist" / "lawpacks"

    # First run generates artifacts.
    lock1 = ingest_lawpack(
        module_dir=module_dir,
        out_dir=out_dir,
        as_of_date="2025-01-01",
        repo_root=tmp_path,
        tool_version="test",
        strict=True,
    )

    lock_path = module_dir / "lawpack.lock.json"
    artifact_path = tmp_path / lock1["artifact_path"]

    lock_mtime = lock_path.stat().st_mtime_ns
    art_mtime = artifact_path.stat().st_mtime_ns

    # Sleep to make mtime changes unambiguous on coarse filesystems.
    time.sleep(0.02)

    lock2 = ingest_lawpack(
        module_dir=module_dir,
        out_dir=out_dir,
        as_of_date="2025-01-01",
        repo_root=tmp_path,
        tool_version="test",
        strict=True,
        skip_existing=True,
    )

    assert lock2["lawpack_digest_sha256"] == lock1["lawpack_digest_sha256"]
    assert lock_path.stat().st_mtime_ns == lock_mtime
    assert artifact_path.stat().st_mtime_ns == art_mtime


def test_lawpack_strict_skip_existing_requires_existing_lock(tmp_path: pathlib.Path):
    src_module = pathlib.Path("modules/legal/jurisdictions/ex/civil")
    module_dir = tmp_path / "module"
    shutil.copytree(src_module, module_dir)

    # Remove the lock so strict+skip_existing behaves like a pure check.
    (module_dir / "lawpack.lock.json").unlink(missing_ok=True)

    out_dir = tmp_path / "dist" / "lawpacks"
    assert not out_dir.exists()

    with pytest.raises(FileNotFoundError):
        ingest_lawpack(
            module_dir=module_dir,
            out_dir=out_dir,
            as_of_date="2025-01-01",
            repo_root=tmp_path,
            tool_version="test",
            strict=True,
            skip_existing=True,
        )

    # No output dir created in check mode.
    assert not out_dir.exists()
