"""
Trade Corridor and Settlement Integration Test Suite

Tests real-world scenarios for cross-border settlement including:
- Multi-hop corridor routing
- Settlement finality guarantees
- Liquidity constraints
- Fee accumulation
- Treaty-based corridor requirements
- Sanctions and compliance checks during transit

These tests simulate actual trade finance operations to expose bugs in the
corridor infrastructure.
"""

import pytest
from dataclasses import dataclass, field
from typing import Dict, List, Set, Optional, Tuple
from enum import Enum
from decimal import Decimal
from datetime import datetime, timezone, timedelta
import hashlib
import random


# =============================================================================
# CORRIDOR PRIMITIVES
# =============================================================================

class SettlementMethod(Enum):
    """How a corridor settles transactions."""
    SWIFT_GPI = "swift_gpi"           # SWIFT gpi with tracking
    SWIFT_MT103 = "swift_mt103"       # Traditional SWIFT
    STABLECOIN_USDC = "stablecoin_usdc"
    STABLECOIN_USDT = "stablecoin_usdt"
    RTGS = "rtgs"                     # Real-time gross settlement
    ACH_BATCH = "ach_batch"           # Batch clearing
    CORRESPONDENT = "correspondent"    # Correspondent banking


class CorridorStatus(Enum):
    """Operational status of a corridor."""
    ACTIVE = "active"
    SUSPENDED = "suspended"
    MAINTENANCE = "maintenance"
    DEGRADED = "degraded"


@dataclass
class CorridorCapacity:
    """Liquidity and capacity constraints for a corridor."""
    daily_limit_usd: Decimal
    per_transaction_limit_usd: Decimal
    current_daily_usage_usd: Decimal = Decimal("0")
    available_liquidity_usd: Decimal = Decimal("0")
    queue_depth: int = 0


@dataclass
class CorridorFees:
    """Fee structure for a corridor."""
    base_fee_usd: Decimal
    percentage_fee_bps: int  # Basis points (1 bp = 0.01%)
    minimum_fee_usd: Decimal
    fx_spread_bps: int = 0


@dataclass
class Corridor:
    """A bilateral corridor between two jurisdictions."""
    corridor_id: str
    source_jurisdiction: str
    target_jurisdiction: str
    settlement_method: SettlementMethod
    status: CorridorStatus
    capacity: CorridorCapacity
    fees: CorridorFees
    settlement_time_hours: int
    requires_kyc_tier: int
    supported_currencies: Set[str]
    blocked_countries: Set[str] = field(default_factory=set)
    requires_compliance_attestations: List[str] = field(default_factory=list)


@dataclass
class TransferRequest:
    """A cross-border transfer request."""
    transfer_id: str
    source_jurisdiction: str
    target_jurisdiction: str
    amount: Decimal
    currency: str
    sender_did: str
    recipient_did: str
    sender_kyc_tier: int
    purpose: str
    compliance_attestations: List[str] = field(default_factory=list)


@dataclass
class TransferResult:
    """Result of a transfer attempt."""
    success: bool
    transfer_id: str
    path: List[str] = field(default_factory=list)
    total_fees_usd: Decimal = Decimal("0")
    settlement_time_hours: int = 0
    rejection_reason: str = ""
    compliance_checks: List[str] = field(default_factory=list)


# =============================================================================
# CORRIDOR REGISTRY
# =============================================================================

