#!/usr/bin/env python3
"""
Elite-tier deep verification tests for v0.4.44.

These tests verify:
- Schema consistency and completeness
- Cross-module integration
- Determinism guarantees
- Edge cases and boundary conditions
- Spec compliance at the deepest level
"""

import pytest
import json
import hashlib
from pathlib import Path
from datetime import datetime, timezone, timedelta
from typing import Dict, Any, List

# Core imports
from tools.agentic import (
    MonitorStatus,
    MonitorMode,
    MonitorConfig,
    SanctionsListMonitor,
    LicenseStatusMonitor,
    CorridorStateMonitor,
    GuidanceUpdateMonitor,
    CheckpointDueMonitor,
    PolicyEvaluator,
    PolicyEvaluationResult,
    ScheduledAction,
    AuditTrailEntry,
    AgenticExecutionEngine,
    EXTENDED_POLICIES,
    create_sanctions_monitor,
    create_license_monitor,
    create_corridor_monitor,
)

from tools.mass_primitives import (
    AgenticTriggerType,
    AgenticTrigger,
    AgenticPolicy,
    ImpactLevel,
    LicenseStatus,
    TransitionKind,
    STANDARD_POLICIES,
    SmartAsset,
    GenesisDocument,
    RegistryCredential,
    OperationalManifest,
    stack_digest,
    json_canonicalize,
)

from tools.regpack import (
    SanctionsChecker,
    SanctionsEntry,
    RegPackManager,
)

from tools.arbitration import (
    Ruling,
    ArbitrationRulingVC,
)


# =============================================================================
# SCHEMA COMPLETENESS TESTS
# =============================================================================

class TestSchemaCompleteness:
    """Verify all schemas are complete and internally consistent."""
    
    @pytest.fixture
    def schemas_dir(self):
        return Path(__file__).parent.parent / "schemas"
    
    def test_all_agentic_schemas_have_required_fields(self, schemas_dir):
        """Every agentic schema must have $id, title, description, type."""
        agentic_schemas = list(schemas_dir.glob("agentic.*.schema.json"))
        assert len(agentic_schemas) == 6, "Expected 6 agentic schemas"
        
        for schema_path in agentic_schemas:
            with open(schema_path) as f:
                schema = json.load(f)
            
            assert "$schema" in schema, f"{schema_path.name} missing $schema"
            assert "$id" in schema, f"{schema_path.name} missing $id"
            assert "title" in schema, f"{schema_path.name} missing title"
            assert "description" in schema, f"{schema_path.name} missing description"
            assert "type" in schema, f"{schema_path.name} missing type"
    
    def test_trigger_schema_has_all_trigger_types(self, schemas_dir):
        """Trigger schema enum must include all AgenticTriggerType values."""
        schema_path = schemas_dir / "agentic.trigger.schema.json"
        with open(schema_path) as f:
            schema = json.load(f)
        
        schema_trigger_types = set(schema["properties"]["trigger_type"]["enum"])
        impl_trigger_types = set(t.value for t in AgenticTriggerType)
        
        missing = impl_trigger_types - schema_trigger_types
        assert not missing, f"Schema missing trigger types: {missing}"
    
    def test_policy_schema_has_all_actions(self, schemas_dir):
        """Policy schema enum must include all TransitionKind values used."""
        schema_path = schemas_dir / "agentic.policy.schema.json"
        with open(schema_path) as f:
            schema = json.load(f)
        
        schema_actions = set(schema["properties"]["action"]["enum"])
        
        # Check that common actions are present
        required_actions = {"halt", "resume", "update_manifest", "arbitration_enforce"}
        missing = required_actions - schema_actions
        assert not missing, f"Schema missing action types: {missing}"
    
    def test_monitor_schema_has_all_monitor_types(self, schemas_dir):
        """Monitor schema enum must include all monitor types."""
        schema_path = schemas_dir / "agentic.environment-monitor.schema.json"
        with open(schema_path) as f:
            schema = json.load(f)
        
        schema_types = set(schema["properties"]["monitor_type"]["enum"])
        
        required_types = {"sanctions_list", "license_status", "corridor_state", 
                         "guidance_update", "checkpoint_due"}
        missing = required_types - schema_types
        assert not missing, f"Schema missing monitor types: {missing}"


# =============================================================================
# DETERMINISM DEEP TESTS
# =============================================================================

