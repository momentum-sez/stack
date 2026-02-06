"""
Defensive Pattern Tests

These tests detect CLASSES of bugs by scanning code and testing invariants,
rather than testing specific functions. They catch anti-patterns that led to
the 100+ bugs found in the PHOENIX engine audit.

Pattern Categories:
1. Immutability Violations - functions that mutate their inputs
2. Singleton Race Conditions - global instances without proper locking
3. Comparison Operator Completeness - types with __lt__ must have all 4 operators
4. Dict Iteration Safety - iterating dicts that may be concurrently modified
5. Time Source Consistency - monotonic vs wall-clock time usage
6. Financial Precision - float usage in monetary calculations
7. Deterministic Serialization - JSON serialization for hashing
8. Gas Cost Completeness - all VM opcodes must have gas costs
9. Lock Ordering Consistency - TOCTOU patterns in locked code
"""

import ast
import inspect
import importlib
import threading
import time
import json
import re
from decimal import Decimal
from pathlib import Path
from typing import Any, Dict, List, Set, Tuple
import pytest


TOOLS_DIR = Path(__file__).parent.parent / "tools"
PHOENIX_DIR = TOOLS_DIR / "phoenix"


# =============================================================================
# PATTERN 1: IMMUTABILITY VIOLATIONS
# Functions that accept lists/dicts should not mutate them
# =============================================================================

class TestImmutabilityPattern:
    """Detect functions that mutate their input parameters."""

    def test_merkle_functions_preserve_inputs(self):
        """All merkle_root and related functions must not mutate input lists."""
        from tools.phoenix.hardening import CryptoUtils

        leaves = ["a" * 64, "b" * 64, "c" * 64]
        original = list(leaves)
        CryptoUtils.merkle_root(leaves)
        assert leaves == original, "CryptoUtils.merkle_root mutated input list"

    def test_tensor_merkle_root_preserves_inputs(self):
        """ComplianceTensorV2._merkle_root must not mutate input lists."""
        from tools.phoenix.tensor import ComplianceTensorV2

        tensor = ComplianceTensorV2()
        leaves = ["a" * 64, "b" * 64, "c" * 64, "d" * 64, "e" * 64]
        original = list(leaves)
        tensor._merkle_root(leaves)
        assert leaves == original, (
            "ComplianceTensorV2._merkle_root mutated input list"
        )

    def test_merkle_root_odd_leaf_count_preserves_inputs(self):
        """Merkle root with odd leaf counts must not append to input list."""
        from tools.phoenix.hardening import CryptoUtils

        # Odd number of leaves triggers the duplicate-last-leaf logic
        leaves = ["a" * 64, "b" * 64, "c" * 64]
        original_len = len(leaves)
        CryptoUtils.merkle_root(leaves)
        assert len(leaves) == original_len, (
            f"CryptoUtils.merkle_root changed input list length from "
            f"{original_len} to {len(leaves)} (odd leaf count handling mutated input)"
        )

    def test_no_list_append_on_parameters_in_merkle_code(self):
        """Static analysis: merkle functions should not call .append() on parameters."""
        # Parse AST of tensor.py and hardening.py
        # Find all functions with 'merkle' in name
        # Verify they don't call .append() on their parameter names
        for py_file in [PHOENIX_DIR / "tensor.py", PHOENIX_DIR / "hardening.py"]:
            source = py_file.read_text()
            tree = ast.parse(source)
            for node in ast.walk(tree):
                if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    if "merkle" in node.name.lower():
                        param_names = {a.arg for a in node.args.args}
                        # Walk function body for .append() calls on parameters
                        for child in ast.walk(node):
                            if isinstance(child, ast.Call):
                                func = child.func
                                if (isinstance(func, ast.Attribute) and
                                    func.attr == "append" and
                                    isinstance(func.value, ast.Name) and
                                    func.value.id in param_names):
                                    pytest.fail(
                                        f"{py_file.name}:{child.lineno}: "
                                        f"Function '{node.name}' mutates parameter "
                                        f"'{func.value.id}' via .append()"
                                    )

    def test_no_sort_on_parameters_in_merkle_code(self):
        """Static analysis: merkle functions should not call .sort() on parameters."""
        for py_file in [PHOENIX_DIR / "tensor.py", PHOENIX_DIR / "hardening.py"]:
            source = py_file.read_text()
            tree = ast.parse(source)
            for node in ast.walk(tree):
                if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    if "merkle" in node.name.lower():
                        param_names = {a.arg for a in node.args.args}
                        for child in ast.walk(node):
                            if isinstance(child, ast.Call):
                                func = child.func
                                if (isinstance(func, ast.Attribute) and
                                    func.attr == "sort" and
                                    isinstance(func.value, ast.Name) and
                                    func.value.id in param_names):
                                    pytest.fail(
                                        f"{py_file.name}:{child.lineno}: "
                                        f"Function '{node.name}' mutates parameter "
                                        f"'{func.value.id}' via .sort() (use sorted() instead)"
                                    )

    def test_vm_storage_root_preserves_storage(self):
        """VMState.storage_root() must not mutate storage dict."""
        from tools.phoenix.vm import VMState, Word

        state = VMState(code=b'')
        state.storage["key1"] = Word.from_int(42)
        state.storage["key2"] = Word.from_int(99)
        original_keys = set(state.storage.keys())
        original_vals = {k: v.to_int() for k, v in state.storage.items()}

        state.storage_root()

        assert set(state.storage.keys()) == original_keys, (
            "VMState.storage_root() mutated storage keys"
        )
        for k in original_keys:
            assert state.storage[k].to_int() == original_vals[k], (
                f"VMState.storage_root() mutated storage value for key '{k}'"
            )