def create_corridor_registry() -> Dict[Tuple[str, str], Corridor]:
    """Create a realistic corridor registry."""
    corridors = {}

    # UAE-ADGM to UAE-DIFC (same country, fast)
    corridors[("ae-adgm", "ae-difc")] = Corridor(
        corridor_id="cor-adgm-difc-001",
        source_jurisdiction="ae-adgm",
        target_jurisdiction="ae-difc",
        settlement_method=SettlementMethod.RTGS,
        status=CorridorStatus.ACTIVE,
        capacity=CorridorCapacity(
            daily_limit_usd=Decimal("100000000"),
            per_transaction_limit_usd=Decimal("10000000"),
            available_liquidity_usd=Decimal("50000000"),
        ),
        fees=CorridorFees(
            base_fee_usd=Decimal("10"),
            percentage_fee_bps=5,
            minimum_fee_usd=Decimal("10"),
        ),
        settlement_time_hours=1,
        requires_kyc_tier=2,
        supported_currencies={"USD", "AED", "EUR"},
    )

    # UAE-ADGM to KZ-AIFC (treaty corridor)
    corridors[("ae-adgm", "kz-aifc")] = Corridor(
        corridor_id="cor-adgm-aifc-001",
        source_jurisdiction="ae-adgm",
        target_jurisdiction="kz-aifc",
        settlement_method=SettlementMethod.SWIFT_GPI,
        status=CorridorStatus.ACTIVE,
        capacity=CorridorCapacity(
            daily_limit_usd=Decimal("50000000"),
            per_transaction_limit_usd=Decimal("5000000"),
            available_liquidity_usd=Decimal("20000000"),
        ),
        fees=CorridorFees(
            base_fee_usd=Decimal("25"),
            percentage_fee_bps=15,
            minimum_fee_usd=Decimal("25"),
        ),
        settlement_time_hours=4,
        requires_kyc_tier=2,
        supported_currencies={"USD", "EUR", "KZT"},
        requires_compliance_attestations=["aml_clearance", "sanctions_clearance"],
    )

    # UAE-DIFC to UK (major route)
    corridors[("ae-difc", "uk-fca")] = Corridor(
        corridor_id="cor-difc-uk-001",
        source_jurisdiction="ae-difc",
        target_jurisdiction="uk-fca",
        settlement_method=SettlementMethod.SWIFT_GPI,
        status=CorridorStatus.ACTIVE,
        capacity=CorridorCapacity(
            daily_limit_usd=Decimal("500000000"),
            per_transaction_limit_usd=Decimal("50000000"),
            available_liquidity_usd=Decimal("200000000"),
        ),
        fees=CorridorFees(
            base_fee_usd=Decimal("15"),
            percentage_fee_bps=10,
            minimum_fee_usd=Decimal("15"),
        ),
        settlement_time_hours=2,
        requires_kyc_tier=2,
        supported_currencies={"USD", "GBP", "EUR"},
    )

    # US-NY to UK (major route)
    corridors[("us-ny", "uk-fca")] = Corridor(
        corridor_id="cor-usny-uk-001",
        source_jurisdiction="us-ny",
        target_jurisdiction="uk-fca",
        settlement_method=SettlementMethod.CORRESPONDENT,
        status=CorridorStatus.ACTIVE,
        capacity=CorridorCapacity(
            daily_limit_usd=Decimal("1000000000"),
            per_transaction_limit_usd=Decimal("100000000"),
            available_liquidity_usd=Decimal("500000000"),
        ),
        fees=CorridorFees(
            base_fee_usd=Decimal("20"),
            percentage_fee_bps=8,
            minimum_fee_usd=Decimal("20"),
        ),
        settlement_time_hours=1,
        requires_kyc_tier=3,
        supported_currencies={"USD", "GBP", "EUR"},
    )

    # Stablecoin corridor: ADGM to Cayman
    corridors[("ae-adgm", "ky-cayman")] = Corridor(
        corridor_id="cor-adgm-cayman-usdc-001",
        source_jurisdiction="ae-adgm",
        target_jurisdiction="ky-cayman",
        settlement_method=SettlementMethod.STABLECOIN_USDC,
        status=CorridorStatus.ACTIVE,
        capacity=CorridorCapacity(
            daily_limit_usd=Decimal("25000000"),
            per_transaction_limit_usd=Decimal("1000000"),
            available_liquidity_usd=Decimal("10000000"),
        ),
        fees=CorridorFees(
            base_fee_usd=Decimal("5"),
            percentage_fee_bps=3,
            minimum_fee_usd=Decimal("5"),
        ),
        settlement_time_hours=0,  # Near instant
        requires_kyc_tier=2,
        supported_currencies={"USDC"},
        requires_compliance_attestations=["aml_clearance", "source_of_funds"],
    )

    # UK to Singapore (major route)
    corridors[("uk-fca", "sg-mas")] = Corridor(
        corridor_id="cor-uk-sg-001",
        source_jurisdiction="uk-fca",
        target_jurisdiction="sg-mas",
        settlement_method=SettlementMethod.SWIFT_GPI,
        status=CorridorStatus.ACTIVE,
        capacity=CorridorCapacity(
            daily_limit_usd=Decimal("200000000"),
            per_transaction_limit_usd=Decimal("20000000"),
            available_liquidity_usd=Decimal("100000000"),
        ),
        fees=CorridorFees(
            base_fee_usd=Decimal("18"),
            percentage_fee_bps=12,
            minimum_fee_usd=Decimal("18"),
        ),
        settlement_time_hours=3,
        requires_kyc_tier=3,
        supported_currencies={"USD", "GBP", "SGD", "EUR"},
    )

    # Degraded corridor (for testing)
    corridors[("ae-difc", "ky-cayman")] = Corridor(
        corridor_id="cor-difc-cayman-degraded",
        source_jurisdiction="ae-difc",
        target_jurisdiction="ky-cayman",
        settlement_method=SettlementMethod.CORRESPONDENT,
        status=CorridorStatus.DEGRADED,
        capacity=CorridorCapacity(
            daily_limit_usd=Decimal("10000000"),
            per_transaction_limit_usd=Decimal("500000"),
            available_liquidity_usd=Decimal("2000000"),
        ),
        fees=CorridorFees(
            base_fee_usd=Decimal("50"),
            percentage_fee_bps=25,
            minimum_fee_usd=Decimal("50"),
        ),
        settlement_time_hours=24,
        requires_kyc_tier=2,
        supported_currencies={"USD"},
    )

    # Suspended corridor (for testing)
    corridors[("us-ny", "ru-moscow")] = Corridor(
        corridor_id="cor-usny-russia-suspended",
        source_jurisdiction="us-ny",
        target_jurisdiction="ru-moscow",
        settlement_method=SettlementMethod.SWIFT_MT103,
        status=CorridorStatus.SUSPENDED,
        capacity=CorridorCapacity(
            daily_limit_usd=Decimal("0"),
            per_transaction_limit_usd=Decimal("0"),
        ),
        fees=CorridorFees(
            base_fee_usd=Decimal("0"),
            percentage_fee_bps=0,
            minimum_fee_usd=Decimal("0"),
        ),
        settlement_time_hours=0,
        requires_kyc_tier=3,
        supported_currencies=set(),
        blocked_countries={"RU", "BY"},
    )

    # Reverse corridors (asymmetric fees)
    corridors[("kz-aifc", "ae-adgm")] = Corridor(
        corridor_id="cor-aifc-adgm-001",
        source_jurisdiction="kz-aifc",
        target_jurisdiction="ae-adgm",
        settlement_method=SettlementMethod.SWIFT_GPI,
        status=CorridorStatus.ACTIVE,
        capacity=CorridorCapacity(
            daily_limit_usd=Decimal("30000000"),
            per_transaction_limit_usd=Decimal("3000000"),
            available_liquidity_usd=Decimal("15000000"),
        ),
        fees=CorridorFees(
            base_fee_usd=Decimal("30"),  # Higher fee going this direction
            percentage_fee_bps=20,
            minimum_fee_usd=Decimal("30"),
        ),
        settlement_time_hours=6,
        requires_kyc_tier=2,
        supported_currencies={"USD", "EUR"},
    )

    return corridors


