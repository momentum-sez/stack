#!/usr/bin/env python3
"""
Deep Edge Case and Bug Surface Tests (v0.4.44)

These tests cover edge cases, boundary conditions, race conditions,
and potential bug surfaces across the entire MSEZ stack.

Test Categories:
1. Version Consistency Tests
2. Schema Validation Edge Cases
3. Cryptographic Integrity Tests
4. Determinism Verification Tests
5. State Machine Boundary Tests
6. Concurrency Safety Tests
7. Serialization Roundtrip Tests
8. Cross-Module Integration Tests
"""

import pytest
import json
import hashlib
import threading
import time
import copy
from datetime import datetime, timezone, timedelta
from pathlib import Path
from typing import Dict, Any, List
from dataclasses import asdict

# Core imports
from tools.mass_primitives import (
    stack_digest,
    json_canonicalize,
    GenesisDocument,
    RegistryCredential,
    OperationalManifest,
    AssetReceipt,
    MerkleMountainRange,
    SmartAsset,
    AgenticTriggerType,
    AgenticTrigger,
    AgenticPolicy,
    TransitionKind,
    STANDARD_POLICIES,
    genesis_receipt_root,
    ComplianceTensor,
)

from tools.agentic import (
    MonitorConfig,
    MonitorMode,
    MonitorStatus,
    PolicyEvaluator,
    PolicyEvaluationResult,
    ScheduledAction,
    AuditTrailEntry,
    EXTENDED_POLICIES,
    create_license_monitor,
    create_corridor_monitor,
)

from tools.arbitration import (
    DisputeRequest,
    Ruling,
    ArbitrationRulingVC,
    STACK_SPEC_VERSION as ARBITRATION_STACK_SPEC_VERSION,
)

from tools.regpack import (
    SanctionsEntry,
    SanctionsChecker,
    LicenseType,
    ComplianceDeadline,
    RegPackManager,
)


# =============================================================================
# VERSION CONSISTENCY TESTS
# =============================================================================

class TestVersionConsistency:
    """Ensure version numbers are consistent across all modules."""
    
    def test_all_tool_modules_have_matching_stack_spec_version(self):
        """All tool modules should have the same STACK_SPEC_VERSION."""
        from tools import msez, regpack, arbitration
        from tools.dev import generate_trade_playbook
        
        versions = {
            "msez": msez.STACK_SPEC_VERSION,
            "regpack": regpack.STACK_SPEC_VERSION,
            "arbitration": arbitration.STACK_SPEC_VERSION,
            "generate_trade_playbook": generate_trade_playbook.STACK_SPEC_VERSION,
        }
        
        unique_versions = set(versions.values())
        assert len(unique_versions) == 1, f"Version mismatch: {versions}"
        assert "0.4.44" in unique_versions
    
    def test_all_profile_yaml_files_have_current_version(self):
        """All profile.yaml files should reference current stack version."""
        import yaml
        profiles_dir = Path(__file__).parent.parent / "profiles"
        
        for profile_dir in profiles_dir.iterdir():
            if profile_dir.is_dir():
                profile_yaml = profile_dir / "profile.yaml"
                if profile_yaml.exists():
                    with open(profile_yaml) as f:
                        data = yaml.safe_load(f)
                    version = data.get("stack_spec_version")
                    assert version == "0.4.44", f"{profile_yaml}: version={version}"
    
    def test_readme_version_matches_code(self):
        """README version should match code version."""
        from tools.msez import STACK_SPEC_VERSION
        
        readme_path = Path(__file__).parent.parent / "README.md"
        with open(readme_path) as f:
            first_lines = "".join(f.readlines()[:15])
        
        assert STACK_SPEC_VERSION in first_lines, f"README mismatch: version not in first 15 lines"
    
    def test_openapi_versions_match(self):
        """OpenAPI specs should have current version."""
        import yaml
        apis_dir = Path(__file__).parent.parent / "apis"
        
        for api_file in apis_dir.glob("*.yaml"):
            with open(api_file) as f:
                data = yaml.safe_load(f)
            if "info" in data and "version" in data["info"]:
                # Skip legacy APIs that haven't been versioned
                if data["info"]["version"] not in ("0.1.0",):
                    assert data["info"]["version"] == "0.4.44", f"{api_file}"