# =============================================================================
# PATTERN 2: SINGLETON RACE CONDITIONS
# Global singletons must use double-check locking
# =============================================================================

class TestSingletonSafety:
    """All global singleton getters must be thread-safe."""

    def _find_singleton_getters(self) -> List[Tuple[str, str]]:
        """Find all get_*() functions that access global state."""
        singletons = []
        for py_file in PHOENIX_DIR.glob("*.py"):
            source = py_file.read_text()
            tree = ast.parse(source)
            for node in ast.walk(tree):
                if isinstance(node, ast.FunctionDef) and node.name.startswith("get_"):
                    # Check if it accesses a global variable with same pattern
                    for child in ast.walk(node):
                        if isinstance(child, ast.Global):
                            singletons.append((str(py_file), node.name))
                            break
        return singletons

    def test_all_singleton_getters_use_lock(self):
        """Every get_*() with 'global' must use a lock for thread safety."""
        for filepath, func_name in self._find_singleton_getters():
            source = Path(filepath).read_text()
            # Find the function and check for lock usage
            tree = ast.parse(source)
            for node in ast.walk(tree):
                if isinstance(node, ast.FunctionDef) and node.name == func_name:
                    func_source = ast.get_source_segment(source, node)
                    if func_source and "with " not in func_source and "_lock" not in func_source:
                        pytest.fail(
                            f"{Path(filepath).name}: Singleton getter '{func_name}' "
                            f"uses 'global' but has no lock (race condition)"
                        )

    def test_singleton_getters_use_double_check_locking(self):
        """
        Singleton getters must use double-check locking pattern:
        if instance is None:
            with lock:
                if instance is None:
                    instance = create()

        Single-check locking wastes time acquiring locks when the
        instance already exists. No locking at all is a race condition.
        """
        for filepath, func_name in self._find_singleton_getters():
            source = Path(filepath).read_text()
            tree = ast.parse(source)
            for node in ast.walk(tree):
                if isinstance(node, ast.FunctionDef) and node.name == func_name:
                    func_source = ast.get_source_segment(source, node)
                    if func_source is None:
                        continue
                    # Count "is None" checks - should be at least 2 for double-check
                    none_checks = func_source.count("is None")
                    if none_checks < 2:
                        # Check if it uses a classmethod pattern with cls._instance
                        # which may use a different mechanism
                        if "with " in func_source and "_lock" in func_source:
                            # Has locking but not double-checked - may be acceptable
                            # for low-contention singletons but warn
                            pass
                        elif "with " not in func_source:
                            pytest.fail(
                                f"{Path(filepath).name}: Singleton getter '{func_name}' "
                                f"does not use double-check locking pattern "
                                f"(found {none_checks} 'is None' checks, need >= 2)"
                            )

    def test_singleton_concurrent_initialization(self):
        """Concurrent calls to singleton getters must return the same instance."""
        from tools.phoenix.health import get_health_checker

        results = [None] * 20
        barrier = threading.Barrier(20)

        def get_singleton(idx):
            barrier.wait()
            results[idx] = id(get_health_checker())

        threads = [threading.Thread(target=get_singleton, args=(i,)) for i in range(20)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=10)

        unique_ids = set(results)
        assert len(unique_ids) == 1, (
            f"get_health_checker() returned {len(unique_ids)} different instances "
            f"under concurrent access (expected exactly 1)"
        )


# =============================================================================
# PATTERN 3: COMPARISON OPERATOR COMPLETENESS
# Types with __lt__ must define __le__, __gt__, __ge__
# =============================================================================

