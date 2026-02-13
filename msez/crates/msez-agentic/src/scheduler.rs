//! # Action Scheduler — MASS Protocol v0.2 Chapter 17
//!
//! Schedules and executes compliance actions with configurable delays,
//! retry semantics, and approval gates.
//!
//! ## Design
//!
//! The scheduler receives `ScheduledAction` directives from the evaluation
//! engine and manages their lifecycle:
//!
//! 1. **Pending** — action is waiting for its `execute_at` time or approval gate.
//! 2. **Executing** — action is being executed.
//! 3. **Completed** — action executed successfully.
//! 4. **Failed** — action execution failed (may retry if retries remain).
//! 5. **Cancelled** — action was cancelled before execution.
//!
//! ## Cron Scheduling
//!
//! Recurring evaluations can be scheduled via `CronSchedule`, which supports
//! common patterns (hourly, daily, weekly, monthly, yearly). The scheduler
//! checks whether a cron schedule should fire based on the current timestamp.

use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};

use crate::policy::{AuthorizationRequirement, PolicyAction};

// ---------------------------------------------------------------------------
// ActionStatus
// ---------------------------------------------------------------------------

/// The lifecycle status of a scheduled action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    /// Waiting for execution time or approval.
    Pending,
    /// Currently being executed.
    Executing,
    /// Executed successfully.
    Completed,
    /// Execution failed.
    Failed,
    /// Cancelled before execution.
    Cancelled,
}

impl ActionStatus {
    /// Return whether this is a terminal status (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

impl std::fmt::Display for ActionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => f.write_str("pending"),
            Self::Executing => f.write_str("executing"),
            Self::Completed => f.write_str("completed"),
            Self::Failed => f.write_str("failed"),
            Self::Cancelled => f.write_str("cancelled"),
        }
    }
}

// ---------------------------------------------------------------------------
// ScheduledAction
// ---------------------------------------------------------------------------

/// A scheduled compliance action produced by the evaluation engine.
///
/// Carries the action to execute, the originating policy, authorization
/// requirements, retry semantics, and deadline constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledAction {
    /// Unique identifier for this scheduled action.
    pub action_id: String,
    /// The asset this action applies to.
    pub asset_id: String,
    /// The action to execute.
    pub action: PolicyAction,
    /// The policy that produced this action.
    pub policy_id: String,
    /// Current lifecycle status.
    pub status: ActionStatus,
    /// Authorization requirement that must be met before execution.
    pub authorization_requirement: AuthorizationRequirement,
    /// When the action was scheduled.
    pub scheduled_at: DateTime<Utc>,
    /// When the action should execute (None = as soon as authorized).
    pub execute_at: Option<DateTime<Utc>>,
    /// Deadline by which the action must complete (None = no deadline).
    pub deadline: Option<DateTime<Utc>>,
    /// Number of retry attempts remaining.
    pub retries_remaining: u32,
    /// Maximum number of retries allowed.
    pub max_retries: u32,
    /// Error message from the last failed attempt (if any).
    pub last_error: Option<String>,
}

impl ScheduledAction {
    /// Create a new scheduled action with default settings.
    pub fn new(
        asset_id: String,
        action: PolicyAction,
        policy_id: String,
        authorization_requirement: AuthorizationRequirement,
    ) -> Self {
        let action_id = uuid::Uuid::new_v4().to_string();
        Self {
            action_id,
            asset_id,
            action,
            policy_id,
            status: ActionStatus::Pending,
            authorization_requirement,
            scheduled_at: Utc::now(),
            execute_at: None,
            deadline: None,
            retries_remaining: 3,
            max_retries: 3,
            last_error: None,
        }
    }

    /// Builder: set the execution time.
    pub fn with_execute_at(mut self, execute_at: DateTime<Utc>) -> Self {
        self.execute_at = Some(execute_at);
        self
    }

