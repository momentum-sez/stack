"""Regression tests for core tool module bug fixes.

Each test class targets a specific module and documents the bug number it guards
against.  These tests are designed to be runnable in isolation with ``pytest``.

Modules covered:
    - tools.netting   (Bugs #68-#70)
    - tools.mmr        (Bugs #71-#73)
    - tools.artifacts  (Bugs #74-#76)
    - tools.vc         (Bug #78)
    - tools.mass_primitives  (Bug #101)
    - tools.smart_asset      (receipt chain continuity, asset_id derivation)
"""

from __future__ import annotations

import hashlib
import os
import pathlib
import shutil
import tempfile
import warnings
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from typing import Any, Dict, List

import pytest

# ──────────────────────────────────────────────────────────────────────────────
# Netting imports
# ──────────────────────────────────────────────────────────────────────────────
from tools.netting import (
    Currency,
    NettingConstraints,
    NettingEngine,
    Obligation,
    Party,
    PartyConstraint,
    SettlementLeg,
    SettlementPlan,
    SettlementRail,
)

# ──────────────────────────────────────────────────────────────────────────────
# MMR imports
# ──────────────────────────────────────────────────────────────────────────────
from tools.mmr import (
    Peak,
    _peak_plan,
    append_peaks,
    bag_peaks,
    build_inclusion_proof,
    build_peaks,
    mmr_leaf_hash,
    mmr_node_hash,
    verify_inclusion_proof,
)

# ──────────────────────────────────────────────────────────────────────────────
# Artifacts imports
# ──────────────────────────────────────────────────────────────────────────────
from tools.artifacts import (
    normalize_artifact_type,
    normalize_digest,
    resolve_artifact_by_digest,
    store_artifact_file,
)

# ──────────────────────────────────────────────────────────────────────────────
# VC imports
# ──────────────────────────────────────────────────────────────────────────────
from tools.vc import validate_credential

# ──────────────────────────────────────────────────────────────────────────────
# Mass Primitives imports
# ──────────────────────────────────────────────────────────────────────────────
from tools.mass_primitives import (
    GenesisDocument,
    JurisdictionalBinding,
    OperationalManifest,
    RegistryCredential,
    SmartAsset,
    TransitionEnvelope,
    TransitionKind,
    genesis_receipt_root,
)

# ──────────────────────────────────────────────────────────────────────────────
# Smart Asset imports
# ──────────────────────────────────────────────────────────────────────────────
from tools.smart_asset import (
    asset_id_from_genesis,
    build_genesis,
    state_root_from_state,
    verify_receipt_chain_continuity,
)


# ╔══════════════════════════════════════════════════════════════════════════════╗
# ║  HELPERS                                                                   ║
# ╚══════════════════════════════════════════════════════════════════════════════╝


def _make_party(pid: str, name: str = "") -> Party:
    return Party(party_id=pid, name=name or pid)


def _make_currency(code: str = "USD", precision: int = 2) -> Currency:
    return Currency(code=code, precision=precision)


def _make_rail(
    rail_id: str = "rail:default",
    corridor_id: str = "corridor:default",
    currencies: set | None = None,
    priority: int = 0,
) -> SettlementRail:
    return SettlementRail(
        rail_id=rail_id,
        corridor_id=corridor_id,
        supported_currencies=currencies or {"USD", "EUR"},
        priority=priority,
    )


def _random_hex32() -> str:
    """Return a random 64-character lowercase hex string (32 bytes)."""
    return hashlib.sha256(os.urandom(32)).hexdigest()


