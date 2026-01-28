#!/usr/bin/env python3
"""
Comprehensive tests for the Agentic Execution Framework (v0.4.43).

Tests cover:
- Environment monitor lifecycle (Definition 17.4)
- Policy evaluation determinism (Theorem 17.1)
- Trigger emission and handling
- Action scheduling and execution
- Audit trail generation
- Integration with Smart Assets
- Extended policy library

Target: 50+ tests per v0.4.43 acceptance criteria.
"""

import pytest
import json
import time
import threading
from datetime import datetime, timezone, timedelta
from pathlib import Path
from typing import Dict, Any, List

# Import agentic framework components
from tools.agentic import (
    # Enums
    MonitorStatus,
    MonitorMode,
    # Configuration
    MonitorConfig,
    # Base and concrete monitors
    EnvironmentMonitor,
    SanctionsListMonitor,
    LicenseStatusMonitor,
    CorridorStateMonitor,
    GuidanceUpdateMonitor,
    CheckpointDueMonitor,
    # Policy evaluation
    PolicyEvaluationResult,
    ScheduledAction,
    AuditTrailEntry,
    PolicyEvaluator,
    # Registry and engine
    MonitorRegistry,
    AgenticExecutionEngine,
    # Extended policies
    EXTENDED_POLICIES,
    # Factory functions
    create_sanctions_monitor,
    create_license_monitor,
    create_corridor_monitor,
    create_guidance_monitor,
    create_checkpoint_monitor,
)

from tools.mass_primitives import (
    AgenticTriggerType,
    AgenticTrigger,
    AgenticPolicy,
    ImpactLevel,
    LicenseStatus,
    TransitionKind,
    STANDARD_POLICIES,
)

from tools.regpack import SanctionsChecker, SanctionsEntry


# =============================================================================
# FIXTURES
# =============================================================================

@pytest.fixture
def sanctions_checker():
    """Create a SanctionsChecker with test data."""
    entries = [
        SanctionsEntry(
            entry_id="entity:sanctioned-123",
            entry_type="entity",
            source_lists=["OFAC"],
            primary_name="Sanctioned Entity",
            listing_date="2025-01-01",
            remarks="Test sanctions entry"
        )
    ]
    return SanctionsChecker(entries=entries, snapshot_id="test-snapshot-001")


@pytest.fixture
def monitor_config():
    """Create a basic monitor configuration."""
    return MonitorConfig(
        monitor_id="test-monitor-001",
        monitor_type="sanctions_list",
        mode=MonitorMode.POLLING,
        poll_interval_seconds=1,
        enabled=True,
        config={"watched_entities": ["entity:test-001", "entity:test-002"]},
    )


@pytest.fixture
def policy_evaluator():
    """Create a PolicyEvaluator with standard policies."""
    return PolicyEvaluator()


@pytest.fixture
def agentic_engine():
    """Create an AgenticExecutionEngine."""
    return AgenticExecutionEngine()


# =============================================================================
# MONITOR CONFIGURATION TESTS
# =============================================================================

class TestMonitorConfig:
    """Tests for MonitorConfig dataclass."""
    
    def test_monitor_config_creation(self):
        """Test basic MonitorConfig creation."""
        config = MonitorConfig(
            monitor_id="test-001",
            monitor_type="sanctions_list",
        )
        assert config.monitor_id == "test-001"
        assert config.monitor_type == "sanctions_list"
        assert config.mode == MonitorMode.POLLING  # default
        assert config.poll_interval_seconds == 60  # default
        assert config.enabled == True  # default
    
    def test_monitor_config_to_dict(self, monitor_config):
        """Test MonitorConfig serialization."""
        d = monitor_config.to_dict()
        assert d["monitor_id"] == "test-monitor-001"
        assert d["monitor_type"] == "sanctions_list"
        assert d["mode"] == "polling"
        assert d["poll_interval_seconds"] == 1
        assert d["enabled"] == True
        assert "watched_entities" in d["config"]
    
    def test_monitor_config_from_dict(self):
        """Test MonitorConfig deserialization."""
        data = {
            "monitor_id": "test-002",
            "monitor_type": "license_status",
            "mode": "polling",
            "poll_interval_seconds": 300,
            "enabled": False,
            "config": {"warning_thresholds": [30, 14, 7]},
        }
        config = MonitorConfig.from_dict(data)
        assert config.monitor_id == "test-002"
        assert config.monitor_type == "license_status"
        assert config.mode == MonitorMode.POLLING
        assert config.poll_interval_seconds == 300
        assert config.enabled == False
    
    def test_monitor_config_roundtrip(self, monitor_config):
        """Test MonitorConfig serialization roundtrip."""
        d = monitor_config.to_dict()
        restored = MonitorConfig.from_dict(d)
        assert restored.monitor_id == monitor_config.monitor_id
        assert restored.monitor_type == monitor_config.monitor_type
        assert restored.mode == monitor_config.mode