    /// Builder: set the deadline.
    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Builder: set the maximum number of retries.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self.retries_remaining = max_retries;
        self
    }

    /// Check whether this action is ready to execute at the given time.
    ///
    /// An action is ready if:
    /// - Status is `Pending`
    /// - `execute_at` is None or in the past
    /// - `deadline` is None or in the future
    pub fn is_ready(&self, now: DateTime<Utc>) -> bool {
        if self.status != ActionStatus::Pending {
            return false;
        }
        if let Some(execute_at) = self.execute_at {
            if now < execute_at {
                return false;
            }
        }
        if let Some(deadline) = self.deadline {
            if now > deadline {
                return false;
            }
        }
        true
    }

    /// Check whether this action has exceeded its deadline.
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.deadline.is_some_and(|d| now > d)
    }

    /// Check whether this action can be retried.
    pub fn can_retry(&self) -> bool {
        self.status == ActionStatus::Failed && self.retries_remaining > 0
    }
}

impl PartialEq for ScheduledAction {
    fn eq(&self, other: &Self) -> bool {
        self.action_id == other.action_id
    }
}

impl Eq for ScheduledAction {}

// ---------------------------------------------------------------------------
// SchedulePattern — cron-like recurring schedules
// ---------------------------------------------------------------------------

/// A schedule pattern for recurring policy evaluations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulePattern {
    /// Every hour at minute 0.
    Hourly,
    /// Every day at midnight UTC.
    Daily,
    /// Every Monday at midnight UTC.
    Weekly,
    /// First day of each month at midnight UTC.
    Monthly,
    /// January 1st at midnight UTC.
    Yearly,
}

// ---------------------------------------------------------------------------
// CronSchedule
// ---------------------------------------------------------------------------

/// A cron-like schedule definition for recurring policy evaluations.
///
/// Supports common patterns via `SchedulePattern`. The `should_fire()` method
/// checks whether the schedule should trigger at a given timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronSchedule {
    /// Unique schedule identifier.
    pub schedule_id: String,
    /// Human-readable description.
    pub description: String,
    /// The recurrence pattern.
    pub pattern: SchedulePattern,
    /// Whether this schedule is currently active.
    pub active: bool,
    /// Last time this schedule fired.
    pub last_fired: Option<DateTime<Utc>>,
}

impl CronSchedule {
    /// Create a new cron schedule.
    pub fn new(
        schedule_id: impl Into<String>,
        description: impl Into<String>,
        pattern: SchedulePattern,
    ) -> Self {
        Self {
            schedule_id: schedule_id.into(),
            description: description.into(),
            pattern,
            active: true,
            last_fired: None,
        }
    }

    /// Check whether this schedule should fire at the given timestamp.
    ///
    /// Returns `true` if:
    /// 1. The schedule is active.
    /// 2. The pattern matches the current time.
    /// 3. The schedule hasn't already fired at this time slot.
    pub fn should_fire(&self, now: DateTime<Utc>) -> bool {
        if !self.active {
            return false;
        }

        let pattern_matches = match self.pattern {
            SchedulePattern::Hourly => now.minute() == 0,
            SchedulePattern::Daily => now.hour() == 0 && now.minute() == 0,
            SchedulePattern::Weekly => {
                now.weekday() == chrono::Weekday::Mon && now.hour() == 0 && now.minute() == 0
            }
            SchedulePattern::Monthly => now.day() == 1 && now.hour() == 0 && now.minute() == 0,
            SchedulePattern::Yearly => {
                now.month() == 1 && now.day() == 1 && now.hour() == 0 && now.minute() == 0
            }
        };

        if !pattern_matches {
            return false;
        }

        // Don't fire twice for the same time slot.
        match self.last_fired {
            Some(last) => {
                // Truncate to minutes for comparison.
                let now_slot = now.date_naive().and_hms_opt(now.hour(), now.minute(), 0);
                let last_slot = last.date_naive().and_hms_opt(last.hour(), last.minute(), 0);
                now_slot != last_slot
            }
            None => true,
        }
    }

    /// Record that this schedule fired at the given timestamp.
    pub fn mark_fired(&mut self, at: DateTime<Utc>) {
        self.last_fired = Some(at);
    }
}

// ---------------------------------------------------------------------------
// ActionScheduler
// ---------------------------------------------------------------------------