class TestDeterminismDeep:
    """Deep verification of Theorem 17.1 (Agentic Determinism)."""
    
    def test_policy_evaluation_order_is_deterministic(self):
        """Policy evaluation must occur in sorted policy_id order."""
        evaluator = PolicyEvaluator()
        
        # Add policies in random order
        policies = [
            AgenticPolicy("zeta_policy", AgenticTriggerType.CHECKPOINT_DUE, action=TransitionKind.HALT),
            AgenticPolicy("alpha_policy", AgenticTriggerType.CHECKPOINT_DUE, action=TransitionKind.UPDATE_MANIFEST),
            AgenticPolicy("mu_policy", AgenticTriggerType.CHECKPOINT_DUE, action=TransitionKind.HALT),
        ]
        
        for p in policies:
            evaluator.register_policy(p)
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={}
        )
        
        results = evaluator.evaluate(trigger)
        
        # Extract policy IDs that matched
        matched_ids = [r.policy_id for r in results if r.policy_id in ["alpha_policy", "mu_policy", "zeta_policy"]]
        
        # Should be in sorted order
        assert matched_ids == sorted(matched_ids), "Evaluation order not deterministic"
    
    def test_identical_triggers_produce_identical_results_hash(self):
        """Hash of evaluation results must be identical for identical inputs."""
        evaluator = PolicyEvaluator()
        
        # Create identical triggers (same content, different instances)
        data = {"field": "value", "count": 42}
        trigger1 = AgenticTrigger(AgenticTriggerType.GUIDANCE_UPDATE, data.copy())
        trigger2 = AgenticTrigger(AgenticTriggerType.GUIDANCE_UPDATE, data.copy())
        
        # Evaluate both
        results1 = evaluator.evaluate(trigger1, asset_id="asset:test")
        results2 = evaluator.evaluate(trigger2, asset_id="asset:test")
        
        # Compare result structure (excluding timestamps which will differ)
        def normalize_results(results):
            return [(r.policy_id, r.matched, r.action) for r in results]
        
        assert normalize_results(results1) == normalize_results(results2)
    
    def test_condition_evaluation_is_pure(self):
        """Condition evaluation must have no side effects."""
        policy = AgenticPolicy(
            policy_id="pure_test",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            condition={"type": "threshold", "field": "count", "threshold": 10},
            action=TransitionKind.UPDATE_MANIFEST,
        )
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"count": 15}
        )
        
        environment = {"state": "initial"}
        
        # Evaluate multiple times
        results = []
        for _ in range(10):
            result = policy.evaluate_condition(trigger, environment)
            results.append(result)
        
        # All results must be identical
        assert all(r == results[0] for r in results)
        
        # Environment must be unchanged
        assert environment == {"state": "initial"}


# =============================================================================
# EDGE CASE TESTS
# =============================================================================

class TestEdgeCases:
    """Test edge cases and boundary conditions."""
    
    def test_empty_trigger_data(self):
        """Handle triggers with empty data."""
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={}
        )
        
        evaluator = PolicyEvaluator()
        results = evaluator.evaluate(trigger)
        
        # Should not crash
        assert isinstance(results, list)
    
    def test_policy_with_no_condition(self):
        """Policy with no condition should always match on trigger type."""
        evaluator = PolicyEvaluator()
        
        policy = AgenticPolicy(
            policy_id="no_condition",
            trigger_type=AgenticTriggerType.DISPUTE_FILED,
            condition=None,  # No condition
            action=TransitionKind.HALT,
        )
        evaluator.register_policy(policy)
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.DISPUTE_FILED,
            data={"any": "data"}
        )
        
        results = evaluator.evaluate(trigger)
        matched = [r for r in results if r.policy_id == "no_condition"]
        
        assert len(matched) == 1
        assert matched[0].matched == True
    
    def test_disabled_policy_never_matches(self):
        """Disabled policy should never match."""
        evaluator = PolicyEvaluator()
        
        policy = AgenticPolicy(
            policy_id="disabled_policy",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            action=TransitionKind.HALT,
            enabled=False,  # Disabled
        )
        evaluator.register_policy(policy)
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={}
        )
        
        results = evaluator.evaluate(trigger)
        matched = [r for r in results if r.policy_id == "disabled_policy" and r.matched]
        
        assert len(matched) == 0
    
    def test_threshold_condition_boundary(self):
        """Test threshold condition at exact boundary."""
        evaluator = PolicyEvaluator()
        
        policy = AgenticPolicy(
            policy_id="boundary_test",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            condition={"type": "threshold", "field": "count", "threshold": 100},
            action=TransitionKind.UPDATE_MANIFEST,
        )
        evaluator.register_policy(policy)
        
        # At threshold (should match - >= comparison)
        trigger_at = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"count": 100}
        )
        results_at = evaluator.evaluate(trigger_at)
        matched_at = [r for r in results_at if r.policy_id == "boundary_test"]
        assert matched_at[0].matched == True
        
        # Below threshold
        trigger_below = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"count": 99}
        )
        results_below = evaluator.evaluate(trigger_below)
        matched_below = [r for r in results_below if r.policy_id == "boundary_test"]
        assert matched_below[0].matched == False
    
    def test_missing_field_in_condition(self):
        """Handle condition referencing missing field."""
        evaluator = PolicyEvaluator()
        
        policy = AgenticPolicy(
            policy_id="missing_field",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            condition={"type": "threshold", "field": "nonexistent", "threshold": 10},
            action=TransitionKind.UPDATE_MANIFEST,
        )
        evaluator.register_policy(policy)
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"other_field": 100}
        )
        
        results = evaluator.evaluate(trigger)
        matched = [r for r in results if r.policy_id == "missing_field"]
        
        # Should not match (missing field defaults to 0)
        assert matched[0].matched == False


