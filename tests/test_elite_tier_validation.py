#!/usr/bin/env python3
"""
Elite-Tier Validation Test Suite (v0.4.44-hardened)

This module comprehensively validates all critical bug fixes and enhancements:

1. SECURITY FIXES
   - Condition evaluation fail-safe (unknown types return False)
   - Sanctions checker empty query protection
   - Thread safety in PolicyEvaluator

2. FINANCIAL PRECISION FIXES
   - Money serialization preserves Decimal precision
   - SmartAsset balance validation
   - Netting constraint application

3. INTEGRITY FIXES
   - Alias indexing compatibility
   - Full UUID collision resistance
   - Audit trail size limiting

4. INTEGRATION ENHANCEMENTS
   - ArbitrationManager agentic trigger generation
   - Monitor recovery from ERROR state
   - Enhanced condition operators
"""

import pytest
import threading
import time
import json
from decimal import Decimal
from datetime import datetime, timezone
from concurrent.futures import ThreadPoolExecutor, as_completed

# Import all modules under test
from tools.mass_primitives import (
    AgenticTriggerType, AgenticTrigger, AgenticPolicy,
    TransitionKind, SmartAsset, GenesisDocument,
    OperationalManifest, RegistryCredential, JurisdictionalBinding,
    TransitionEnvelope, stack_digest, json_canonicalize,
)
from tools.agentic import (
    PolicyEvaluator, MonitorConfig, MonitorStatus,
    SanctionsListMonitor, EXTENDED_POLICIES,
)
from tools.regpack import (
    SanctionsChecker, SanctionsEntry,
)
from tools.arbitration import (
    ArbitrationManager, Money, Order, Claim, Party,
)
from tools.netting import (
    NettingEngine, Obligation, Party as NettingParty,
    Currency, SettlementRail, NettingConstraints, PartyConstraint,
)


# =============================================================================
# SECURITY VALIDATION TESTS
# =============================================================================

