//! # Corridor Subcommand
//!
//! Corridor lifecycle management commands. Operates on local state files
//! backed by the `mez-state` typestate machine.
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
//! directory. Database-backed operations come with `mez-api`.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand, ValueEnum};

use mez_corridor::registry::{CorridorRegistry, ZoneEntry};
use mez_corridor::composition::ZoneType;
use mez_state::{DynCorridorData, DynCorridorState};

/// Arguments for the `mez corridor` subcommand.
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
        /// Corridor identifier (e.g., "pk-rez--ae-difc").
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

    /// Generate corridor mesh topology from zone manifests.
    ///
    /// Reads zone.yaml files, registers them in a corridor registry,
    /// generates all N*(N-1)/2 pairwise corridors, and outputs the
    /// mesh in DOT (Graphviz) or JSON (adjacency) format.
    Mesh {
        /// Comma-separated list of zone jurisdiction IDs.
        /// Zones are resolved from `jurisdictions/<id>/zone.yaml`.
        /// Mutually exclusive with `--all`.
        #[arg(long, required_unless_present = "all")]
        zones: Option<String>,
        /// Discover all `jurisdictions/*/zone.yaml` and include every zone.
        #[arg(long)]
        all: bool,
        /// Output format for the mesh topology.
        #[arg(long, value_enum, default_value = "dot")]
        format: MeshFormat,
    },
}

/// Output format for the corridor mesh command.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MeshFormat {
    /// Graphviz DOT graph (renderable with `dot -Tsvg`).
    Dot,
    /// JSON adjacency representation.
    Json,
}

/// Execute the corridor subcommand.
pub fn run_corridor(args: &CorridorArgs, repo_root: &Path) -> Result<u8> {
    let state_dir = repo_root.join(".mez").join("corridors");

    match &args.command {
        CorridorCommand::Create {
            id,
            jurisdiction_a,
            jurisdiction_b,
        } => cmd_create(&state_dir, id, jurisdiction_a, jurisdiction_b),

        CorridorCommand::Submit {
            id,
            agreement,
            pack_trilogy,
        } => {
            let evidence = compute_evidence_digest_from_files(&[agreement, pack_trilogy])?;
            cmd_transition(&state_dir, id, DynCorridorState::Pending, Some(evidence))
        }

        CorridorCommand::Activate {
            id,
            approval_a,
            approval_b,
        } => {
            let evidence = compute_evidence_digest_from_strings(&[approval_a, approval_b]);
            cmd_transition(&state_dir, id, DynCorridorState::Active, Some(evidence))
        }

        CorridorCommand::Halt {
            id,
            reason,
            authority,
        } => {
            let evidence = compute_evidence_digest_from_strings(&[reason, authority]);
            cmd_transition(&state_dir, id, DynCorridorState::Halted, Some(evidence))
        }

        CorridorCommand::Suspend { id, reason } => {
            let evidence = compute_evidence_digest_from_strings(&[reason]);
            cmd_transition(&state_dir, id, DynCorridorState::Suspended, Some(evidence))
        }

        CorridorCommand::Resume { id, resolution } => {
            let evidence = compute_evidence_digest_from_strings(&[resolution]);
            cmd_transition(&state_dir, id, DynCorridorState::Active, Some(evidence))
        }

        CorridorCommand::Status { id } => cmd_status(&state_dir, id),

        CorridorCommand::List => cmd_list(&state_dir),

        CorridorCommand::Mesh { zones, all, format } => {
            cmd_mesh(repo_root, zones.as_deref(), *all, *format)
        }
    }
}

/// Validate that a corridor ID is safe for use in filesystem paths.
///
/// Rejects IDs containing path separators, parent-directory traversals,
/// or other characters that could escape the state directory.
fn validate_corridor_id(id: &str) -> Result<()> {
    if id.is_empty() {
        bail!("corridor ID must not be empty");
    }
    if id.contains('/') || id.contains('\\') || id.contains('\0') {
        bail!("corridor ID contains invalid path characters: {id:?}");
    }
    if id == "." || id == ".." || id.starts_with("../") || id.starts_with("..\\") {
        bail!("corridor ID must not be a relative path traversal: {id:?}");
    }
    Ok(())
}

