#!/usr/bin/env python3
"""
Agentic Execution Framework (v0.4.44 GENESIS)

MASS Protocol v0.2 Chapter 17 — Agentic Execution

This module implements the complete agentic execution framework including:
- Environment monitors for observing external state changes
- Policy evaluation engine for deterministic trigger → action mapping
- Action scheduling with retry semantics
- Audit trail generation for compliance

Definition 17.1 (Agentic Trigger):
    Environmental events that may cause autonomous state transitions.

Definition 17.2 (Agentic Policy):
    Mappings from triggers to authorized transitions.

Theorem 17.1 (Agentic Determinism):
    Given identical trigger events and environment state, agentic execution
    is deterministic and produces identical state transitions.
"""

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime, timezone, timedelta
from enum import Enum
from typing import Any, Dict, List, Optional, Callable, Tuple, Set
import hashlib
import json
import threading
import uuid


# =============================================================================
# IMPORTS FROM MASS PRIMITIVES
# =============================================================================

from tools.mass_primitives import (
    AgenticTriggerType,
    AgenticTrigger,
    AgenticPolicy,
    ImpactLevel,
    LicenseStatus,
    RulingDisposition,
    TransitionKind,
    STANDARD_POLICIES,
    stack_digest,
    json_canonicalize,
    SmartAsset,
)

from tools.regpack import (
    SanctionsChecker,
    RegPackManager,
    SanctionsEntry,
)


# =============================================================================
# MONITOR STATUS AND CONFIGURATION
# =============================================================================

class MonitorStatus(Enum):
    """Status of an environment monitor."""
    STOPPED = "stopped"
    STARTING = "starting"
    RUNNING = "running"
    STOPPING = "stopping"
    ERROR = "error"
    PAUSED = "paused"


class MonitorMode(Enum):
    """Operating mode for environment monitors."""
    POLLING = "polling"       # Periodic polling
    WEBHOOK = "webhook"       # Event-driven
    HYBRID = "hybrid"         # Both modes


@dataclass
class MonitorConfig:
    """
    Configuration for an environment monitor.
    
    Per Definition 17.4 (Environment Monitor Configuration).
    """
    monitor_id: str
    monitor_type: str
    mode: MonitorMode = MonitorMode.POLLING
    poll_interval_seconds: int = 60
    enabled: bool = True
    config: Dict[str, Any] = field(default_factory=dict)
    
    # Retry configuration
    max_retries: int = 3
    retry_delay_seconds: int = 5
    
    # Alert thresholds
    error_threshold: int = 5  # Consecutive errors before alert
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "monitor_id": self.monitor_id,
            "monitor_type": self.monitor_type,
            "mode": self.mode.value,
            "poll_interval_seconds": self.poll_interval_seconds,
            "enabled": self.enabled,
            "config": self.config,
            "max_retries": self.max_retries,
            "retry_delay_seconds": self.retry_delay_seconds,
            "error_threshold": self.error_threshold,
        }
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'MonitorConfig':
        return cls(
            monitor_id=data["monitor_id"],
            monitor_type=data["monitor_type"],
            mode=MonitorMode(data.get("mode", "polling")),
            poll_interval_seconds=data.get("poll_interval_seconds", 60),
            enabled=data.get("enabled", True),
            config=data.get("config", {}),
            max_retries=data.get("max_retries", 3),
            retry_delay_seconds=data.get("retry_delay_seconds", 5),
            error_threshold=data.get("error_threshold", 5),
        )


# =============================================================================
# ENVIRONMENT MONITOR BASE CLASS
# =============================================================================

class EnvironmentMonitor(ABC):
    """
    Definition 17.4 (Environment Monitor).
    
    Abstract base class for environment monitors that observe external
    conditions and emit triggers when state changes occur.
    
    Environment monitors implement the Observer pattern, continuously
    watching for changes and emitting AgenticTrigger events.
    """
    
    def __init__(self, config: MonitorConfig):
        self.config = config
        self.status = MonitorStatus.STOPPED
        self.last_poll_time: Optional[datetime] = None
        self.last_state: Optional[Dict[str, Any]] = None
        self.last_error: Optional[str] = None
        self.last_error_time: Optional[datetime] = None
        self.consecutive_errors = 0
        self._listeners: List[Callable[[AgenticTrigger], None]] = []
        self._poll_thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()
    
    def _record_error(self, error: Exception, context: str = "") -> None:
        """Record an error for debugging and monitoring purposes."""
        self.consecutive_errors += 1
        self.last_error = f"{context}: {type(error).__name__}: {error}" if context else f"{type(error).__name__}: {error}"
        self.last_error_time = datetime.now(timezone.utc)
        
    @property
    def monitor_id(self) -> str:
        return self.config.monitor_id
    
    @property
    def monitor_type(self) -> str:
        return self.config.monitor_type
    
    @abstractmethod
    def poll(self) -> Optional[Dict[str, Any]]:
        """
        Poll for current state.
        
        Returns the current state, or None if polling failed.
        Implementations should handle their own error recovery.
        """
        pass
    
    @abstractmethod
    def detect_changes(
        self, 
        old_state: Optional[Dict[str, Any]], 
        new_state: Dict[str, Any]
    ) -> List[AgenticTrigger]:
        """
        Detect changes between old and new state.
        
        Returns a list of triggers for any detected changes.
        """
        pass
    
    def add_listener(self, callback: Callable[[AgenticTrigger], None]) -> None:
        """Add a trigger listener."""
        self._listeners.append(callback)
    
    def remove_listener(self, callback: Callable[[AgenticTrigger], None]) -> None:
        """Remove a trigger listener."""
        if callback in self._listeners:
            self._listeners.remove(callback)
    
    def emit_trigger(self, trigger: AgenticTrigger) -> None:
        """Emit a trigger to all listeners."""
        for listener in self._listeners:
            try:
                listener(trigger)
            except Exception as e:
                # Log but don't fail on listener errors
                pass
    
    def start(self) -> None:
        """Start the monitor."""
        if self.status == MonitorStatus.RUNNING:
            return
        
        self.status = MonitorStatus.STARTING
        self._stop_event.clear()
        
        if self.config.mode in (MonitorMode.POLLING, MonitorMode.HYBRID):
            self._poll_thread = threading.Thread(
                target=self._poll_loop,
                daemon=True
            )
            self._poll_thread.start()
        
        self.status = MonitorStatus.RUNNING
    
    def stop(self) -> None:
        """Stop the monitor."""
        if self.status == MonitorStatus.STOPPED:
            return
        
        self.status = MonitorStatus.STOPPING
        self._stop_event.set()
        
        if self._poll_thread:
            self._poll_thread.join(timeout=5)
            self._poll_thread = None
        
        self.status = MonitorStatus.STOPPED
    
    def pause(self) -> None:
        """Pause the monitor temporarily."""
        if self.status == MonitorStatus.RUNNING:
            self.status = MonitorStatus.PAUSED
    
    def resume(self) -> None:
        """Resume a paused monitor."""
        if self.status == MonitorStatus.PAUSED:
            self.status = MonitorStatus.RUNNING
    
    def recover(self) -> bool:
        """
        Attempt to recover from ERROR state.
        
        CRITICAL FIX: Previously there was no way to recover from ERROR state.
        This method resets error counters and attempts to resume normal operation.
        
        Returns True if recovery was successful.
        """
        if self.status != MonitorStatus.ERROR:
            return False
        
        # Reset error counters
        self.consecutive_errors = 0
        
        # Attempt a single poll to verify recovery
        try:
            new_state = self.poll()
            if new_state is not None:
                self.last_state = new_state
                self.last_poll_time = datetime.now(timezone.utc)
                self.status = MonitorStatus.RUNNING
                return True
            else:
                # Poll returned None but didn't throw - stay in error
                return False
        except Exception as e:
            # Poll threw exception - recovery failed, record for debugging
            self._record_error(e, "recover_attempt")
            return False
    
    def reset(self) -> None:
        """
        Full reset of monitor state.
        
        Stops the monitor, clears all state, and resets to STOPPED status.
        """
        self.stop()
        self.last_state = None
        self.last_poll_time = None
        self.last_error = None
        self.last_error_time = None
        self.consecutive_errors = 0
        self._listeners.clear()
    
    def _poll_loop(self) -> None:
        """Internal polling loop."""
        while not self._stop_event.is_set():
            if self.status == MonitorStatus.RUNNING:
                try:
                    new_state = self.poll()
                    if new_state is not None:
                        triggers = self.detect_changes(self.last_state, new_state)
                        for trigger in triggers:
                            self.emit_trigger(trigger)
                        self.last_state = new_state
                        self.last_poll_time = datetime.now(timezone.utc)
                        self.consecutive_errors = 0
                    else:
                        self.consecutive_errors += 1
                        if self.consecutive_errors >= self.config.error_threshold:
                            self.status = MonitorStatus.ERROR
                except Exception as e:
                    self.consecutive_errors += 1
                    if self.consecutive_errors >= self.config.error_threshold:
                        self.status = MonitorStatus.ERROR
            
            # Wait for poll interval or stop event
            self._stop_event.wait(timeout=self.config.poll_interval_seconds)
    
    def get_status(self) -> Dict[str, Any]:
        """Get current monitor status including error tracking."""
        return {
            "monitor_id": self.monitor_id,
            "monitor_type": self.monitor_type,
            "status": self.status.value,
            "last_poll_time": self.last_poll_time.isoformat() if self.last_poll_time else None,
            "consecutive_errors": self.consecutive_errors,
            "last_error": self.last_error,
            "last_error_time": self.last_error_time.isoformat() if self.last_error_time else None,
            "config": self.config.to_dict(),
        }


