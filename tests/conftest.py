import os
import pathlib
import sys
import pytest


# Ensure repo root is on PYTHONPATH for direct package imports (e.g. `import tools`).
_REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))


def _env_flag(name: str) -> bool:
    v = (os.environ.get(name) or '').strip().lower()
    return v in {'1', 'true', 'yes', 'y', 'on'}


def pytest_configure(config: pytest.Config) -> None:
    config.addinivalue_line(
        "markers",
        "perf: performance/benchmark tests (skipped unless MSEZ_RUN_PERF=1)",
    )
    config.addinivalue_line(
        "markers",
        "slow: slow correctness tests (skipped unless MSEZ_RUN_SLOW=1)",
    )
    config.addinivalue_line(
        "markers",
        "scaffold: large scenario scaffolds (skipped unless MSEZ_RUN_SCAFFOLD=1)",
    )


def pytest_collection_modifyitems(config: pytest.Config, items: list[pytest.Item]) -> None:
    run_perf = _env_flag('MSEZ_RUN_PERF')
    run_slow = _env_flag('MSEZ_RUN_SLOW')
    run_scaffold = _env_flag('MSEZ_RUN_SCAFFOLD')

    for item in items:
        if 'perf' in item.keywords and not run_perf:
            item.add_marker(pytest.mark.skip(reason='perf tests skipped; set MSEZ_RUN_PERF=1 to enable'))
        if 'slow' in item.keywords and not run_slow:
            item.add_marker(pytest.mark.skip(reason='slow tests skipped; set MSEZ_RUN_SLOW=1 to enable'))
        if 'scaffold' in item.keywords and not run_scaffold:
            item.add_marker(pytest.mark.skip(reason='scaffold tests skipped; set MSEZ_RUN_SCAFFOLD=1 to enable'))