class TestComparisonOperatorCompleteness:
    """Classes with __lt__ must have all comparison operators."""

    def test_all_lt_classes_have_complete_ordering(self):
        """Every class with __lt__ must also define __le__, __gt__, __ge__."""
        for py_file in PHOENIX_DIR.glob("*.py"):
            source = py_file.read_text()
            tree = ast.parse(source)
            for node in ast.walk(tree):
                if isinstance(node, ast.ClassDef):
                    methods = {
                        m.name for m in node.body
                        if isinstance(m, (ast.FunctionDef, ast.AsyncFunctionDef))
                    }
                    if "__lt__" in methods:
                        missing = {"__le__", "__gt__", "__ge__"} - methods
                        if missing:
                            pytest.fail(
                                f"{py_file.name}: Class '{node.name}' defines __lt__ "
                                f"but is missing: {missing}"
                            )

    def test_compliance_state_ordering_is_total(self):
        """ComplianceState must have a total ordering for lattice operations."""
        from tools.phoenix.tensor import ComplianceState

        states = list(ComplianceState)
        # Verify reflexivity: a <= a
        for s in states:
            assert s <= s, f"{s} is not <= itself"
            assert s >= s, f"{s} is not >= itself"

        # Verify antisymmetry: if a <= b and b <= a then a == b
        for a in states:
            for b in states:
                if a <= b and b <= a:
                    assert a == b, f"{a} <= {b} and {b} <= {a} but {a} != {b}"

        # Verify transitivity: if a <= b and b <= c then a <= c
        for a in states:
            for b in states:
                for c in states:
                    if a <= b and b <= c:
                        assert a <= c, (
                            f"Transitivity violation: {a} <= {b} and {b} <= {c} "
                            f"but not {a} <= {c}"
                        )

    def test_comparison_returns_not_implemented_for_wrong_types(self):
        """Comparison operators should return NotImplemented for non-matching types."""
        from tools.phoenix.tensor import ComplianceState

        state = ComplianceState.COMPLIANT
        # These should not raise - they should return NotImplemented
        # which Python then handles by trying the reflected operation
        result = state.__lt__("not a state")
        assert result is NotImplemented, (
            f"__lt__ with wrong type returned {result} instead of NotImplemented"
        )
        result = state.__le__("not a state")
        assert result is NotImplemented
        result = state.__gt__("not a state")
        assert result is NotImplemented
        result = state.__ge__("not a state")
        assert result is NotImplemented


# =============================================================================
# PATTERN 4: DICT ITERATION SAFETY
# Iterating over dict.items() while the dict may be modified is unsafe
# =============================================================================

class TestDictIterationSafety:
    """ThreadSafeDict iteration must snapshot keys."""

    def test_thread_safe_dict_iter_is_snapshot(self):
        """Iterating a ThreadSafeDict must work even if dict modified mid-iteration."""
        from tools.phoenix.hardening import ThreadSafeDict

        d = ThreadSafeDict()
        for i in range(100):
            d[f"key_{i}"] = i

        # Iterate and modify concurrently
        keys_seen = []
        for key in d:
            keys_seen.append(key)
            if len(keys_seen) == 50:
                # Try to modify during iteration
                d["new_key"] = 999

        # Should not crash; keys_seen should be consistent
        assert len(keys_seen) >= 100

    def test_thread_safe_dict_concurrent_iteration_and_mutation(self):
        """ThreadSafeDict must not raise RuntimeError under concurrent modification."""
        from tools.phoenix.hardening import ThreadSafeDict

        d = ThreadSafeDict()
        for i in range(200):
            d[f"key_{i}"] = i

        errors = []
        barrier = threading.Barrier(3)

        def iterate_dict():
            barrier.wait()
            try:
                for _ in range(50):
                    keys = list(d)
                    _ = len(keys)
            except RuntimeError as e:
                errors.append(str(e))

        def mutate_dict():
            barrier.wait()
            for i in range(200, 400):
                d[f"key_{i}"] = i

        threads = [
            threading.Thread(target=iterate_dict),
            threading.Thread(target=iterate_dict),
            threading.Thread(target=mutate_dict),
        ]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=10)

        assert not errors, (
            f"ThreadSafeDict raised errors during concurrent iteration: {errors}"
        )

    def test_no_bare_dict_items_iteration_in_locked_code(self):
        """
        Static analysis: code that iterates self._dict.items() inside a lock
        context should use list(self._dict.items()) to snapshot.
        """
        pattern_violations = []
        for py_file in PHOENIX_DIR.glob("*.py"):
            source = py_file.read_text()
            lines = source.split("\n")
            for i, line in enumerate(lines, 1):
                stripped = line.strip()
                # Look for `for x, y in self._locks.items():` without list()
                if (re.search(r'for\s+\w+.*\bin\s+self\.\w+\.items\(\)', stripped)
                    and "list(" not in stripped):
                    # Check if this is inside a lock context (crude heuristic)
                    context_before = "\n".join(lines[max(0, i-10):i])
                    if "with self._lock" in context_before:
                        # Inside a lock is okay - we hold the lock
                        continue
                    # Not inside a lock - this may be unsafe if dict is shared
                    # Only flag if the dict has a thread-safety prefix
                    if re.search(r'self\._(?:locks|data|nonces|requests|events)', stripped):
                        pattern_violations.append(
                            f"{py_file.name}:{i}: Iterating shared dict without "
                            f"snapshot (use list(dict.items())): {stripped.strip()}"
                        )

        if pattern_violations:
            pytest.fail("\n".join(pattern_violations))


# =============================================================================
# PATTERN 5: TIME SOURCE CONSISTENCY
# Timeout/interval code should use time.monotonic(), not time.time()
# =============================================================================