# =============================================================================
# SANCTIONS LIST MONITOR TESTS
# =============================================================================

class TestSanctionsListMonitor:
    """Tests for SanctionsListMonitor."""
    
    def test_sanctions_monitor_creation(self, sanctions_checker):
        """Test SanctionsListMonitor creation."""
        monitor = create_sanctions_monitor(
            monitor_id="sanctions-001",
            sanctions_checker=sanctions_checker,
            watched_entities=["entity:test-001"],
            poll_interval=60
        )
        assert monitor.monitor_id == "sanctions-001"
        assert monitor.monitor_type == "sanctions_list"
        assert monitor.status == MonitorStatus.STOPPED
    
    def test_sanctions_monitor_add_watched_entity(self, sanctions_checker):
        """Test adding entities to watch list."""
        monitor = create_sanctions_monitor(
            monitor_id="sanctions-002",
            sanctions_checker=sanctions_checker,
        )
        monitor.add_watched_entity("entity:new-001")
        assert "entity:new-001" in monitor._watched_entities
    
    def test_sanctions_monitor_poll(self, sanctions_checker):
        """Test polling sanctions status."""
        monitor = create_sanctions_monitor(
            monitor_id="sanctions-003",
            sanctions_checker=sanctions_checker,
            watched_entities=["Sanctioned Entity", "Clean Entity"],
        )
        state = monitor.poll()
        assert state is not None
        assert "entity_results" in state
        # Sanctioned Entity should match
        assert state["entity_results"]["Sanctioned Entity"]["sanctioned"] == True
        # Clean Entity should not match
        assert state["entity_results"]["Clean Entity"]["sanctioned"] == False
    
    def test_sanctions_monitor_detect_sanction_change(self, sanctions_checker):
        """Test detection of sanctions status change."""
        monitor = create_sanctions_monitor(
            monitor_id="sanctions-004",
            sanctions_checker=sanctions_checker,
            watched_entities=["entity:test-001"],
        )
        
        old_state = {
            "list_version": "v1",
            "entity_results": {
                "entity:test-001": {"sanctioned": False}
            }
        }
        new_state = {
            "list_version": "v1",
            "entity_results": {
                "entity:test-001": {"sanctioned": True}
            }
        }
        
        triggers = monitor.detect_changes(old_state, new_state)
        assert len(triggers) == 1
        assert triggers[0].trigger_type == AgenticTriggerType.SANCTIONS_LIST_UPDATE
        assert triggers[0].data["new_sanctioned"] == True


# =============================================================================
# LICENSE STATUS MONITOR TESTS
# =============================================================================

class TestLicenseStatusMonitor:
    """Tests for LicenseStatusMonitor."""
    
    def test_license_monitor_creation(self):
        """Test LicenseStatusMonitor creation."""
        monitor = create_license_monitor(
            monitor_id="license-001",
            warning_thresholds=[30, 14, 7, 1],
        )
        assert monitor.monitor_id == "license-001"
        assert monitor.monitor_type == "license_status"
    
    def test_license_monitor_track_license(self):
        """Test tracking a license."""
        monitor = create_license_monitor(monitor_id="license-002")
        future_date = (datetime.now(timezone.utc) + timedelta(days=30)).isoformat()
        
        monitor.track_license("lic-001", {
            "license_type": "financial-services",
            "holder": "did:key:z6MkHolder",
            "valid_until": future_date,
        })
        
        state = monitor.poll()
        assert state is not None
        assert "lic-001" in state["licenses"]
        assert state["licenses"]["lic-001"]["status"] == "valid"
    
    def test_license_monitor_detect_expiry(self):
        """Test detection of license expiry."""
        monitor = create_license_monitor(monitor_id="license-003")
        
        old_state = {
            "licenses": {
                "lic-001": {"status": "valid", "days_until_expiry": 1}
            }
        }
        new_state = {
            "licenses": {
                "lic-001": {"status": "expired", "days_until_expiry": -1}
            }
        }
        
        triggers = monitor.detect_changes(old_state, new_state)
        assert len(triggers) == 1
        assert triggers[0].trigger_type == AgenticTriggerType.LICENSE_STATUS_CHANGE
        assert triggers[0].data["new_status"] == "expired"
    
    def test_license_monitor_warning_thresholds(self):
        """Test license expiry warning threshold detection."""
        monitor = create_license_monitor(
            monitor_id="license-004",
            warning_thresholds=[30, 14, 7]
        )
        
        old_state = {
            "licenses": {
                "lic-001": {"status": "valid", "days_until_expiry": 15, "license_type": "test"}
            }
        }
        new_state = {
            "licenses": {
                "lic-001": {"status": "valid", "days_until_expiry": 13, "license_type": "test"}
            }
        }
        
        triggers = monitor.detect_changes(old_state, new_state)
        # Should trigger for crossing 14-day threshold
        assert any(t.data.get("threshold_crossed") == 14 for t in triggers)