def _make_smart_asset(
    *,
    holders: Dict[str, int] | None = None,
    asset_name: str = "TestAsset",
) -> SmartAsset:
    """Create a minimal SmartAsset for testing transitions."""
    genesis = GenesisDocument(
        asset_name=asset_name,
        asset_class="test",
        initial_bindings=["harbor:test"],
        governance={"quorum": 1},
    )
    binding = JurisdictionalBinding(
        harbor_id="harbor:test",
        lawpack_digest="a" * 64,
    )
    registry = RegistryCredential(
        asset_id=genesis.asset_id,
        bindings=[binding],
        registry_vc_digest="b" * 64,
        effective_from=datetime.now(timezone.utc).isoformat(),
    )
    manifest = OperationalManifest(
        asset_id=genesis.asset_id,
        version=1,
        config={},
        quorum_threshold=1,
        authorized_governors=["did:key:z6MkTest"],
    )
    state: Dict[str, Any] = {}
    if holders:
        state["balances"] = {k: Decimal(str(v)) for k, v in holders.items()}
        state["total_supply"] = Decimal(str(sum(holders.values())))
    asset = SmartAsset(
        genesis=genesis,
        registry=registry,
        manifest=manifest,
        state=state,
    )
    return asset


# ╔══════════════════════════════════════════════════════════════════════════════╗
# ║  NETTING TESTS                                                            ║
# ╚══════════════════════════════════════════════════════════════════════════════╝


class TestNettingBlockedCounterparties:
    """Bug #68: blocked counterparties must be checked during rail selection."""

    def _build_engine(
        self,
        blocked: set | None = None,
    ) -> NettingEngine:
        """Set up a simple two-party netting scenario with optional blocked counterparties."""
        party_a = _make_party("A")
        party_b = _make_party("B")
        usd = _make_currency("USD")

        obligations = [
            Obligation("obl-1", "c1", party_a, party_b, usd, Decimal("100")),
            Obligation("obl-2", "c1", party_b, party_a, usd, Decimal("60")),
        ]
        rails = [_make_rail("rail:swift", currencies={"USD"}, priority=1)]

        constraints = NettingConstraints()
        if blocked:
            constraints.party_constraints["A"] = PartyConstraint(
                party_id="A",
                blocked_counterparties=blocked,
            )
        return NettingEngine(obligations, rails, constraints)

    def test_blocked_counterparty_rail_excluded(self):
        """When B is blocked for A, no rail should be selected for A->B legs."""
        engine = self._build_engine(blocked={"B"})
        plan = engine.compute_plan("plan:test:blocked")

        # A owes B net 40 but B is blocked; the leg from A to B should not exist.
        for leg in plan.settlement_legs:
            assert not (leg.payer == "A" and leg.payee == "B"), (
                "Leg from A to blocked counterparty B should not be generated"
            )

    def test_unblocked_counterparty_produces_legs(self):
        """Without blocked counterparties, legs should be generated normally."""
        engine = self._build_engine(blocked=None)
        plan = engine.compute_plan("plan:test:unblocked")
        assert len(plan.settlement_legs) > 0, "Should produce at least one settlement leg"

    def test_negative_obligation_skipped(self):
        """Obligation with amount <= 0 is skipped during gross position computation."""
        party_a = _make_party("A")
        party_b = _make_party("B")
        usd = _make_currency("USD")

        obligations = [
            Obligation("obl-neg", "c1", party_a, party_b, usd, Decimal("-50")),
            Obligation("obl-zero", "c1", party_a, party_b, usd, Decimal("0")),
            Obligation("obl-pos", "c1", party_a, party_b, usd, Decimal("100")),
        ]
        rails = [_make_rail("rail:swift", currencies={"USD"}, priority=1)]
        engine = NettingEngine(obligations, rails)

        gross = engine.compute_gross_positions()
        # Only the positive obligation should contribute
        assert gross["A"]["USD"]["payable"] == Decimal("100")
        assert gross["B"]["USD"]["receivable"] == Decimal("100")

        # Trace should record skipped obligations
        skipped = [t for t in engine.trace if t.action == "invalid_obligation_skipped"]
        assert len(skipped) == 2, "Both negative and zero obligations should be skipped"

    def test_netting_iteration_bounded(self):
        """Bug #68: the greedy matching loop has an O(n*m) iteration cap."""
        parties = [_make_party(f"P{i}") for i in range(10)]
        usd = _make_currency("USD")

        # Create a dense obligation graph to stress the iteration bound
        obligations = []
        seq = 0
        for i in range(len(parties)):
            for j in range(len(parties)):
                if i != j:
                    seq += 1
                    obligations.append(
                        Obligation(
                            f"obl-{seq}",
                            "c1",
                            parties[i],
                            parties[j],
                            usd,
                            Decimal("10"),
                        )
                    )

        rails = [_make_rail("rail:swift", currencies={"USD"}, priority=1)]
        engine = NettingEngine(obligations, rails)
        # This should complete without hanging
        plan = engine.compute_plan("plan:test:bounded")
        assert isinstance(plan, SettlementPlan)

    def test_reduction_ratio_uses_decimal(self):
        """Bug #69: reduction_ratio values must be Decimal, not float."""
        party_a = _make_party("A")
        party_b = _make_party("B")
        usd = _make_currency("USD")

        obligations = [
            Obligation("obl-1", "c1", party_a, party_b, usd, Decimal("100")),
            Obligation("obl-2", "c1", party_b, party_a, usd, Decimal("60")),
        ]
        rails = [_make_rail("rail:swift", currencies={"USD"}, priority=1)]
        engine = NettingEngine(obligations, rails)
        plan = engine.compute_plan("plan:test:decimal")

        for ccy, ratio in plan.reduction_ratio.items():
            assert isinstance(ratio, Decimal), (
                f"reduction_ratio[{ccy}] is {type(ratio).__name__}, expected Decimal"
            )

    def test_settlement_leg_never_negative(self):
        """Bug #70: all settlement leg amounts must be >= 0."""
        party_a = _make_party("A")
        party_b = _make_party("B")
        party_c = _make_party("C")
        usd = _make_currency("USD")

        obligations = [
            Obligation("obl-1", "c1", party_a, party_b, usd, Decimal("200")),
            Obligation("obl-2", "c1", party_b, party_c, usd, Decimal("150")),
            Obligation("obl-3", "c1", party_c, party_a, usd, Decimal("100")),
        ]
        rails = [_make_rail("rail:swift", currencies={"USD"}, priority=1)]
        engine = NettingEngine(obligations, rails)
        plan = engine.compute_plan("plan:test:nonneg")

        for leg in plan.settlement_legs:
            assert leg.amount >= Decimal("0"), (
                f"Leg {leg.leg_id} has negative amount {leg.amount}"
            )