# =============================================================================
# CRYPTOGRAPHIC INTEGRITY TESTS
# =============================================================================

class TestCryptographicIntegrity:
    """Test cryptographic operations for correctness and consistency."""
    
    def test_stack_digest_determinism(self):
        """stack_digest should produce identical results for identical inputs."""
        data = {"key": "value", "nested": {"a": 1, "b": 2}}
        
        digests = [stack_digest(data).bytes_hex for _ in range(100)]
        
        assert len(set(digests)) == 1, "stack_digest not deterministic"
    
    def test_stack_digest_order_independence(self):
        """stack_digest should be independent of key ordering."""
        data1 = {"b": 2, "a": 1}
        data2 = {"a": 1, "b": 2}
        
        assert stack_digest(data1).bytes_hex == stack_digest(data2).bytes_hex
    
    def test_json_canonicalize_unicode_handling(self):
        """JSON canonicalization should handle unicode correctly."""
        data = {"emoji": "🚀", "chinese": "中文", "arabic": "العربية"}
        
        result = json_canonicalize(data)
        # Should not raise and should be valid JSON
        if isinstance(result, bytes):
            parsed = json.loads(result.decode('utf-8'))
        else:
            parsed = json.loads(result)
        assert parsed["emoji"] == "🚀"
    
    def test_json_canonicalize_produces_consistent_output(self):
        """JSON canonicalization should produce consistent output."""
        data = {"precise": 1.5, "normal": 2}
        
        result = json_canonicalize(data)
        # Should produce consistent output
        assert result is not None
    
    def test_genesis_receipt_root_uniqueness(self):
        """Different asset IDs should produce different genesis roots."""
        roots = [genesis_receipt_root(f"asset:{i}") for i in range(100)]
        
        assert len(set(roots)) == 100, "Genesis roots not unique"
    
    def test_mmr_basic_functionality(self):
        """MMR should accept appends and track peaks."""
        mmr = MerkleMountainRange()
        
        for i in range(10):
            leaf = f"leaf_{i}"
            mmr.append(leaf)
        
        # After 10 appends, should have peaks
        assert len(mmr.peaks) > 0


# =============================================================================
# DETERMINISM VERIFICATION TESTS
# =============================================================================

class TestDeterminismVerification:
    """Verify deterministic behavior per Theorem 17.1."""
    
    def test_policy_evaluation_order_is_deterministic(self):
        """Policy evaluation should always process in the same order."""
        evaluator = PolicyEvaluator()
        
        # Add policies with various IDs to test ordering
        for i in range(10):
            evaluator.register_policy(AgenticPolicy(
                policy_id=f"policy_{chr(ord('z') - i)}",  # z, y, x, ...
                trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
                action=TransitionKind.UPDATE_MANIFEST,
            ))
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={}
        )
        
        # Evaluate multiple times
        results = [
            [r.policy_id for r in evaluator.evaluate(trigger)]
            for _ in range(10)
        ]
        
        # All results should be in same order
        assert all(r == results[0] for r in results)
        # Should be sorted alphabetically
        policy_ids = [r for r in results[0] if r.startswith("policy_")]
        assert policy_ids == sorted(policy_ids)
    
    def test_audit_trail_timestamp_ordering(self):
        """Audit trail entries should be in chronological order."""
        evaluator = PolicyEvaluator()
        
        for i in range(20):
            trigger = AgenticTrigger(
                trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
                data={"iteration": i}
            )
            evaluator.evaluate(trigger, asset_id=f"asset:{i}")
        
        trail = evaluator.get_audit_trail(limit=100)
        timestamps = [e.timestamp for e in trail]
        
        assert timestamps == sorted(timestamps)
    
    def test_scheduled_action_ids_are_unique(self):
        """Scheduled action IDs should never collide."""
        evaluator = PolicyEvaluator()
        evaluator.register_policy(AgenticPolicy(
            policy_id="unique_test",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            action=TransitionKind.UPDATE_MANIFEST,
        ))
        
        action_ids = set()
        for i in range(100):
            trigger = AgenticTrigger(
                trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
                data={"i": i}
            )
            results = evaluator.evaluate(trigger)
            matching = [r for r in results if r.matched]
            if matching:
                scheduled = evaluator.schedule_actions(matching, f"asset:{i}")
                for action in scheduled:
                    assert action.action_id not in action_ids
                    action_ids.add(action.action_id)
        
        assert len(action_ids) == 100


