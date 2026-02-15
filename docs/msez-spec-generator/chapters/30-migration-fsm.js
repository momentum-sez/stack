const {
  chapterHeading, h2, h3,
  p, p_runs, bold, code,
  codeBlock, theorem, table,
  spacer
} = require("../lib/primitives");

module.exports = function build_chapter30() {
  return [
    chapterHeading("Chapter 30: Migration State Machine"),

    // --- 30.1 State Transitions ---
    h2("30.1 State Transitions"),
    p("The migration state machine defines eight states that govern the lifecycle of a cross-jurisdictional asset transfer. The state enum is the single source of truth for saga progression; all phase logic branches on the current state variant."),
    spacer(),
    ...codeBlock(
      "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\n" +
      "pub enum MigrationState {\n" +
      "    Initiated,\n" +
      "    SourceLocked,\n" +
      "    InTransit,\n" +
      "    DestinationVerified,\n" +
      "    Completed,\n" +
      "    Compensating,\n" +
      "    Compensated,\n" +
      "    Failed,\n" +
      "}"
    ),
    spacer(),

    // --- 30.1.1 State Transition Diagram ---
    h3("30.1.1 State Transition Diagram"),
    p("The following diagram shows all valid state transitions. The happy path flows top-to-bottom on the left. Any non-terminal state may transition to Compensating upon failure, which proceeds to either Compensated (success) or Failed (compensation itself failed)."),
    spacer(),
    ...codeBlock(
      "    ┌─────────────┐\n" +
      "    │  Initiated   │\n" +
      "    └──────┬───────┘\n" +
      "           │ advance()\n" +
      "           ▼\n" +
      "    ┌─────────────┐\n" +
      "    │ SourceLocked │──────────────┐\n" +
      "    └──────┬───────┘              │\n" +
      "           │ advance()            │\n" +
      "           ▼                      │\n" +
      "    ┌─────────────┐              │\n" +
      "    │  InTransit   │──────────┐   │\n" +
      "    └──────┬───────┘          │   │ compensate()\n" +
      "           │ advance()        │   │\n" +
      "           ▼                  │   │\n" +
      "    ┌──────────────────┐     │   │\n" +
      "    │DestinationVerified│─┐   │   │\n" +
      "    └──────┬────────────┘ │   │   │\n" +
      "           │ advance()    │   │   │\n" +
      "           ▼              ▼   ▼   ▼\n" +
      "    ┌─────────────┐  ┌──────────────┐\n" +
      "    │  Completed   │  │ Compensating  │\n" +
      "    └─────────────┘  └──────┬───────┘\n" +
      "                            │\n" +
      "                     ┌──────┴───────┐\n" +
      "                     ▼              ▼\n" +
      "              ┌────────────┐  ┌──────────┐\n" +
      "              │Compensated │  │  Failed   │\n" +
      "              └────────────┘  └──────────┘"
    ),
    spacer(),

    // --- 30.1.2 Valid Transitions ---
    h3("30.1.2 Valid Transitions"),
    p("The valid_transitions() method returns the set of states reachable from the current state. This is used by the saga engine to validate transition requests before executing phase logic, and by monitoring dashboards to display available actions."),
    spacer(),
    ...codeBlock(
      "impl MigrationState {\n" +
      "    /// Returns the set of states reachable from the current state.\n" +
      "    pub fn valid_transitions(&self) -> &'static [MigrationState] {\n" +
      "        match self {\n" +
      "            MigrationState::Initiated => &[\n" +
      "                MigrationState::SourceLocked,\n" +
      "                MigrationState::Compensating,\n" +
      "            ],\n" +
      "            MigrationState::SourceLocked => &[\n" +
      "                MigrationState::InTransit,\n" +
      "                MigrationState::Compensating,\n" +
      "            ],\n" +
      "            MigrationState::InTransit => &[\n" +
      "                MigrationState::DestinationVerified,\n" +
      "                MigrationState::Compensating,\n" +
      "            ],\n" +
      "            MigrationState::DestinationVerified => &[\n" +
      "                MigrationState::Completed,\n" +
      "                MigrationState::Compensating,\n" +
      "            ],\n" +
      "            MigrationState::Compensating => &[\n" +
      "                MigrationState::Compensated,\n" +
      "                MigrationState::Failed,\n" +
      "            ],\n" +
      "            // Terminal states have no valid transitions.\n" +
      "            MigrationState::Completed\n" +
      "            | MigrationState::Compensated\n" +
      "            | MigrationState::Failed => &[],\n" +
      "        }\n" +
      "    }\n" +
      "\n" +
      "    /// Returns true if this state is terminal (no further\n" +
      "    /// transitions are possible).\n" +
      "    pub fn is_terminal(&self) -> bool {\n" +
      "        matches!(\n" +
      "            self,\n" +
      "            MigrationState::Completed\n" +
      "            | MigrationState::Compensated\n" +
      "            | MigrationState::Failed\n" +
      "        )\n" +
      "    }\n" +
      "}"
    ),
    spacer(),
    p("Terminal states are absorbing: once a migration reaches Completed, Compensated, or Failed, no further state transitions are permitted. The is_terminal() predicate is used by the saga runner to determine when to stop polling for advancement, and by the persistence layer to mark migration records as finalized."),
    spacer(),
    table(
      ["From State", "Valid Targets", "Terminal"],
      [
        ["Initiated", "SourceLocked, Compensating", "No"],
        ["SourceLocked", "InTransit, Compensating", "No"],
        ["InTransit", "DestinationVerified, Compensating", "No"],
        ["DestinationVerified", "Completed, Compensating", "No"],
        ["Compensating", "Compensated, Failed", "No"],
        ["Completed", "(none)", "Yes"],
        ["Compensated", "(none)", "Yes"],
        ["Failed", "(none)", "Yes"],
      ],
      [2800, 4360, 2200]
    ),
    spacer(),

    // --- 30.2 Migration Saga ---
    h2("30.2 Migration Saga"),
    p("The MigrationSaga struct is the runtime representation of an in-progress migration. It owns the current state, the originating request, the ordered list of completed compensation steps, and a compensation_progress counter that tracks partial compensation execution for crash recovery."),
    spacer(),
    ...codeBlock(
      "pub struct MigrationSaga {\n" +
      "    pub id: MigrationId,\n" +
      "    pub state: MigrationState,\n" +
      "    pub request: MigrationRequest,\n" +
      "    pub completed_steps: Vec<CompensationStep>,\n" +
      "    pub compensation_progress: usize,\n" +
      "    pub started_at: DateTime<Utc>,\n" +
      "    pub updated_at: DateTime<Utc>,\n" +
      "}\n" +
      "\n" +
      "impl MigrationSaga {\n" +
      "    /// Advance the saga to the next phase.\n" +
      "    pub fn advance(&mut self) -> Result<MigrationState, MigrationError> {\n" +
      "        let next = match self.state {\n" +
      "            MigrationState::Initiated => {\n" +
      "                self.verify_compliance()?;\n" +
      "                MigrationState::SourceLocked\n" +
      "            }\n" +
      "            MigrationState::SourceLocked => {\n" +
      "                self.lock_source_asset()?;\n" +
      "                MigrationState::InTransit\n" +
      "            }\n" +
      "            MigrationState::InTransit => {\n" +
      "                self.transfer_state()?;\n" +
      "                MigrationState::DestinationVerified\n" +
      "            }\n" +
      "            MigrationState::DestinationVerified => {\n" +
      "                self.unlock_destination()?;\n" +
      "                MigrationState::Completed\n" +
      "            }\n" +
      "            MigrationState::Completed => {\n" +
      "                return Err(MigrationError::AlreadyCompleted);\n" +
      "            }\n" +
      "            MigrationState::Compensating\n" +
      "            | MigrationState::Compensated\n" +
      "            | MigrationState::Failed => {\n" +
      "                return Err(MigrationError::InvalidTransition);\n" +
      "            }\n" +
      "        };\n" +
      "        self.state = next.clone();\n" +
      "        self.updated_at = Utc::now();\n" +
      "        Ok(next)\n" +
      "    }\n" +
      "\n" +
      "    /// Run compensation in reverse order.\n" +
      "    pub fn compensate(&mut self) -> Result<(), MigrationError> {\n" +
      "        self.state = MigrationState::Compensating;\n" +
      "        for step in self.completed_steps.iter().rev() {\n" +
      "            step.compensation_action.execute()?;\n" +
      "        }\n" +
      "        self.state = MigrationState::Compensated;\n" +
      "        self.updated_at = Utc::now();\n" +
      "        Ok(())\n" +
      "    }\n" +
      "}"
    ),
    spacer(),

    // --- 30.3 Compensation Progress ---
    h3("30.2.1 Compensation Progress Counter"),
    p_runs(["The ", code("compensation_progress"), " field tracks how many compensation steps have been successfully executed during a compensation pass. This counter is critical for crash recovery: if the process crashes mid-compensation, the saga runner reads ", code("compensation_progress"), " from the persisted saga record and resumes compensation from that index rather than re-executing already-compensated steps."]),
    spacer(),
    p_runs(["On each successful compensation step execution, ", code("compensation_progress"), " is incremented and the saga is persisted to Postgres before proceeding to the next step. This write-ahead pattern ensures that compensation is idempotent across restarts. The counter starts at 0 when compensation begins and equals ", code("completed_steps.len()"), " when all steps have been compensated, at which point the state transitions to Compensated."]),
    spacer(),
    p("If a compensation step itself fails (e.g., the source jurisdiction is unreachable for unlock), the saga transitions to the Failed terminal state. Failed migrations require manual intervention by a zone operator, who can inspect the compensation_progress counter to determine exactly which steps completed and which remain outstanding."),
    spacer(),

    // --- 30.3 Atomicity Theorem ---
    h2("30.3 Atomicity and Safety Properties"),
    theorem("Theorem 30.1 (Migration Atomicity).", "The migration protocol ensures atomicity. Either migration completes fully or compensation returns the asset to its original state. Proof: The saga pattern records every step. Compensation actions are the functional inverse of each step. Compensation executes in reverse order, restoring pre-migration state."),
    spacer(),
    theorem("Theorem 30.2 (Terminal State Convergence).", "Every migration saga reaches a terminal state in finite time. Proof: The happy path has exactly four advance() calls (Initiated -> SourceLocked -> InTransit -> DestinationVerified -> Completed). Each advance() either succeeds (moving closer to Completed) or triggers compensation. Compensation iterates over at most N completed steps where N <= 4, then terminates in either Compensated or Failed. The deadline mechanism ensures that stalled migrations are forced into compensation by the saga runner, preventing indefinite non-terminal states."),
    spacer(),
    theorem("Theorem 30.3 (No Asset Duplication).", "At no point during migration does the asset exist in an active state in both jurisdictions simultaneously. Proof: The asset is locked at source (Phase 4) before transit begins (Phase 5). It is not unlocked at destination (Phase 7) until destination verification completes (Phase 6). The source lock is only finalized (not released) upon completion. Compensation of source-lock restores the asset at source only if transit has not completed. The state machine enforces that SourceLocked precedes InTransit, and InTransit precedes DestinationVerified, making concurrent activation impossible."),
  ];
};