# ╔══════════════════════════════════════════════════════════════════════════════╗
# ║  MMR TESTS                                                                ║
# ╚══════════════════════════════════════════════════════════════════════════════╝


class TestMMRPeakValidation:
    """Bugs #71-#73: peak decomposition, ordering, and proof verification."""

    def test_peak_calculation_sums_to_size(self):
        """Bug #71: peaks leaf counts must sum exactly to the input size."""
        for size in [1, 2, 3, 4, 5, 7, 8, 15, 16, 17, 31, 32, 33, 1023, 1024]:
            peaks = _peak_plan(size)
            total = sum(cnt for _, cnt in peaks)
            assert total == size, (
                f"_peak_plan({size}): peaks sum to {total}, expected {size}"
            )

    def test_peak_plan_zero_returns_empty(self):
        """Size 0 should return an empty peak list."""
        assert _peak_plan(0) == []

    def test_peak_plan_negative_raises(self):
        """Negative sizes must raise ValueError."""
        with pytest.raises(ValueError, match="size must be >= 0"):
            _peak_plan(-1)

    def test_peak_heights_strictly_decreasing(self):
        """Bug #71: peak heights should be strictly decreasing (binary representation)."""
        for size in [1, 3, 5, 7, 10, 15, 17, 31, 33, 100, 255, 1000]:
            peaks = _peak_plan(size)
            heights = [h for h, _ in peaks]
            for i in range(1, len(heights)):
                assert heights[i] < heights[i - 1], (
                    f"_peak_plan({size}): heights not strictly decreasing: {heights}"
                )

    def test_empty_proof_rejected(self):
        """Bug #73: empty or None proof must return False."""
        assert verify_inclusion_proof({}) is False
        assert verify_inclusion_proof(None) is False

    def test_incomplete_proof_rejected(self):
        """Proof with missing required fields must return False."""
        assert verify_inclusion_proof({"size": 1}) is False
        assert verify_inclusion_proof({"size": 1, "root": "ab" * 32}) is False

    def test_peak_heights_strictly_decreasing_after_append(self):
        """Bug #72: appending leaves must maintain strictly decreasing peak heights."""
        # Start with a few leaves
        leaves = [_random_hex32() for _ in range(5)]
        leaf_hashes = [mmr_leaf_hash(lr) for lr in leaves]
        peaks = build_peaks(leaf_hashes)

        # Append more leaves one at a time
        for _ in range(10):
            new_leaf = mmr_leaf_hash(_random_hex32())
            peaks = append_peaks(peaks, [new_leaf])

            heights = [p.height for p in peaks]
            for i in range(1, len(heights)):
                assert heights[i] < heights[i - 1], (
                    f"Peak heights not strictly decreasing after append: {heights}"
                )

    def test_build_and_verify_inclusion_proof(self):
        """Roundtrip: build an inclusion proof then verify it."""
        next_roots = [_random_hex32() for _ in range(8)]

        for idx in range(len(next_roots)):
            proof = build_inclusion_proof(next_roots, idx)
            assert verify_inclusion_proof(proof) is True, (
                f"Valid proof for leaf {idx} should verify"
            )

    def test_tampered_proof_rejected(self):
        """A proof with a modified leaf_hash must fail verification."""
        next_roots = [_random_hex32() for _ in range(4)]
        proof = build_inclusion_proof(next_roots, 0)

        # Tamper with the leaf hash
        proof["leaf_hash"] = _random_hex32()
        assert verify_inclusion_proof(proof) is False

    def test_proof_with_wrong_root_rejected(self):
        """A proof whose root was substituted must fail."""
        next_roots = [_random_hex32() for _ in range(4)]
        proof = build_inclusion_proof(next_roots, 1)
        proof["root"] = _random_hex32()
        assert verify_inclusion_proof(proof) is False

    def test_append_peaks_equivalence(self):
        """Incrementally appending leaves must produce the same peaks as building from scratch."""
        leaves = [_random_hex32() for _ in range(12)]
        leaf_hashes = [mmr_leaf_hash(lr) for lr in leaves]

        # Build all at once
        full_peaks = build_peaks(leaf_hashes)

        # Build incrementally
        incremental_peaks = build_peaks(leaf_hashes[:4])
        incremental_peaks = append_peaks(incremental_peaks, leaf_hashes[4:8])
        incremental_peaks = append_peaks(incremental_peaks, leaf_hashes[8:])

        assert len(full_peaks) == len(incremental_peaks)
        for fp, ip in zip(full_peaks, incremental_peaks):
            assert fp.height == ip.height
            assert fp.hash == ip.hash