# =============================================================================
# SANCTIONS LIST MONITOR
# =============================================================================

class SanctionsListMonitor(EnvironmentMonitor):
    """
    Monitor for sanctions list updates (OFAC, EU, UN).
    
    Emits SANCTIONS_LIST_UPDATE triggers when:
    - New entries are added to sanctions lists
    - Existing entries are modified
    - Entries are removed (de-listing)
    """
    
    def __init__(self, config: MonitorConfig, sanctions_checker: SanctionsChecker):
        super().__init__(config)
        self.sanctions_checker = sanctions_checker
        self._watched_entities: Set[str] = set()
        
        # Load watched entities from config
        for entity in config.config.get("watched_entities", []):
            self._watched_entities.add(entity)
    
    def add_watched_entity(self, entity_id: str) -> None:
        """Add an entity to the watch list."""
        self._watched_entities.add(entity_id)
    
    def remove_watched_entity(self, entity_id: str) -> None:
        """Remove an entity from the watch list."""
        self._watched_entities.discard(entity_id)
    
    def poll(self) -> Optional[Dict[str, Any]]:
        """Poll current sanctions state for watched entities."""
        try:
            results = {}
            for entity_id in self._watched_entities:
                check_result = self.sanctions_checker.check_entity(entity_id)
                # check_entity returns SanctionsCheckResult with 'matched' field
                is_sanctioned = check_result.matched
                results[entity_id] = {
                    "sanctioned": is_sanctioned,
                    "checked_at": datetime.now(timezone.utc).isoformat(),
                }
            
            return {
                "list_version": self.sanctions_checker.snapshot_id,
                "last_updated": datetime.now(timezone.utc).isoformat(),
                "entity_results": results,
            }
        except Exception as e:
            self._record_error(e, "SanctionsListMonitor.poll")
            return None
    
    def detect_changes(
        self, 
        old_state: Optional[Dict[str, Any]], 
        new_state: Dict[str, Any]
    ) -> List[AgenticTrigger]:
        """Detect sanctions status changes."""
        triggers = []
        
        if old_state is None:
            # First poll - no changes to detect
            return triggers
        
        old_results = old_state.get("entity_results", {})
        new_results = new_state.get("entity_results", {})
        
        # Check for status changes
        for entity_id, new_status in new_results.items():
            old_status = old_results.get(entity_id, {})
            
            if old_status.get("sanctioned") != new_status.get("sanctioned"):
                trigger = AgenticTrigger(
                    trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
                    data={
                        "entity_id": entity_id,
                        "old_sanctioned": old_status.get("sanctioned"),
                        "new_sanctioned": new_status.get("sanctioned"),
                        "list_version": new_state.get("list_version"),
                        "impact_level": ImpactLevel.CRITICAL.value if new_status.get("sanctioned") else ImpactLevel.MEDIUM.value,
                        "affected_parties": [entity_id] if new_status.get("sanctioned") else [],
                    }
                )
                triggers.append(trigger)
        
        # Check for list version changes (bulk update)
        if old_state.get("list_version") != new_state.get("list_version"):
            trigger = AgenticTrigger(
                trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
                data={
                    "update_type": "list_version_change",
                    "old_version": old_state.get("list_version"),
                    "new_version": new_state.get("list_version"),
                    "impact_level": ImpactLevel.MEDIUM.value,
                }
            )
            triggers.append(trigger)
        
        return triggers


# =============================================================================
# LICENSE STATUS MONITOR
# =============================================================================