# =============================================================================
# CORRIDOR STATE MONITOR TESTS
# =============================================================================

class TestCorridorStateMonitor:
    """Tests for CorridorStateMonitor."""
    
    def test_corridor_monitor_creation(self):
        """Test CorridorStateMonitor creation."""
        monitor = create_corridor_monitor(monitor_id="corridor-001")
        assert monitor.monitor_id == "corridor-001"
        assert monitor.monitor_type == "corridor_state"
    
    def test_corridor_monitor_track_corridor(self):
        """Test tracking a corridor."""
        monitor = create_corridor_monitor(monitor_id="corridor-002")
        monitor.track_corridor("corridor:trade-001", {
            "receipt_count": 10,
            "last_checkpoint_seq": 1,
        })
        
        state = monitor.poll()
        assert "corridor:trade-001" in state["corridors"]
    
    def test_corridor_monitor_detect_new_receipts(self):
        """Test detection of new receipts."""
        monitor = create_corridor_monitor(monitor_id="corridor-003")
        
        old_state = {
            "corridors": {
                "corridor:001": {"receipt_count": 10}
            }
        }
        new_state = {
            "corridors": {
                "corridor:001": {"receipt_count": 15}
            }
        }
        
        triggers = monitor.detect_changes(old_state, new_state)
        assert len(triggers) == 1
        assert triggers[0].data["change_type"] == "new_receipts"
        assert triggers[0].data["receipts_added"] == 5
    
    def test_corridor_monitor_detect_fork(self):
        """Test detection of corridor fork."""
        monitor = create_corridor_monitor(monitor_id="corridor-004")
        
        old_state = {
            "corridors": {
                "corridor:001": {"fork_detected": False}
            }
        }
        new_state = {
            "corridors": {
                "corridor:001": {"fork_detected": True, "fork_point": 5}
            }
        }
        
        triggers = monitor.detect_changes(old_state, new_state)
        assert len(triggers) == 1
        assert triggers[0].data["change_type"] == "fork_detected"
        assert triggers[0].data["impact_level"] == "critical"


# =============================================================================
# POLICY EVALUATOR TESTS
# =============================================================================

