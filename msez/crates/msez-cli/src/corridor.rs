//! # Corridor Subcommand
//!
//! Corridor lifecycle management commands. Operates on local state files
//! backed by the `msez-state` typestate machine.
//!
//! ## Subcommands
//!
//! - `create` — Create a new corridor draft.
//! - `submit` — Submit a draft corridor for regulatory review.
//! - `activate` — Activate an approved corridor.
//! - `halt` — Emergency halt an active corridor.
//! - `suspend` — Temporarily suspend an active corridor.
//! - `resume` — Resume a suspended corridor.
//! - `status` — Show current corridor state.
//! - `list` — List all known corridors.
//!
//! ## Phase 1
//!
//! For Phase 1, corridor state is stored as JSON files in a local state
//! directory. Database-backed operations come with `msez-api`.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};

use msez_state::{DynCorridorData, DynCorridorState};

/// Arguments for the `msez corridor` subcommand.
#[derive(Args, Debug)]
pub struct CorridorArgs {
    #[command(subcommand)]
    pub command: CorridorCommand,
}

/// Corridor subcommands.
#[derive(Subcommand, Debug)]
pub enum CorridorCommand {
    /// Create a new corridor in DRAFT state.
    Create {
        /// Corridor identifier (e.g., "pk-rsez--ae-difc").
        #[arg(long)]
        id: String,
        /// Jurisdiction A identifier.
        #[arg(long)]
        jurisdiction_a: String,
        /// Jurisdiction B identifier.
        #[arg(long)]
        jurisdiction_b: String,
    },

    /// Submit a draft corridor for regulatory approval (DRAFT → PENDING).
    Submit {
        /// Corridor identifier.
        #[arg(long)]
        id: String,
        /// Path to the bilateral agreement artifact.
        #[arg(long)]
        agreement: PathBuf,
        /// Path to the pack trilogy artifact.
        #[arg(long)]
        pack_trilogy: PathBuf,
    },

    /// Activate an approved corridor (PENDING → ACTIVE).
    Activate {
        /// Corridor identifier.
        #[arg(long)]
        id: String,
        /// Regulatory approval digest from jurisdiction A.
        #[arg(long)]
        approval_a: String,
        /// Regulatory approval digest from jurisdiction B.
        #[arg(long)]
        approval_b: String,
    },

    /// Halt an active corridor by authority order (ACTIVE → HALTED).
    Halt {
        /// Corridor identifier.
        #[arg(long)]
        id: String,
        /// Reason for halting.
        #[arg(long)]
        reason: String,
        /// Authority jurisdiction issuing the halt.
        #[arg(long)]
        authority: String,
    },

    /// Temporarily suspend an active corridor (ACTIVE → SUSPENDED).
    Suspend {
        /// Corridor identifier.
        #[arg(long)]
        id: String,
        /// Reason for suspension.
        #[arg(long)]
        reason: String,
    },

    /// Resume a suspended corridor (SUSPENDED → ACTIVE).
    Resume {
        /// Corridor identifier.
        #[arg(long)]
        id: String,
        /// Resolution attestation digest.
        #[arg(long)]
        resolution: String,
    },

    /// Show current corridor state.
    Status {
        /// Corridor identifier.
        #[arg(long)]
        id: String,
    },

    /// List all known corridors.
    List,
}

/// Execute the corridor subcommand.
pub fn run_corridor(args: &CorridorArgs, repo_root: &Path) -> Result<u8> {
    let state_dir = repo_root.join(".msez").join("corridors");

    match &args.command {
        CorridorCommand::Create {
            id,
            jurisdiction_a,
            jurisdiction_b,
        } => cmd_create(&state_dir, id, jurisdiction_a, jurisdiction_b),

        CorridorCommand::Submit {
            id,
            agreement: _,
            pack_trilogy: _,
        } => cmd_transition(&state_dir, id, DynCorridorState::Pending),

        CorridorCommand::Activate {
            id,
            approval_a: _,
            approval_b: _,
        } => cmd_transition(&state_dir, id, DynCorridorState::Active),

        CorridorCommand::Halt {
            id,
            reason: _,
            authority: _,
        } => cmd_transition(&state_dir, id, DynCorridorState::Halted),

        CorridorCommand::Suspend { id, reason: _ } => {
            cmd_transition(&state_dir, id, DynCorridorState::Suspended)
        }

        CorridorCommand::Resume {
            id,
            resolution: _,
        } => cmd_transition(&state_dir, id, DynCorridorState::Active),

        CorridorCommand::Status { id } => cmd_status(&state_dir, id),

        CorridorCommand::List => cmd_list(&state_dir),
    }
}