class LicenseStatusMonitor(EnvironmentMonitor):
    """
    Monitor for license status changes.
    
    Emits LICENSE_STATUS_CHANGE triggers when:
    - License expires
    - License is renewed
    - License is suspended/revoked
    - License expiry is approaching (configurable warning period)
    """
    
    def __init__(self, config: MonitorConfig, regpack_manager: Optional[RegPackManager] = None):
        super().__init__(config)
        self.regpack_manager = regpack_manager
        self._tracked_licenses: Dict[str, Dict[str, Any]] = {}
        
        # Warning thresholds (days before expiry)
        self.warning_thresholds = config.config.get("warning_thresholds", [30, 14, 7, 1])
    
    def track_license(self, license_id: str, license_data: Dict[str, Any]) -> None:
        """Add a license to track."""
        self._tracked_licenses[license_id] = license_data
    
    def untrack_license(self, license_id: str) -> None:
        """Stop tracking a license."""
        self._tracked_licenses.pop(license_id, None)
    
    def poll(self) -> Optional[Dict[str, Any]]:
        """Poll current license statuses."""
        try:
            now = datetime.now(timezone.utc)
            results = {}
            
            for license_id, license_data in self._tracked_licenses.items():
                expiry_str = license_data.get("valid_until")
                if expiry_str:
                    expiry = datetime.fromisoformat(expiry_str.replace('Z', '+00:00'))
                    days_until_expiry = (expiry - now).days
                    
                    if days_until_expiry < 0:
                        status = LicenseStatus.EXPIRED
                    elif license_data.get("suspended"):
                        status = LicenseStatus.SUSPENDED
                    elif license_data.get("revoked"):
                        status = LicenseStatus.REVOKED
                    else:
                        status = LicenseStatus.VALID
                else:
                    days_until_expiry = None
                    status = LicenseStatus.VALID
                
                results[license_id] = {
                    "status": status.value,
                    "days_until_expiry": days_until_expiry,
                    "license_type": license_data.get("license_type"),
                    "holder": license_data.get("holder"),
                    "checked_at": now.isoformat(),
                }
            
            return {
                "licenses": results,
                "poll_time": now.isoformat(),
            }
        except Exception as e:
            self._record_error(e, "LicenseExpiryMonitor.poll")
            return None
    
    def detect_changes(
        self, 
        old_state: Optional[Dict[str, Any]], 
        new_state: Dict[str, Any]
    ) -> List[AgenticTrigger]:
        """Detect license status changes."""
        triggers = []
        
        if old_state is None:
            return triggers
        
        old_licenses = old_state.get("licenses", {})
        new_licenses = new_state.get("licenses", {})
        
        for license_id, new_data in new_licenses.items():
            old_data = old_licenses.get(license_id, {})
            
            # Status change
            if old_data.get("status") != new_data.get("status"):
                trigger = AgenticTrigger(
                    trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
                    data={
                        "license_id": license_id,
                        "old_status": old_data.get("status"),
                        "new_status": new_data.get("status"),
                        "license_type": new_data.get("license_type"),
                        "holder": new_data.get("holder"),
                        "impact_level": self._get_status_change_impact(
                            old_data.get("status"),
                            new_data.get("status")
                        ),
                    }
                )
                triggers.append(trigger)
            
            # Expiry warning thresholds
            old_days = old_data.get("days_until_expiry")
            new_days = new_data.get("days_until_expiry")
            
            if new_days is not None and old_days is not None:
                for threshold in self.warning_thresholds:
                    if old_days > threshold >= new_days:
                        trigger = AgenticTrigger(
                            trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
                            data={
                                "license_id": license_id,
                                "warning_type": "expiry_approaching",
                                "days_until_expiry": new_days,
                                "threshold_crossed": threshold,
                                "license_type": new_data.get("license_type"),
                                "holder": new_data.get("holder"),
                                "impact_level": ImpactLevel.MEDIUM.value if threshold > 7 else ImpactLevel.HIGH.value,
                            }
                        )
                        triggers.append(trigger)
        
        return triggers
    
    def _get_status_change_impact(self, old_status: str, new_status: str) -> str:
        """Determine impact level of a status change."""
        if new_status in (LicenseStatus.EXPIRED.value, LicenseStatus.REVOKED.value):
            return ImpactLevel.CRITICAL.value
        elif new_status == LicenseStatus.SUSPENDED.value:
            return ImpactLevel.HIGH.value
        elif new_status == LicenseStatus.VALID.value and old_status in (
            LicenseStatus.EXPIRED.value, 
            LicenseStatus.SUSPENDED.value
        ):
            return ImpactLevel.MEDIUM.value  # Renewal/restoration
        return ImpactLevel.LOW.value


# =============================================================================
# CORRIDOR STATE MONITOR
# =============================================================================

class CorridorStateMonitor(EnvironmentMonitor):
    """
    Monitor for corridor state changes.
    
    Emits CORRIDOR_STATE_CHANGE triggers when:
    - New receipts are added
    - Checkpoints are created
    - Fork alarms are raised
    - Settlement anchors become available
    """
    
    def __init__(self, config: MonitorConfig):
        super().__init__(config)
        self._tracked_corridors: Dict[str, Dict[str, Any]] = {}
    
    def track_corridor(self, corridor_id: str, initial_state: Dict[str, Any]) -> None:
        """Add a corridor to track."""
        self._tracked_corridors[corridor_id] = initial_state
    
    def untrack_corridor(self, corridor_id: str) -> None:
        """Stop tracking a corridor."""
        self._tracked_corridors.pop(corridor_id, None)
    
    def update_corridor_state(self, corridor_id: str, new_state: Dict[str, Any]) -> None:
        """Update tracked corridor state (for push-based updates)."""
        if corridor_id in self._tracked_corridors:
            self._tracked_corridors[corridor_id] = new_state
    
    def poll(self) -> Optional[Dict[str, Any]]:
        """Poll current corridor states."""
        try:
            return {
                "corridors": dict(self._tracked_corridors),
                "poll_time": datetime.now(timezone.utc).isoformat(),
            }
        except Exception as e:
            self._record_error(e, "CorridorStateMonitor.poll")
            return None
    
    def detect_changes(
        self, 
        old_state: Optional[Dict[str, Any]], 
        new_state: Dict[str, Any]
    ) -> List[AgenticTrigger]:
        """Detect corridor state changes."""
        triggers = []
        
        if old_state is None:
            return triggers
        
        old_corridors = old_state.get("corridors", {})
        new_corridors = new_state.get("corridors", {})
        
        for corridor_id, new_data in new_corridors.items():
            old_data = old_corridors.get(corridor_id, {})
            
            # Receipt count change
            old_receipts = old_data.get("receipt_count", 0)
            new_receipts = new_data.get("receipt_count", 0)
            
            if new_receipts > old_receipts:
                trigger = AgenticTrigger(
                    trigger_type=AgenticTriggerType.CORRIDOR_STATE_CHANGE,
                    data={
                        "corridor_id": corridor_id,
                        "change_type": "new_receipts",
                        "old_receipt_count": old_receipts,
                        "new_receipt_count": new_receipts,
                        "receipts_added": new_receipts - old_receipts,
                        "impact_level": ImpactLevel.LOW.value,
                    }
                )
                triggers.append(trigger)
            
            # Checkpoint change
            old_checkpoint = old_data.get("last_checkpoint_seq")
            new_checkpoint = new_data.get("last_checkpoint_seq")
            
            if new_checkpoint != old_checkpoint and new_checkpoint is not None:
                trigger = AgenticTrigger(
                    trigger_type=AgenticTriggerType.CORRIDOR_STATE_CHANGE,
                    data={
                        "corridor_id": corridor_id,
                        "change_type": "new_checkpoint",
                        "old_checkpoint_seq": old_checkpoint,
                        "new_checkpoint_seq": new_checkpoint,
                        "impact_level": ImpactLevel.MEDIUM.value,
                    }
                )
                triggers.append(trigger)
            
            # Settlement anchor availability
            old_anchor = old_data.get("settlement_anchor_available", False)
            new_anchor = new_data.get("settlement_anchor_available", False)
            
            if new_anchor and not old_anchor:
                trigger = AgenticTrigger(
                    trigger_type=AgenticTriggerType.SETTLEMENT_ANCHOR_AVAILABLE,
                    data={
                        "corridor_id": corridor_id,
                        "anchor_digest": new_data.get("settlement_anchor_digest"),
                        "impact_level": ImpactLevel.MEDIUM.value,
                    }
                )
                triggers.append(trigger)
            
            # Fork detection
            old_fork = old_data.get("fork_detected", False)
            new_fork = new_data.get("fork_detected", False)
            
            if new_fork and not old_fork:
                trigger = AgenticTrigger(
                    trigger_type=AgenticTriggerType.CORRIDOR_STATE_CHANGE,
                    data={
                        "corridor_id": corridor_id,
                        "change_type": "fork_detected",
                        "fork_point": new_data.get("fork_point"),
                        "impact_level": ImpactLevel.CRITICAL.value,
                    }
                )
                triggers.append(trigger)
        
        return triggers