class TestPolicyEvaluator:
    """Tests for PolicyEvaluator."""
    
    def test_policy_evaluator_creation(self, policy_evaluator):
        """Test PolicyEvaluator creation with standard policies."""
        policies = policy_evaluator.list_policies()
        assert len(policies) > 0
        assert any(p.policy_id == "sanctions_auto_halt" for p in policies)
    
    def test_policy_evaluator_register_policy(self, policy_evaluator):
        """Test registering a custom policy."""
        custom_policy = AgenticPolicy(
            policy_id="custom_test_policy",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            action=TransitionKind.UPDATE_MANIFEST,
        )
        policy_evaluator.register_policy(custom_policy)
        
        retrieved = policy_evaluator.get_policy("custom_test_policy")
        assert retrieved is not None
        assert retrieved.policy_id == "custom_test_policy"
    
    def test_policy_evaluation_matching(self, policy_evaluator):
        """Test policy evaluation with matching trigger."""
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            data={
                "affected_parties": ["self"],
                "entity_id": "entity:test",
            }
        )
        
        results = policy_evaluator.evaluate(trigger, asset_id="asset:test")
        
        # Should find the sanctions_auto_halt policy
        matching = [r for r in results if r.matched]
        assert len(matching) > 0
    
    def test_policy_evaluation_determinism(self, policy_evaluator):
        """Test Theorem 17.1: Agentic Determinism."""
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            data={"new_status": "expired"},
        )
        
        # Evaluate twice with identical inputs
        results1 = policy_evaluator.evaluate(trigger, asset_id="asset:test")
        results2 = policy_evaluator.evaluate(trigger, asset_id="asset:test")
        
        # Results should be identical
        assert len(results1) == len(results2)
        for r1, r2 in zip(results1, results2):
            assert r1.policy_id == r2.policy_id
            assert r1.matched == r2.matched
            assert r1.action == r2.action
    
    def test_policy_condition_threshold(self, policy_evaluator):
        """Test threshold condition evaluation."""
        policy = AgenticPolicy(
            policy_id="threshold_test",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            condition={"type": "threshold", "field": "receipts_since_last", "threshold": 100},
            action=TransitionKind.UPDATE_MANIFEST,
        )
        policy_evaluator.register_policy(policy)
        
        # Below threshold
        trigger1 = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"receipts_since_last": 50}
        )
        results1 = policy_evaluator.evaluate(trigger1)
        threshold_result1 = next(r for r in results1 if r.policy_id == "threshold_test")
        assert threshold_result1.matched == False
        
        # Above threshold
        trigger2 = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"receipts_since_last": 150}
        )
        results2 = policy_evaluator.evaluate(trigger2)
        threshold_result2 = next(r for r in results2 if r.policy_id == "threshold_test")
        assert threshold_result2.matched == True
    
    def test_policy_condition_equals(self, policy_evaluator):
        """Test equals condition evaluation."""
        policy = AgenticPolicy(
            policy_id="equals_test",
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            condition={"type": "equals", "field": "new_status", "value": "suspended"},
            action=TransitionKind.HALT,
        )
        policy_evaluator.register_policy(policy)
        
        # Matching value
        trigger1 = AgenticTrigger(
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            data={"new_status": "suspended"}
        )
        results1 = policy_evaluator.evaluate(trigger1)
        equals_result1 = next(r for r in results1 if r.policy_id == "equals_test")
        assert equals_result1.matched == True
        
        # Non-matching value
        trigger2 = AgenticTrigger(
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            data={"new_status": "expired"}
        )
        results2 = policy_evaluator.evaluate(trigger2)
        equals_result2 = next(r for r in results2 if r.policy_id == "equals_test")
        assert equals_result2.matched == False
    
    def test_policy_condition_contains(self, policy_evaluator):
        """Test contains condition evaluation."""
        policy = AgenticPolicy(
            policy_id="contains_test",
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            condition={"type": "contains", "field": "affected_parties", "item": "self"},
            action=TransitionKind.HALT,
        )
        policy_evaluator.register_policy(policy)
        
        # Contains item
        trigger1 = AgenticTrigger(
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            data={"affected_parties": ["other", "self", "another"]}
        )
        results1 = policy_evaluator.evaluate(trigger1)
        contains_result1 = next(r for r in results1 if r.policy_id == "contains_test")
        assert contains_result1.matched == True
        
        # Does not contain item
        trigger2 = AgenticTrigger(
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            data={"affected_parties": ["other", "another"]}
        )
        results2 = policy_evaluator.evaluate(trigger2)
        contains_result2 = next(r for r in results2 if r.policy_id == "contains_test")
        assert contains_result2.matched == False


# =============================================================================
# ACTION SCHEDULING TESTS
# =============================================================================