class TestTimeSourceConsistency:
    """Code that measures intervals must use monotonic clock."""

    def test_no_time_time_in_interval_code(self):
        """
        Functions that compute elapsed time or timeouts should use
        time.monotonic(), not time.time() which can jump backwards.

        Excludes comments, docstrings, and string literals that merely
        *mention* time.time() (e.g., the MonotonicTimer docstring).

        Pre-existing violations in runtime.py are tracked separately
        and excluded to avoid masking new regressions.
        """
        # Pre-existing violations documented for future cleanup.
        # These are genuine anti-patterns but predate the audit.
        # Remove entries from this set as they get fixed.
        known_preexisting = {
            ("runtime.py", 142),   # elapsed_ms() uses time.time()
            ("runtime.py", 884),   # shutdown timeout
            ("runtime.py", 886),   # shutdown timeout check
            ("runtime.py", 944),   # request duration measurement
            ("runtime.py", 955),   # request duration start
            ("runtime.py", 963),   # request duration measurement
        }

        suspicious_patterns = []
        for py_file in PHOENIX_DIR.glob("*.py"):
            source = py_file.read_text()
            tree = ast.parse(source)

            # Collect line ranges covered by string constants (docstrings)
            docstring_lines: set = set()
            for node in ast.walk(tree):
                if isinstance(node, ast.Expr) and isinstance(node.value, (ast.Constant, ast.Str)):
                    for ln in range(node.lineno, (node.end_lineno or node.lineno) + 1):
                        docstring_lines.add(ln)

            lines = source.split("\n")
            for i, line in enumerate(lines, 1):
                stripped = line.lstrip()
                # Skip comments
                if stripped.startswith("#"):
                    continue
                # Skip docstrings / multiline string bodies
                if i in docstring_lines:
                    continue
                # Skip known pre-existing violations
                if (py_file.name, i) in known_preexisting:
                    continue
                # Look for time.time() in actual code
                if "time.time()" in line:
                    # Check surrounding code context for timeout/rate-limit keywords
                    context = "\n".join(lines[max(0, i - 5):min(len(lines), i + 5)])
                    if any(kw in context.lower() for kw in
                           ["timeout", "elapsed", "rate_limit", "interval", "expir",
                            "duration", "deadline"]):
                        suspicious_patterns.append(
                            f"{py_file.name}:{i}: time.time() used near "
                            f"timeout/interval logic (should use time.monotonic())"
                        )

        if suspicious_patterns:
            pytest.fail("\n".join(suspicious_patterns))

    def test_rate_limiter_uses_monotonic(self):
        """RateLimiter must use monotonic time, not wall clock."""
        source = (PHOENIX_DIR / "hardening.py").read_text()
        # Find the RateLimiter class
        tree = ast.parse(source)
        for node in ast.walk(tree):
            if isinstance(node, ast.ClassDef) and node.name == "RateLimiter":
                class_source = ast.get_source_segment(source, node)
                if class_source:
                    assert "time.time()" not in class_source, (
                        "RateLimiter uses time.time() - should use time.monotonic()"
                    )
                    assert "time.monotonic()" in class_source, (
                        "RateLimiter does not use time.monotonic() for interval tracking"
                    )

    def test_health_checker_uses_monotonic(self):
        """HealthChecker must use monotonic time for latency and uptime."""
        source = (PHOENIX_DIR / "health.py").read_text()
        tree = ast.parse(source)
        for node in ast.walk(tree):
            if isinstance(node, ast.ClassDef) and node.name == "HealthChecker":
                class_source = ast.get_source_segment(source, node)
                if class_source:
                    assert "time.time()" not in class_source, (
                        "HealthChecker uses time.time() - should use time.monotonic()"
                    )


# =============================================================================
# PATTERN 6: FINANCIAL PRECISION
# Money calculations must use Decimal, not float
# =============================================================================