/// Create a new corridor state file in DRAFT state.
fn cmd_create(
    state_dir: &Path,
    id: &str,
    jurisdiction_a: &str,
    jurisdiction_b: &str,
) -> Result<u8> {
    std::fs::create_dir_all(state_dir)
        .context("failed to create corridor state directory")?;

    let state_file = state_dir.join(format!("{id}.json"));
    if state_file.exists() {
        bail!("corridor already exists: {id}");
    }

    let now = chrono::Utc::now();
    let data = DynCorridorData {
        id: msez_core::CorridorId::new(),
        jurisdiction_a: msez_core::JurisdictionId::new(jurisdiction_a)
            .context("invalid jurisdiction_a")?,
        jurisdiction_b: msez_core::JurisdictionId::new(jurisdiction_b)
            .context("invalid jurisdiction_b")?,
        state: DynCorridorState::Draft,
        created_at: now,
        updated_at: now,
        transition_log: Vec::new(),
    };

    let json = serde_json::to_string_pretty(&data)?;
    std::fs::write(&state_file, json)?;

    println!("OK: created corridor {id} in DRAFT state");
    Ok(0)
}

/// Transition a corridor to a new state.
fn cmd_transition(state_dir: &Path, id: &str, target: DynCorridorState) -> Result<u8> {
    let state_file = state_dir.join(format!("{id}.json"));
    if !state_file.exists() {
        bail!("corridor not found: {id}");
    }

    let content = std::fs::read_to_string(&state_file)?;
    let mut data: DynCorridorData = serde_json::from_str(&content)?;

    // Validate transition is allowed.
    let valid_targets = data.state.valid_transitions();
    if !valid_targets.contains(&target) {
        bail!(
            "invalid transition: {} → {} (valid targets: {:?})",
            data.state,
            target,
            valid_targets.iter().map(|s| s.as_str()).collect::<Vec<_>>()
        );
    }

    let from = data.state.as_str().to_string();
    let to = target.as_str().to_string();
    let now = chrono::Utc::now();

    data.transition_log.push(msez_state::TransitionRecord {
        from_state: from.clone(),
        to_state: to.clone(),
        timestamp: now,
        evidence_digest: None,
    });
    data.state = target;
    data.updated_at = now;

    let json = serde_json::to_string_pretty(&data)?;
    std::fs::write(&state_file, json)?;

    println!("OK: corridor {id} transitioned {from} → {to}");
    Ok(0)
}

/// Show corridor status.
fn cmd_status(state_dir: &Path, id: &str) -> Result<u8> {
    let state_file = state_dir.join(format!("{id}.json"));
    if !state_file.exists() {
        bail!("corridor not found: {id}");
    }

    let content = std::fs::read_to_string(&state_file)?;
    let data: DynCorridorData = serde_json::from_str(&content)?;

    println!("Corridor: {id}");
    println!("  State: {}", data.state);
    println!("  Jurisdiction A: {}", data.jurisdiction_a);
    println!("  Jurisdiction B: {}", data.jurisdiction_b);
    println!("  Created: {}", data.created_at);
    println!("  Updated: {}", data.updated_at);
    println!("  Transitions: {}", data.transition_log.len());

    for (i, t) in data.transition_log.iter().enumerate() {
        println!(
            "    [{i}] {} → {} at {}",
            t.from_state, t.to_state, t.timestamp
        );
    }

    Ok(0)
}

/// List all corridors in the state directory.
fn cmd_list(state_dir: &Path) -> Result<u8> {
    if !state_dir.is_dir() {
        println!("No corridors found (state directory does not exist).");
        return Ok(0);
    }

    let mut count = 0;
    let mut entries: Vec<(String, DynCorridorState)> = Vec::new();

    for entry in std::fs::read_dir(state_dir)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(data) = serde_json::from_str::<DynCorridorData>(&content) {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    entries.push((name, data.state));
                    count += 1;
                }
            }
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    if entries.is_empty() {
        println!("No corridors found.");
    } else {
        println!("Corridors ({count}):");
        for (name, state) in &entries {
            println!("  {name}: {state}");
        }
    }

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corridor_create_and_status() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        let result = cmd_create(&state_dir, "test-corridor", "PK-RSEZ", "AE-DIFC");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        let result = cmd_status(&state_dir, "test-corridor");
        assert!(result.is_ok());
    }

    #[test]
    fn corridor_transition_draft_to_pending() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "test-cor", "PK", "AE").unwrap();
        let result = cmd_transition(&state_dir, "test-cor", DynCorridorState::Pending);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn corridor_invalid_transition_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "test-cor", "PK", "AE").unwrap();
        // Draft → Active is not valid (must go through Pending).
        let result = cmd_transition(&state_dir, "test-cor", DynCorridorState::Active);
        assert!(result.is_err());
    }

    #[test]
    fn corridor_list_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        let result = cmd_list(&state_dir);
        assert!(result.is_ok());
    }
}