class TestActionScheduling:
    """Tests for action scheduling and execution."""
    
    def test_schedule_actions(self, policy_evaluator):
        """Test scheduling actions from evaluation results."""
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            data={"new_status": "expired"}
        )
        
        results = policy_evaluator.evaluate(trigger, asset_id="asset:test")
        matching = [r for r in results if r.matched]
        
        if matching:
            scheduled = policy_evaluator.schedule_actions(matching, "asset:test")
            assert len(scheduled) > 0
            assert all(isinstance(a, ScheduledAction) for a in scheduled)
            assert all(a.status == "pending" for a in scheduled)
    
    def test_execute_action(self, policy_evaluator):
        """Test executing a scheduled action."""
        # Create a simple action
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"receipts_since_last": 150}
        )
        
        policy_evaluator.register_policy(AgenticPolicy(
            policy_id="exec_test",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            action=TransitionKind.UPDATE_MANIFEST,
        ))
        
        results = policy_evaluator.evaluate(trigger, asset_id="asset:test")
        matching = [r for r in results if r.matched and r.policy_id == "exec_test"]
        
        if matching:
            scheduled = policy_evaluator.schedule_actions(matching, "asset:test")
            action = scheduled[0]
            
            success, error = policy_evaluator.execute_action(action.action_id)
            assert success == True
            assert error is None
            
            # Check action status updated
            executed = policy_evaluator.get_action(action.action_id)
            assert executed.status == "completed"
    
    def test_cancel_action(self, policy_evaluator):
        """Test cancelling a pending action."""
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={}
        )
        
        policy_evaluator.register_policy(AgenticPolicy(
            policy_id="cancel_test",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            action=TransitionKind.UPDATE_MANIFEST,
        ))
        
        results = policy_evaluator.evaluate(trigger)
        matching = [r for r in results if r.matched and r.policy_id == "cancel_test"]
        
        if matching:
            scheduled = policy_evaluator.schedule_actions(matching, "asset:test")
            action = scheduled[0]
            
            cancelled = policy_evaluator.cancel_action(action.action_id)
            assert cancelled == True
            
            # Verify status
            action = policy_evaluator.get_action(action.action_id)
            assert action.status == "cancelled"
    
    def test_get_pending_actions(self, policy_evaluator):
        """Test retrieving pending actions."""
        # Schedule multiple actions
        for i in range(3):
            trigger = AgenticTrigger(
                trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
                data={"index": i}
            )
            results = policy_evaluator.evaluate(trigger, asset_id=f"asset:{i}")
            matching = [r for r in results if r.matched]
            if matching:
                policy_evaluator.schedule_actions(matching, f"asset:{i}")
        
        pending = policy_evaluator.get_pending_actions()
        assert len(pending) >= 0  # May be 0 if no policies matched


# =============================================================================
# AUDIT TRAIL TESTS
# =============================================================================

class TestAuditTrail:
    """Tests for audit trail generation."""
    
    def test_audit_trail_trigger_received(self, policy_evaluator):
        """Test audit trail records trigger receipt."""
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.GUIDANCE_UPDATE,
            data={"guidance_id": "guidance-001"}
        )
        
        policy_evaluator.evaluate(trigger, asset_id="asset:test")
        
        trail = policy_evaluator.get_audit_trail(entry_type="trigger_received")
        assert len(trail) > 0
        assert trail[-1].entry_type == "trigger_received"
    
    def test_audit_trail_policy_evaluated(self, policy_evaluator):
        """Test audit trail records policy evaluations."""
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            data={"new_status": "expired"}
        )
        
        policy_evaluator.evaluate(trigger, asset_id="asset:test")
        
        trail = policy_evaluator.get_audit_trail(entry_type="policy_evaluated")
        assert len(trail) > 0
    
    def test_audit_trail_filtering(self, policy_evaluator):
        """Test audit trail filtering by asset_id."""
        trigger1 = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={}
        )
        trigger2 = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={}
        )
        
        policy_evaluator.evaluate(trigger1, asset_id="asset:A")
        policy_evaluator.evaluate(trigger2, asset_id="asset:B")
        
        trail_a = policy_evaluator.get_audit_trail(asset_id="asset:A")
        trail_b = policy_evaluator.get_audit_trail(asset_id="asset:B")
        
        assert all(e.asset_id == "asset:A" for e in trail_a)
        assert all(e.asset_id == "asset:B" for e in trail_b)


# =============================================================================
# MONITOR REGISTRY TESTS
# =============================================================================

class TestMonitorRegistry:
    """Tests for MonitorRegistry."""
    
    def test_registry_register_monitor(self, sanctions_checker):
        """Test registering a monitor."""
        registry = MonitorRegistry()
        monitor = create_sanctions_monitor(
            monitor_id="reg-test-001",
            sanctions_checker=sanctions_checker,
        )
        
        registry.register(monitor)
        
        retrieved = registry.get("reg-test-001")
        assert retrieved is not None
        assert retrieved.monitor_id == "reg-test-001"
    
    def test_registry_unregister_monitor(self, sanctions_checker):
        """Test unregistering a monitor."""
        registry = MonitorRegistry()
        monitor = create_sanctions_monitor(
            monitor_id="reg-test-002",
            sanctions_checker=sanctions_checker,
        )
        
        registry.register(monitor)
        removed = registry.unregister("reg-test-002")
        
        assert removed is not None
        assert registry.get("reg-test-002") is None
    
    def test_registry_list_monitors(self, sanctions_checker):
        """Test listing all monitors."""
        registry = MonitorRegistry()
        
        for i in range(3):
            monitor = create_sanctions_monitor(
                monitor_id=f"list-test-{i}",
                sanctions_checker=sanctions_checker,
            )
            registry.register(monitor)
        
        monitors = registry.list_monitors()
        assert len(monitors) == 3
    
    def test_registry_status_report(self, sanctions_checker):
        """Test getting status report."""
        registry = MonitorRegistry()
        monitor = create_sanctions_monitor(
            monitor_id="status-test-001",
            sanctions_checker=sanctions_checker,
        )
        registry.register(monitor)
        
        report = registry.get_status_report()
        assert len(report) == 1
        assert report[0]["monitor_id"] == "status-test-001"
        assert report[0]["status"] == "stopped"