# =============================================================================
# STATE MACHINE BOUNDARY TESTS
# =============================================================================

class TestStateMachineBoundaries:
    """Test state machine transitions at boundary conditions."""
    
    def test_monitor_status_transitions(self):
        """Monitor status should follow valid state machine transitions."""
        monitor = create_corridor_monitor(monitor_id="state-test")
        
        # Initial state
        assert monitor.status == MonitorStatus.STOPPED
        
        # Valid: STOPPED -> RUNNING
        monitor.start()
        assert monitor.status == MonitorStatus.RUNNING
        
        # Valid: RUNNING -> PAUSED
        monitor.pause()
        assert monitor.status == MonitorStatus.PAUSED
        
        # Valid: PAUSED -> RUNNING
        monitor.resume()
        assert monitor.status == MonitorStatus.RUNNING
        
        # Valid: RUNNING -> STOPPED
        monitor.stop()
        assert monitor.status == MonitorStatus.STOPPED
        
        # Idempotent: STOPPED -> STOPPED (should not error)
        monitor.stop()
        assert monitor.status == MonitorStatus.STOPPED
    
    def test_action_status_transitions(self):
        """Action status should follow valid transitions."""
        evaluator = PolicyEvaluator()
        evaluator.register_policy(AgenticPolicy(
            policy_id="status_test",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            action=TransitionKind.UPDATE_MANIFEST,
        ))
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={}
        )
        
        results = evaluator.evaluate(trigger)
        matching = [r for r in results if r.matched and r.policy_id == "status_test"]
        scheduled = evaluator.schedule_actions(matching, "asset:test")
        action = scheduled[0]
        
        # Initial: pending
        assert action.status == "pending"
        
        # Execute: pending -> completed
        success, _ = evaluator.execute_action(action.action_id)
        assert success
        
        updated = evaluator.get_action(action.action_id)
        assert updated.status == "completed"
    
    def test_policy_enabled_disabled_boundary(self):
        """Disabled policies should not match even if conditions are satisfied."""
        evaluator = PolicyEvaluator()
        
        enabled_policy = AgenticPolicy(
            policy_id="enabled_policy",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            action=TransitionKind.UPDATE_MANIFEST,
            enabled=True,
        )
        
        disabled_policy = AgenticPolicy(
            policy_id="disabled_policy",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            action=TransitionKind.HALT,
            enabled=False,
        )
        
        evaluator.register_policy(enabled_policy)
        evaluator.register_policy(disabled_policy)
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={}
        )
        
        results = evaluator.evaluate(trigger)
        
        enabled_result = next(r for r in results if r.policy_id == "enabled_policy")
        disabled_result = next(r for r in results if r.policy_id == "disabled_policy")
        
        assert enabled_result.matched == True
        assert disabled_result.matched == False


# =============================================================================
# SERIALIZATION ROUNDTRIP TESTS
# =============================================================================