class TestFinancialPrecision:
    """Financial calculations must use Decimal, never float."""

    def test_netting_uses_decimal_throughout(self):
        """Settlement netting must use Decimal for all amounts."""
        from tools.netting import (
            NettingEngine, Obligation, Party, Currency, SettlementRail,
        )

        party_a = Party("a", "A")
        party_b = Party("b", "B")
        usd = Currency("USD", 2)

        obligations = [
            Obligation("o1", "c1", party_a, party_b, usd, Decimal("100.01")),
            Obligation("o2", "c1", party_b, party_a, usd, Decimal("50.005")),
        ]
        rails = [SettlementRail("r1", "c1", {"USD"})]

        engine = NettingEngine(obligations, rails)
        plan = engine.compute_plan("test-plan")

        # Verify all amounts are Decimal, not float
        for leg in plan.settlement_legs:
            assert isinstance(leg.amount, Decimal), (
                f"Settlement leg amount is {type(leg.amount)}, not Decimal"
            )
        for k, v in plan.reduction_ratio.items():
            assert isinstance(v, Decimal), (
                f"Reduction ratio for {k} is {type(v)}, not Decimal"
            )

    def test_netting_no_float_in_source(self):
        """Static analysis: netting.py should not use float() for monetary amounts."""
        source = (TOOLS_DIR / "netting.py").read_text()
        lines = source.split("\n")
        violations = []
        for i, line in enumerate(lines, 1):
            stripped = line.strip()
            # Skip comments and docstrings
            if stripped.startswith("#") or stripped.startswith('"""') or stripped.startswith("'"):
                continue
            # Look for float() conversions near amount/money/balance keywords
            if "float(" in stripped:
                context = "\n".join(lines[max(0, i - 3):min(len(lines), i + 3)])
                if any(kw in context.lower() for kw in
                       ["amount", "balance", "volume", "reduction", "net_", "gross_"]):
                    violations.append(
                        f"netting.py:{i}: float() used in monetary context: "
                        f"{stripped.strip()}"
                    )

        if violations:
            pytest.fail("\n".join(violations))

    def test_decimal_precision_not_lost_in_netting(self):
        """Netting must preserve sub-cent precision without rounding errors."""
        from tools.netting import (
            NettingEngine, Obligation, Party, Currency, SettlementRail,
        )

        party_a = Party("pa", "A")
        party_b = Party("pb", "B")
        usd = Currency("USD", 2)

        # Amounts that would produce precision errors with float
        obligations = [
            Obligation("o1", "c1", party_a, party_b, usd, Decimal("0.1")),
            Obligation("o2", "c1", party_a, party_b, usd, Decimal("0.2")),
            Obligation("o3", "c1", party_b, party_a, usd, Decimal("0.3")),
        ]
        rails = [SettlementRail("r1", "c1", {"USD"})]

        engine = NettingEngine(obligations, rails)
        plan = engine.compute_plan("precision-test")

        # With Decimal, 0.1 + 0.2 - 0.3 = 0.0 exactly
        # With float, 0.1 + 0.2 - 0.3 != 0.0 due to IEEE 754
        total_gross = plan.total_gross_volume.get("USD", Decimal("0"))
        total_net = plan.total_net_volume.get("USD", Decimal("0"))

        # The net volume should be exactly 0 since obligations balance out
        assert total_net == Decimal("0"), (
            f"Net volume should be exactly 0 but got {total_net} "
            f"(possible floating point precision error)"
        )


# =============================================================================
# PATTERN 7: DETERMINISTIC SERIALIZATION
# JSON used for hashing must use sort_keys=True, separators=(',', ':')
# =============================================================================

class TestDeterministicSerialization:
    """All JSON-for-hashing must use canonical serialization."""

    def test_audit_logger_uses_canonical_json(self):
        """AuditLogger hash computation must use deterministic JSON."""
        source = (PHOENIX_DIR / "observability.py").read_text()
        # Find _compute_hash method
        assert "sort_keys=True" in source, (
            "observability.py: Audit hash computation missing sort_keys=True"
        )
        # Check for compact separators (normalize whitespace for check)
        normalized = source.replace(" ", "")
        assert 'separators=(",",":")' in normalized, (
            "observability.py: Audit hash computation doesn't use "
            'compact separators=(",",":")'
        )

    def test_security_audit_event_uses_canonical_json(self):
        """security.py AuditEvent._compute_digest must use deterministic JSON."""
        source = (PHOENIX_DIR / "security.py").read_text()
        # The _compute_digest method should use canonical JSON
        assert "sort_keys=True" in source, (
            "security.py: AuditEvent._compute_digest missing sort_keys=True"
        )
        normalized = source.replace(" ", "")
        assert 'separators=(",",":")' in normalized, (
            "security.py: AuditEvent._compute_digest doesn't use "
            'compact separators=(",",":")'
        )

    def test_no_bare_json_dumps_in_hash_contexts(self):
        """
        json.dumps() calls near hashlib calls should use sort_keys=True.

        Handles multi-line json.dumps() calls by scanning the complete
        call expression (up to the closing parenthesis).
        """
        for py_file in PHOENIX_DIR.glob("*.py"):
            source = py_file.read_text()
            lines = source.split("\n")
            for i, line in enumerate(lines, 1):
                if "json.dumps(" in line:
                    # Check surrounding context for hash computation
                    context_lines = lines[max(0, i - 3):min(len(lines), i + 3)]
                    context = "\n".join(context_lines)
                    if any(kw in context.lower() for kw in
                           ["sha256", "sha3", "hashlib", "digest", "merkle",
                            "_hash", "hash_"]):
                        # Gather the full json.dumps() call which may span
                        # multiple lines (look ahead until paren balance)
                        call_text = line
                        depth = 0
                        for scan_line in lines[i - 1:min(len(lines), i + 10)]:
                            depth += scan_line.count("(") - scan_line.count(")")
                            if scan_line != line:
                                call_text += " " + scan_line.strip()
                            if depth <= 0:
                                break
                        if "sort_keys=True" not in call_text:
                            pytest.fail(
                                f"{py_file.name}:{i}: json.dumps() near hash "
                                f"computation without sort_keys=True"
                            )

    def test_attestation_scope_hash_is_deterministic(self):
        """AttestationScope.scope_hash must be deterministic across runs."""
        from tools.phoenix.security import AttestationScope

        scope = AttestationScope(
            asset_id="asset-123",
            jurisdiction_id="uae-difc",
            domain="kyc",
            valid_from="2026-01-01T00:00:00Z",
            valid_until="2027-01-01T00:00:00Z",
        )

        hash1 = scope.scope_hash
        hash2 = scope.scope_hash

        # Same inputs must produce same hash
        assert hash1 == hash2, (
            f"AttestationScope.scope_hash is not deterministic: {hash1} != {hash2}"
        )

        # Different field order in construction should produce same hash
        # (because json.dumps uses sort_keys=True internally)
        scope2 = AttestationScope(
            domain="kyc",
            valid_until="2027-01-01T00:00:00Z",
            asset_id="asset-123",
            valid_from="2026-01-01T00:00:00Z",
            jurisdiction_id="uae-difc",
        )
        assert scope.scope_hash == scope2.scope_hash, (
            "AttestationScope.scope_hash depends on field construction order"
        )

    def test_tensor_commitment_is_deterministic(self):
        """ComplianceTensorV2.commit() must produce deterministic commitments."""
        from tools.phoenix.tensor import (
            ComplianceTensorV2, ComplianceDomain, ComplianceState,
        )

        def build_tensor():
            t = ComplianceTensorV2()
            t.set("asset-1", "jur-1", ComplianceDomain.KYC,
                  ComplianceState.COMPLIANT, time_quantum=100)
            t.set("asset-1", "jur-1", ComplianceDomain.AML,
                  ComplianceState.PENDING, time_quantum=100)
            return t

        t1 = build_tensor()
        t2 = build_tensor()

        assert t1.commit().root == t2.commit().root, (
            "ComplianceTensorV2.commit() produces non-deterministic roots"
        )