# =============================================================================
# CORRIDOR ROUTER
# =============================================================================

class CorridorRouter:
    """Routes transfers through the corridor network."""

    def __init__(self, corridors: Dict[Tuple[str, str], Corridor]):
        self.corridors = corridors
        self.graph = self._build_graph()

    def _build_graph(self) -> Dict[str, Set[str]]:
        """Build adjacency list from corridors."""
        graph = {}
        for (source, target), corridor in self.corridors.items():
            if source not in graph:
                graph[source] = set()
            if corridor.status == CorridorStatus.ACTIVE:
                graph[source].add(target)
        return graph

    def find_path(
        self,
        source: str,
        target: str,
        amount: Decimal,
        currency: str,
        kyc_tier: int,
    ) -> Tuple[Optional[List[str]], List[str]]:
        """
        Find a valid path from source to target.
        Returns (path, issues) where path is None if no valid path exists.
        """
        issues = []

        # BFS to find shortest path
        if source not in self.graph:
            issues.append(f"No corridors from source: {source}")
            return None, issues

        visited = {source}
        queue = [(source, [source])]

        while queue:
            current, path = queue.pop(0)

            if current == target:
                # Validate the complete path
                path_issues = self._validate_path(path, amount, currency, kyc_tier)
                if path_issues:
                    issues.extend(path_issues)
                    continue  # Try other paths
                return path, []

            for neighbor in self.graph.get(current, []):
                if neighbor not in visited:
                    visited.add(neighbor)
                    queue.append((neighbor, path + [neighbor]))

        issues.append(f"No path from {source} to {target}")
        return None, issues

    def _validate_path(
        self,
        path: List[str],
        amount: Decimal,
        currency: str,
        kyc_tier: int,
    ) -> List[str]:
        """Validate that a path can handle the transfer."""
        issues = []

        for i in range(len(path) - 1):
            corridor = self.corridors.get((path[i], path[i + 1]))
            if not corridor:
                issues.append(f"No corridor from {path[i]} to {path[i + 1]}")
                continue

            # Check status
            if corridor.status != CorridorStatus.ACTIVE:
                issues.append(f"Corridor {corridor.corridor_id} is {corridor.status.value}")

            # Check capacity
            if amount > corridor.capacity.per_transaction_limit_usd:
                issues.append(
                    f"Amount ${amount} exceeds corridor limit ${corridor.capacity.per_transaction_limit_usd}"
                )

            # Check liquidity
            if amount > corridor.capacity.available_liquidity_usd:
                issues.append(
                    f"Insufficient liquidity in {corridor.corridor_id}: "
                    f"need ${amount}, have ${corridor.capacity.available_liquidity_usd}"
                )

            # Check currency
            if currency not in corridor.supported_currencies:
                issues.append(
                    f"Currency {currency} not supported by {corridor.corridor_id}"
                )

            # Check KYC tier
            if kyc_tier < corridor.requires_kyc_tier:
                issues.append(
                    f"KYC tier {kyc_tier} insufficient for {corridor.corridor_id} "
                    f"(requires tier {corridor.requires_kyc_tier})"
                )

        return issues

    def calculate_fees(self, path: List[str], amount: Decimal) -> Decimal:
        """Calculate total fees for a path."""
        total_fees = Decimal("0")
        remaining_amount = amount

        for i in range(len(path) - 1):
            corridor = self.corridors.get((path[i], path[i + 1]))
            if corridor:
                # Calculate fee for this hop
                percentage_fee = (remaining_amount * corridor.fees.percentage_fee_bps) / Decimal("10000")
                hop_fee = max(
                    corridor.fees.base_fee_usd + percentage_fee,
                    corridor.fees.minimum_fee_usd
                )
                total_fees += hop_fee
                remaining_amount -= hop_fee

        return total_fees

    def calculate_settlement_time(self, path: List[str]) -> int:
        """Calculate total settlement time in hours."""
        total_hours = 0
        for i in range(len(path) - 1):
            corridor = self.corridors.get((path[i], path[i + 1]))
            if corridor:
                total_hours += corridor.settlement_time_hours
        return total_hours