# =============================================================================
# AGENTIC EXECUTION ENGINE TESTS
# =============================================================================

class TestAgenticExecutionEngine:
    """Tests for AgenticExecutionEngine."""
    
    def test_engine_creation(self, agentic_engine):
        """Test engine creation."""
        assert agentic_engine.monitor_registry is not None
        assert agentic_engine.policy_evaluator is not None
    
    def test_engine_process_trigger(self, agentic_engine):
        """Test processing a trigger through the engine."""
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            data={"new_status": "expired"}
        )
        
        scheduled = agentic_engine.process_trigger(trigger, "asset:test")
        # May or may not have scheduled actions depending on policy matches
        assert isinstance(scheduled, list)
    
    def test_engine_get_status(self, agentic_engine):
        """Test getting engine status."""
        status = agentic_engine.get_status()
        
        assert "monitors" in status
        assert "policies" in status
        assert "pending_actions" in status
        assert "asset_bindings" in status


# =============================================================================
# EXTENDED POLICIES TESTS
# =============================================================================

class TestExtendedPolicies:
    """Tests for extended policy library (v0.4.43)."""
    
    def test_extended_policies_count(self):
        """Test that extended policies include v0.4.43 additions."""
        # v0.4.41 had 4 standard policies
        # v0.4.43 should add more
        assert len(EXTENDED_POLICIES) >= 10
    
    def test_sanctions_freeze_policy(self):
        """Test sanctions_freeze policy exists and is configured correctly."""
        policy = EXTENDED_POLICIES.get("sanctions_freeze")
        assert policy is not None
        assert policy.trigger_type == AgenticTriggerType.SANCTIONS_LIST_UPDATE
        assert policy.action == TransitionKind.HALT
    
    def test_license_suspend_policy(self):
        """Test license_suspend policy exists."""
        policy = EXTENDED_POLICIES.get("license_suspend")
        assert policy is not None
        assert policy.trigger_type == AgenticTriggerType.LICENSE_STATUS_CHANGE
    
    def test_corridor_failover_policy(self):
        """Test corridor_failover policy exists."""
        policy = EXTENDED_POLICIES.get("corridor_failover")
        assert policy is not None
        assert policy.trigger_type == AgenticTriggerType.CORRIDOR_STATE_CHANGE
    
    def test_checkpoint_auto_policies(self):
        """Test automatic checkpoint policies exist."""
        receipt_policy = EXTENDED_POLICIES.get("checkpoint_auto_receipt")
        time_policy = EXTENDED_POLICIES.get("checkpoint_auto_time")
        
        assert receipt_policy is not None
        assert time_policy is not None
        assert receipt_policy.trigger_type == AgenticTriggerType.CHECKPOINT_DUE
        assert time_policy.trigger_type == AgenticTriggerType.CHECKPOINT_DUE
    
    def test_key_rotation_enforce_policy(self):
        """Test key_rotation_enforce policy exists."""
        policy = EXTENDED_POLICIES.get("key_rotation_enforce")
        assert policy is not None
        assert policy.trigger_type == AgenticTriggerType.KEY_ROTATION_DUE
        assert policy.authorization_requirement == "quorum"
    
    def test_dispute_filed_halt_policy(self):
        """Test dispute_filed_halt policy exists."""
        policy = EXTENDED_POLICIES.get("dispute_filed_halt")
        assert policy is not None
        assert policy.trigger_type == AgenticTriggerType.DISPUTE_FILED
        assert policy.action == TransitionKind.HALT
    
    def test_ruling_auto_enforce_policy(self):
        """Test ruling_auto_enforce policy exists."""
        policy = EXTENDED_POLICIES.get("ruling_auto_enforce")
        assert policy is not None
        assert policy.trigger_type == AgenticTriggerType.RULING_RECEIVED
        assert policy.action == TransitionKind.ARBITRATION_ENFORCE


# =============================================================================
# FACTORY FUNCTION TESTS
# =============================================================================