# =============================================================================
# GUIDANCE UPDATE MONITOR
# =============================================================================

class GuidanceUpdateMonitor(EnvironmentMonitor):
    """
    Monitor for regulatory guidance updates.
    
    Emits GUIDANCE_UPDATE triggers when:
    - New regulatory guidance is published
    - Existing guidance is modified
    - Guidance becomes effective
    - Compliance deadlines approach
    """
    
    def __init__(self, config: MonitorConfig):
        super().__init__(config)
        self._tracked_guidance: Dict[str, Dict[str, Any]] = {}
        self._compliance_deadlines: Dict[str, datetime] = {}
    
    def track_guidance(self, guidance_id: str, guidance_data: Dict[str, Any]) -> None:
        """Add guidance to track."""
        self._tracked_guidance[guidance_id] = guidance_data
        
        # Extract compliance deadline if present
        deadline_str = guidance_data.get("compliance_deadline")
        if deadline_str:
            self._compliance_deadlines[guidance_id] = datetime.fromisoformat(
                deadline_str.replace('Z', '+00:00')
            )
    
    def poll(self) -> Optional[Dict[str, Any]]:
        """Poll current guidance state."""
        try:
            now = datetime.now(timezone.utc)
            results = {}
            
            for guidance_id, guidance_data in self._tracked_guidance.items():
                deadline = self._compliance_deadlines.get(guidance_id)
                days_until_deadline = (deadline - now).days if deadline else None
                
                results[guidance_id] = {
                    "guidance_data": guidance_data,
                    "days_until_deadline": days_until_deadline,
                    "is_effective": self._is_effective(guidance_data, now),
                }
            
            return {
                "guidance": results,
                "poll_time": now.isoformat(),
            }
        except Exception as e:
            self._record_error(e, "RegulatoryGuidanceMonitor.poll")
            return None
    
    def _is_effective(self, guidance_data: Dict[str, Any], now: datetime) -> bool:
        """Check if guidance is currently effective."""
        effective_from_str = guidance_data.get("effective_from")
        if effective_from_str:
            effective_from = datetime.fromisoformat(effective_from_str.replace('Z', '+00:00'))
            return now >= effective_from
        return True
    
    def detect_changes(
        self, 
        old_state: Optional[Dict[str, Any]], 
        new_state: Dict[str, Any]
    ) -> List[AgenticTrigger]:
        """Detect guidance changes."""
        triggers = []
        
        if old_state is None:
            return triggers
        
        old_guidance = old_state.get("guidance", {})
        new_guidance = new_state.get("guidance", {})
        
        for guidance_id, new_data in new_guidance.items():
            old_data = old_guidance.get(guidance_id, {})
            
            # Became effective
            if new_data.get("is_effective") and not old_data.get("is_effective"):
                trigger = AgenticTrigger(
                    trigger_type=AgenticTriggerType.GUIDANCE_UPDATE,
                    data={
                        "guidance_id": guidance_id,
                        "change_type": "became_effective",
                        "guidance_data": new_data.get("guidance_data"),
                        "impact_level": ImpactLevel.HIGH.value,
                    }
                )
                triggers.append(trigger)
            
            # Compliance deadline approaching
            old_days = old_data.get("days_until_deadline")
            new_days = new_data.get("days_until_deadline")
            
            if new_days is not None and old_days is not None:
                for threshold in [30, 14, 7, 1]:
                    if old_days > threshold >= new_days:
                        trigger = AgenticTrigger(
                            trigger_type=AgenticTriggerType.COMPLIANCE_DEADLINE,
                            data={
                                "guidance_id": guidance_id,
                                "days_until_deadline": new_days,
                                "threshold_crossed": threshold,
                                "impact_level": ImpactLevel.HIGH.value if threshold <= 7 else ImpactLevel.MEDIUM.value,
                            }
                        )
                        triggers.append(trigger)
        
        return triggers


# =============================================================================
# CHECKPOINT DUE MONITOR
# =============================================================================