# ╔══════════════════════════════════════════════════════════════════════════════╗
# ║  ARTIFACTS (CAS) TESTS                                                    ║
# ╚══════════════════════════════════════════════════════════════════════════════╝


class TestCASIntegrity:
    """Bugs #74-#76: CAS hash verification, collision detection, and directory creation."""

    @pytest.fixture(autouse=True)
    def _setup_cas_dirs(self, tmp_path: pathlib.Path):
        """Create a temporary CAS layout for each test."""
        self.repo_root = tmp_path / "repo"
        self.repo_root.mkdir()
        self.store_root = tmp_path / "repo" / "dist" / "artifacts"
        self.store_root.mkdir(parents=True)

    def _store_content(
        self,
        content: bytes,
        artifact_type: str = "testdata",
        digest: str | None = None,
        dest_name: str | None = None,
    ) -> pathlib.Path:
        """Write content to a temp file, then store it via CAS."""
        if digest is None:
            digest = hashlib.sha256(content).hexdigest()
        src = self.repo_root / "tmp_src"
        src.write_bytes(content)
        return store_artifact_file(
            artifact_type=artifact_type,
            digest_sha256=digest,
            src_path=src,
            repo_root=self.repo_root,
            store_root=self.store_root,
        )

    def test_hash_mismatch_warns(self):
        """Bug #74: resolving an artifact whose content hash differs emits a warning."""
        real_content = b"correct content"
        real_digest = hashlib.sha256(real_content).hexdigest()

        # Store with the correct digest
        dest = self._store_content(real_content, digest=real_digest)

        # Now overwrite the file with different content (simulating corruption)
        dest.write_bytes(b"corrupted content")

        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            result = resolve_artifact_by_digest(
                artifact_type="testdata",
                digest_sha256=real_digest,
                repo_root=self.repo_root,
                store_roots=[self.store_root],
            )
            # Should still return the path but issue a warning
            assert result == dest
            matching = [
                x for x in w if "CAS integrity warning" in str(x.message)
            ]
            assert len(matching) >= 1, "Expected a CAS integrity warning"

    def test_hash_collision_detected_on_store(self):
        """Bug #75: collision detection when existing file content does not match expected digest.

        If an artifact already sits at the CAS path but its content hash does not
        match the digest we are about to store, the engine must raise ValueError
        to prevent silent data corruption.
        """
        # Compute a "claimed" digest (from content_a) to use as the CAS key.
        content_a = b"original artifact"
        digest = hashlib.sha256(content_a).hexdigest()

        # Manually place a *different* file at the destination path so the
        # existing content hash will not match the expected digest.
        tdir = self.store_root / "testdata"
        tdir.mkdir(parents=True, exist_ok=True)
        existing_file = tdir / digest
        existing_file.write_bytes(b"rogue content -- wrong hash")

        # Now try to store through the API with overwrite=False.  The engine
        # should detect that the existing file's hash differs from `digest`.
        src_b = self.repo_root / "tmp_src_b"
        src_b.write_bytes(content_a)  # source content is irrelevant; the check is on existing file

        with pytest.raises(ValueError, match="Hash collision detected"):
            store_artifact_file(
                artifact_type="testdata",
                digest_sha256=digest,
                src_path=src_b,
                repo_root=self.repo_root,
                store_root=self.store_root,
                dest_name=digest,
                overwrite=False,
            )

    def test_same_content_same_digest_no_error(self):
        """Storing the same content with the same digest should succeed (idempotent)."""
        content = b"idempotent content"
        digest = hashlib.sha256(content).hexdigest()

        path1 = self._store_content(content, digest=digest)
        path2 = self._store_content(content, digest=digest)
        assert path1 == path2

    def test_parent_directories_created(self):
        """Bug #76: store_artifact_file must create parent directories (os.makedirs)."""
        content = b"nested artifact content"
        digest = hashlib.sha256(content).hexdigest()

        # Use a deeply nested store root that does not yet exist
        deep_root = self.repo_root / "deep" / "nested" / "store"
        src = self.repo_root / "tmp_src_deep"
        src.write_bytes(content)

        result = store_artifact_file(
            artifact_type="testdata",
            digest_sha256=digest,
            src_path=src,
            repo_root=self.repo_root,
            store_root=deep_root,
        )
        assert result.exists()
        assert result.read_bytes() == content

    def test_resolve_nonexistent_raises(self):
        """Resolving a digest that does not exist in any store should raise FileNotFoundError."""
        with pytest.raises(FileNotFoundError):
            resolve_artifact_by_digest(
                artifact_type="testdata",
                digest_sha256="ab" * 32,
                repo_root=self.repo_root,
                store_roots=[self.store_root],
            )

    def test_invalid_digest_raises(self):
        """A digest that is not 64 hex chars must raise ValueError."""
        with pytest.raises(ValueError):
            normalize_digest("not-a-hex-digest")

    def test_invalid_artifact_type_raises(self):
        """An artifact type with forbidden characters must raise ValueError."""
        with pytest.raises(ValueError):
            normalize_artifact_type("INVALID/TYPE")