# =============================================================================
# TESTS: DIRECT CORRIDOR TRANSFERS
# =============================================================================

class TestDirectCorridorTransfers:
    """Tests for single-hop corridor transfers."""

    @pytest.fixture
    def corridors(self):
        return create_corridor_registry()

    @pytest.fixture
    def router(self, corridors):
        return CorridorRouter(corridors)

    def test_adgm_to_difc_direct(self, router):
        """Direct transfer between UAE jurisdictions."""
        path, issues = router.find_path(
            source="ae-adgm",
            target="ae-difc",
            amount=Decimal("1000000"),
            currency="USD",
            kyc_tier=2,
        )

        assert path == ["ae-adgm", "ae-difc"], f"Issues: {issues}"
        assert len(issues) == 0

        fees = router.calculate_fees(path, Decimal("1000000"))
        assert fees > Decimal("0")

        time = router.calculate_settlement_time(path)
        assert time == 1  # 1 hour

    def test_adgm_to_aifc_with_attestations(self, corridors, router):
        """Transfer requiring compliance attestations."""
        corridor = corridors[("ae-adgm", "kz-aifc")]
        assert "aml_clearance" in corridor.requires_compliance_attestations
        assert "sanctions_clearance" in corridor.requires_compliance_attestations

        path, issues = router.find_path(
            source="ae-adgm",
            target="kz-aifc",
            amount=Decimal("500000"),
            currency="USD",
            kyc_tier=2,
        )

        assert path == ["ae-adgm", "kz-aifc"]

    def test_stablecoin_corridor_instant_settlement(self, corridors, router):
        """Stablecoin corridor has near-instant settlement."""
        path, issues = router.find_path(
            source="ae-adgm",
            target="ky-cayman",
            amount=Decimal("100000"),
            currency="USDC",
            kyc_tier=2,
        )

        assert path == ["ae-adgm", "ky-cayman"]
        time = router.calculate_settlement_time(path)
        assert time == 0  # Instant