# =============================================================================
# INTEGRATION TESTS
# =============================================================================

class TestCrossModuleIntegration:
    """Test integration between agentic framework and other modules."""
    
    def test_arbitration_trigger_integration(self):
        """Verify arbitration triggers work with agentic policies."""
        evaluator = PolicyEvaluator()
        
        # Register dispute-related policies
        for policy in EXTENDED_POLICIES.values():
            evaluator.register_policy(policy)
        
        # Simulate dispute filed
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.DISPUTE_FILED,
            data={
                "dispute_id": "dispute:test-001",
                "claimant": "did:key:z6MkClaimant",
                "respondent": "did:key:z6MkRespondent",
                "claim_amount": "10000 USD",
            }
        )
        
        results = evaluator.evaluate(trigger, asset_id="asset:disputed")
        matched = [r for r in results if r.matched]
        
        # Should have matched dispute_filed_halt policy
        assert any(r.policy_id == "dispute_filed_halt" for r in matched)
    
    def test_ruling_enforcement_integration(self):
        """Verify ruling enforcement triggers correct action."""
        evaluator = PolicyEvaluator()
        
        for policy in EXTENDED_POLICIES.values():
            evaluator.register_policy(policy)
        
        # Simulate ruling received with auto_enforce
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.RULING_RECEIVED,
            data={
                "ruling_id": "ruling:test-001",
                "disposition": "in_favor_claimant",
                "auto_enforce": True,
            }
        )
        
        results = evaluator.evaluate(trigger, asset_id="asset:ruled")
        matched = [r for r in results if r.matched]
        
        # Should match ruling_auto_enforce
        auto_enforce = [r for r in matched if r.policy_id == "ruling_auto_enforce"]
        assert len(auto_enforce) == 1
        assert auto_enforce[0].action == TransitionKind.ARBITRATION_ENFORCE
    
    def test_sanctions_check_integration(self):
        """Verify sanctions monitor integrates with SanctionsChecker."""
        entries = [
            SanctionsEntry(
                entry_id="sanc-001",
                entry_type="entity",
                source_lists=["OFAC", "EU"],
                primary_name="Bad Actor Corp",
            )
        ]
        checker = SanctionsChecker(entries=entries, snapshot_id="snap-001")
        
        monitor = create_sanctions_monitor(
            monitor_id="int-test-001",
            sanctions_checker=checker,
            watched_entities=["Bad Actor Corp", "Good Company"],
        )
        
        state = monitor.poll()
        assert state is not None
        assert state["entity_results"]["Bad Actor Corp"]["sanctioned"] == True
        assert state["entity_results"]["Good Company"]["sanctioned"] == False


# =============================================================================
# SPEC COMPLIANCE DEEP TESTS
# =============================================================================