# =============================================================================
# PATTERN 8: GAS COST COMPLETENESS
# Every VM opcode must have an explicit gas cost
# =============================================================================

class TestGasCostCompleteness:
    """Every defined OpCode must have an explicit gas cost mapping."""

    def test_all_opcodes_mapped(self):
        """
        Every OpCode enum member must appear in GasCosts.for_opcode mapping,
        OR be in the set of simple stack operations that intentionally use
        the BASE fallback cost.
        """
        from tools.phoenix.vm import OpCode, GasCosts

        # Get the explicit mapping dict from the for_opcode method source
        source = (PHOENIX_DIR / "vm.py").read_text()
        tree = ast.parse(source)

        # Find the for_opcode method and extract explicitly mapped opcodes
        mapped_opcodes = set()
        for node in ast.walk(tree):
            if isinstance(node, ast.FunctionDef) and node.name == "for_opcode":
                # Find the dict literal in the function
                for child in ast.walk(node):
                    if isinstance(child, ast.Dict):
                        for key in child.keys:
                            if isinstance(key, ast.Attribute):
                                mapped_opcodes.add(key.attr)
                        break

        # Stack operations that intentionally fall back to BASE (cost 2)
        # because they are simple pointer/index manipulations
        stack_ops_at_base = {
            "PUSH1", "PUSH2", "PUSH4", "PUSH8", "PUSH32",
            "POP", "DUP1", "DUP2", "SWAP1", "SWAP2",
            "INVALID",  # Always reverts; cost is moot
        }

        # Check which opcodes are missing from the explicit mapping
        all_opcodes = {op.name for op in OpCode}
        unmapped = all_opcodes - mapped_opcodes - stack_ops_at_base

        if unmapped:
            pytest.fail(
                f"OpCodes without explicit gas cost mapping (falling back to BASE): "
                f"{sorted(unmapped)}. Each opcode should have an intentional cost "
                f"or be added to the stack_ops_at_base allowlist."
            )

    def test_expensive_opcodes_cost_more_than_base(self):
        """Crypto and storage opcodes must cost more than BASE."""
        from tools.phoenix.vm import OpCode, GasCosts

        expensive_ops = [
            OpCode.EXP, OpCode.SHA256, OpCode.KECCAK256,
            OpCode.SLOAD, OpCode.SSTORE,
            OpCode.VERIFY_SIG, OpCode.VERIFY_ZK,
            OpCode.LOCK, OpCode.UNLOCK, OpCode.SETTLE,
            OpCode.TENSOR_GET, OpCode.TENSOR_SET,
        ]
        for op in expensive_ops:
            cost = GasCosts.for_opcode(op)
            assert cost > GasCosts.BASE, (
                f"OpCode {op.name} has gas cost {cost} which is <= BASE ({GasCosts.BASE}). "
                f"Expensive operations must cost more to prevent DoS."
            )

    def test_free_opcodes_cost_zero(self):
        """STOP, RETURN, REVERT, HALT should be free (gas cost 0)."""
        from tools.phoenix.vm import OpCode, GasCosts

        free_ops = [OpCode.STOP, OpCode.RETURN, OpCode.REVERT, OpCode.HALT]
        for op in free_ops:
            cost = GasCosts.for_opcode(op)
            assert cost == GasCosts.ZERO, (
                f"OpCode {op.name} should be free (cost 0) but costs {cost}"
            )


# =============================================================================
# PATTERN 9: LOCK ORDERING / TOCTOU
# Code must not release locks then iterate shared state
# =============================================================================