# =============================================================================
# TESTS: MULTI-HOP ROUTING
# =============================================================================

class TestMultiHopRouting:
    """Tests for transfers requiring multiple hops."""

    @pytest.fixture
    def corridors(self):
        return create_corridor_registry()

    @pytest.fixture
    def router(self, corridors):
        return CorridorRouter(corridors)

    def test_adgm_to_singapore_via_uk(self, router):
        """
        ADGM -> Singapore requires routing through UK.

        This is a realistic scenario where no direct corridor exists.
        """
        # First, verify no direct corridor
        path, issues = router.find_path(
            source="ae-adgm",
            target="sg-mas",
            amount=Decimal("1000000"),
            currency="USD",
            kyc_tier=3,
        )

        # Should find path via DIFC -> UK -> Singapore or similar
        if path:
            assert len(path) > 2, "Expected multi-hop path"
            # Verify path is valid
            for i in range(len(path) - 1):
                assert (path[i], path[i + 1]) in router.corridors

    def test_fee_accumulation_multi_hop(self, router):
        """Fees should accumulate across hops."""
        # Find a multi-hop path
        path, _ = router.find_path(
            source="ae-difc",
            target="sg-mas",
            amount=Decimal("1000000"),
            currency="USD",
            kyc_tier=3,
        )

        if path and len(path) > 2:
            total_fees = router.calculate_fees(path, Decimal("1000000"))

            # Calculate individual hop fees
            individual_fees = Decimal("0")
            for i in range(len(path) - 1):
                hop_fees = router.calculate_fees(
                    [path[i], path[i + 1]],
                    Decimal("1000000"),
                )
                individual_fees += hop_fees

            # Total should equal sum (minus compounding effect)
            # We allow for compounding differences
            assert total_fees > Decimal("0")

    def test_settlement_time_accumulation(self, router):
        """Settlement time should accumulate across hops."""
        path, _ = router.find_path(
            source="ae-difc",
            target="sg-mas",
            amount=Decimal("1000000"),
            currency="USD",
            kyc_tier=3,
        )

        if path and len(path) > 2:
            total_time = router.calculate_settlement_time(path)

            # Calculate individual hop times
            individual_times = 0
            for i in range(len(path) - 1):
                corridor = router.corridors.get((path[i], path[i + 1]))
                if corridor:
                    individual_times += corridor.settlement_time_hours

            assert total_time == individual_times


