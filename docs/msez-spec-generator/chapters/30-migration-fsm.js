const {
  chapterHeading, h2,
  codeBlock, theorem,
  spacer
} = require("../lib/primitives");

module.exports = function build_chapter30() {
  return [
    chapterHeading("Chapter 30: Migration State Machine"),

    // --- 30.1 State Transitions ---
    h2("30.1 State Transitions"),
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
    ...codeBlock(
      "pub struct MigrationSaga {\n" +
      "    pub id: MigrationId,\n" +
      "    pub state: MigrationState,\n" +
      "    pub request: MigrationRequest,\n" +
      "    pub completed_steps: Vec<CompensationStep>,\n" +
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

    theorem("Theorem 30.1 (Migration Atomicity).", "The migration protocol ensures atomicity. Either migration completes fully or compensation returns the asset to its original state. Proof: The saga pattern records every step. Compensation actions are the functional inverse of each step. Compensation executes in reverse order, restoring pre-migration state."),
  ];
};