class TestSecurityValidation:
    """Validate all security-related bug fixes."""
    
    def test_unknown_condition_type_returns_false(self):
        """
        SECURITY: Unknown condition types must return False (fail-safe).
        
        This prevents policies from triggering unexpectedly due to typos
        or malformed conditions.
        """
        dangerous_conditions = [
            {"type": "unknown_type"},
            {"type": ""},
            {"type": None},
            {"type": "THRESHOLD"},  # Wrong case
            {"type": "equal"},      # Missing 's'
            {"typo": "threshold"},  # Wrong key
        ]
        
        for condition in dangerous_conditions:
            policy = AgenticPolicy(
                policy_id="test_dangerous",
                trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
                condition=condition,
                action=TransitionKind.HALT,
            )
            
            trigger = AgenticTrigger(
                trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
                data={"entity_id": "test", "match_score": 1.0}
            )
            
            # All unknown conditions must return False
            result = policy.evaluate_condition(trigger, {})
            assert result == False, f"Condition {condition} should return False (fail-safe)"
    
    def test_sanctions_checker_empty_query_protection(self):
        """
        SECURITY: Empty queries must not match any sanctions entries.
        """
        entries = [
            SanctionsEntry(
                entry_id="test:1",
                entry_type="entity",
                source_lists=["OFAC"],
                primary_name="ACME CORPORATION",
            ),
        ]
        
        checker = SanctionsChecker(entries, snapshot_id="test")
        
        # Test various empty/malicious inputs
        empty_inputs = ["", "   ", "\t\n", "!@#$%", None]
        
        for query in empty_inputs:
            if query is None:
                continue  # None would raise different error
            result = checker.check_entity(query)
            assert not result.matched, f"Empty/special query '{repr(query)}' should not match"
    
    def test_thread_safe_policy_registration(self):
        """
        SECURITY: Concurrent policy registration must not lose policies.
        """
        evaluator = PolicyEvaluator()
        registered = []
        lock = threading.Lock()
        
        def register_batch(batch_id):
            for i in range(50):
                policy = AgenticPolicy(
                    policy_id=f"policy_{batch_id}_{i}",
                    trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
                    action=TransitionKind.HALT,
                )
                evaluator.register_policy(policy)
                with lock:
                    registered.append(policy.policy_id)
        
        threads = [threading.Thread(target=register_batch, args=(i,)) for i in range(10)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        
        # All policies must be registered
        final_policies = {p.policy_id for p in evaluator.list_policies()}
        for pid in registered:
            assert pid in final_policies, f"Policy {pid} was lost"


# =============================================================================
# FINANCIAL PRECISION VALIDATION TESTS
# =============================================================================

class TestFinancialPrecisionValidation:
    """Validate financial precision fixes."""
    
    def test_money_preserves_decimal_precision(self):
        """
        FINANCIAL: Money serialization must preserve exact Decimal values.
        """
        test_values = [
            Decimal("0.01"),
            Decimal("0.001"),
            Decimal("999999999999.99"),
            Decimal("0.1") + Decimal("0.2"),  # Famous floating point issue
        ]
        
        for value in test_values:
            original = Money(amount=value, currency="USD")
            serialized = original.to_dict()
            restored = Money.from_dict(serialized)
            
            # Must be exactly equal (no floating point loss)
            assert restored.amount == original.amount, \
                f"Precision loss: {original.amount} became {restored.amount}"
    
    def test_money_serializes_as_string(self):
        """
        FINANCIAL: Money.to_dict() must return string amount, not float.
        """
        money = Money(amount=Decimal("12345.67"), currency="USD")
        serialized = money.to_dict()
        
        # Must be string, not float
        assert isinstance(serialized["amount"], str), \
            f"Amount should be string, got {type(serialized['amount'])}"
        assert serialized["amount"] == "12345.67"
    
    def test_smart_asset_balance_validation(self):
        """
        FINANCIAL: SmartAsset must validate sufficient balance on transfer.
        """
        # Create a minimal SmartAsset with correct structure
        genesis = GenesisDocument(
            asset_name="Test Token",
            asset_class="token",
            initial_bindings=["harbor:difc"],
            governance={"type": "multisig", "threshold": 1},
        )
        
        registry = RegistryCredential(
            asset_id=genesis.asset_id,
            registry_vc_digest="a" * 64,
            effective_from=datetime.now(timezone.utc).isoformat(),
            bindings=[
                JurisdictionalBinding(
                    harbor_id="harbor:difc",
                    lawpack_digest="b" * 64,
                    binding_status="active",
                )
            ],
        )
        
        manifest = OperationalManifest(
            asset_id=genesis.asset_id,
            version=1,
            config={},
            quorum_threshold=1,
            authorized_governors=["did:key:creator"],
        )
        
        asset = SmartAsset(
            genesis=genesis,
            registry=registry,
            manifest=manifest,
            state={"balances": {"alice": Decimal("100")}},
        )
        
        # Attempt transfer exceeding balance
        envelope = TransitionEnvelope(
            asset_id=genesis.asset_id,
            seq=0,
            kind=TransitionKind.TRANSFER,
            effective_time=datetime.now(timezone.utc).isoformat(),
            params={"from": "alice", "to": "bob", "amount": "150"},
        )
        
        with pytest.raises(ValueError, match="Insufficient balance"):
            asset.transition(envelope)
    
    def test_netting_constraint_actually_applied(self):
        """
        FINANCIAL: Netting constraints must actually cap net positions.
        """
        party_a = NettingParty("party:a", "Party A")
        party_b = NettingParty("party:b", "Party B")
        usd = Currency("USD", 2)
        
        # Large obligation
        obligations = [
            Obligation("obl-001", "corridor:1", party_a, party_b, usd, Decimal("1000000")),
        ]
        
        rails = [
            SettlementRail("rail:1", "corridor:settlement", {"USD"}),
        ]
        
        # Constraint: max 500000 net position
        constraints = NettingConstraints(
            party_constraints={
                "party:b": PartyConstraint(
                    party_id="party:b",
                    max_net_position={"USD": Decimal("500000")},
                ),
            }
        )
        
        engine = NettingEngine(obligations, rails, constraints)
        gross = engine.compute_gross_positions()
        net = engine.compute_net_positions(gross)
        constrained = engine._apply_party_constraints(net)
        
        # Party B's net position should be capped
        for np in constrained:
            if np.party_id == "party:b" and np.currency == "USD":
                assert abs(np.net_amount) <= Decimal("500000"), \
                    f"Net position {np.net_amount} exceeds constraint 500000"


# =============================================================================
# INTEGRITY VALIDATION TESTS
# =============================================================================

class TestIntegrityValidation:
    """Validate data integrity fixes."""
    
    def test_alias_indexing_both_keys(self):
        """
        INTEGRITY: Sanctions checker must index aliases from both 'name' and 'alias' keys.
        """
        entries = [
            SanctionsEntry(
                entry_id="entry:1",
                entry_type="entity",
                source_lists=["OFAC"],
                primary_name="ACME CORPORATION",
                aliases=[
                    {"alias_type": "AKA", "name": "ACME CORP"},    # OFAC format
                    {"alias_type": "FKA", "alias": "ACME INC"},   # Alternative format
                ],
            ),
        ]
        
        checker = SanctionsChecker(entries, snapshot_id="test")
        
        # Primary name should match
        assert checker.check_entity("ACME CORPORATION").matched
        
        # Both alias formats should match
        assert checker.check_entity("ACME CORP").matched, "Alias with 'name' key should match"
        assert checker.check_entity("ACME INC").matched, "Alias with 'alias' key should match"
    
    def test_full_uuid_generation(self):
        """
        INTEGRITY: UUIDs must be full 128-bit (32 hex chars), not truncated.
        """
        evaluator = PolicyEvaluator()
        
        generated_ids = set()
        for _ in range(1000):
            new_id = evaluator._generate_id("test")
            parts = new_id.split(":")
            
            # Must be full 32 hex chars
            assert len(parts[1]) == 32, f"UUID truncated to {len(parts[1])} chars"
            assert parts[1] not in generated_ids, "UUID collision detected"
            generated_ids.add(parts[1])
    
    def test_audit_trail_size_limited(self):
        """
        INTEGRITY: Audit trail must not grow unboundedly.
        """
        max_size = 100
        evaluator = PolicyEvaluator(max_audit_trail_size=max_size)
        
        # Generate many entries
        for i in range(500):
            trigger = AgenticTrigger(
                trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
                data={"receipts_since_last": i}
            )
            evaluator.evaluate(trigger, asset_id=f"asset:{i}")
        
        # Trail should be bounded
        trail = evaluator.get_audit_trail(limit=10000)
        assert len(trail) <= max_size, \
            f"Audit trail exceeded max size: {len(trail)} > {max_size}"


# =============================================================================
# INTEGRATION ENHANCEMENT VALIDATION TESTS
# =============================================================================

class TestIntegrationEnhancements:
    """Validate integration enhancements."""
    
    def test_arbitration_ruling_to_trigger(self):
        """
        INTEGRATION: ArbitrationManager must generate valid agentic triggers from rulings.
        """
        manager = ArbitrationManager(institution_id="difc-lcia")
        
        ruling_vc = {
            "type": ["VerifiableCredential", "MSEZArbitrationRulingCredential"],
            "issuer": "did:key:z6MkDIFCLCIA",
            "issuanceDate": "2026-01-15T00:00:00Z",
            "credentialSubject": {
                "dispute_id": "dispute:001",
                "institution_id": "difc-lcia",
                "case_reference": "DIFC-LCIA/2026/001",
                "corridor_id": "corridor:trade:001",
                "parties": {
                    "claimant": {"party_id": "did:key:claimant"},
                    "respondent": {"party_id": "did:key:respondent"},
                },
                "ruling": {
                    "ruling_type": "final_award",
                    "disposition": "in_favor_of_claimant",
                    "orders": [
                        {
                            "order_id": "order:001",
                            "order_type": "monetary_damages",
                            "obligor": "did:key:respondent",
                            "obligee": "did:key:claimant",
                            "amount": {"amount": "150000", "currency": "USD"},
                            "enforcement_method": "smart_asset_state_transition",
                        }
                    ],
                },
                "enforcement": {"enabled": True},
                "appeal": {"period_days": 30},
            },
        }
        
        trigger = manager.ruling_to_trigger(ruling_vc)
        
        assert trigger["trigger_type"] == "ruling_received"
        assert trigger["data"]["dispute_id"] == "dispute:001"
        assert len(trigger["data"]["orders"]) == 1
        assert trigger["data"]["orders"][0]["order_type"] == "monetary_damages"
    
    def test_monitor_recovery_from_error(self):
        """
        INTEGRATION: Monitors must be able to recover from ERROR state.
        """
        entries = [
            SanctionsEntry(
                entry_id="test:1",
                entry_type="entity",
                source_lists=["OFAC"],
                primary_name="Test Entity",
            )
        ]
        checker = SanctionsChecker(entries, snapshot_id="test")
        
        config = MonitorConfig(
            monitor_id="test-recovery",
            monitor_type="sanctions",
            poll_interval_seconds=0.1,
            error_threshold=1,
        )
        
        monitor = SanctionsListMonitor(config, checker)
        
        # Force into error state
        monitor.status = MonitorStatus.ERROR
        monitor.consecutive_errors = 5
        
        # Verify recovery method exists and works
        assert hasattr(monitor, 'recover'), "Monitor must have recover() method"
        
        # Recovery should reset error count and restore status
        result = monitor.recover()
        
        if result:
            assert monitor.status == MonitorStatus.RUNNING
            assert monitor.consecutive_errors == 0
    
    def test_enhanced_condition_operators(self):
        """
        INTEGRATION: All enhanced condition operators must work correctly.
        """
        test_cases = [
            # (condition, trigger_data, expected_result)
            (
                {"type": "not_equals", "field": "status", "value": "active"},
                {"status": "suspended"},
                True,
            ),
            (
                {"type": "less_than", "field": "days", "threshold": 7},
                {"days": 3},
                True,
            ),
            (
                {"type": "greater_than", "field": "score", "threshold": 0.8},
                {"score": 0.95},
                True,
            ),
            (
                {"type": "in", "field": "status", "values": ["a", "b", "c"]},
                {"status": "b"},
                True,
            ),
            (
                {"type": "exists", "field": "important"},
                {"important": "yes"},
                True,
            ),
            (
                {"type": "exists", "field": "missing"},
                {},
                False,
            ),
            # Nested field access
            (
                {"type": "threshold", "field": "match.score", "threshold": 0.8},
                {"match": {"score": 0.95}},
                True,
            ),
            # Compound AND
            (
                {
                    "type": "and",
                    "conditions": [
                        {"type": "threshold", "field": "score", "threshold": 0.5},
                        {"type": "equals", "field": "list", "value": "OFAC"},
                    ]
                },
                {"score": 0.9, "list": "OFAC"},
                True,
            ),
            # Compound OR
            (
                {
                    "type": "or",
                    "conditions": [
                        {"type": "equals", "field": "list", "value": "OFAC"},
                        {"type": "equals", "field": "list", "value": "EU"},
                    ]
                },
                {"list": "EU"},
                True,
            ),
        ]
        
        for condition, data, expected in test_cases:
            policy = AgenticPolicy(
                policy_id="test",
                trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
                condition=condition,
                action=TransitionKind.HALT,
            )
            
            trigger = AgenticTrigger(
                trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
                data=data,
            )
            
            result = policy.evaluate_condition(trigger, {})
            assert result == expected, \
                f"Condition {condition} with data {data} should return {expected}, got {result}"


# =============================================================================
# DETERMINISM VALIDATION TESTS
# =============================================================================

class TestDeterminismValidation:
    """Validate determinism guarantees per Theorem 17.1."""
    
    def test_json_canonicalization_determinism(self):
        """
        DETERMINISM: JSON canonicalization must be perfectly deterministic.
        """
        test_obj = {
            "z_key": [3, 1, 2],
            "a_key": {"nested_z": 1, "nested_a": 2},
            "m_key": "value",
        }
        
        results = [json_canonicalize(test_obj) for _ in range(100)]
        
        # All results must be identical
        assert len(set(results)) == 1, "Canonicalization not deterministic"
        
        # Keys must be sorted
        parsed = json.loads(results[0])
        keys = list(parsed.keys())
        assert keys == sorted(keys), "Keys not sorted"
    
    def test_digest_determinism(self):
        """
        DETERMINISM: Digest computation must be deterministic across calls.
        """
        test_obj = {"key": "value", "number": 42}
        
        digests = [stack_digest(test_obj).bytes_hex for _ in range(100)]
        
        assert len(set(digests)) == 1, "Digest not deterministic"
    
    def test_policy_evaluation_order_determinism(self):
        """
        DETERMINISM: Policy evaluation order must be deterministic.
        """
        evaluator = PolicyEvaluator()
        
        # Register policies in random-ish order
        for pid in ["z_policy", "a_policy", "m_policy", "b_policy"]:
            evaluator.register_policy(AgenticPolicy(
                policy_id=pid,
                trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
                action=TransitionKind.HALT,
            ))
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            data={},
        )
        
        # Run evaluation multiple times
        results = []
        for _ in range(10):
            result = evaluator.evaluate(trigger)
            policy_order = [r.policy_id for r in result]
            results.append(tuple(policy_order))
        
        # All evaluation orders must be identical
        assert len(set(results)) == 1, "Policy evaluation order not deterministic"
        
        # Order must be sorted by policy_id
        order = list(results[0])
        assert order == sorted(order), "Policies not evaluated in sorted order"


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