# ╔══════════════════════════════════════════════════════════════════════════════╗
# ║  VC VALIDATION TESTS                                                      ║
# ╚══════════════════════════════════════════════════════════════════════════════╝


class TestVCValidation:
    """Bug #78: VCs with future issuance dates must be rejected."""

    def test_future_issuance_date_rejected(self):
        """Bug #78: a VC whose issuanceDate is more than 60 seconds in the future is invalid."""
        future_time = (datetime.now(timezone.utc) + timedelta(hours=1)).strftime(
            "%Y-%m-%dT%H:%M:%SZ"
        )
        credential = {
            "issuanceDate": future_time,
        }
        errors = validate_credential(credential)
        future_errors = [e for e in errors if "future" in e.lower()]
        assert len(future_errors) >= 1, (
            f"Expected a future-issuance error, got: {errors}"
        )

    def test_past_issuance_date_accepted(self):
        """A VC issued in the past should not produce issuance-related errors."""
        past_time = (datetime.now(timezone.utc) - timedelta(hours=1)).strftime(
            "%Y-%m-%dT%H:%M:%SZ"
        )
        credential = {
            "issuanceDate": past_time,
        }
        errors = validate_credential(credential)
        future_errors = [e for e in errors if "future" in e.lower()]
        assert len(future_errors) == 0, f"Unexpected future-issuance error: {errors}"

    def test_expired_credential_detected(self):
        """A VC whose expirationDate is in the past should produce an error."""
        past_time = (datetime.now(timezone.utc) - timedelta(days=1)).strftime(
            "%Y-%m-%dT%H:%M:%SZ"
        )
        credential = {
            "issuanceDate": (datetime.now(timezone.utc) - timedelta(days=2)).strftime(
                "%Y-%m-%dT%H:%M:%SZ"
            ),
            "expirationDate": past_time,
        }
        errors = validate_credential(credential)
        expiry_errors = [e for e in errors if "expired" in e.lower()]
        assert len(expiry_errors) >= 1, f"Expected an expiry error, got: {errors}"

    def test_no_expiration_date_accepted(self):
        """A VC without expirationDate (never expires) should not produce expiry errors."""
        credential = {
            "issuanceDate": (datetime.now(timezone.utc) - timedelta(hours=1)).strftime(
                "%Y-%m-%dT%H:%M:%SZ"
            ),
        }
        errors = validate_credential(credential)
        expiry_errors = [e for e in errors if "expired" in e.lower()]
        assert len(expiry_errors) == 0

    def test_invalid_issuance_date_format(self):
        """A malformed issuanceDate string should produce an error."""
        credential = {
            "issuanceDate": "not-a-date",
        }
        errors = validate_credential(credential)
        assert len(errors) >= 1
        assert any("invalid" in e.lower() or "issuancedate" in e.lower() for e in errors)

    def test_within_clock_skew_accepted(self):
        """A VC issued within 60 seconds in the future should be accepted (clock skew)."""
        # 30 seconds in the future should be within the 60s tolerance
        skew_time = (datetime.now(timezone.utc) + timedelta(seconds=30)).strftime(
            "%Y-%m-%dT%H:%M:%SZ"
        )
        credential = {
            "issuanceDate": skew_time,
        }
        errors = validate_credential(credential)
        future_errors = [e for e in errors if "future" in e.lower()]
        assert len(future_errors) == 0, (
            f"Issuance within clock-skew tolerance should not fail: {errors}"
        )