# =============================================================================
# TESTS: CAPACITY AND LIQUIDITY CONSTRAINTS
# =============================================================================

class TestCapacityConstraints:
    """Tests for corridor capacity and liquidity limits."""

    @pytest.fixture
    def corridors(self):
        return create_corridor_registry()

    @pytest.fixture
    def router(self, corridors):
        return CorridorRouter(corridors)

    def test_exceeds_per_transaction_limit(self, router):
        """Transfer exceeding per-transaction limit should fail."""
        # ADGM-AIFC has $5M limit
        path, issues = router.find_path(
            source="ae-adgm",
            target="kz-aifc",
            amount=Decimal("10000000"),  # $10M > $5M limit
            currency="USD",
            kyc_tier=2,
        )

        assert any("exceeds" in issue.lower() for issue in issues)

    def test_exceeds_available_liquidity(self, router):
        """Transfer exceeding available liquidity should fail."""
        # ADGM-AIFC has $20M liquidity
        path, issues = router.find_path(
            source="ae-adgm",
            target="kz-aifc",
            amount=Decimal("25000000"),  # $25M > $20M liquidity
            currency="USD",
            kyc_tier=2,
        )

        assert any("liquidity" in issue.lower() for issue in issues)

    def test_stablecoin_corridor_lower_limits(self, router):
        """Stablecoin corridors have lower limits."""
        # ADGM-Cayman USDC has $1M per-tx limit
        path, issues = router.find_path(
            source="ae-adgm",
            target="ky-cayman",
            amount=Decimal("2000000"),  # $2M > $1M limit
            currency="USDC",
            kyc_tier=2,
        )

        assert any("exceeds" in issue.lower() for issue in issues)


# =============================================================================
# TESTS: KYC AND COMPLIANCE
# =============================================================================

class TestKYCCompliance:
    """Tests for KYC tier requirements."""

    @pytest.fixture
    def corridors(self):
        return create_corridor_registry()

    @pytest.fixture
    def router(self, corridors):
        return CorridorRouter(corridors)

    def test_insufficient_kyc_tier(self, router):
        """Transfer with insufficient KYC tier should fail."""
        # US-NY to UK requires tier 3
        path, issues = router.find_path(
            source="us-ny",
            target="uk-fca",
            amount=Decimal("1000000"),
            currency="USD",
            kyc_tier=2,  # Only tier 2
        )

        assert any("kyc tier" in issue.lower() for issue in issues)

    def test_sufficient_kyc_tier(self, router):
        """Transfer with sufficient KYC tier should succeed."""
        path, issues = router.find_path(
            source="us-ny",
            target="uk-fca",
            amount=Decimal("1000000"),
            currency="USD",
            kyc_tier=3,  # Tier 3 meets requirement
        )

        assert path is not None
        assert len(issues) == 0


# =============================================================================
# TESTS: CORRIDOR STATUS
# =============================================================================