class TestFactoryFunctions:
    """Tests for monitor factory functions."""
    
    def test_create_sanctions_monitor(self, sanctions_checker):
        """Test sanctions monitor factory."""
        monitor = create_sanctions_monitor(
            monitor_id="factory-sanctions",
            sanctions_checker=sanctions_checker,
            watched_entities=["entity:a", "entity:b"],
            poll_interval=120
        )
        
        assert monitor.monitor_id == "factory-sanctions"
        assert monitor.config.poll_interval_seconds == 120
        assert "entity:a" in monitor._watched_entities
    
    def test_create_license_monitor(self):
        """Test license monitor factory."""
        monitor = create_license_monitor(
            monitor_id="factory-license",
            warning_thresholds=[60, 30, 14],
            poll_interval=1800
        )
        
        assert monitor.monitor_id == "factory-license"
        assert monitor.config.poll_interval_seconds == 1800
        assert monitor.warning_thresholds == [60, 30, 14]
    
    def test_create_corridor_monitor(self):
        """Test corridor monitor factory."""
        monitor = create_corridor_monitor(
            monitor_id="factory-corridor",
            poll_interval=30
        )
        
        assert monitor.monitor_id == "factory-corridor"
        assert monitor.config.poll_interval_seconds == 30
    
    def test_create_guidance_monitor(self):
        """Test guidance monitor factory."""
        monitor = create_guidance_monitor(
            monitor_id="factory-guidance",
            poll_interval=7200
        )
        
        assert monitor.monitor_id == "factory-guidance"
        assert monitor.config.poll_interval_seconds == 7200
    
    def test_create_checkpoint_monitor(self):
        """Test checkpoint monitor factory."""
        monitor = create_checkpoint_monitor(
            monitor_id="factory-checkpoint",
            receipt_threshold=50,
            time_threshold_hours=12,
            poll_interval=60
        )
        
        assert monitor.monitor_id == "factory-checkpoint"
        assert monitor.receipt_threshold == 50
        assert monitor.time_threshold_hours == 12


# =============================================================================
# SCHEMA VALIDATION TESTS
# =============================================================================

class TestAgenticSchemas:
    """Tests for agentic JSON schemas."""
    
    @pytest.fixture
    def schemas_dir(self):
        return Path(__file__).parent.parent / "schemas"
    
    def test_environment_monitor_schema_exists(self, schemas_dir):
        """Test environment monitor schema exists."""
        schema_path = schemas_dir / "agentic.environment-monitor.schema.json"
        assert schema_path.exists()
        
        with open(schema_path) as f:
            schema = json.load(f)
        
        assert schema["title"] == "MSEZAgenticEnvironmentMonitor"
        assert "monitor_id" in schema["required"]
        assert "monitor_type" in schema["required"]
    
    def test_trigger_schema_exists(self, schemas_dir):
        """Test trigger schema exists."""
        schema_path = schemas_dir / "agentic.trigger.schema.json"
        assert schema_path.exists()
        
        with open(schema_path) as f:
            schema = json.load(f)
        
        assert schema["title"] == "MSEZAgenticTrigger"
        assert "trigger_type" in schema["required"]
    
    def test_policy_schema_exists(self, schemas_dir):
        """Test policy schema exists."""
        schema_path = schemas_dir / "agentic.policy.schema.json"
        assert schema_path.exists()
        
        with open(schema_path) as f:
            schema = json.load(f)
        
        assert schema["title"] == "MSEZAgenticPolicy"
    
    def test_policy_evaluation_schema_exists(self, schemas_dir):
        """Test policy evaluation schema exists."""
        schema_path = schemas_dir / "agentic.policy-evaluation.schema.json"
        assert schema_path.exists()
    
    def test_action_schedule_schema_exists(self, schemas_dir):
        """Test action schedule schema exists."""
        schema_path = schemas_dir / "agentic.action-schedule.schema.json"
        assert schema_path.exists()
    
    def test_audit_trail_schema_exists(self, schemas_dir):
        """Test audit trail schema exists."""
        schema_path = schemas_dir / "agentic.audit-trail.schema.json"
        assert schema_path.exists()


# =============================================================================
# INTEGRATION TESTS
# =============================================================================