/// The action scheduler for automated compliance responses.
///
/// Manages the lifecycle of scheduled actions: scheduling, execution,
/// retry, cancellation, and deadline enforcement.
#[derive(Debug)]
pub struct ActionScheduler {
    /// All scheduled actions, keyed by action_id.
    actions: Vec<ScheduledAction>,
    /// Cron schedules for recurring evaluations.
    schedules: Vec<CronSchedule>,
}

impl ActionScheduler {
    /// Create a new empty action scheduler.
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            schedules: Vec::new(),
        }
    }

    /// Schedule an action for execution.
    pub fn schedule(&mut self, action: ScheduledAction) -> String {
        let id = action.action_id.clone();
        self.actions.push(action);
        id
    }

    /// Cancel a scheduled action by ID.
    ///
    /// Returns `true` if the action was found and cancelled, `false` if not
    /// found or already in a terminal state.
    pub fn cancel(&mut self, action_id: &str) -> bool {
        if let Some(action) = self.actions.iter_mut().find(|a| a.action_id == action_id) {
            if !action.status.is_terminal() {
                action.status = ActionStatus::Cancelled;
                return true;
            }
        }
        false
    }

    /// Mark an action as executing.
    pub fn mark_executing(&mut self, action_id: &str) -> bool {
        if let Some(action) = self.actions.iter_mut().find(|a| a.action_id == action_id) {
            if action.status == ActionStatus::Pending {
                action.status = ActionStatus::Executing;
                return true;
            }
        }
        false
    }

    /// Mark an action as completed.
    pub fn mark_completed(&mut self, action_id: &str) -> bool {
        if let Some(action) = self.actions.iter_mut().find(|a| a.action_id == action_id) {
            if action.status == ActionStatus::Executing {
                action.status = ActionStatus::Completed;
                return true;
            }
        }
        false
    }

    /// Mark an action as failed with an error message.
    ///
    /// If retries remain, the action is reset to `Pending` for retry.
    pub fn mark_failed(&mut self, action_id: &str, error: String) -> bool {
        if let Some(action) = self.actions.iter_mut().find(|a| a.action_id == action_id) {
            if action.status == ActionStatus::Executing {
                action.last_error = Some(error);
                if action.retries_remaining > 0 {
                    action.retries_remaining -= 1;
                    action.status = ActionStatus::Pending;
                } else {
                    action.status = ActionStatus::Failed;
                }
                return true;
            }
        }
        false
    }

    /// Return all actions ready to execute at the given time.
    pub fn ready_actions(&self, now: DateTime<Utc>) -> Vec<&ScheduledAction> {
        self.actions.iter().filter(|a| a.is_ready(now)).collect()
    }

    /// Return all actions that have exceeded their deadline.
    pub fn expired_actions(&self, now: DateTime<Utc>) -> Vec<&ScheduledAction> {
        self.actions
            .iter()
            .filter(|a| a.is_expired(now) && !a.status.is_terminal())
            .collect()
    }

    /// Get an action by ID.
    pub fn get_action(&self, action_id: &str) -> Option<&ScheduledAction> {
        self.actions.iter().find(|a| a.action_id == action_id)
    }

    /// Return the count of actions in each status.
    pub fn status_counts(&self) -> std::collections::HashMap<ActionStatus, usize> {
        let mut counts = std::collections::HashMap::new();
        for action in &self.actions {
            *counts.entry(action.status).or_insert(0) += 1;
        }
        counts
    }

    /// Return total number of scheduled actions.
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    /// Add a cron schedule for recurring evaluations.
    pub fn add_schedule(&mut self, schedule: CronSchedule) {
        self.schedules.push(schedule);
    }

    /// Check all cron schedules and return those that should fire now.
    pub fn check_schedules(&mut self, now: DateTime<Utc>) -> Vec<String> {
        let mut fired = Vec::new();
        for schedule in &mut self.schedules {
            if schedule.should_fire(now) {
                fired.push(schedule.schedule_id.clone());
                schedule.mark_fired(now);
            }
        }
        fired
    }

    /// Return all registered cron schedules.
    pub fn schedules(&self) -> &[CronSchedule] {
        &self.schedules
    }
}