class CheckpointDueMonitor(EnvironmentMonitor):
    """
    Monitor for checkpoint due conditions.
    
    Emits CHECKPOINT_DUE triggers when:
    - Receipt count threshold exceeded
    - Time since last checkpoint exceeded
    - Manual checkpoint requested
    """
    
    def __init__(self, config: MonitorConfig):
        super().__init__(config)
        self._tracked_assets: Dict[str, Dict[str, Any]] = {}
        
        # Thresholds from config
        self.receipt_threshold = config.config.get("receipt_threshold", 100)
        self.time_threshold_hours = config.config.get("time_threshold_hours", 24)
    
    def track_asset(self, asset_id: str, state: Dict[str, Any]) -> None:
        """Add an asset to track."""
        self._tracked_assets[asset_id] = state
    
    def poll(self) -> Optional[Dict[str, Any]]:
        """Poll asset checkpoint states."""
        try:
            now = datetime.now(timezone.utc)
            return {
                "assets": dict(self._tracked_assets),
                "poll_time": now.isoformat(),
            }
        except Exception as e:
            self._record_error(e, "AssetCheckpointMonitor.poll")
            return None
    
    def detect_changes(
        self, 
        old_state: Optional[Dict[str, Any]], 
        new_state: Dict[str, Any]
    ) -> List[AgenticTrigger]:
        """Detect checkpoint due conditions."""
        triggers = []
        now = datetime.now(timezone.utc)
        
        for asset_id, asset_state in new_state.get("assets", {}).items():
            receipts_since_checkpoint = asset_state.get("receipts_since_last_checkpoint", 0)
            last_checkpoint_str = asset_state.get("last_checkpoint_time")
            
            # Receipt threshold
            if receipts_since_checkpoint >= self.receipt_threshold:
                trigger = AgenticTrigger(
                    trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
                    data={
                        "asset_id": asset_id,
                        "reason": "receipt_threshold_exceeded",
                        "receipts_since_last": receipts_since_checkpoint,
                        "threshold": self.receipt_threshold,
                        "impact_level": ImpactLevel.MEDIUM.value,
                    }
                )
                triggers.append(trigger)
            
            # Time threshold
            if last_checkpoint_str:
                last_checkpoint = datetime.fromisoformat(last_checkpoint_str.replace('Z', '+00:00'))
                hours_since = (now - last_checkpoint).total_seconds() / 3600
                
                if hours_since >= self.time_threshold_hours:
                    trigger = AgenticTrigger(
                        trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
                        data={
                            "asset_id": asset_id,
                            "reason": "time_threshold_exceeded",
                            "hours_since_last": hours_since,
                            "threshold": self.time_threshold_hours,
                            "impact_level": ImpactLevel.LOW.value,
                        }
                    )
                    triggers.append(trigger)
        
        return triggers


# =============================================================================
# POLICY EVALUATION ENGINE
# =============================================================================

@dataclass
class PolicyEvaluationResult:
    """
    Result of evaluating a trigger against a policy.
    
    Per Theorem 17.1 (Agentic Determinism), evaluation results are
    deterministic given identical inputs.
    """
    policy_id: str
    trigger_id: str
    matched: bool
    action: Optional[TransitionKind] = None
    authorization_requirement: Optional[str] = None
    condition_details: Optional[Dict[str, Any]] = None
    evaluated_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "policy_id": self.policy_id,
            "trigger_id": self.trigger_id,
            "matched": self.matched,
            "action": self.action.value if self.action else None,
            "authorization_requirement": self.authorization_requirement,
            "condition_details": self.condition_details,
            "evaluated_at": self.evaluated_at,
        }


@dataclass
class ScheduledAction:
    """
    An action scheduled for execution.
    
    Actions may be immediate or deferred with retry semantics.
    """
    action_id: str
    asset_id: str
    action_type: TransitionKind
    trigger_id: str
    policy_id: str
    scheduled_at: str
    execute_at: str
    status: str = "pending"  # pending, executing, completed, failed, cancelled
    retry_count: int = 0
    max_retries: int = 3
    error_message: Optional[str] = None
    completed_at: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "action_id": self.action_id,
            "asset_id": self.asset_id,
            "action_type": self.action_type.value,
            "trigger_id": self.trigger_id,
            "policy_id": self.policy_id,
            "scheduled_at": self.scheduled_at,
            "execute_at": self.execute_at,
            "status": self.status,
            "retry_count": self.retry_count,
            "max_retries": self.max_retries,
            "error_message": self.error_message,
            "completed_at": self.completed_at,
        }


@dataclass
class AuditTrailEntry:
    """
    Entry in the agentic execution audit trail.
    
    Provides complete traceability for compliance and debugging.
    """
    entry_id: str
    entry_type: str  # trigger_received, policy_evaluated, action_scheduled, action_executed
    timestamp: str
    asset_id: Optional[str] = None
    trigger_data: Optional[Dict[str, Any]] = None
    evaluation_result: Optional[Dict[str, Any]] = None
    action_data: Optional[Dict[str, Any]] = None
    metadata: Optional[Dict[str, Any]] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "entry_id": self.entry_id,
            "entry_type": self.entry_type,
            "timestamp": self.timestamp,
            "asset_id": self.asset_id,
            "trigger_data": self.trigger_data,
            "evaluation_result": self.evaluation_result,
            "action_data": self.action_data,
            "metadata": self.metadata,
        }