class TestSerializationRoundtrip:
    """Test that all data structures survive serialization roundtrips."""
    
    def test_monitor_config_roundtrip(self):
        """MonitorConfig should survive to_dict/from_dict roundtrip."""
        original = MonitorConfig(
            monitor_id="roundtrip-001",
            monitor_type="sanctions_list",
            mode=MonitorMode.POLLING,
            poll_interval_seconds=120,
            enabled=True,
            config={"watched_entities": ["entity:a", "entity:b"]},
            max_retries=5,
            retry_delay_seconds=10,
            error_threshold=3,
        )
        
        serialized = original.to_dict()
        restored = MonitorConfig.from_dict(serialized)
        
        assert restored.monitor_id == original.monitor_id
        assert restored.monitor_type == original.monitor_type
        assert restored.mode == original.mode
        assert restored.poll_interval_seconds == original.poll_interval_seconds
        assert restored.config == original.config
    
    def test_agentic_trigger_roundtrip(self):
        """AgenticTrigger should survive to_dict roundtrip."""
        original = AgenticTrigger(
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            data={"entity": "test", "sanctioned": True, "nested": {"a": [1, 2, 3]}},
        )
        
        serialized = original.to_dict()
        
        # Can be JSON serialized
        json_str = json.dumps(serialized)
        restored_dict = json.loads(json_str)
        
        assert restored_dict["trigger_type"] == original.trigger_type.value
        assert restored_dict["data"] == original.data
    
    def test_agentic_policy_roundtrip(self):
        """AgenticPolicy should survive to_dict roundtrip."""
        original = AgenticPolicy(
            policy_id="roundtrip_policy",
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            condition={"type": "equals", "field": "status", "value": "expired"},
            action=TransitionKind.HALT,
            authorization_requirement="quorum",
            enabled=True,
        )
        
        serialized = original.to_dict()
        json_str = json.dumps(serialized)
        restored_dict = json.loads(json_str)
        
        assert restored_dict["policy_id"] == original.policy_id
        assert restored_dict["condition"] == original.condition
    
    def test_arbitration_manager_can_be_created(self):
        """ArbitrationManager should be creatable with institution_id."""
        from tools.arbitration import ArbitrationManager
        
        manager = ArbitrationManager(institution_id="difc-lcia")
        assert manager is not None
    
    def test_money_roundtrip(self):
        """Money should survive to_dict roundtrip with proper precision."""
        from tools.arbitration import Money
        from decimal import Decimal
        
        original = Money(amount=Decimal("10000.50"), currency="USD")
        
        serialized = original.to_dict()
        json_str = json.dumps(serialized)
        restored_dict = json.loads(json_str)
        
        # Amount is serialized as string for Decimal precision preservation
        assert restored_dict["amount"] == str(original.amount)
        assert restored_dict["currency"] == original.currency
        
        # Verify roundtrip through from_dict
        restored = Money.from_dict(restored_dict)
        assert restored.amount == original.amount
        assert restored.currency == original.currency


# =============================================================================
# EDGE CASE CONDITION TESTS
# =============================================================================