class TestSpecComplianceDeep:
    """Deep verification of MASS Protocol v0.2 Chapter 17 compliance."""
    
    def test_all_standard_policies_have_valid_trigger_types(self):
        """Every standard policy must reference a valid trigger type."""
        for policy_id, policy in STANDARD_POLICIES.items():
            assert isinstance(policy.trigger_type, AgenticTriggerType), \
                f"Policy {policy_id} has invalid trigger_type"
    
    def test_all_extended_policies_have_valid_trigger_types(self):
        """Every extended policy must reference a valid trigger type."""
        for policy_id, policy in EXTENDED_POLICIES.items():
            assert isinstance(policy.trigger_type, AgenticTriggerType), \
                f"Policy {policy_id} has invalid trigger_type"
    
    def test_all_policies_have_valid_actions(self):
        """Every policy must have a valid TransitionKind action."""
        all_policies = {**STANDARD_POLICIES, **EXTENDED_POLICIES}
        
        for policy_id, policy in all_policies.items():
            assert isinstance(policy.action, TransitionKind), \
                f"Policy {policy_id} has invalid action type"
    
    def test_authorization_requirements_are_valid(self):
        """All authorization requirements must be recognized values."""
        valid_auth = {"automatic", "quorum", "unanimous", "governance"}
        all_policies = {**STANDARD_POLICIES, **EXTENDED_POLICIES}
        
        for policy_id, policy in all_policies.items():
            assert policy.authorization_requirement in valid_auth, \
                f"Policy {policy_id} has invalid authorization_requirement: {policy.authorization_requirement}"
    
    def test_impact_levels_are_valid(self):
        """All ImpactLevel enum values should be used correctly."""
        valid_levels = {e.value for e in ImpactLevel}
        
        # Check that sanctions monitor uses valid impact levels
        entries = [
            SanctionsEntry(
                entry_id="test-001",
                entry_type="entity",
                source_lists=["OFAC"],
                primary_name="Test Entity",
            )
        ]
        checker = SanctionsChecker(entries=entries, snapshot_id="test-snap")
        monitor = create_sanctions_monitor(
            monitor_id="level-test",
            sanctions_checker=checker,
            watched_entities=["Test Entity"],
        )
        
        old_state = {
            "list_version": "v1",
            "entity_results": {"Test Entity": {"sanctioned": False}}
        }
        new_state = {
            "list_version": "v2",
            "entity_results": {"Test Entity": {"sanctioned": True}}
        }
        
        triggers = monitor.detect_changes(old_state, new_state)
        
        for trigger in triggers:
            if "impact_level" in trigger.data:
                assert trigger.data["impact_level"] in valid_levels


# =============================================================================
# AUDIT TRAIL COMPLETENESS TESTS
# =============================================================================

class TestAuditTrailCompleteness:
    """Verify audit trail captures all required events."""
    
    def test_trigger_receipt_creates_audit_entry(self):
        """Every trigger must create an audit trail entry."""
        evaluator = PolicyEvaluator()
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"test": True}
        )
        
        evaluator.evaluate(trigger, asset_id="asset:audit-test")
        
        trail = evaluator.get_audit_trail(entry_type="trigger_received")
        assert len(trail) > 0
        
        # Verify entry has required fields
        entry = trail[-1]
        assert entry.entry_type == "trigger_received"
        assert entry.timestamp is not None
        assert entry.trigger_data is not None
    
    def test_policy_evaluation_creates_audit_entries(self):
        """Every policy evaluation must create an audit entry."""
        evaluator = PolicyEvaluator()
        
        # Register a custom policy
        evaluator.register_policy(AgenticPolicy(
            policy_id="audit_test_policy",
            trigger_type=AgenticTriggerType.GUIDANCE_UPDATE,
            action=TransitionKind.UPDATE_MANIFEST,
        ))
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.GUIDANCE_UPDATE,
            data={}
        )
        
        evaluator.evaluate(trigger, asset_id="asset:audit-test-2")
        
        trail = evaluator.get_audit_trail(entry_type="policy_evaluated")
        
        # Should have entry for our policy
        our_entries = [e for e in trail if e.evaluation_result and 
                      e.evaluation_result.get("policy_id") == "audit_test_policy"]
        assert len(our_entries) > 0
    
    def test_action_scheduling_creates_audit_entry(self):
        """Action scheduling must create an audit entry."""
        evaluator = PolicyEvaluator()
        
        evaluator.register_policy(AgenticPolicy(
            policy_id="schedule_audit_test",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            action=TransitionKind.UPDATE_MANIFEST,
        ))
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={}
        )
        
        results = evaluator.evaluate(trigger, asset_id="asset:schedule-test")
        matched = [r for r in results if r.matched]
        
        if matched:
            evaluator.schedule_actions(matched, "asset:schedule-test")
            
            trail = evaluator.get_audit_trail(entry_type="action_scheduled")
            assert len(trail) > 0


# =============================================================================
# MONITOR LIFECYCLE TESTS
# =============================================================================