class PolicyEvaluator:
    """
    Definition 17.5 (Policy Evaluation Engine).
    
    Evaluates triggers against policies and schedules actions.
    
    The evaluator guarantees determinism per Theorem 17.1:
    Given identical trigger events and policy state, evaluation
    produces identical results.
    
    Thread Safety:
    This class is thread-safe. All mutations to internal state
    are protected by locks.
    """
    
    def __init__(self, max_audit_trail_size: int = 10000):
        self._policies: Dict[str, AgenticPolicy] = {}
        self._scheduled_actions: Dict[str, ScheduledAction] = {}
        self._audit_trail: List[AuditTrailEntry] = []
        self._action_handlers: Dict[TransitionKind, Callable] = {}
        
        # Thread safety locks
        self._policy_lock = threading.RLock()
        self._action_lock = threading.RLock()
        self._audit_lock = threading.RLock()
        
        # Audit trail size limit to prevent memory leaks
        self._max_audit_trail_size = max_audit_trail_size
        
        # Load standard policies
        for policy_id, policy in STANDARD_POLICIES.items():
            self._policies[policy_id] = policy
    
    # BUG FIX #97: Known safe condition types for policy trigger evaluation.
    # Conditions from external sources must be validated/sanitized against
    # this allowlist to prevent injection of unsupported condition types.
    _VALID_CONDITION_TYPES = frozenset({
        "threshold", "equals", "not_equals", "contains", "in",
        "less_than", "greater_than", "exists", "and", "or",
    })

    def _validate_condition(self, condition: Optional[Dict[str, Any]], path: str = "condition") -> List[str]:
        """Validate a policy condition structure recursively.

        Returns a list of validation error messages (empty if valid).
        """
        errors: List[str] = []
        if condition is None:
            return errors
        if not isinstance(condition, dict):
            errors.append(f"{path}: condition must be a dict, got {type(condition).__name__}")
            return errors
        ctype = condition.get("type")
        if ctype is None:
            errors.append(f"{path}: missing 'type' field")
            return errors
        if ctype not in self._VALID_CONDITION_TYPES:
            errors.append(f"{path}: unknown condition type {ctype!r}")
            return errors
        # Validate nested conditions for compound types
        if ctype in ("and", "or"):
            sub = condition.get("conditions")
            if not isinstance(sub, list):
                errors.append(f"{path}.conditions: must be a list for '{ctype}' condition")
            else:
                for i, sc in enumerate(sub):
                    errors.extend(self._validate_condition(sc, f"{path}.conditions[{i}]"))
        return errors

    def register_policy(self, policy: AgenticPolicy) -> None:
        """Register a policy for evaluation. Thread-safe.

        BUG FIX #97: Validates the policy condition structure before
        registration to prevent injection of unsupported condition types.
        """
        errors = self._validate_condition(policy.condition)
        if errors:
            raise ValueError(f"Invalid policy condition for {policy.policy_id!r}: {'; '.join(errors)}")
        with self._policy_lock:
            self._policies[policy.policy_id] = policy
    
    def unregister_policy(self, policy_id: str) -> None:
        """Unregister a policy. Thread-safe."""
        with self._policy_lock:
            self._policies.pop(policy_id, None)
    
    def get_policy(self, policy_id: str) -> Optional[AgenticPolicy]:
        """Get a registered policy. Thread-safe."""
        with self._policy_lock:
            return self._policies.get(policy_id)
    
    def list_policies(self) -> List[AgenticPolicy]:
        """List all registered policies. Thread-safe."""
        with self._policy_lock:
            return list(self._policies.values())
    
    def register_action_handler(
        self, 
        action_type: TransitionKind, 
        handler: Callable[[ScheduledAction], bool]
    ) -> None:
        """Register a handler for an action type."""
        self._action_handlers[action_type] = handler
    
    def evaluate(
        self, 
        trigger: AgenticTrigger,
        asset_id: Optional[str] = None,
        environment: Optional[Dict[str, Any]] = None
    ) -> List[PolicyEvaluationResult]:
        """
        Evaluate a trigger against all registered policies.
        
        Theorem 17.1 (Agentic Determinism):
        This evaluation is deterministic given identical inputs.
        
        Thread-safe: Uses locks to protect shared state.
        
        Returns list of evaluation results for matching policies.
        """
        environment = environment or {}
        trigger_id = self._generate_id("trigger")
        results = []
        
        # Record trigger receipt in audit trail
        self._add_audit_entry(
            entry_type="trigger_received",
            asset_id=asset_id,
            trigger_data=trigger.to_dict(),
        )
        
        # Get a snapshot of policies under lock for thread safety
        with self._policy_lock:
            policy_snapshot = [(pid, self._policies[pid]) for pid in sorted(self._policies.keys())]
        
        # Evaluate against each policy in deterministic order
        for policy_id, policy in policy_snapshot:
            matched = policy.evaluate_condition(trigger, environment)
            
            result = PolicyEvaluationResult(
                policy_id=policy_id,
                trigger_id=trigger_id,
                matched=matched,
                action=policy.action if matched else None,
                authorization_requirement=policy.authorization_requirement if matched else None,
                condition_details={
                    "condition": policy.condition,
                    "trigger_type_match": trigger.trigger_type == policy.trigger_type,
                    "policy_enabled": policy.enabled,
                }
            )
            
            results.append(result)
            
            # Record evaluation in audit trail
            self._add_audit_entry(
                entry_type="policy_evaluated",
                asset_id=asset_id,
                evaluation_result=result.to_dict(),
            )
        
        return results
    
    def schedule_actions(
        self,
        evaluation_results: List[PolicyEvaluationResult],
        asset_id: str,
        delay_seconds: int = 0
    ) -> List[ScheduledAction]:
        """
        Schedule actions for matched policy evaluations. Thread-safe.
        
        Returns list of scheduled actions.
        """
        scheduled = []
        now = datetime.now(timezone.utc)
        execute_at = now + timedelta(seconds=delay_seconds)
        
        for result in evaluation_results:
            if result.matched and result.action:
                action = ScheduledAction(
                    action_id=self._generate_id("action"),
                    asset_id=asset_id,
                    action_type=result.action,
                    trigger_id=result.trigger_id,
                    policy_id=result.policy_id,
                    scheduled_at=now.isoformat(),
                    execute_at=execute_at.isoformat(),
                )
                
                with self._action_lock:
                    self._scheduled_actions[action.action_id] = action
                scheduled.append(action)
                
                # Record in audit trail
                self._add_audit_entry(
                    entry_type="action_scheduled",
                    asset_id=asset_id,
                    action_data=action.to_dict(),
                )
        
        return scheduled
    
    def execute_action(self, action_id: str) -> Tuple[bool, Optional[str]]:
        """
        Execute a scheduled action. Thread-safe.
        
        Returns (success, error_message).
        """
        with self._action_lock:
            action = self._scheduled_actions.get(action_id)
            if not action:
                return False, f"Action not found: {action_id}"
            
            if action.status not in ("pending", "failed"):
                return False, f"Action not executable in status: {action.status}"
            
            action.status = "executing"
        
        try:
            handler = self._action_handlers.get(action.action_type)
            if handler:
                success = handler(action)
            else:
                # Default: mark as completed (actual execution delegated)
                success = True
            
            if success:
                action.status = "completed"
                action.completed_at = datetime.now(timezone.utc).isoformat()
            else:
                action.status = "failed"
                action.retry_count += 1
                
                if action.retry_count >= action.max_retries:
                    action.error_message = "Max retries exceeded"
                else:
                    action.status = "pending"  # Will retry
            
            # Record in audit trail
            self._add_audit_entry(
                entry_type="action_executed",
                asset_id=action.asset_id,
                action_data=action.to_dict(),
            )
            
            return success, action.error_message
            
        except Exception as e:
            action.status = "failed"
            action.error_message = str(e)
            action.retry_count += 1
            
            self._add_audit_entry(
                entry_type="action_executed",
                asset_id=action.asset_id,
                action_data=action.to_dict(),
                metadata={"error": str(e)},
            )
            
            return False, str(e)
    
    def get_pending_actions(self) -> List[ScheduledAction]:
        """Get all pending actions."""
        return [a for a in self._scheduled_actions.values() if a.status == "pending"]
    
    def get_action(self, action_id: str) -> Optional[ScheduledAction]:
        """Get a specific action."""
        return self._scheduled_actions.get(action_id)
    
    def cancel_action(self, action_id: str) -> bool:
        """Cancel a pending action."""
        action = self._scheduled_actions.get(action_id)
        if action and action.status == "pending":
            action.status = "cancelled"
            return True
        return False
    
    def get_audit_trail(
        self, 
        asset_id: Optional[str] = None,
        entry_type: Optional[str] = None,
        limit: int = 100
    ) -> List[AuditTrailEntry]:
        """
        Get audit trail entries with optional filtering. Thread-safe.
        """
        with self._audit_lock:
            entries = list(self._audit_trail)  # Copy to avoid concurrent modification
        
        if asset_id:
            entries = [e for e in entries if e.asset_id == asset_id]
        
        if entry_type:
            entries = [e for e in entries if e.entry_type == entry_type]
        
        return entries[-limit:]
    
    def _add_audit_entry(
        self,
        entry_type: str,
        asset_id: Optional[str] = None,
        trigger_data: Optional[Dict[str, Any]] = None,
        evaluation_result: Optional[Dict[str, Any]] = None,
        action_data: Optional[Dict[str, Any]] = None,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> None:
        """
        Add an entry to the audit trail. Thread-safe.
        
        Implements circular buffer behavior when max_audit_trail_size is exceeded
        to prevent unbounded memory growth.
        """
        entry = AuditTrailEntry(
            entry_id=self._generate_id("audit"),
            entry_type=entry_type,
            timestamp=datetime.now(timezone.utc).isoformat(),
            asset_id=asset_id,
            trigger_data=trigger_data,
            evaluation_result=evaluation_result,
            action_data=action_data,
            metadata=metadata,
        )
        
        with self._audit_lock:
            self._audit_trail.append(entry)
            # Prevent unbounded growth - keep only recent entries
            if len(self._audit_trail) > self._max_audit_trail_size:
                # Remove oldest 10% when limit exceeded
                trim_count = self._max_audit_trail_size // 10
                self._audit_trail = self._audit_trail[trim_count:]
    
    def _generate_id(self, prefix: str) -> str:
        """Generate a unique ID with full UUID for collision resistance."""
        return f"{prefix}:{uuid.uuid4().hex}"


# =============================================================================
# MONITOR REGISTRY
# =============================================================================

class MonitorRegistry:
    """
    Registry for managing environment monitors.
    
    Provides centralized monitor lifecycle management.
    """
    
    def __init__(self):
        self._monitors: Dict[str, EnvironmentMonitor] = {}
    
    def register(self, monitor: EnvironmentMonitor) -> None:
        """Register a monitor."""
        self._monitors[monitor.monitor_id] = monitor
    
    def unregister(self, monitor_id: str) -> Optional[EnvironmentMonitor]:
        """Unregister and return a monitor."""
        monitor = self._monitors.pop(monitor_id, None)
        if monitor and monitor.status == MonitorStatus.RUNNING:
            monitor.stop()
        return monitor
    
    def get(self, monitor_id: str) -> Optional[EnvironmentMonitor]:
        """Get a monitor by ID."""
        return self._monitors.get(monitor_id)
    
    def list_monitors(self) -> List[EnvironmentMonitor]:
        """List all registered monitors."""
        return list(self._monitors.values())
    
    def start_all(self) -> None:
        """Start all registered monitors."""
        for monitor in self._monitors.values():
            if monitor.config.enabled:
                monitor.start()
    
    def stop_all(self) -> None:
        """Stop all registered monitors."""
        for monitor in self._monitors.values():
            monitor.stop()
    
    def get_status_report(self) -> List[Dict[str, Any]]:
        """Get status report for all monitors."""
        return [m.get_status() for m in self._monitors.values()]


# =============================================================================
# AGENTIC EXECUTION ENGINE
# =============================================================================

class AgenticExecutionEngine:
    """
    Definition 17.6 (Agentic Execution Engine).
    
    Central coordinator for agentic execution, integrating:
    - Environment monitors
    - Policy evaluation
    - Action scheduling and execution
    - Audit trail management
    """
    
    def __init__(self):
        self.monitor_registry = MonitorRegistry()
        self.policy_evaluator = PolicyEvaluator()
        self._asset_bindings: Dict[str, str] = {}  # asset_id -> set of monitor_ids
    
    def bind_asset_to_monitors(
        self, 
        asset_id: str, 
        monitor_ids: List[str]
    ) -> None:
        """Bind an asset to specific monitors."""
        for monitor_id in monitor_ids:
            monitor = self.monitor_registry.get(monitor_id)
            if monitor:
                # Add trigger listener that routes to policy evaluation
                def on_trigger(trigger: AgenticTrigger, aid=asset_id):
                    self.process_trigger(trigger, aid)
                
                monitor.add_listener(on_trigger)
        
        self._asset_bindings[asset_id] = monitor_ids
    
    def process_trigger(
        self, 
        trigger: AgenticTrigger, 
        asset_id: str,
        environment: Optional[Dict[str, Any]] = None
    ) -> List[ScheduledAction]:
        """
        Process a trigger for an asset.
        
        1. Evaluate against policies
        2. Schedule matching actions
        3. Return scheduled actions
        """
        results = self.policy_evaluator.evaluate(
            trigger, 
            asset_id=asset_id,
            environment=environment
        )
        
        # Only schedule actions that matched
        matched_results = [r for r in results if r.matched]
        
        if matched_results:
            return self.policy_evaluator.schedule_actions(
                matched_results,
                asset_id
            )
        
        return []
    
    def execute_pending_actions(self) -> List[Tuple[str, bool, Optional[str]]]:
        """
        Execute all pending actions.
        
        Returns list of (action_id, success, error_message).
        """
        results = []
        
        for action in self.policy_evaluator.get_pending_actions():
            success, error = self.policy_evaluator.execute_action(action.action_id)
            results.append((action.action_id, success, error))
        
        return results
    
    def start(self) -> None:
        """Start the agentic execution engine."""
        self.monitor_registry.start_all()
    
    def stop(self) -> None:
        """Stop the agentic execution engine."""
        self.monitor_registry.stop_all()
    
    def get_status(self) -> Dict[str, Any]:
        """Get engine status."""
        return {
            "monitors": self.monitor_registry.get_status_report(),
            "policies": [p.to_dict() for p in self.policy_evaluator.list_policies()],
            "pending_actions": [a.to_dict() for a in self.policy_evaluator.get_pending_actions()],
            "asset_bindings": dict(self._asset_bindings),
        }


# =============================================================================
# EXTENDED STANDARD POLICIES (v0.4.44)
# =============================================================================

# Extend STANDARD_POLICIES with v0.4.44 additions
EXTENDED_POLICIES = {
    # From v0.4.41
    **STANDARD_POLICIES,

    # v0.4.44 additions
    "sanctions_freeze": AgenticPolicy(
        policy_id="sanctions_freeze",
        trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
        condition={"type": "equals", "field": "new_sanctioned", "value": True},
        action=TransitionKind.HALT,
        authorization_requirement="automatic"
    ),
    "sanctions_notify": AgenticPolicy(
        policy_id="sanctions_notify",
        trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
        condition={"type": "equals", "field": "update_type", "value": "list_version_change"},
        action=TransitionKind.UPDATE_MANIFEST,
        authorization_requirement="automatic"
    ),
    "license_suspend": AgenticPolicy(
        policy_id="license_suspend",
        trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
        condition={"type": "equals", "field": "new_status", "value": "suspended"},
        action=TransitionKind.HALT,
        authorization_requirement="automatic"
    ),
    "license_renew_reminder": AgenticPolicy(
        policy_id="license_renew_reminder",
        trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
        condition={"type": "equals", "field": "warning_type", "value": "expiry_approaching"},
        action=TransitionKind.UPDATE_MANIFEST,
        authorization_requirement="quorum"
    ),
    "corridor_failover": AgenticPolicy(
        policy_id="corridor_failover",
        trigger_type=AgenticTriggerType.CORRIDOR_STATE_CHANGE,
        condition={"type": "equals", "field": "change_type", "value": "fork_detected"},
        action=TransitionKind.HALT,
        authorization_requirement="quorum"
    ),
    "checkpoint_auto_receipt": AgenticPolicy(
        policy_id="checkpoint_auto_receipt",
        trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
        condition={"type": "equals", "field": "reason", "value": "receipt_threshold_exceeded"},
        action=TransitionKind.UPDATE_MANIFEST,
        authorization_requirement="automatic"
    ),
    "checkpoint_auto_time": AgenticPolicy(
        policy_id="checkpoint_auto_time",
        trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
        condition={"type": "equals", "field": "reason", "value": "time_threshold_exceeded"},
        action=TransitionKind.UPDATE_MANIFEST,
        authorization_requirement="automatic"
    ),
    "key_rotation_enforce": AgenticPolicy(
        policy_id="key_rotation_enforce",
        trigger_type=AgenticTriggerType.KEY_ROTATION_DUE,
        action=TransitionKind.UPDATE_MANIFEST,
        authorization_requirement="quorum"
    ),
    "dispute_filed_halt": AgenticPolicy(
        policy_id="dispute_filed_halt",
        trigger_type=AgenticTriggerType.DISPUTE_FILED,
        action=TransitionKind.HALT,
        authorization_requirement="automatic"
    ),
    "ruling_auto_enforce": AgenticPolicy(
        policy_id="ruling_auto_enforce",
        trigger_type=AgenticTriggerType.RULING_RECEIVED,
        condition={"type": "equals", "field": "auto_enforce", "value": True},
        action=TransitionKind.ARBITRATION_ENFORCE,
        authorization_requirement="automatic"
    ),
    "appeal_period_expired": AgenticPolicy(
        policy_id="appeal_period_expired",
        trigger_type=AgenticTriggerType.APPEAL_PERIOD_EXPIRED,
        action=TransitionKind.ARBITRATION_ENFORCE,
        authorization_requirement="automatic"
    ),
    "settlement_anchor_notify": AgenticPolicy(
        policy_id="settlement_anchor_notify",
        trigger_type=AgenticTriggerType.SETTLEMENT_ANCHOR_AVAILABLE,
        action=TransitionKind.UPDATE_MANIFEST,
        authorization_requirement="automatic"
    ),
    "watcher_quorum_checkpoint": AgenticPolicy(
        policy_id="watcher_quorum_checkpoint",
        trigger_type=AgenticTriggerType.WATCHER_QUORUM_REACHED,
        action=TransitionKind.UPDATE_MANIFEST,
        authorization_requirement="automatic"
    ),
    "compliance_deadline_warn": AgenticPolicy(
        policy_id="compliance_deadline_warn",
        trigger_type=AgenticTriggerType.COMPLIANCE_DEADLINE,
        condition={"type": "threshold", "field": "days_until_deadline", "threshold": 0},
        action=TransitionKind.UPDATE_MANIFEST,
        authorization_requirement="quorum"
    ),
    "guidance_effective_update": AgenticPolicy(
        policy_id="guidance_effective_update",
        trigger_type=AgenticTriggerType.GUIDANCE_UPDATE,
        condition={"type": "equals", "field": "change_type", "value": "became_effective"},
        action=TransitionKind.UPDATE_MANIFEST,
        authorization_requirement="automatic"
    ),
}


# =============================================================================
# FACTORY FUNCTIONS
# =============================================================================

def create_sanctions_monitor(
    monitor_id: str,
    sanctions_checker: SanctionsChecker,
    watched_entities: Optional[List[str]] = None,
    poll_interval: int = 300
) -> SanctionsListMonitor:
    """Create a configured sanctions list monitor."""
    config = MonitorConfig(
        monitor_id=monitor_id,
        monitor_type="sanctions_list",
        poll_interval_seconds=poll_interval,
        config={"watched_entities": watched_entities or []},
    )
    return SanctionsListMonitor(config, sanctions_checker)


def create_license_monitor(
    monitor_id: str,
    regpack_manager: Optional[RegPackManager] = None,
    warning_thresholds: Optional[List[int]] = None,
    poll_interval: int = 3600
) -> LicenseStatusMonitor:
    """Create a configured license status monitor."""
    config = MonitorConfig(
        monitor_id=monitor_id,
        monitor_type="license_status",
        poll_interval_seconds=poll_interval,
        config={"warning_thresholds": warning_thresholds or [30, 14, 7, 1]},
    )
    return LicenseStatusMonitor(config, regpack_manager)


def create_corridor_monitor(
    monitor_id: str,
    poll_interval: int = 60
) -> CorridorStateMonitor:
    """Create a configured corridor state monitor."""
    config = MonitorConfig(
        monitor_id=monitor_id,
        monitor_type="corridor_state",
        poll_interval_seconds=poll_interval,
    )
    return CorridorStateMonitor(config)


def create_guidance_monitor(
    monitor_id: str,
    poll_interval: int = 3600
) -> GuidanceUpdateMonitor:
    """Create a configured guidance update monitor."""
    config = MonitorConfig(
        monitor_id=monitor_id,
        monitor_type="guidance_update",
        poll_interval_seconds=poll_interval,
    )
    return GuidanceUpdateMonitor(config)


def create_checkpoint_monitor(
    monitor_id: str,
    receipt_threshold: int = 100,
    time_threshold_hours: int = 24,
    poll_interval: int = 300
) -> CheckpointDueMonitor:
    """Create a configured checkpoint due monitor."""
    config = MonitorConfig(
        monitor_id=monitor_id,
        monitor_type="checkpoint_due",
        poll_interval_seconds=poll_interval,
        config={
            "receipt_threshold": receipt_threshold,
            "time_threshold_hours": time_threshold_hours,
        },
    )
    return CheckpointDueMonitor(config)


# =============================================================================
# MODULE EXPORTS
# =============================================================================

__all__ = [
    # Status enums
    'MonitorStatus',
    'MonitorMode',
    
    # Configuration
    'MonitorConfig',
    
    # Base monitor
    'EnvironmentMonitor',
    
    # Concrete monitors
    'SanctionsListMonitor',
    'LicenseStatusMonitor',
    'CorridorStateMonitor',
    'GuidanceUpdateMonitor',
    'CheckpointDueMonitor',
    
    # Policy evaluation
    'PolicyEvaluationResult',
    'ScheduledAction',
    'AuditTrailEntry',
    'PolicyEvaluator',
    
    # Registry and engine
    'MonitorRegistry',
    'AgenticExecutionEngine',
    
    # Extended policies
    'EXTENDED_POLICIES',
    
    # Factory functions
    'create_sanctions_monitor',
    'create_license_monitor',
    'create_corridor_monitor',
    'create_guidance_monitor',
    'create_checkpoint_monitor',
]