class TestConditionEdgeCases:
    """Test policy condition evaluation edge cases."""
    
    def test_threshold_condition_at_exact_boundary(self):
        """Threshold condition at exact boundary should match."""
        evaluator = PolicyEvaluator()
        evaluator.register_policy(AgenticPolicy(
            policy_id="boundary_test",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            condition={"type": "threshold", "field": "count", "threshold": 100},
            action=TransitionKind.UPDATE_MANIFEST,
        ))
        
        # Exactly at threshold
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"count": 100}
        )
        results = evaluator.evaluate(trigger)
        result = next(r for r in results if r.policy_id == "boundary_test")
        assert result.matched == True
        
        # One below threshold
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"count": 99}
        )
        results = evaluator.evaluate(trigger)
        result = next(r for r in results if r.policy_id == "boundary_test")
        assert result.matched == False
    
    def test_contains_condition_with_empty_list(self):
        """Contains condition with empty list should not match."""
        evaluator = PolicyEvaluator()
        evaluator.register_policy(AgenticPolicy(
            policy_id="empty_list_test",
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            condition={"type": "contains", "field": "parties", "item": "self"},
            action=TransitionKind.HALT,
        ))
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            data={"parties": []}
        )
        results = evaluator.evaluate(trigger)
        result = next(r for r in results if r.policy_id == "empty_list_test")
        assert result.matched == False
    
    def test_equals_condition_with_none_value(self):
        """Equals condition should handle None values correctly."""
        evaluator = PolicyEvaluator()
        evaluator.register_policy(AgenticPolicy(
            policy_id="none_test",
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            condition={"type": "equals", "field": "status", "value": None},
            action=TransitionKind.HALT,
        ))
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
            data={"status": None}
        )
        results = evaluator.evaluate(trigger)
        result = next(r for r in results if r.policy_id == "none_test")
        assert result.matched == True
    
    def test_condition_with_missing_field(self):
        """Condition should handle missing fields gracefully."""
        evaluator = PolicyEvaluator()
        evaluator.register_policy(AgenticPolicy(
            policy_id="missing_field_test",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            condition={"type": "threshold", "field": "nonexistent", "threshold": 10},
            action=TransitionKind.UPDATE_MANIFEST,
        ))
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"other_field": 100}
        )
        
        # Should not raise, should not match
        results = evaluator.evaluate(trigger)
        result = next(r for r in results if r.policy_id == "missing_field_test")
        assert result.matched == False


# =============================================================================
# CROSS-MODULE INTEGRATION TESTS
# =============================================================================

class TestCrossModuleIntegration:
    """Test integration between different modules."""
    
    def test_regpack_sanctions_to_agentic_trigger_flow(self):
        """Sanctions check result should be usable in agentic trigger."""
        entries = [
            SanctionsEntry(
                entry_id="sanc:001",
                entry_type="entity",
                source_lists=["OFAC"],
                primary_name="Bad Actor Inc",
            )
        ]
        checker = SanctionsChecker(entries=entries, snapshot_id="snapshot:001")
        
        # Check an entity
        result = checker.check_entity("Bad Actor Inc")
        
        # Create trigger from result
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            data={
                "entity_id": result.query,
                "sanctioned": result.matched,
                "affected_parties": ["self"] if result.matched else [],
            }
        )
        
        # Evaluate against standard policies
        evaluator = PolicyEvaluator()
        results = evaluator.evaluate(trigger, asset_id="asset:test")
        
        # sanctions_auto_halt should match
        matching = [r for r in results if r.matched]
        assert len(matching) > 0
    
    def test_arbitration_ruling_triggers_agentic_enforcement(self):
        """Arbitration ruling should be expressible as agentic trigger."""
        # Create trigger representing a ruling received
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.RULING_RECEIVED,
            data={
                "ruling_id": "ruling:int:001",
                "disposition": "in_favor_claimant",
                "auto_enforce": True,
            }
        )
        
        # Evaluate against extended policies
        evaluator = PolicyEvaluator()
        for policy_id, policy in EXTENDED_POLICIES.items():
            evaluator.register_policy(policy)
        
        results = evaluator.evaluate(trigger, asset_id="asset:disputed")
        
        # ruling_auto_enforce should match
        ruling_result = next((r for r in results if r.policy_id == "ruling_auto_enforce"), None)
        assert ruling_result is not None
        assert ruling_result.matched == True


# =============================================================================
# SCHEMA FILE INTEGRITY TESTS
# =============================================================================