# ╔══════════════════════════════════════════════════════════════════════════════╗
# ║  MASS PRIMITIVES (HOLDER VALIDATION) TESTS                                ║
# ╚══════════════════════════════════════════════════════════════════════════════╝


class TestHolderValidation:
    """Bug #101: holder identifiers must be validated as non-empty."""

    def _make_asset_with_balance(
        self,
        holder: str = "alice",
        balance: int = 1000,
    ) -> SmartAsset:
        return _make_smart_asset(holders={holder: balance})

    def test_empty_holder_rejected_on_transfer(self):
        """Bug #101: transferring to an empty holder string must raise ValueError."""
        asset = self._make_asset_with_balance("alice", 1000)
        envelope = TransitionEnvelope(
            asset_id=asset.asset_id,
            seq=0,
            kind=TransitionKind.TRANSFER,
            effective_time=datetime.now(timezone.utc).isoformat(),
            params={"from": "alice", "to": "", "amount": 100},
        )
        with pytest.raises(ValueError, match="[Mm]issing.*holder"):
            asset.transition(envelope)

    def test_empty_from_holder_rejected(self):
        """Bug #101: transferring from an empty holder string must raise ValueError."""
        asset = self._make_asset_with_balance("alice", 1000)
        envelope = TransitionEnvelope(
            asset_id=asset.asset_id,
            seq=0,
            kind=TransitionKind.TRANSFER,
            effective_time=datetime.now(timezone.utc).isoformat(),
            params={"from": "", "to": "bob", "amount": 100},
        )
        with pytest.raises(ValueError, match="[Mm]issing.*holder"):
            asset.transition(envelope)

    def test_none_holder_rejected_on_mint(self):
        """Bug #101: minting to a None holder must raise ValueError."""
        asset = self._make_asset_with_balance("alice", 1000)
        envelope = TransitionEnvelope(
            asset_id=asset.asset_id,
            seq=0,
            kind=TransitionKind.MINT,
            effective_time=datetime.now(timezone.utc).isoformat(),
            params={"to": None, "amount": 500},
        )
        with pytest.raises(ValueError, match="[Mm]issing.*holder"):
            asset.transition(envelope)

    def test_non_did_holder_accepted(self):
        """Non-DID holders like 'bob' must be accepted for backwards compatibility."""
        asset = self._make_asset_with_balance("alice", 1000)
        envelope = TransitionEnvelope(
            asset_id=asset.asset_id,
            seq=0,
            kind=TransitionKind.TRANSFER,
            effective_time=datetime.now(timezone.utc).isoformat(),
            params={"from": "alice", "to": "bob", "amount": 100},
        )
        # Should not raise
        receipt = asset.transition(envelope)
        assert receipt.seq == 0
        assert asset.state["balances"]["bob"] == Decimal("100")
        assert asset.state["balances"]["alice"] == Decimal("900")

    def test_did_holder_accepted(self):
        """DID-formatted holders must also be accepted."""
        asset = _make_smart_asset(holders={"did:key:z6MkAlice": 500})
        envelope = TransitionEnvelope(
            asset_id=asset.asset_id,
            seq=0,
            kind=TransitionKind.TRANSFER,
            effective_time=datetime.now(timezone.utc).isoformat(),
            params={
                "from": "did:key:z6MkAlice",
                "to": "did:key:z6MkBob",
                "amount": 200,
            },
        )
        receipt = asset.transition(envelope)
        assert receipt.seq == 0
        assert asset.state["balances"]["did:key:z6MkBob"] == Decimal("200")


