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

        CorridorCommand::Resume { id, resolution: _ } => {
            cmd_transition(&state_dir, id, DynCorridorState::Active)
        }

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
    std::fs::create_dir_all(state_dir).context("failed to create corridor state directory")?;

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

    // ── Additional coverage tests ────────────────────────────────────

    #[test]
    fn corridor_create_duplicate_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "dup-cor", "PK", "AE").unwrap();
        let result = cmd_create(&state_dir, "dup-cor", "PK", "AE");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("already exists"));
    }

    #[test]
    fn corridor_create_with_empty_jurisdiction_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        let result = cmd_create(&state_dir, "bad-cor", "", "AE");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("jurisdiction_a"));
    }

    #[test]
    fn corridor_create_with_whitespace_jurisdiction_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        let result = cmd_create(&state_dir, "bad-cor", "   ", "AE");
        assert!(result.is_err());
    }

    #[test]
    fn corridor_transition_nonexistent_corridor() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_transition(&state_dir, "nonexistent", DynCorridorState::Pending);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("corridor not found"));
    }

    #[test]
    fn corridor_full_lifecycle_draft_to_halted() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "lifecycle-cor", "PK", "AE").unwrap();
        cmd_transition(&state_dir, "lifecycle-cor", DynCorridorState::Pending).unwrap();
        cmd_transition(&state_dir, "lifecycle-cor", DynCorridorState::Active).unwrap();
        let result = cmd_transition(&state_dir, "lifecycle-cor", DynCorridorState::Halted);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn corridor_full_lifecycle_to_suspended_and_resume() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "suspend-cor", "PK", "AE").unwrap();
        cmd_transition(&state_dir, "suspend-cor", DynCorridorState::Pending).unwrap();
        cmd_transition(&state_dir, "suspend-cor", DynCorridorState::Active).unwrap();
        cmd_transition(&state_dir, "suspend-cor", DynCorridorState::Suspended).unwrap();
        // Resume goes back to Active.
        let result = cmd_transition(&state_dir, "suspend-cor", DynCorridorState::Active);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn corridor_full_lifecycle_to_deprecated() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "dep-cor", "PK", "AE").unwrap();
        cmd_transition(&state_dir, "dep-cor", DynCorridorState::Pending).unwrap();
        cmd_transition(&state_dir, "dep-cor", DynCorridorState::Active).unwrap();
        cmd_transition(&state_dir, "dep-cor", DynCorridorState::Halted).unwrap();
        let result = cmd_transition(&state_dir, "dep-cor", DynCorridorState::Deprecated);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn corridor_deprecated_has_no_transitions() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "term-cor", "PK", "AE").unwrap();
        cmd_transition(&state_dir, "term-cor", DynCorridorState::Pending).unwrap();
        cmd_transition(&state_dir, "term-cor", DynCorridorState::Active).unwrap();
        cmd_transition(&state_dir, "term-cor", DynCorridorState::Halted).unwrap();
        cmd_transition(&state_dir, "term-cor", DynCorridorState::Deprecated).unwrap();

        // No valid transitions from Deprecated.
        let result = cmd_transition(&state_dir, "term-cor", DynCorridorState::Active);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid transition"));
    }

    #[test]
    fn corridor_pending_to_halted_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "bad-cor", "PK", "AE").unwrap();
        cmd_transition(&state_dir, "bad-cor", DynCorridorState::Pending).unwrap();
        // Pending → Halted not valid.
        let result = cmd_transition(&state_dir, "bad-cor", DynCorridorState::Halted);
        assert!(result.is_err());
    }

    #[test]
    fn corridor_status_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_status(&state_dir, "ghost");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("corridor not found"));
    }

    #[test]
    fn corridor_status_shows_transition_log() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "log-cor", "PK", "AE").unwrap();
        cmd_transition(&state_dir, "log-cor", DynCorridorState::Pending).unwrap();
        cmd_transition(&state_dir, "log-cor", DynCorridorState::Active).unwrap();

        let result = cmd_status(&state_dir, "log-cor");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Read the state file to verify the log.
        let state_file = state_dir.join("log-cor.json");
        let content = std::fs::read_to_string(&state_file).unwrap();
        let data: DynCorridorData = serde_json::from_str(&content).unwrap();
        assert_eq!(data.transition_log.len(), 2);
        assert_eq!(data.transition_log[0].from_state, "DRAFT");
        assert_eq!(data.transition_log[0].to_state, "PENDING");
        assert_eq!(data.transition_log[1].from_state, "PENDING");
        assert_eq!(data.transition_log[1].to_state, "ACTIVE");
    }

    #[test]
    fn corridor_list_with_entries() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "alpha", "PK", "AE").unwrap();
        cmd_create(&state_dir, "beta", "US", "UK").unwrap();
        cmd_create(&state_dir, "gamma", "CN", "JP").unwrap();

        let result = cmd_list(&state_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn corridor_list_ignores_non_json_files() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Create non-JSON files.
        std::fs::write(state_dir.join("readme.txt"), b"not a corridor").unwrap();
        std::fs::write(state_dir.join("notes.md"), b"# notes").unwrap();

        let result = cmd_list(&state_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn corridor_list_ignores_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Create a JSON file with invalid corridor data.
        std::fs::write(state_dir.join("bad.json"), b"not valid json").unwrap();

        // Should not error, just skip the invalid file.
        let result = cmd_list(&state_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn corridor_create_writes_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "json-test", "PK-RSEZ", "AE-DIFC").unwrap();

        let state_file = state_dir.join("json-test.json");
        assert!(state_file.exists());

        let content = std::fs::read_to_string(&state_file).unwrap();
        let data: DynCorridorData = serde_json::from_str(&content).unwrap();
        assert_eq!(data.state, DynCorridorState::Draft);
        assert!(data.transition_log.is_empty());
    }

    #[test]
    fn run_corridor_create_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        let args = CorridorArgs {
            command: CorridorCommand::Create {
                id: "run-test".to_string(),
                jurisdiction_a: "PK".to_string(),
                jurisdiction_b: "AE".to_string(),
            },
        };
        let result = run_corridor(&args, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_corridor_submit_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        // First create.
        let create_args = CorridorArgs {
            command: CorridorCommand::Create {
                id: "submit-test".to_string(),
                jurisdiction_a: "PK".to_string(),
                jurisdiction_b: "AE".to_string(),
            },
        };
        run_corridor(&create_args, dir.path()).unwrap();

        // Then submit.
        let submit_args = CorridorArgs {
            command: CorridorCommand::Submit {
                id: "submit-test".to_string(),
                agreement: PathBuf::from("agreement.json"),
                pack_trilogy: PathBuf::from("trilogy.json"),
            },
        };
        let result = run_corridor(&submit_args, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_corridor_activate_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        let create_args = CorridorArgs {
            command: CorridorCommand::Create {
                id: "act-test".to_string(),
                jurisdiction_a: "PK".to_string(),
                jurisdiction_b: "AE".to_string(),
            },
        };
        run_corridor(&create_args, dir.path()).unwrap();

        let submit_args = CorridorArgs {
            command: CorridorCommand::Submit {
                id: "act-test".to_string(),
                agreement: PathBuf::from("agreement.json"),
                pack_trilogy: PathBuf::from("trilogy.json"),
            },
        };
        run_corridor(&submit_args, dir.path()).unwrap();

        let activate_args = CorridorArgs {
            command: CorridorCommand::Activate {
                id: "act-test".to_string(),
                approval_a: "digest_a".to_string(),
                approval_b: "digest_b".to_string(),
            },
        };
        let result = run_corridor(&activate_args, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_corridor_halt_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        // Build to Active.
        let steps = [
            CorridorArgs {
                command: CorridorCommand::Create {
                    id: "halt-test".to_string(),
                    jurisdiction_a: "PK".to_string(),
                    jurisdiction_b: "AE".to_string(),
                },
            },
            CorridorArgs {
                command: CorridorCommand::Submit {
                    id: "halt-test".to_string(),
                    agreement: PathBuf::from("a.json"),
                    pack_trilogy: PathBuf::from("t.json"),
                },
            },
            CorridorArgs {
                command: CorridorCommand::Activate {
                    id: "halt-test".to_string(),
                    approval_a: "da".to_string(),
                    approval_b: "db".to_string(),
                },
            },
        ];
        for step in &steps {
            run_corridor(step, dir.path()).unwrap();
        }

        let halt_args = CorridorArgs {
            command: CorridorCommand::Halt {
                id: "halt-test".to_string(),
                reason: "Fork detected".to_string(),
                authority: "PK".to_string(),
            },
        };
        let result = run_corridor(&halt_args, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_corridor_suspend_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        let steps = [
            CorridorArgs {
                command: CorridorCommand::Create {
                    id: "susp-test".to_string(),
                    jurisdiction_a: "PK".to_string(),
                    jurisdiction_b: "AE".to_string(),
                },
            },
            CorridorArgs {
                command: CorridorCommand::Submit {
                    id: "susp-test".to_string(),
                    agreement: PathBuf::from("a.json"),
                    pack_trilogy: PathBuf::from("t.json"),
                },
            },
            CorridorArgs {
                command: CorridorCommand::Activate {
                    id: "susp-test".to_string(),
                    approval_a: "da".to_string(),
                    approval_b: "db".to_string(),
                },
            },
        ];
        for step in &steps {
            run_corridor(step, dir.path()).unwrap();
        }

        let suspend_args = CorridorArgs {
            command: CorridorCommand::Suspend {
                id: "susp-test".to_string(),
                reason: "Maintenance".to_string(),
            },
        };
        let result = run_corridor(&suspend_args, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_corridor_resume_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        let steps = [
            CorridorArgs {
                command: CorridorCommand::Create {
                    id: "res-test".to_string(),
                    jurisdiction_a: "PK".to_string(),
                    jurisdiction_b: "AE".to_string(),
                },
            },
            CorridorArgs {
                command: CorridorCommand::Submit {
                    id: "res-test".to_string(),
                    agreement: PathBuf::from("a.json"),
                    pack_trilogy: PathBuf::from("t.json"),
                },
            },
            CorridorArgs {
                command: CorridorCommand::Activate {
                    id: "res-test".to_string(),
                    approval_a: "da".to_string(),
                    approval_b: "db".to_string(),
                },
            },
            CorridorArgs {
                command: CorridorCommand::Suspend {
                    id: "res-test".to_string(),
                    reason: "Maintenance".to_string(),
                },
            },
        ];
        for step in &steps {
            run_corridor(step, dir.path()).unwrap();
        }

        let resume_args = CorridorArgs {
            command: CorridorCommand::Resume {
                id: "res-test".to_string(),
                resolution: "resolved".to_string(),
            },
        };
        let result = run_corridor(&resume_args, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_corridor_status_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        let create_args = CorridorArgs {
            command: CorridorCommand::Create {
                id: "stat-test".to_string(),
                jurisdiction_a: "PK".to_string(),
                jurisdiction_b: "AE".to_string(),
            },
        };
        run_corridor(&create_args, dir.path()).unwrap();

        let status_args = CorridorArgs {
            command: CorridorCommand::Status {
                id: "stat-test".to_string(),
            },
        };
        let result = run_corridor(&status_args, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_corridor_list_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        let list_args = CorridorArgs {
            command: CorridorCommand::List,
        };
        let result = run_corridor(&list_args, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn corridor_transition_updates_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "ts-cor", "PK", "AE").unwrap();

        let state_file = state_dir.join("ts-cor.json");
        let content1 = std::fs::read_to_string(&state_file).unwrap();
        let data1: DynCorridorData = serde_json::from_str(&content1).unwrap();
        let created_at = data1.created_at;

        // Small sleep to ensure timestamp differs.
        std::thread::sleep(std::time::Duration::from_millis(10));

        cmd_transition(&state_dir, "ts-cor", DynCorridorState::Pending).unwrap();

        let content2 = std::fs::read_to_string(&state_file).unwrap();
        let data2: DynCorridorData = serde_json::from_str(&content2).unwrap();
        assert_eq!(data2.state, DynCorridorState::Pending);
        // created_at should not change.
        assert_eq!(data2.created_at, created_at);
        // updated_at should be >= original.
        assert!(data2.updated_at >= data1.updated_at);
    }

    #[test]
    fn corridor_transition_error_message_includes_states() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "err-cor", "PK", "AE").unwrap();
        let result = cmd_transition(&state_dir, "err-cor", DynCorridorState::Halted);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("DRAFT"));
        assert!(err_msg.contains("HALTED"));
        assert!(err_msg.contains("invalid transition"));
    }
}