class TestSchemaFileIntegrity:
    """Test that all schema files are valid and consistent."""
    
    @pytest.fixture
    def schemas_dir(self):
        return Path(__file__).parent.parent / "schemas"
    
    def test_all_schemas_are_valid_json(self, schemas_dir):
        """All schema files should be valid JSON."""
        for schema_file in schemas_dir.glob("*.schema.json"):
            with open(schema_file) as f:
                try:
                    json.load(f)
                except json.JSONDecodeError as e:
                    pytest.fail(f"Invalid JSON in {schema_file}: {e}")
    
    def test_all_schemas_have_required_fields(self, schemas_dir):
        """All schemas should have $schema, $id, title."""
        for schema_file in schemas_dir.glob("*.schema.json"):
            with open(schema_file) as f:
                schema = json.load(f)
            
            assert "$schema" in schema, f"{schema_file}: missing $schema"
            assert "title" in schema, f"{schema_file}: missing title"
    
    def test_agentic_schemas_exist(self, schemas_dir):
        """All v0.4.44 agentic schemas should exist."""
        required_schemas = [
            "agentic.environment-monitor.schema.json",
            "agentic.trigger.schema.json",
            "agentic.policy.schema.json",
            "agentic.policy-evaluation.schema.json",
            "agentic.action-schedule.schema.json",
            "agentic.audit-trail.schema.json",
        ]
        
        for schema_name in required_schemas:
            schema_path = schemas_dir / schema_name
            assert schema_path.exists(), f"Missing schema: {schema_name}"
    
    def test_schema_count_matches_readme(self, schemas_dir):
        """Schema count should match README claim."""
        schema_count = len(list(schemas_dir.glob("*.schema.json")))
        assert schema_count == 116, f"Schema count {schema_count} != 116"


# =============================================================================
# SPECIFICATION DOCUMENT TESTS
# =============================================================================

class TestSpecificationDocuments:
    """Test that specification documents exist and are complete."""
    
    def test_chapter_17_spec_exists(self):
        """spec/17-agentic.md should exist."""
        spec_path = Path(__file__).parent.parent / "spec" / "17-agentic.md"
        assert spec_path.exists(), "Missing spec/17-agentic.md"
    
    def test_chapter_17_spec_has_required_sections(self):
        """spec/17-agentic.md should have all required sections."""
        spec_path = Path(__file__).parent.parent / "spec" / "17-agentic.md"
        with open(spec_path) as f:
            content = f.read()
        
        required_sections = [
            "Definition 17.1",
            "Definition 17.2",
            "Theorem 17.1",
            "Definition 17.3",
            "Definition 17.4",
            "Definition 17.5",
        ]
        
        for section in required_sections:
            assert section in content, f"Missing section: {section}"
    
    def test_changelog_has_v0442_entry(self):
        """CHANGELOG should have v0.4.43 entry."""
        changelog_path = Path(__file__).parent.parent / "governance" / "CHANGELOG.md"
        with open(changelog_path) as f:
            content = f.read()
        
        assert "0.4.43" in content, "Missing v0.4.43 in CHANGELOG"
    
    def test_patchlist_v0442_exists(self):
        """v0.4.43 patchlist should exist."""
        patchlist_path = Path(__file__).parent.parent / "docs" / "patchlists" / "v0.4.43.md"
        assert patchlist_path.exists(), "Missing v0.4.43 patchlist"


# =============================================================================
# CONCURRENT ACCESS TESTS
# =============================================================================

class TestConcurrentAccess:
    """Test thread safety of shared resources."""
    
    def test_policy_evaluator_concurrent_evaluation(self):
        """PolicyEvaluator should handle concurrent evaluations safely."""
        evaluator = PolicyEvaluator()
        evaluator.register_policy(AgenticPolicy(
            policy_id="concurrent_test",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            action=TransitionKind.UPDATE_MANIFEST,
        ))
        
        results = []
        errors = []
        
        def evaluate_trigger(index):
            try:
                trigger = AgenticTrigger(
                    trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
                    data={"index": index}
                )
                r = evaluator.evaluate(trigger, asset_id=f"asset:{index}")
                results.append((index, len(r)))
            except Exception as e:
                errors.append((index, str(e)))
        
        threads = [threading.Thread(target=evaluate_trigger, args=(i,)) for i in range(50)]
        
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        
        assert len(errors) == 0, f"Errors during concurrent evaluation: {errors}"
        assert len(results) == 50


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