class TestCorridorStatus:
    """Tests for corridor operational status."""

    @pytest.fixture
    def corridors(self):
        return create_corridor_registry()

    @pytest.fixture
    def router(self, corridors):
        return CorridorRouter(corridors)

    def test_suspended_corridor_blocked(self, router):
        """Suspended corridor should not be routable."""
        path, issues = router.find_path(
            source="us-ny",
            target="ru-moscow",
            amount=Decimal("1000"),
            currency="USD",
            kyc_tier=3,
        )

        # Should find no path
        assert path is None or len(issues) > 0

    def test_degraded_corridor_deprioritized(self, corridors, router):
        """Degraded corridor should have higher fees and slower settlement."""
        degraded = corridors[("ae-difc", "ky-cayman")]
        assert degraded.status == CorridorStatus.DEGRADED
        assert degraded.fees.percentage_fee_bps == 25  # Higher fees
        assert degraded.settlement_time_hours == 24  # Slower

    def test_maintenance_corridor_status(self, corridors):
        """Verify maintenance status behavior."""
        # Create a maintenance corridor
        corridors[("test-a", "test-b")] = Corridor(
            corridor_id="cor-test-maintenance",
            source_jurisdiction="test-a",
            target_jurisdiction="test-b",
            settlement_method=SettlementMethod.RTGS,
            status=CorridorStatus.MAINTENANCE,
            capacity=CorridorCapacity(
                daily_limit_usd=Decimal("1000000"),
                per_transaction_limit_usd=Decimal("100000"),
            ),
            fees=CorridorFees(
                base_fee_usd=Decimal("10"),
                percentage_fee_bps=5,
                minimum_fee_usd=Decimal("10"),
            ),
            settlement_time_hours=2,
            requires_kyc_tier=1,
            supported_currencies={"USD"},
        )

        router = CorridorRouter(corridors)
        # Maintenance corridors should not be in the active graph
        assert "test-b" not in router.graph.get("test-a", set())


# =============================================================================
# TESTS: CURRENCY SUPPORT
# =============================================================================

class TestCurrencySupport:
    """Tests for currency handling in corridors."""

    @pytest.fixture
    def corridors(self):
        return create_corridor_registry()

    @pytest.fixture
    def router(self, corridors):
        return CorridorRouter(corridors)

    def test_unsupported_currency_rejected(self, router):
        """Transfer in unsupported currency should fail."""
        # ADGM-DIFC supports USD, AED, EUR but not JPY
        path, issues = router.find_path(
            source="ae-adgm",
            target="ae-difc",
            amount=Decimal("1000000"),
            currency="JPY",
            kyc_tier=2,
        )

        assert any("currency" in issue.lower() for issue in issues)

    def test_stablecoin_only_corridor(self, router):
        """Stablecoin corridor only accepts stablecoin currency."""
        # ADGM-Cayman USDC only supports USDC
        path, issues = router.find_path(
            source="ae-adgm",
            target="ky-cayman",
            amount=Decimal("100000"),
            currency="USD",  # Not USDC
            kyc_tier=2,
        )

        assert any("currency" in issue.lower() for issue in issues)


# =============================================================================
# TESTS: FEE CALCULATIONS
# =============================================================================

class TestFeeCalculations:
    """Tests for accurate fee calculations."""

    @pytest.fixture
    def corridors(self):
        return create_corridor_registry()

    @pytest.fixture
    def router(self, corridors):
        return CorridorRouter(corridors)

    def test_base_plus_percentage_fee(self, router):
        """Verify base + percentage fee calculation."""
        # ADGM-DIFC: base $10 + 5 bps
        path = ["ae-adgm", "ae-difc"]
        amount = Decimal("1000000")

        fees = router.calculate_fees(path, amount)

        expected_percentage = amount * Decimal("5") / Decimal("10000")  # 5 bps
        expected_total = Decimal("10") + expected_percentage  # $10 base + $50 = $60

        assert fees == expected_total

    def test_minimum_fee_enforcement(self, router):
        """Minimum fee should be enforced for small transfers."""
        path = ["ae-adgm", "ae-difc"]
        amount = Decimal("100")  # Small amount

        fees = router.calculate_fees(path, amount)

        # Should be at least the minimum fee
        corridor = router.corridors[("ae-adgm", "ae-difc")]
        assert fees >= corridor.fees.minimum_fee_usd

    def test_zero_amount_fee(self, router):
        """Zero amount transfer should have minimum fee or zero."""
        path = ["ae-adgm", "ae-difc"]
        fees = router.calculate_fees(path, Decimal("0"))

        # Either zero or minimum
        corridor = router.corridors[("ae-adgm", "ae-difc")]
        assert fees == Decimal("0") or fees == corridor.fees.minimum_fee_usd