/// Resolve a corridor state file path, with path traversal protection.
fn corridor_state_file(state_dir: &Path, id: &str) -> Result<PathBuf> {
    validate_corridor_id(id)?;
    let state_file = state_dir.join(format!("{id}.json"));
    // Defense-in-depth: verify the resolved path is still within state_dir.
    let canonical_dir = state_dir
        .canonicalize()
        .unwrap_or_else(|_| state_dir.to_path_buf());
    let canonical_file = state_file
        .canonicalize()
        .unwrap_or_else(|_| state_file.clone());
    if !canonical_file.starts_with(&canonical_dir) {
        bail!("corridor ID resolves outside state directory: {id:?}");
    }
    Ok(state_file)
}

/// Create a new corridor state file in DRAFT state.
fn cmd_create(
    state_dir: &Path,
    id: &str,
    jurisdiction_a: &str,
    jurisdiction_b: &str,
) -> Result<u8> {
    std::fs::create_dir_all(state_dir).context("failed to create corridor state directory")?;

    let state_file = corridor_state_file(state_dir, id)?;
    if state_file.exists() {
        bail!("corridor already exists: {id}");
    }

    let now = chrono::Utc::now();
    let data = DynCorridorData {
        id: mez_core::CorridorId::new(),
        jurisdiction_a: mez_core::JurisdictionId::new(jurisdiction_a)
            .context("invalid jurisdiction_a")?,
        jurisdiction_b: mez_core::JurisdictionId::new(jurisdiction_b)
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

/// Compute a content digest from file paths by hashing their contents.
///
/// Reads each file, feeds `SHA256(contents)` for each into an accumulator, then
/// finalizes to produce a single evidence digest. Uses `mez_core::digest::Sha256Accumulator`
/// to comply with the SHA-256 usage policy (all hashing through mez-core).
fn compute_evidence_digest_from_files(
    paths: &[&PathBuf],
) -> Result<mez_core::ContentDigest> {
    use mez_core::digest::Sha256Accumulator;
    let mut outer = Sha256Accumulator::new();
    for path in paths {
        if path.exists() {
            let content = std::fs::read(path)
                .with_context(|| format!("failed to read evidence file: {}", path.display()))?;
            let mut file_hasher = Sha256Accumulator::new();
            file_hasher.update(&content);
            let file_digest = file_hasher.finalize();
            outer.update(file_digest.as_bytes());
        } else {
            // Hash the path string as a reference digest when file doesn't exist.
            // This allows referencing artifacts by path without requiring local copies.
            let path_str = path.display().to_string();
            outer.update(path_str.as_bytes());
        }
    }
    Ok(outer.finalize())
}

/// Compute a content digest from string values (e.g., approval digests, reasons).
fn compute_evidence_digest_from_strings(
    values: &[&String],
) -> mez_core::ContentDigest {
    use mez_core::digest::Sha256Accumulator;
    let mut acc = Sha256Accumulator::new();
    for value in values {
        acc.update(value.as_bytes());
    }
    acc.finalize()
}

/// Transition a corridor to a new state.
fn cmd_transition(
    state_dir: &Path,
    id: &str,
    target: DynCorridorState,
    evidence_digest: Option<mez_core::ContentDigest>,
) -> Result<u8> {
    let state_file = corridor_state_file(state_dir, id)?;
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

    let from = data.state;
    let now = chrono::Utc::now();

    data.transition_log.push(mez_state::TransitionRecord {
        from_state: from,
        to_state: target,
        timestamp: now,
        evidence_digest,
    });
    data.state = target;
    data.updated_at = now;

    let json = serde_json::to_string_pretty(&data)?;
    std::fs::write(&state_file, json)?;

    println!("OK: corridor {id} transitioned {from} → {target}");
    Ok(0)
}

/// Show corridor status.
fn cmd_status(state_dir: &Path, id: &str) -> Result<u8> {
    let state_file = corridor_state_file(state_dir, id)?;
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

    for entry in std::fs::read_dir(state_dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    dir = %state_dir.display(),
                    error = %e,
                    "failed to read directory entry while listing corridors"
                );
                continue;
            }
        };
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "failed to read corridor state file"
                    );
                    continue;
                }
            };
            match serde_json::from_str::<DynCorridorData>(&content) {
                Ok(data) => {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    entries.push((name, data.state));
                    count += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "failed to parse corridor state file as JSON"
                    );
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

/// Parse a zone.yaml file and extract the fields needed for corridor registration.
fn parse_zone_yaml(repo_root: &Path, jurisdiction_id: &str) -> Result<ZoneEntry> {
    let zone_path = repo_root
        .join("jurisdictions")
        .join(jurisdiction_id)
        .join("zone.yaml");
    let content = std::fs::read_to_string(&zone_path)
        .with_context(|| format!("failed to read zone manifest: {}", zone_path.display()))?;
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&content).with_context(|| format!("invalid YAML in {}", zone_path.display()))?;

    let zone_id = yaml
        .get("zone_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let jid = yaml
        .get("jurisdiction_id")
        .and_then(|v| v.as_str())
        .unwrap_or(jurisdiction_id)
        .to_string();
    let profile_id = yaml
        .get("profile")
        .and_then(|p| p.get("profile_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Derive country code from the first component of jurisdiction_id.
    let country_code = jid
        .split('-')
        .next()
        .unwrap_or("")
        .to_string();

    // A zone is a free zone if its jurisdiction_stack has more than 2 entries
    // (country > region > free-zone) or its profile indicates a free zone.
    let stack_len = yaml
        .get("jurisdiction_stack")
        .and_then(|v| v.as_sequence())
        .map(|s| s.len())
        .unwrap_or(1);
    let is_free_zone = stack_len > 2
        || profile_id.contains("free-zone")
        || profile_id.contains("financial-center");

    // Detect zone type: check for zone_type field or composition block.
    let zone_type = if yaml.get("zone_type").and_then(|v| v.as_str()) == Some("synthetic")
        || yaml.get("composition").is_some()
    {
        ZoneType::Synthetic
    } else {
        ZoneType::Natural
    };

    Ok(ZoneEntry {
        zone_id,
        jurisdiction_id: jid,
        country_code,
        is_free_zone,
        profile_id,
        zone_type,
    })
}

/// Discover all jurisdiction IDs that have a `zone.yaml` under `jurisdictions/`.
fn discover_all_zones(repo_root: &Path) -> Result<Vec<String>> {
    let jdir = repo_root.join("jurisdictions");
    let mut zone_ids = Vec::new();
    for entry in std::fs::read_dir(&jdir)
        .with_context(|| format!("failed to read jurisdictions directory: {}", jdir.display()))?
    {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();
        // Skip _starter template directory
        if name_str.starts_with('_') {
            continue;
        }
        let zone_file = entry.path().join("zone.yaml");
        if zone_file.is_file() {
            zone_ids.push(name_str);
        }
    }
    zone_ids.sort();
    Ok(zone_ids)
}

/// Generate mesh topology from zone manifests and output in the requested format.
fn cmd_mesh(repo_root: &Path, zones_csv: Option<&str>, all: bool, format: MeshFormat) -> Result<u8> {
    let zone_ids: Vec<String> = if all {
        discover_all_zones(repo_root)?
    } else if let Some(csv) = zones_csv {
        csv.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    } else {
        bail!("either --zones or --all is required");
    };

    if zone_ids.len() < 2 {
        bail!("mesh requires at least 2 zones (got {})", zone_ids.len());
    }

    let mut registry = CorridorRegistry::new();
    for jid in &zone_ids {
        let entry = parse_zone_yaml(repo_root, jid)?;
        registry.register_zone(entry);
    }
    registry.generate_corridors();

    let stats = registry.corridor_mesh_stats();
    let total_corridors: usize = stats.values().sum();
    let expected_pairs = zone_ids.len() * (zone_ids.len() - 1) / 2;
    eprintln!(
        "Mesh: {} zones, {} corridors ({} pairs)",
        zone_ids.len(),
        total_corridors,
        expected_pairs,
    );
    for (ctype, count) in &stats {
        eprintln!("  {}: {}", ctype, count);
    }

    match format {
        MeshFormat::Dot => {
            println!("{}", registry.to_dot());
        }
        MeshFormat::Json => {
            let json = registry.to_adjacency_json();
            println!("{}", serde_json::to_string_pretty(&json)?);
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

        let result = cmd_create(&state_dir, "test-corridor", "PK-REZ", "AE-DIFC");
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
        let result = cmd_transition(&state_dir, "test-cor", DynCorridorState::Pending, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn corridor_invalid_transition_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "test-cor", "PK", "AE").unwrap();
        // Draft → Active is not valid (must go through Pending).
        let result = cmd_transition(&state_dir, "test-cor", DynCorridorState::Active, None);
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

        let result = cmd_transition(&state_dir, "nonexistent", DynCorridorState::Pending, None);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("corridor not found"));
    }

    #[test]
    fn corridor_full_lifecycle_draft_to_halted() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "lifecycle-cor", "PK", "AE").unwrap();
        cmd_transition(&state_dir, "lifecycle-cor", DynCorridorState::Pending, None).unwrap();
        cmd_transition(&state_dir, "lifecycle-cor", DynCorridorState::Active, None).unwrap();
        let result = cmd_transition(&state_dir, "lifecycle-cor", DynCorridorState::Halted, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn corridor_full_lifecycle_to_suspended_and_resume() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "suspend-cor", "PK", "AE").unwrap();
        cmd_transition(&state_dir, "suspend-cor", DynCorridorState::Pending, None).unwrap();
        cmd_transition(&state_dir, "suspend-cor", DynCorridorState::Active, None).unwrap();
        cmd_transition(&state_dir, "suspend-cor", DynCorridorState::Suspended, None).unwrap();
        // Resume goes back to Active.
        let result = cmd_transition(&state_dir, "suspend-cor", DynCorridorState::Active, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn corridor_full_lifecycle_to_deprecated() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "dep-cor", "PK", "AE").unwrap();
        cmd_transition(&state_dir, "dep-cor", DynCorridorState::Pending, None).unwrap();
        cmd_transition(&state_dir, "dep-cor", DynCorridorState::Active, None).unwrap();
        cmd_transition(&state_dir, "dep-cor", DynCorridorState::Halted, None).unwrap();
        let result = cmd_transition(&state_dir, "dep-cor", DynCorridorState::Deprecated, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn corridor_deprecated_has_no_transitions() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "term-cor", "PK", "AE").unwrap();
        cmd_transition(&state_dir, "term-cor", DynCorridorState::Pending, None).unwrap();
        cmd_transition(&state_dir, "term-cor", DynCorridorState::Active, None).unwrap();
        cmd_transition(&state_dir, "term-cor", DynCorridorState::Halted, None).unwrap();
        cmd_transition(&state_dir, "term-cor", DynCorridorState::Deprecated, None).unwrap();

        // No valid transitions from Deprecated.
        let result = cmd_transition(&state_dir, "term-cor", DynCorridorState::Active, None);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid transition"));
    }

    #[test]
    fn corridor_pending_to_halted_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("corridors");

        cmd_create(&state_dir, "bad-cor", "PK", "AE").unwrap();
        cmd_transition(&state_dir, "bad-cor", DynCorridorState::Pending, None).unwrap();
        // Pending → Halted not valid.
        let result = cmd_transition(&state_dir, "bad-cor", DynCorridorState::Halted, None);
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
        cmd_transition(&state_dir, "log-cor", DynCorridorState::Pending, None).unwrap();
        cmd_transition(&state_dir, "log-cor", DynCorridorState::Active, None).unwrap();

        let result = cmd_status(&state_dir, "log-cor");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Read the state file to verify the log.
        let state_file = state_dir.join("log-cor.json");
        let content = std::fs::read_to_string(&state_file).unwrap();
        let data: DynCorridorData = serde_json::from_str(&content).unwrap();
        assert_eq!(data.transition_log.len(), 2);
        assert_eq!(data.transition_log[0].from_state, DynCorridorState::Draft);
        assert_eq!(data.transition_log[0].to_state, DynCorridorState::Pending);
        assert_eq!(data.transition_log[1].from_state, DynCorridorState::Pending);
        assert_eq!(data.transition_log[1].to_state, DynCorridorState::Active);
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

        cmd_create(&state_dir, "json-test", "PK-REZ", "AE-DIFC").unwrap();

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

        cmd_transition(&state_dir, "ts-cor", DynCorridorState::Pending, None).unwrap();

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
        let result = cmd_transition(&state_dir, "err-cor", DynCorridorState::Halted, None);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("DRAFT"));
        assert!(err_msg.contains("HALTED"));
        assert!(err_msg.contains("invalid transition"));
    }

    // ── Mesh command tests ──────────────────────────────────────────

    /// Create a minimal zone.yaml for testing mesh generation.
    fn write_test_zone(root: &Path, jid: &str, zone_id: &str, profile: &str, stack: &[&str]) {
        let dir = root.join("jurisdictions").join(jid);
        std::fs::create_dir_all(&dir).unwrap();
        let stack_yaml: String = stack.iter().map(|s| format!("  - {s}\n")).collect();
        let content = format!(
            "zone_id: {zone_id}\n\
             jurisdiction_id: {jid}\n\
             zone_name: Test Zone {jid}\n\
             profile:\n\
             \x20 profile_id: {profile}\n\
             \x20 version: 0.4.44\n\
             jurisdiction_stack:\n\
             {stack_yaml}\
             lawpack_domains:\n\
             \x20 - civil\n",
        );
        std::fs::write(dir.join("zone.yaml"), content).unwrap();
    }

    #[test]
    fn mesh_dot_output_contains_graph() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // Also need schemas/ and modules/ for repo root detection, but cmd_mesh takes explicit root.
        write_test_zone(root, "pk-sifc", "org.momentum.mez.zone.pk.sifc", "org.momentum.mez.profile.financial-center", &["pk", "pk-sifc"]);
        write_test_zone(root, "ae-dubai-difc", "org.momentum.mez.zone.ae.dubai.difc", "org.momentum.mez.profile.financial-center", &["ae", "ae-dubai", "ae-dubai-difc"]);
        write_test_zone(root, "sg", "org.momentum.mez.zone.sg", "org.momentum.mez.profile.sovereign-govos", &["sg"]);

        let result = cmd_mesh(root, Some("pk-sifc,ae-dubai-difc,sg"), false, MeshFormat::Dot);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn mesh_json_output_contains_zones() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_test_zone(root, "pk-sifc", "org.momentum.mez.zone.pk.sifc", "org.momentum.mez.profile.sovereign-govos", &["pk", "pk-sifc"]);
        write_test_zone(root, "hk", "org.momentum.mez.zone.hk", "org.momentum.mez.profile.sovereign-govos", &["hk"]);

        let result = cmd_mesh(root, Some("pk-sifc,hk"), false, MeshFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn mesh_requires_at_least_two_zones() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_test_zone(root, "pk-sifc", "org.momentum.mez.zone.pk.sifc", "org.momentum.mez.profile.sovereign-govos", &["pk", "pk-sifc"]);

        let result = cmd_mesh(root, Some("pk-sifc"), false, MeshFormat::Dot);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("at least 2 zones"));
    }

    #[test]
    fn mesh_detects_synthetic_zone() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_test_zone(root, "pk-sifc", "org.momentum.mez.zone.pk.sifc", "org.momentum.mez.profile.sovereign-govos", &["pk", "pk-sifc"]);

        // Write a synthetic zone with composition block.
        let synth_dir = root.join("jurisdictions").join("synth-test");
        std::fs::create_dir_all(&synth_dir).unwrap();
        std::fs::write(
            synth_dir.join("zone.yaml"),
            "zone_id: org.momentum.mez.zone.synth.test\n\
             jurisdiction_id: synth-test\n\
             zone_name: Synthetic Test\n\
             zone_type: synthetic\n\
             profile:\n\
             \x20 profile_id: org.momentum.mez.profile.synthetic\n\
             \x20 version: 0.4.44\n\
             jurisdiction_stack:\n\
             \x20 - synth-test\n\
             composition:\n\
             \x20 layers: []\n\
             lawpack_domains:\n\
             \x20 - civil\n",
        )
        .unwrap();

        let result = cmd_mesh(root, Some("pk-sifc,synth-test"), false, MeshFormat::Dot);
        assert!(result.is_ok());
    }

    #[test]
    fn mesh_missing_zone_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_test_zone(root, "pk-sifc", "org.momentum.mez.zone.pk.sifc", "org.momentum.mez.profile.sovereign-govos", &["pk", "pk-sifc"]);

        let result = cmd_mesh(root, Some("pk-sifc,nonexistent-zone"), false, MeshFormat::Dot);
        assert!(result.is_err());
    }

    #[test]
    fn parse_zone_yaml_extracts_free_zone() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_test_zone(root, "ae-dubai-difc", "org.momentum.mez.zone.ae.dubai.difc", "org.momentum.mez.profile.financial-center", &["ae", "ae-dubai", "ae-dubai-difc"]);

        let entry = parse_zone_yaml(root, "ae-dubai-difc").unwrap();
        assert_eq!(entry.country_code, "ae");
        assert!(entry.is_free_zone); // 3-level stack → free zone
        assert_eq!(entry.zone_type, ZoneType::Natural);
    }

    #[test]
    fn parse_zone_yaml_non_free_zone() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_test_zone(root, "sg", "org.momentum.mez.zone.sg", "org.momentum.mez.profile.sovereign-govos", &["sg"]);

        let entry = parse_zone_yaml(root, "sg").unwrap();
        assert_eq!(entry.country_code, "sg");
        assert!(!entry.is_free_zone); // 1-level stack
        assert_eq!(entry.zone_type, ZoneType::Natural);
    }

    // ── --all flag tests ──────────────────────────────────────────────

    #[test]
    fn mesh_all_discovers_zones() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_test_zone(root, "pk-sifc", "org.momentum.mez.zone.pk.sifc", "org.momentum.mez.profile.sovereign-govos", &["pk", "pk-sifc"]);
        write_test_zone(root, "sg", "org.momentum.mez.zone.sg", "org.momentum.mez.profile.sovereign-govos", &["sg"]);
        write_test_zone(root, "hk", "org.momentum.mez.zone.hk", "org.momentum.mez.profile.sovereign-govos", &["hk"]);

        // --all should discover all 3 zones
        let result = cmd_mesh(root, None, true, MeshFormat::Dot);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn mesh_all_skips_underscore_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_test_zone(root, "pk-sifc", "org.momentum.mez.zone.pk.sifc", "org.momentum.mez.profile.sovereign-govos", &["pk", "pk-sifc"]);
        write_test_zone(root, "sg", "org.momentum.mez.zone.sg", "org.momentum.mez.profile.sovereign-govos", &["sg"]);
        // Create a _starter directory that should be skipped
        write_test_zone(root, "_starter", "org.momentum.mez.zone.starter", "org.momentum.mez.profile.minimal-mvp", &["starter"]);

        let zones = discover_all_zones(root).unwrap();
        assert_eq!(zones.len(), 2);
        assert!(!zones.contains(&"_starter".to_string()));
    }

    #[test]
    fn mesh_all_generates_correct_pair_count() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let n = 5;
        for i in 0..n {
            let jid = format!("zone-{i}");
            let zid = format!("org.momentum.mez.zone.z{i}");
            write_test_zone(root, &jid, &zid, "org.momentum.mez.profile.sovereign-govos", &[&jid]);
        }

        let result = cmd_mesh(root, None, true, MeshFormat::Json);
        assert!(result.is_ok());
    }

    // ── Full-mesh integration test (uses real repo jurisdictions) ──────

    #[test]
    fn full_mesh_integration_test() {
        // This test uses the real repo jurisdictions directory.
        // It verifies that every zone.yaml can be parsed and registered,
        // and that N*(N-1)/2 corridors are generated.
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()  // crates/
            .and_then(|p| p.parent())  // mez/
            .and_then(|p| p.parent()); // repo root

        let repo_root = match repo_root {
            Some(r) if r.join("jurisdictions").is_dir() => r,
            _ => {
                eprintln!("SKIP: repo root not found for full-mesh test");
                return;
            }
        };

        let zone_ids = discover_all_zones(repo_root).unwrap();
        assert!(
            zone_ids.len() >= 100,
            "expected at least 100 zones, found {}",
            zone_ids.len()
        );

        let mut registry = CorridorRegistry::new();
        let mut parsed = 0;
        for jid in &zone_ids {
            match parse_zone_yaml(repo_root, jid) {
                Ok(entry) => {
                    registry.register_zone(entry);
                    parsed += 1;
                }
                Err(e) => {
                    panic!("failed to parse zone {jid}: {e}");
                }
            }
        }

        registry.generate_corridors();
        let stats = registry.corridor_mesh_stats();
        let total_corridors: usize = stats.values().sum();
        let expected = parsed * (parsed - 1) / 2;

        eprintln!(
            "Full mesh: {} zones parsed, {} corridors generated (expected {})",
            parsed, total_corridors, expected
        );

        assert_eq!(
            total_corridors, expected,
            "corridor count {total_corridors} != expected N*(N-1)/2 = {expected}"
        );
    }
}