# ╔══════════════════════════════════════════════════════════════════════════════╗
# ║  SMART ASSET TESTS                                                        ║
# ╚══════════════════════════════════════════════════════════════════════════════╝


class TestSmartAssetGenesis:
    """Tests for smart_asset.py genesis and state root utilities."""

    def test_asset_id_deterministic(self):
        """asset_id_from_genesis must be deterministic for identical inputs."""
        genesis = {
            "type": "SmartAssetGenesis",
            "stack_spec_version": "0.4.44",
            "asset_name": "TestCoin",
            "asset_class": "token",
            "created_at": "2025-01-01T00:00:00Z",
        }
        id1 = asset_id_from_genesis(genesis)
        id2 = asset_id_from_genesis(genesis)
        assert id1 == id2
        assert len(id1) == 64  # sha256 hex

    def test_asset_id_excludes_asset_id_field(self):
        """The asset_id field itself must be excluded from the digest computation."""
        genesis = {
            "type": "SmartAssetGenesis",
            "stack_spec_version": "0.4.44",
            "asset_name": "TestCoin",
            "asset_class": "token",
            "created_at": "2025-01-01T00:00:00Z",
        }
        id_without = asset_id_from_genesis(genesis)

        genesis_with = dict(genesis)
        genesis_with["asset_id"] = "deadbeef" * 8
        id_with = asset_id_from_genesis(genesis_with)

        assert id_without == id_with, (
            "asset_id field must be excluded from digest computation"
        )

    def test_build_genesis_populates_asset_id(self):
        """build_genesis must populate the asset_id convenience field."""
        g = build_genesis(
            stack_spec_version="0.4.44",
            asset_name="TestCoin",
            asset_class="token",
            created_at="2025-01-01T00:00:00Z",
        )
        assert "asset_id" in g
        assert len(g["asset_id"]) == 64

    def test_state_root_deterministic(self):
        """state_root_from_state must be deterministic."""
        state = {"balances": {"alice": "1000"}, "total_supply": "1000"}
        r1 = state_root_from_state(state)
        r2 = state_root_from_state(state)
        assert r1 == r2
        assert len(r1) == 64