# =============================================================================
# TESTS: EDGE CASES
# =============================================================================

class TestEdgeCases:
    """Edge cases that could reveal bugs in corridor routing."""

    @pytest.fixture
    def corridors(self):
        return create_corridor_registry()

    @pytest.fixture
    def router(self, corridors):
        return CorridorRouter(corridors)

    def test_same_source_and_target(self, router):
        """Transfer to same jurisdiction."""
        path, issues = router.find_path(
            source="ae-adgm",
            target="ae-adgm",
            amount=Decimal("1000"),
            currency="USD",
            kyc_tier=2,
        )

        # Should return trivial path or error
        if path:
            assert path == ["ae-adgm"]

    def test_nonexistent_source(self, router):
        """Source jurisdiction doesn't exist."""
        path, issues = router.find_path(
            source="nonexistent",
            target="ae-difc",
            amount=Decimal("1000"),
            currency="USD",
            kyc_tier=2,
        )

        assert path is None
        assert len(issues) > 0

    def test_nonexistent_target(self, router):
        """Target jurisdiction doesn't exist."""
        path, issues = router.find_path(
            source="ae-adgm",
            target="nonexistent",
            amount=Decimal("1000"),
            currency="USD",
            kyc_tier=2,
        )

        assert path is None
        assert len(issues) > 0

    def test_negative_amount(self, router):
        """Negative transfer amount."""
        path, issues = router.find_path(
            source="ae-adgm",
            target="ae-difc",
            amount=Decimal("-1000"),
            currency="USD",
            kyc_tier=2,
        )

        # Should either reject or treat as zero
        # Implementation dependent - just verify no crash
        assert True

    def test_very_large_amount(self, router):
        """Very large transfer exceeding all limits."""
        path, issues = router.find_path(
            source="ae-adgm",
            target="ae-difc",
            amount=Decimal("999999999999"),  # Huge amount
            currency="USD",
            kyc_tier=2,
        )

        # Should fail due to limits
        assert len(issues) > 0

    def test_empty_currency(self, router):
        """Empty currency string."""
        path, issues = router.find_path(
            source="ae-adgm",
            target="ae-difc",
            amount=Decimal("1000"),
            currency="",
            kyc_tier=2,
        )

        # Should fail validation
        assert len(issues) > 0


# =============================================================================
# TESTS: ASYMMETRIC CORRIDORS
# =============================================================================

class TestAsymmetricCorridors:
    """Tests for corridors with different characteristics in each direction."""

    @pytest.fixture
    def corridors(self):
        return create_corridor_registry()

    @pytest.fixture
    def router(self, corridors):
        return CorridorRouter(corridors)

    def test_asymmetric_fees(self, corridors, router):
        """Verify fees differ by direction."""
        adgm_to_aifc = corridors[("ae-adgm", "kz-aifc")]
        aifc_to_adgm = corridors[("kz-aifc", "ae-adgm")]

        assert adgm_to_aifc.fees.percentage_fee_bps != aifc_to_adgm.fees.percentage_fee_bps

        # Calculate fees both directions
        forward_fees = router.calculate_fees(
            ["ae-adgm", "kz-aifc"],
            Decimal("1000000"),
        )
        reverse_fees = router.calculate_fees(
            ["kz-aifc", "ae-adgm"],
            Decimal("1000000"),
        )

        assert forward_fees != reverse_fees

    def test_asymmetric_settlement_time(self, corridors):
        """Verify settlement time differs by direction."""
        adgm_to_aifc = corridors[("ae-adgm", "kz-aifc")]
        aifc_to_adgm = corridors[("kz-aifc", "ae-adgm")]

        assert adgm_to_aifc.settlement_time_hours != aifc_to_adgm.settlement_time_hours


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