class TestMonitorLifecycle:
    """Test monitor state machine transitions."""
    
    def test_monitor_initial_state_is_stopped(self):
        """New monitors should start in STOPPED state."""
        monitor = create_corridor_monitor(monitor_id="lifecycle-001")
        assert monitor.status == MonitorStatus.STOPPED
    
    def test_monitor_start_changes_status(self):
        """Starting a monitor should change status to RUNNING."""
        monitor = create_corridor_monitor(monitor_id="lifecycle-002")
        monitor.start()
        assert monitor.status == MonitorStatus.RUNNING
        monitor.stop()
    
    def test_monitor_stop_changes_status(self):
        """Stopping a monitor should change status to STOPPED."""
        monitor = create_corridor_monitor(monitor_id="lifecycle-003")
        monitor.start()
        monitor.stop()
        assert monitor.status == MonitorStatus.STOPPED
    
    def test_monitor_pause_resume(self):
        """Monitor can be paused and resumed."""
        monitor = create_corridor_monitor(monitor_id="lifecycle-004")
        monitor.start()
        
        monitor.pause()
        assert monitor.status == MonitorStatus.PAUSED
        
        monitor.resume()
        assert monitor.status == MonitorStatus.RUNNING
        
        monitor.stop()
    
    def test_double_start_is_safe(self):
        """Starting an already running monitor should be safe."""
        monitor = create_corridor_monitor(monitor_id="lifecycle-005")
        monitor.start()
        monitor.start()  # Should not crash
        assert monitor.status == MonitorStatus.RUNNING
        monitor.stop()
    
    def test_double_stop_is_safe(self):
        """Stopping an already stopped monitor should be safe."""
        monitor = create_corridor_monitor(monitor_id="lifecycle-006")
        monitor.stop()  # Already stopped
        monitor.stop()  # Should not crash
        assert monitor.status == MonitorStatus.STOPPED


# =============================================================================
# POLICY CONDITION COMPREHENSIVE TESTS
# =============================================================================

class TestPolicyConditions:
    """Comprehensive tests for all condition types."""
    
    def test_condition_type_equals_string(self):
        """Test equals condition with string values."""
        policy = AgenticPolicy(
            policy_id="equals_string",
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            condition={"type": "equals", "field": "new_status", "value": "expired"},
            action=TransitionKind.HALT,
        )
        
        trigger_match = AgenticTrigger(
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            data={"new_status": "expired"}
        )
        assert policy.evaluate_condition(trigger_match, {}) == True
        
        trigger_no_match = AgenticTrigger(
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            data={"new_status": "valid"}
        )
        assert policy.evaluate_condition(trigger_no_match, {}) == False
    
    def test_condition_type_equals_boolean(self):
        """Test equals condition with boolean values."""
        policy = AgenticPolicy(
            policy_id="equals_bool",
            trigger_type=AgenticTriggerType.RULING_RECEIVED,
            condition={"type": "equals", "field": "auto_enforce", "value": True},
            action=TransitionKind.ARBITRATION_ENFORCE,
        )
        
        trigger_true = AgenticTrigger(
            trigger_type=AgenticTriggerType.RULING_RECEIVED,
            data={"auto_enforce": True}
        )
        assert policy.evaluate_condition(trigger_true, {}) == True
        
        trigger_false = AgenticTrigger(
            trigger_type=AgenticTriggerType.RULING_RECEIVED,
            data={"auto_enforce": False}
        )
        assert policy.evaluate_condition(trigger_false, {}) == False
    
    def test_condition_type_contains_list(self):
        """Test contains condition with list values."""
        policy = AgenticPolicy(
            policy_id="contains_list",
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            condition={"type": "contains", "field": "affected_parties", "item": "self"},
            action=TransitionKind.HALT,
        )
        
        trigger_contains = AgenticTrigger(
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            data={"affected_parties": ["other", "self", "third"]}
        )
        assert policy.evaluate_condition(trigger_contains, {}) == True
        
        trigger_not_contains = AgenticTrigger(
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            data={"affected_parties": ["other", "third"]}
        )
        assert policy.evaluate_condition(trigger_not_contains, {}) == False
    
    def test_condition_type_threshold_numeric(self):
        """Test threshold condition with various numeric types."""
        policy = AgenticPolicy(
            policy_id="threshold_numeric",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            condition={"type": "threshold", "field": "receipts", "threshold": 50.5},
            action=TransitionKind.UPDATE_MANIFEST,
        )
        
        # Integer above threshold
        trigger_int = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"receipts": 51}
        )
        assert policy.evaluate_condition(trigger_int, {}) == True
        
        # Float at threshold
        trigger_float = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"receipts": 50.5}
        )
        assert policy.evaluate_condition(trigger_float, {}) == True
        
        # Below threshold
        trigger_below = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"receipts": 50.4}
        )
        assert policy.evaluate_condition(trigger_below, {}) == False


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