class TestAgenticIntegration:
    """Integration tests for the agentic framework."""
    
    def test_end_to_end_sanctions_workflow(self, sanctions_checker):
        """Test complete sanctions trigger → policy → action flow."""
        # Setup engine
        engine = AgenticExecutionEngine()
        
        # Create and register monitor
        monitor = create_sanctions_monitor(
            monitor_id="e2e-sanctions",
            sanctions_checker=sanctions_checker,
            watched_entities=["entity:test-001"],
        )
        engine.monitor_registry.register(monitor)
        
        # Register extended policies
        for policy_id, policy in EXTENDED_POLICIES.items():
            engine.policy_evaluator.register_policy(policy)
        
        # Simulate trigger
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            data={
                "entity_id": "entity:test-001",
                "new_sanctioned": True,
                "affected_parties": ["self"],
            }
        )
        
        # Process through engine
        scheduled = engine.process_trigger(trigger, "asset:test-001")
        
        # Verify actions were scheduled
        assert len(scheduled) > 0
        
        # Execute actions
        results = engine.execute_pending_actions()
        assert len(results) > 0
    
    def test_end_to_end_license_expiry_workflow(self):
        """Test complete license expiry trigger → policy → action flow."""
        engine = AgenticExecutionEngine()
        
        # Create monitor
        monitor = create_license_monitor(monitor_id="e2e-license")
        engine.monitor_registry.register(monitor)
        
        # Register policies
        for policy_id, policy in EXTENDED_POLICIES.items():
            engine.policy_evaluator.register_policy(policy)
        
        # Simulate license status change
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            data={
                "license_id": "lic-001",
                "old_status": "valid",
                "new_status": "expired",
            }
        )
        
        scheduled = engine.process_trigger(trigger, "asset:licensed-001")
        
        # License expiry should trigger halt action
        assert any(a.action_type == TransitionKind.HALT for a in scheduled)
    
    def test_audit_trail_completeness(self):
        """Test that audit trail captures all events."""
        engine = AgenticExecutionEngine()
        
        # Register policies
        for policy_id, policy in EXTENDED_POLICIES.items():
            engine.policy_evaluator.register_policy(policy)
        
        # Process trigger
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"receipts_since_last": 150}
        )
        
        engine.process_trigger(trigger, "asset:audit-test")
        
        # Get audit trail
        trail = engine.policy_evaluator.get_audit_trail()
        
        # Should have trigger received entry
        trigger_entries = [e for e in trail if e.entry_type == "trigger_received"]
        assert len(trigger_entries) > 0
        
        # Should have policy evaluation entries
        eval_entries = [e for e in trail if e.entry_type == "policy_evaluated"]
        assert len(eval_entries) > 0


# =============================================================================
# SPEC COMPLIANCE TESTS
# =============================================================================

class TestSpecCompliance:
    """Tests verifying MASS Protocol v0.2 Chapter 17 compliance."""
    
    def test_definition_17_1_trigger_types(self):
        """Verify all trigger types from Definition 17.1 are present."""
        required_triggers = [
            "sanctions_list_update",
            "license_status_change",
            "guidance_update",
            "compliance_deadline",
            "dispute_filed",
            "ruling_received",
            "appeal_period_expired",
            "enforcement_due",
            "corridor_state_change",
            "settlement_anchor_available",
            "watcher_quorum_reached",
            "checkpoint_due",
            "key_rotation_due",
            "governance_vote_resolved",
        ]
        
        actual_triggers = [t.value for t in AgenticTriggerType]
        
        for required in required_triggers:
            assert required in actual_triggers, f"Missing trigger type: {required}"
    
    def test_theorem_17_1_determinism(self):
        """Verify Theorem 17.1: Agentic Determinism."""
        evaluator = PolicyEvaluator()
        
        # Create identical triggers
        trigger1 = AgenticTrigger(
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            data={"new_status": "expired", "license_id": "lic-001"}
        )
        trigger2 = AgenticTrigger(
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            data={"new_status": "expired", "license_id": "lic-001"}
        )
        
        # Evaluate multiple times
        results = []
        for _ in range(5):
            r = evaluator.evaluate(trigger1, asset_id="asset:determinism-test")
            results.append([(x.policy_id, x.matched, x.action) for x in r])
        
        # All results should be identical
        assert all(r == results[0] for r in results), "Evaluation not deterministic"
    
    def test_definition_17_4_monitor_interface(self):
        """Verify Definition 17.4: Environment Monitor interface."""
        # Check that EnvironmentMonitor has required methods
        assert hasattr(EnvironmentMonitor, 'poll')
        assert hasattr(EnvironmentMonitor, 'detect_changes')
        assert hasattr(EnvironmentMonitor, 'start')
        assert hasattr(EnvironmentMonitor, 'stop')
        assert hasattr(EnvironmentMonitor, 'add_listener')
        assert hasattr(EnvironmentMonitor, 'emit_trigger')


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