impl Default for ActionScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_action(policy_id: &str) -> ScheduledAction {
        ScheduledAction::new(
            "asset:test".into(),
            PolicyAction::Halt,
            policy_id.into(),
            AuthorizationRequirement::Automatic,
        )
    }

    #[test]
    fn scheduled_action_creation() {
        let action = make_action("test_policy");
        assert_eq!(action.status, ActionStatus::Pending);
        assert_eq!(action.action, PolicyAction::Halt);
        assert_eq!(action.retries_remaining, 3);
        assert!(!action.action_id.is_empty());
    }

    #[test]
    fn scheduled_action_is_ready() {
        let action = make_action("test");
        let now = Utc::now();
        assert!(action.is_ready(now));

        // With future execute_at — not ready yet.
        let future_action = make_action("test").with_execute_at(now + chrono::Duration::hours(1));
        assert!(!future_action.is_ready(now));

        // With past execute_at — ready.
        let past_action = make_action("test").with_execute_at(now - chrono::Duration::hours(1));
        assert!(past_action.is_ready(now));
    }

    #[test]
    fn scheduled_action_deadline_expiry() {
        let now = Utc::now();
        let expired = make_action("test").with_deadline(now - chrono::Duration::hours(1));
        assert!(expired.is_expired(now));
        assert!(!expired.is_ready(now));

        let not_expired = make_action("test").with_deadline(now + chrono::Duration::hours(1));
        assert!(!not_expired.is_expired(now));
    }

    #[test]
    fn scheduler_schedule_and_cancel() {
        let mut scheduler = ActionScheduler::new();
        let action = make_action("test");
        let id = action.action_id.clone();
        scheduler.schedule(action);
        assert_eq!(scheduler.action_count(), 1);

        let cancelled = scheduler.cancel(&id);
        assert!(cancelled);
        assert_eq!(
            scheduler.get_action(&id).unwrap().status,
            ActionStatus::Cancelled
        );

        // Cancelling again should fail (already terminal).
        assert!(!scheduler.cancel(&id));
    }

    #[test]
    fn scheduler_lifecycle() {
        let mut scheduler = ActionScheduler::new();
        let action = make_action("test");
        let id = action.action_id.clone();
        scheduler.schedule(action);

        // Pending → Executing
        assert!(scheduler.mark_executing(&id));
        assert_eq!(
            scheduler.get_action(&id).unwrap().status,
            ActionStatus::Executing
        );

        // Executing → Completed
        assert!(scheduler.mark_completed(&id));
        assert_eq!(
            scheduler.get_action(&id).unwrap().status,
            ActionStatus::Completed
        );
    }

    #[test]
    fn scheduler_retry_on_failure() {
        let mut scheduler = ActionScheduler::new();
        let action = make_action("test").with_max_retries(2);
        let id = action.action_id.clone();
        scheduler.schedule(action);

        // First attempt fails
        scheduler.mark_executing(&id);
        scheduler.mark_failed(&id, "timeout".into());
        // Should be reset to Pending with 1 retry remaining.
        let a = scheduler.get_action(&id).unwrap();
        assert_eq!(a.status, ActionStatus::Pending);
        assert_eq!(a.retries_remaining, 1);

        // Second attempt fails
        scheduler.mark_executing(&id);
        scheduler.mark_failed(&id, "timeout".into());
        let a = scheduler.get_action(&id).unwrap();
        assert_eq!(a.status, ActionStatus::Pending);
        assert_eq!(a.retries_remaining, 0);

        // Third attempt fails — no retries left, permanently failed.
        scheduler.mark_executing(&id);
        scheduler.mark_failed(&id, "timeout".into());
        let a = scheduler.get_action(&id).unwrap();
        assert_eq!(a.status, ActionStatus::Failed);
        assert_eq!(a.last_error.as_deref(), Some("timeout"));
    }

    #[test]
    fn scheduler_ready_and_expired_actions() {
        let mut scheduler = ActionScheduler::new();
        let now = Utc::now();

        // Ready action (no constraints)
        let a1 = make_action("ready");
        scheduler.schedule(a1);

        // Future action (not ready yet)
        let a2 = make_action("future").with_execute_at(now + chrono::Duration::hours(1));
        scheduler.schedule(a2);

        // Expired action
        let a3 = make_action("expired").with_deadline(now - chrono::Duration::hours(1));
        scheduler.schedule(a3);

        let ready = scheduler.ready_actions(now);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].policy_id, "ready");

        let expired = scheduler.expired_actions(now);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].policy_id, "expired");
    }

    #[test]
    fn scheduler_status_counts() {
        let mut scheduler = ActionScheduler::new();
        for i in 0..3 {
            scheduler.schedule(make_action(&format!("p{i}")));
        }
        let a = scheduler.actions[0].action_id.clone();
        scheduler.mark_executing(&a);
        scheduler.mark_completed(&a);

        let counts = scheduler.status_counts();
        assert_eq!(counts.get(&ActionStatus::Pending), Some(&2));
        assert_eq!(counts.get(&ActionStatus::Completed), Some(&1));
    }

    #[test]
    fn cron_schedule_hourly() {
        let schedule = CronSchedule::new("hourly_check", "Hourly check", SchedulePattern::Hourly);
        // At minute 0 → should fire.
        let at_zero = chrono::NaiveDate::from_ymd_opt(2026, 1, 15)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_utc();
        assert!(schedule.should_fire(at_zero));

        // At minute 30 → should not fire.
        let at_thirty = chrono::NaiveDate::from_ymd_opt(2026, 1, 15)
            .unwrap()
            .and_hms_opt(10, 30, 0)
            .unwrap()
            .and_utc();
        assert!(!schedule.should_fire(at_thirty));
    }

    #[test]
    fn cron_schedule_daily() {
        let schedule = CronSchedule::new("daily_check", "Daily check", SchedulePattern::Daily);
        let midnight = chrono::NaiveDate::from_ymd_opt(2026, 1, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        assert!(schedule.should_fire(midnight));

        let noon = chrono::NaiveDate::from_ymd_opt(2026, 1, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc();
        assert!(!schedule.should_fire(noon));
    }

    #[test]
    fn cron_schedule_no_double_fire() {
        let mut schedule = CronSchedule::new("hourly", "Hourly check", SchedulePattern::Hourly);
        let at_zero = chrono::NaiveDate::from_ymd_opt(2026, 1, 15)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_utc();

        assert!(schedule.should_fire(at_zero));
        schedule.mark_fired(at_zero);
        assert!(!schedule.should_fire(at_zero));

        // Next hour should fire.
        let next_hour = chrono::NaiveDate::from_ymd_opt(2026, 1, 15)
            .unwrap()
            .and_hms_opt(11, 0, 0)
            .unwrap()
            .and_utc();
        assert!(schedule.should_fire(next_hour));
    }

    #[test]
    fn scheduler_check_schedules() {
        let mut scheduler = ActionScheduler::new();
        scheduler.add_schedule(CronSchedule::new(
            "hourly",
            "Hourly check",
            SchedulePattern::Hourly,
        ));
        scheduler.add_schedule(CronSchedule::new(
            "daily",
            "Daily check",
            SchedulePattern::Daily,
        ));

        let midnight = chrono::NaiveDate::from_ymd_opt(2026, 1, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

        // Both should fire at midnight.
        let fired = scheduler.check_schedules(midnight);
        assert_eq!(fired.len(), 2);
        assert!(fired.contains(&"hourly".to_string()));
        assert!(fired.contains(&"daily".to_string()));

        // Second check at same time — neither should fire again.
        let fired_again = scheduler.check_schedules(midnight);
        assert!(fired_again.is_empty());
    }

    #[test]
    fn action_status_is_terminal() {
        assert!(!ActionStatus::Pending.is_terminal());
        assert!(!ActionStatus::Executing.is_terminal());
        assert!(ActionStatus::Completed.is_terminal());
        assert!(ActionStatus::Failed.is_terminal());
        assert!(ActionStatus::Cancelled.is_terminal());
    }

    // ── Additional coverage tests ──────────────────────────────────

    #[test]
    fn action_status_display_all_variants() {
        assert_eq!(ActionStatus::Pending.to_string(), "pending");
        assert_eq!(ActionStatus::Executing.to_string(), "executing");
        assert_eq!(ActionStatus::Completed.to_string(), "completed");
        assert_eq!(ActionStatus::Failed.to_string(), "failed");
        assert_eq!(ActionStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn action_status_serde_roundtrip() {
        let statuses = [
            ActionStatus::Pending,
            ActionStatus::Executing,
            ActionStatus::Completed,
            ActionStatus::Failed,
            ActionStatus::Cancelled,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let back: ActionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, s);
        }
    }

    #[test]
    fn scheduled_action_with_max_retries() {
        let action = make_action("test").with_max_retries(5);
        assert_eq!(action.max_retries, 5);
        assert_eq!(action.retries_remaining, 5);
    }

    #[test]
    fn scheduled_action_can_retry_only_when_failed_with_retries() {
        let mut action = make_action("test").with_max_retries(1);
        // Pending: not eligible for retry.
        assert!(!action.can_retry());

        // Simulate fail with retries remaining.
        action.status = ActionStatus::Failed;
        action.retries_remaining = 1;
        assert!(action.can_retry());

        // Exhausted retries.
        action.retries_remaining = 0;
        assert!(!action.can_retry());

        // Completed status: not retryable even with retries remaining.
        action.status = ActionStatus::Completed;
        action.retries_remaining = 5;
        assert!(!action.can_retry());
    }

    #[test]
    fn scheduled_action_equality_by_action_id() {
        let a1 = make_action("policy_a");
        let mut a2 = make_action("policy_b");
        // Different policy_id means different action_id (UUID), so not equal.
        assert_ne!(a1, a2);

        // Same action_id means equal regardless of other fields.
        a2.action_id = a1.action_id.clone();
        assert_eq!(a1, a2);
    }

    #[test]
    fn scheduled_action_is_ready_not_when_non_pending() {
        let now = Utc::now();
        let mut action = make_action("test");
        action.status = ActionStatus::Executing;
        assert!(!action.is_ready(now));

        action.status = ActionStatus::Completed;
        assert!(!action.is_ready(now));

        action.status = ActionStatus::Failed;
        assert!(!action.is_ready(now));

        action.status = ActionStatus::Cancelled;
        assert!(!action.is_ready(now));
    }

    #[test]
    fn scheduled_action_is_expired_no_deadline() {
        let action = make_action("test");
        let now = Utc::now();
        // No deadline means never expired.
        assert!(!action.is_expired(now));
    }

    #[test]
    fn scheduler_mark_executing_fails_when_not_pending() {
        let mut scheduler = ActionScheduler::new();
        let action = make_action("test");
        let id = action.action_id.clone();
        scheduler.schedule(action);

        // Move to Executing.
        assert!(scheduler.mark_executing(&id));
        // Second call fails (already Executing, not Pending).
        assert!(!scheduler.mark_executing(&id));
    }

    #[test]
    fn scheduler_mark_completed_fails_when_not_executing() {
        let mut scheduler = ActionScheduler::new();
        let action = make_action("test");
        let id = action.action_id.clone();
        scheduler.schedule(action);

        // Can't complete from Pending.
        assert!(!scheduler.mark_completed(&id));
    }

    #[test]
    fn scheduler_mark_failed_fails_when_not_executing() {
        let mut scheduler = ActionScheduler::new();
        let action = make_action("test");
        let id = action.action_id.clone();
        scheduler.schedule(action);

        // Can't fail from Pending.
        assert!(!scheduler.mark_failed(&id, "error".into()));
    }

    #[test]
    fn scheduler_cancel_nonexistent_action() {
        let mut scheduler = ActionScheduler::new();
        assert!(!scheduler.cancel("nonexistent"));
    }

    #[test]
    fn scheduler_get_action_nonexistent() {
        let scheduler = ActionScheduler::new();
        assert!(scheduler.get_action("nonexistent").is_none());
    }

    #[test]
    fn scheduler_default_is_empty() {
        let scheduler = ActionScheduler::default();
        assert_eq!(scheduler.action_count(), 0);
        assert!(scheduler.schedules().is_empty());
    }

    #[test]
    fn cron_schedule_weekly() {
        let schedule = CronSchedule::new("weekly", "Weekly check", SchedulePattern::Weekly);
        // Monday at midnight.
        let monday_midnight = chrono::NaiveDate::from_ymd_opt(2026, 1, 19) // 2026-01-19 is Monday
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        assert!(schedule.should_fire(monday_midnight));

        // Tuesday at midnight.
        let tuesday = chrono::NaiveDate::from_ymd_opt(2026, 1, 20)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        assert!(!schedule.should_fire(tuesday));
    }

    #[test]
    fn cron_schedule_monthly() {
        let schedule = CronSchedule::new("monthly", "Monthly check", SchedulePattern::Monthly);
        // First of month at midnight.
        let first = chrono::NaiveDate::from_ymd_opt(2026, 3, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        assert!(schedule.should_fire(first));

        // Second day.
        let second = chrono::NaiveDate::from_ymd_opt(2026, 3, 2)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        assert!(!schedule.should_fire(second));
    }

    #[test]
    fn cron_schedule_yearly() {
        let schedule = CronSchedule::new("yearly", "Yearly check", SchedulePattern::Yearly);
        // Jan 1 at midnight.
        let jan_first = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        assert!(schedule.should_fire(jan_first));

        // Feb 1 at midnight — not yearly.
        let feb_first = chrono::NaiveDate::from_ymd_opt(2026, 2, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        assert!(!schedule.should_fire(feb_first));
    }

    #[test]
    fn cron_schedule_inactive_does_not_fire() {
        let mut schedule = CronSchedule::new("hourly", "Hourly check", SchedulePattern::Hourly);
        schedule.active = false;
        let at_zero = chrono::NaiveDate::from_ymd_opt(2026, 1, 15)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_utc();
        assert!(!schedule.should_fire(at_zero));
    }

    #[test]
    fn scheduler_expired_actions_excludes_terminal() {
        let mut scheduler = ActionScheduler::new();
        let now = Utc::now();
        let action = make_action("test").with_deadline(now - chrono::Duration::hours(1));
        let id = action.action_id.clone();
        scheduler.schedule(action);

        // Pending + expired → shows in expired_actions.
        assert_eq!(scheduler.expired_actions(now).len(), 1);

        // Cancel it (terminal), then it should NOT appear in expired_actions.
        scheduler.cancel(&id);
        assert!(scheduler.expired_actions(now).is_empty());
    }

    #[test]
    fn scheduler_schedules_accessor() {
        let mut scheduler = ActionScheduler::new();
        scheduler.add_schedule(CronSchedule::new(
            "s1",
            "Schedule 1",
            SchedulePattern::Daily,
        ));
        scheduler.add_schedule(CronSchedule::new(
            "s2",
            "Schedule 2",
            SchedulePattern::Hourly,
        ));
        assert_eq!(scheduler.schedules().len(), 2);
    }

    #[test]
    fn scheduled_action_serde_roundtrip() {
        let action = make_action("policy_serde");
        let json = serde_json::to_string(&action).unwrap();
        let back: ScheduledAction = serde_json::from_str(&json).unwrap();
        assert_eq!(back.action_id, action.action_id);
        assert_eq!(back.policy_id, "policy_serde");
        assert_eq!(back.status, ActionStatus::Pending);
        assert_eq!(back.action, PolicyAction::Halt);
    }

    #[test]
    fn cron_schedule_serde_roundtrip() {
        let schedule = CronSchedule::new("test_sched", "Test schedule", SchedulePattern::Daily);
        let json = serde_json::to_string(&schedule).unwrap();
        let back: CronSchedule = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schedule_id, "test_sched");
        assert_eq!(back.pattern, SchedulePattern::Daily);
        assert!(back.active);
        assert!(back.last_fired.is_none());
    }

    #[test]
    fn scheduler_mark_failed_with_retries_resets_to_pending() {
        let mut scheduler = ActionScheduler::new();
        let action = make_action("test").with_max_retries(1);
        let id = action.action_id.clone();
        scheduler.schedule(action);

        scheduler.mark_executing(&id);
        scheduler.mark_failed(&id, "err1".into());

        let a = scheduler.get_action(&id).unwrap();
        // With 1 retry, after first fail it goes back to Pending.
        assert_eq!(a.status, ActionStatus::Pending);
        assert_eq!(a.retries_remaining, 0);
        assert_eq!(a.last_error.as_deref(), Some("err1"));
    }
}