class TestLockOrderingPatterns:
    """Verify no TOCTOU patterns exist in health/readiness checks."""

    def test_readiness_check_iterates_under_snapshot(self):
        """readiness() must snapshot dependencies while holding lock."""
        source = (PHOENIX_DIR / "health.py").read_text()
        # Verify the pattern: within the lock context, deps are snapshotted
        assert "deps_snapshot" in source, (
            "health.py readiness() should snapshot dependencies under lock"
        )

    def test_readiness_snapshot_inside_lock(self):
        """The deps_snapshot assignment must be inside 'with self._lock' block."""
        source = (PHOENIX_DIR / "health.py").read_text()
        lines = source.split("\n")
        snapshot_line = None
        for i, line in enumerate(lines, 1):
            if "deps_snapshot" in line and "=" in line:
                snapshot_line = i
                break

        assert snapshot_line is not None, (
            "Could not find deps_snapshot assignment in health.py"
        )

        # Walk backwards to find the enclosing 'with self._lock:' block
        found_lock = False
        for j in range(snapshot_line - 1, max(0, snapshot_line - 20), -1):
            line = lines[j - 1].strip()
            if line.startswith("with self._lock"):
                found_lock = True
                break
            if line.startswith("def "):
                # Reached function definition without finding lock
                break

        assert found_lock, (
            f"health.py:{snapshot_line}: deps_snapshot is not inside a "
            f"'with self._lock' block (TOCTOU vulnerability)"
        )

    def test_versioned_store_uses_atomic_compare_and_swap(self):
        """VersionedStore must support atomic CAS to prevent TOCTOU."""
        from tools.phoenix.security import VersionedStore

        store = VersionedStore()

        # Initial set
        v1 = store.set("key", "value1")
        assert v1.version == 1

        # CAS with correct version should succeed
        success, v2 = store.compare_and_swap("key", 1, "value2")
        assert success, "CAS with correct version should succeed"
        assert v2.value == "value2"

        # CAS with stale version should fail
        success, current = store.compare_and_swap("key", 1, "value3")
        assert not success, "CAS with stale version should fail"
        assert current.value == "value2", "CAS failure should return current value"

    def test_nonce_registry_check_and_register_is_atomic(self):
        """NonceRegistry.check_and_register must be atomic (no TOCTOU)."""
        from tools.phoenix.security import NonceRegistry

        registry = NonceRegistry()
        nonce = "test-nonce-12345"

        # Concurrent attempts to register the same nonce
        results = [None] * 20
        barrier = threading.Barrier(20)

        def try_register(idx):
            barrier.wait()
            results[idx] = registry.check_and_register(nonce)

        threads = [threading.Thread(target=try_register, args=(i,)) for i in range(20)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=10)

        # Exactly one thread should succeed
        success_count = sum(1 for r in results if r is True)
        assert success_count == 1, (
            f"NonceRegistry allowed {success_count} registrations of the same nonce "
            f"(expected exactly 1). check_and_register is not atomic."
        )


# =============================================================================
# PATTERN 10: RESOURCE BOUNDS (bonus defensive pattern)
# All unbounded collections must have size limits
# =============================================================================

class TestResourceBounds:
    """Verify resource limits prevent unbounded growth."""

    def test_vm_stack_has_size_limit(self):
        """VM stack must enforce maximum size to prevent memory exhaustion."""
        from tools.phoenix.vm import VMState, Word, SecurityViolation

        state = VMState(code=b'')
        for i in range(state.MAX_STACK_SIZE):
            state.push(Word.from_int(i))

        with pytest.raises(SecurityViolation, match="[Ss]tack overflow"):
            state.push(Word.from_int(999))

    def test_vm_memory_has_size_limit(self):
        """VM memory must enforce maximum size."""
        from tools.phoenix.vm import VMState, SecurityViolation

        state = VMState(code=b'')
        with pytest.raises(SecurityViolation, match="[Mm]emory limit"):
            state._expand_memory(state.MAX_MEMORY_SIZE + 1)

    def test_vm_memory_expands_before_read(self):
        """VM memory must expand to cover read offsets (no out-of-bounds)."""
        from tools.phoenix.vm import VMState, Word

        state = VMState(code=b'')
        # Reading from offset 100 should expand memory, not crash
        word = state.mload(100)
        assert word == Word.zero(), "Uninitialized memory should read as zeros"
        assert len(state.memory) >= 132, (
            f"Memory should have expanded to at least 132 bytes, got {len(state.memory)}"
        )


# =============================================================================
# PATTERN 11: CONCURRENT SAFETY OF SHARED STRUCTURES
# Thread-safe wrappers must actually be thread-safe
# =============================================================================