class TestReceiptChainContinuity:
    """Tests for verify_receipt_chain_continuity in smart_asset.py."""

    def _make_chain(self, n: int) -> List[Dict[str, Any]]:
        """Build a valid chain of n receipts with proper hash linkage."""
        from tools.smart_asset import sha256_hex
        from tools.lawpack import jcs_canonicalize

        chain: List[Dict[str, Any]] = []
        for i in range(n):
            receipt: Dict[str, Any] = {
                "seq": i,
                "data": f"receipt-{i}",
            }
            if i > 0:
                prev_hash = sha256_hex(jcs_canonicalize(chain[i - 1]))
                receipt["previous_hash"] = prev_hash
            chain.append(receipt)
        return chain

    def test_valid_chain_no_errors(self):
        """A properly linked chain should produce no errors."""
        chain = self._make_chain(5)
        errors = verify_receipt_chain_continuity(chain)
        assert errors == []

    def test_broken_chain_detected(self):
        """Tampering with a receipt should produce a chain-break error."""
        chain = self._make_chain(5)
        # Tamper with receipt[2]
        chain[2]["data"] = "tampered"
        errors = verify_receipt_chain_continuity(chain)
        assert len(errors) >= 1
        assert any("chain break" in e.lower() or "does not match" in e.lower() for e in errors)

    def test_empty_chain_no_errors(self):
        """An empty chain should produce no errors."""
        errors = verify_receipt_chain_continuity([])
        assert errors == []

    def test_single_receipt_no_errors(self):
        """A single-receipt chain should produce no errors."""
        chain = self._make_chain(1)
        errors = verify_receipt_chain_continuity(chain)
        assert errors == []


# ╔══════════════════════════════════════════════════════════════════════════════╗
# ║  NETTING PLAN SERIALIZATION ROUNDTRIP                                     ║
# ╚══════════════════════════════════════════════════════════════════════════════╝


class TestNettingPlanSerialization:
    """Ensure settlement plans serialize correctly (to_dict covers all fields)."""

    def test_plan_to_dict_roundtrip(self):
        party_a = _make_party("A")
        party_b = _make_party("B")
        usd = _make_currency("USD")

        obligations = [
            Obligation("obl-1", "c1", party_a, party_b, usd, Decimal("500")),
            Obligation("obl-2", "c1", party_b, party_a, usd, Decimal("300")),
        ]
        rails = [_make_rail("rail:swift", currencies={"USD"}, priority=1)]
        engine = NettingEngine(obligations, rails)
        plan = engine.compute_plan("plan:roundtrip")

        d = plan.to_dict()
        assert d["plan_id"] == "plan:roundtrip"
        assert d["netting_method"] == "multi-corridor-greedy-v1"
        assert isinstance(d["reduction_ratio"], dict)
        for v in d["reduction_ratio"].values():
            # Serialized as string
            assert isinstance(v, str)