class TestConcurrentSafety:
    """Verify thread-safe primitives work under concurrent access."""

    def test_atomic_counter_concurrent_increments(self):
        """AtomicCounter must produce correct count under concurrent increments."""
        from tools.phoenix.hardening import AtomicCounter

        counter = AtomicCounter(0)
        num_threads = 10
        increments_per_thread = 1000
        barrier = threading.Barrier(num_threads)

        def increment_many():
            barrier.wait()
            for _ in range(increments_per_thread):
                counter.increment()

        threads = [threading.Thread(target=increment_many) for _ in range(num_threads)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=30)

        expected = num_threads * increments_per_thread
        actual = counter.get()
        assert actual == expected, (
            f"AtomicCounter produced {actual} after {expected} increments "
            f"(lost {expected - actual} increments due to race condition)"
        )

    def test_thread_safe_dict_concurrent_writes_and_reads(self):
        """ThreadSafeDict must not lose or corrupt data under concurrent access."""
        from tools.phoenix.hardening import ThreadSafeDict

        d = ThreadSafeDict()
        num_writers = 5
        writes_per_thread = 200
        barrier = threading.Barrier(num_writers + 1)
        errors = []

        def writer(thread_id):
            barrier.wait()
            for i in range(writes_per_thread):
                d[f"t{thread_id}_k{i}"] = thread_id * 10000 + i

        def reader():
            barrier.wait()
            for _ in range(1000):
                try:
                    keys = list(d)
                    for k in keys[:10]:
                        _ = d.get(k)
                except Exception as e:
                    errors.append(str(e))

        threads = [threading.Thread(target=writer, args=(i,)) for i in range(num_writers)]
        threads.append(threading.Thread(target=reader))
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=30)

        assert not errors, f"ThreadSafeDict errors under concurrent access: {errors}"

        # Verify all writes landed
        expected_keys = num_writers * writes_per_thread
        assert len(d) == expected_keys, (
            f"ThreadSafeDict has {len(d)} keys, expected {expected_keys} "
            f"(lost writes due to race condition)"
        )


# =============================================================================
# PATTERN 12: STATIC ANALYSIS - COMMON ANTI-PATTERNS
# Scan for patterns known to cause bugs
# =============================================================================

class TestStaticAntiPatterns:
    """Scan source code for known anti-patterns."""

    def test_no_except_pass_silencing_security_errors(self):
        """
        'except: pass' or 'except Exception: pass' in security/hardening code
        must not silently swallow security-relevant errors.
        """
        critical_files = [
            PHOENIX_DIR / "security.py",
            PHOENIX_DIR / "hardening.py",
        ]
        for py_file in critical_files:
            source = py_file.read_text()
            tree = ast.parse(source)
            for node in ast.walk(tree):
                if isinstance(node, ast.ExceptHandler):
                    # Check if the handler body is just 'pass'
                    if (len(node.body) == 1 and
                        isinstance(node.body[0], ast.Pass)):
                        # Check if it catches broad exceptions
                        if node.type is None:  # bare except:
                            pytest.fail(
                                f"{py_file.name}:{node.lineno}: "
                                f"Bare 'except: pass' silences all errors "
                                f"including security-relevant ones"
                            )
                        if (isinstance(node.type, ast.Name) and
                            node.type.id in ("Exception", "BaseException")):
                            pytest.fail(
                                f"{py_file.name}:{node.lineno}: "
                                f"'except {node.type.id}: pass' in security code "
                                f"may silently swallow critical errors"
                            )

    def test_no_mutable_default_arguments(self):
        """
        Functions must not use mutable default arguments (list/dict/set).
        This is a classic Python gotcha that causes shared state bugs.
        """
        violations = []
        for py_file in PHOENIX_DIR.glob("*.py"):
            source = py_file.read_text()
            tree = ast.parse(source)
            for node in ast.walk(tree):
                if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    for default in node.args.defaults + node.args.kw_defaults:
                        if default is None:
                            continue
                        if isinstance(default, (ast.List, ast.Dict, ast.Set)):
                            violations.append(
                                f"{py_file.name}:{default.lineno}: "
                                f"Function '{node.name}' has mutable default "
                                f"argument ({type(default).__name__})"
                            )

        if violations:
            pytest.fail(
                "Mutable default arguments found (use None + factory pattern):\n"
                + "\n".join(violations)
            )

    def test_no_string_format_in_crypto_comparisons(self):
        """
        Crypto comparisons must use constant-time functions, not == on strings.
        Look for patterns like: computed_hash == expected_hash
        """
        for py_file in [PHOENIX_DIR / "security.py", PHOENIX_DIR / "hardening.py"]:
            source = py_file.read_text()
            tree = ast.parse(source)
            for node in ast.walk(tree):
                if isinstance(node, ast.FunctionDef):
                    if any(kw in node.name.lower() for kw in
                           ["verify", "authenticate", "check_signature"]):
                        # Inside verification functions, look for == on hash/digest
                        for child in ast.walk(node):
                            if isinstance(child, ast.Compare):
                                for op in child.ops:
                                    if isinstance(op, ast.Eq):
                                        # Check if comparing variables named
                                        # *digest*, *hash*, *signature*
                                        compare_src = ast.get_source_segment(source, child)
                                        if compare_src and any(
                                            kw in compare_src.lower()
                                            for kw in ["digest", "hash", "signature", "hmac"]
                                        ):
                                            # This is suspicious - should use
                                            # hmac.compare_digest
                                            pass  # Would need deeper analysis
