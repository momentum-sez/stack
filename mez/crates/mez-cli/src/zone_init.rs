//! # Zone Init CLI — Bootstrap a new Economic Zone
//!
//! Provides the `mez zone init` subcommand that scaffolds a new zone with
//! real content-addressed regpack digests, compliance domain configuration,
//! and deployment-ready zone.yaml — the "one-command zone deployment" promise.
//!
//! ## Usage
//!
//! ```bash
//! # Bootstrap a Pakistan sovereign GovOS zone:
//! mez zone init --jurisdiction pk --profile sovereign-govos --name "Pakistan SIFC"
//!
//! # Bootstrap a UAE financial center zone:
//! mez zone init --jurisdiction ae --profile digital-financial-center --name "Dubai DIFC"
//!
//! # Dry-run (print zone.yaml without writing):
//! mez zone init --jurisdiction pk --profile sovereign-govos --name "Test" --dry-run
//! ```

use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;

use mez_pack::regpack;

/// Zone profiles — deployment templates matching the 6 zone archetypes.
const PROFILES: &[(&str, &str)] = &[
    ("digital-financial-center", "Digital Financial Center"),
    ("trade-hub", "International Trade Hub"),
    ("tech-park", "Technology & Innovation Park"),
    ("sovereign-govos", "Sovereign GovOS Deployment"),
    ("charter-city", "Charter City / Special Administrative Zone"),
    ("digital-native-free-zone", "Digital-Native Free Zone"),
];

/// Arguments for `mez zone init`.
#[derive(Args, Debug)]
pub struct ZoneInitArgs {
    /// Jurisdiction ID (e.g., pk, ae, ae-dubai-difc, sg, hk).
    #[arg(long)]
    pub jurisdiction: String,

    /// Zone profile template.
    #[arg(long, value_parser = parse_profile)]
    pub profile: String,

    /// Human-readable zone name.
    #[arg(long)]
    pub name: String,

    /// Output directory. Defaults to `jurisdictions/<jurisdiction>/`.
    #[arg(long)]
    pub output: Option<std::path::PathBuf>,

    /// Print the generated zone.yaml to stdout without writing files.
    #[arg(long)]
    pub dry_run: bool,

    /// Force overwrite if the zone directory already exists.
    #[arg(long)]
    pub force: bool,
}

fn parse_profile(s: &str) -> Result<String, String> {
    if PROFILES.iter().any(|(id, _)| *id == s) {
        Ok(s.to_string())
    } else {
        let valid: Vec<&str> = PROFILES.iter().map(|(id, _)| *id).collect();
        Err(format!(
            "unknown profile '{}'. Valid profiles: {}",
            s,
            valid.join(", ")
        ))
    }
}

/// Execute `mez zone init`.
pub fn run_zone_init(args: &ZoneInitArgs, repo_root: &Path) -> Result<u8> {
    let jurisdiction = &args.jurisdiction;
    let profile = &args.profile;
    let name = &args.name;

    // Derive the zone ID from jurisdiction and profile.
    let zone_id = format!(
        "org.momentum.mez.zone.{}",
        jurisdiction.replace('-', ".")
    );
    let profile_id = format!("org.momentum.mez.profile.{profile}");

    // Derive jurisdiction stack from the jurisdiction ID.
    // e.g., "ae-dubai-difc" → ["ae", "ae-dubai", "ae-dubai-difc"]
    let jurisdiction_stack = build_jurisdiction_stack(jurisdiction);

    // Derive country code from the first component.
    let country_code = jurisdiction
        .split('-')
        .next()
        .unwrap_or(jurisdiction);

    // Compute real CAS digests for all available regpack domains.
    println!("Computing regpack CAS digests...");
    let regpack_entries = compute_regpack_digests(country_code)?;

    // Select compliance domains based on profile.
    let compliance_domains = select_compliance_domains(profile);

    // Select lawpack domains based on what exists for the jurisdiction.
    let lawpack_domains = select_lawpack_domains(repo_root, country_code);

    // Select licensepack domains.
    let licensepack_domains = select_licensepack_domains(profile);

    // Generate corridors based on profile.
    let corridors = select_corridors(profile);

    // Build the zone.yaml content.
    let zone_yaml = generate_zone_yaml(
        &zone_id,
        jurisdiction,
        name,
        &profile_id,
        &jurisdiction_stack,
        &lawpack_domains,
        &licensepack_domains,
        &regpack_entries,
        &compliance_domains,
        &corridors,
        profile,
    );

    if args.dry_run {
        println!("{zone_yaml}");
        return Ok(0);
    }

    // Determine output directory.
    let output_dir = if let Some(ref dir) = args.output {
        dir.clone()
    } else {
        repo_root.join("jurisdictions").join(jurisdiction)
    };

    // Check if zone already exists.
    let zone_yaml_path = output_dir.join("zone.yaml");
    if zone_yaml_path.exists() && !args.force {
        anyhow::bail!(
            "Zone already exists at {}. Use --force to overwrite.",
            zone_yaml_path.display()
        );
    }

    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("creating zone directory: {}", output_dir.display()))?;

    std::fs::write(&zone_yaml_path, &zone_yaml)
        .with_context(|| format!("writing zone.yaml: {}", zone_yaml_path.display()))?;

    // Store regpack artifacts in CAS directory.
    let cas_dir = repo_root.join("dist").join("artifacts").join("regpack");
    std::fs::create_dir_all(&cas_dir)
        .with_context(|| format!("creating CAS directory: {}", cas_dir.display()))?;

    for entry in &regpack_entries {
        let artifact_path = cas_dir.join(format!("{}.json", entry.digest));
        if !artifact_path.exists() {
            std::fs::write(&artifact_path, &entry.json_bytes)
                .with_context(|| format!("writing CAS artifact: {}", artifact_path.display()))?;
            println!("  CAS: stored {}/{} → {}", entry.jurisdiction, entry.domain, entry.digest);
        }
    }

    println!();
    println!("Zone initialized:");
    println!("  zone_id:      {zone_id}");
    println!("  jurisdiction:  {jurisdiction}");
    println!("  profile:       {profile}");
    println!("  name:          {name}");
    println!("  zone.yaml:     {}", zone_yaml_path.display());
    println!("  regpacks:      {} domain(s) with real CAS digests", regpack_entries.len());
    println!("  compliance:    {} domain(s)", compliance_domains.len());
    println!();
    println!("Next steps:");
    println!("  1. Verify:    mez validate {}", zone_yaml_path.display());
    println!("  2. Lock:      mez lock {}", zone_yaml_path.display());
    println!("  3. Deploy:    ./deploy/scripts/deploy-zone.sh {profile} {zone_id} {jurisdiction}");

    Ok(0)
}

/// A regpack entry with computed CAS digest.
struct RegpackEntry {
    jurisdiction: String,
    domain: String,
    digest: String,
    json_bytes: Vec<u8>,
}

/// Compute real CAS digests for all available regpack domains for a jurisdiction.
fn compute_regpack_digests(country_code: &str) -> Result<Vec<RegpackEntry>> {
    let domains = regpack::domains_for_jurisdiction(country_code);
    let mut entries = Vec::new();

    if let Some(domains) = domains {
        for domain in domains {
            match regpack::build_regpack_artifact(country_code, domain) {
                Ok((digest, bytes)) => {
                    println!("  {country_code}/{domain}: {digest}");
                    entries.push(RegpackEntry {
                        jurisdiction: country_code.to_string(),
                        domain: domain.to_string(),
                        digest,
                        json_bytes: bytes,
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        jurisdiction = country_code,
                        domain = domain,
                        error = %e,
                        "failed to build regpack — skipping"
                    );
                }
            }
        }
    } else {
        // No jurisdiction-specific regpacks. Generate generic ones.
        // All jurisdictions support at least "financial" and "sanctions".
        for domain in &["financial", "sanctions"] {
            match regpack::build_regpack_artifact(country_code, domain) {
                Ok((digest, bytes)) => {
                    println!("  {country_code}/{domain}: {digest}");
                    entries.push(RegpackEntry {
                        jurisdiction: country_code.to_string(),
                        domain: domain.to_string(),
                        digest,
                        json_bytes: bytes,
                    });
                }
                Err(_) => {
                    // Jurisdiction doesn't have content for this domain — skip silently.
                }
            }
        }
    }

    Ok(entries)
}

/// Build the jurisdiction stack from a hyphenated jurisdiction ID.
fn build_jurisdiction_stack(jurisdiction: &str) -> Vec<String> {
    let parts: Vec<&str> = jurisdiction.split('-').collect();
    let mut stack = Vec::new();
    let mut current = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            current.push('-');
        }
        current.push_str(part);
        stack.push(current.clone());
    }
    stack
}

/// Select compliance domains based on zone profile.
fn select_compliance_domains(profile: &str) -> Vec<&'static str> {
    match profile {
        "sovereign-govos" => vec![
            "aml", "kyc", "sanctions", "tax", "securities", "corporate",
            "licensing", "data_privacy", "consumer_protection", "environmental",
        ],
        "digital-financial-center" => vec![
            "aml", "kyc", "sanctions", "tax", "securities", "corporate",
            "licensing", "data_privacy", "consumer_protection", "environmental",
            "intellectual_property", "employment",
        ],
        "trade-hub" => vec![
            "aml", "kyc", "sanctions", "tax", "corporate", "licensing",
            "data_privacy", "consumer_protection", "environmental",
            "trade_controls", "customs",
        ],
        "tech-park" => vec![
            "aml", "kyc", "sanctions", "tax", "corporate", "licensing",
            "data_privacy", "intellectual_property", "employment",
        ],
        "charter-city" => vec![
            "aml", "kyc", "sanctions", "tax", "securities", "corporate",
            "licensing", "data_privacy", "consumer_protection", "environmental",
            "intellectual_property", "employment",
        ],
        "digital-native-free-zone" => vec![
            "aml", "kyc", "sanctions", "tax", "securities", "corporate",
            "licensing", "data_privacy",
        ],
        _ => vec![
            "aml", "kyc", "sanctions", "tax", "securities", "corporate",
            "licensing", "data_privacy",
        ],
    }
}

/// Select lawpack domains based on what exists on disk.
fn select_lawpack_domains(repo_root: &Path, country_code: &str) -> Vec<String> {
    let legal_dir = repo_root
        .join("modules")
        .join("legal")
        .join("jurisdictions")
        .join(country_code);

    let mut domains = Vec::new();
    if legal_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&legal_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        domains.push(name.to_string());
                    }
                }
            }
        }
    }

    // Fallback: provide minimal lawpack domains.
    if domains.is_empty() {
        domains.push("civil".to_string());
        domains.push("financial".to_string());
    }

    domains.sort();
    domains
}

/// Select licensepack domains based on profile.
fn select_licensepack_domains(profile: &str) -> Vec<&'static str> {
    match profile {
        "digital-financial-center" | "charter-city" => {
            vec!["financial", "corporate", "securities"]
        }
        "sovereign-govos" => vec!["financial", "corporate"],
        "trade-hub" => vec!["financial", "corporate", "trade"],
        "tech-park" => vec!["corporate", "technology"],
        "digital-native-free-zone" => vec!["financial", "corporate", "digital_assets"],
        _ => vec!["financial", "corporate"],
    }
}

/// Select corridor templates based on profile.
fn select_corridors(profile: &str) -> Vec<&'static str> {
    match profile {
        "digital-financial-center" | "charter-city" => vec![
            "org.momentum.mez.corridor.swift.iso20022-cross-border",
            "org.momentum.mez.corridor.stablecoin.regulated-stablecoin",
        ],
        "trade-hub" => vec![
            "org.momentum.mez.corridor.swift.iso20022-cross-border",
            "org.momentum.mez.corridor.trade.commodity-flow",
        ],
        _ => vec!["org.momentum.mez.corridor.swift.iso20022-cross-border"],
    }
}

/// Generate zone.yaml content.
#[allow(clippy::too_many_arguments)]
fn generate_zone_yaml(
    zone_id: &str,
    jurisdiction: &str,
    name: &str,
    profile_id: &str,
    jurisdiction_stack: &[String],
    lawpack_domains: &[String],
    licensepack_domains: &[&str],
    regpack_entries: &[RegpackEntry],
    compliance_domains: &[&str],
    corridors: &[&str],
    profile: &str,
) -> String {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let stack_block: String = jurisdiction_stack
        .iter()
        .map(|s| format!("  - {s}"))
        .collect::<Vec<_>>()
        .join("\n");

    let lawpack_block: String = lawpack_domains
        .iter()
        .map(|d| format!("  - {d}"))
        .collect::<Vec<_>>()
        .join("\n");

    let licensepack_block: String = licensepack_domains
        .iter()
        .map(|d| format!("  - {d}"))
        .collect::<Vec<_>>()
        .join("\n");

    let regpack_block: String = regpack_entries
        .iter()
        .map(|e| {
            format!(
                "  - jurisdiction_id: {}\n    domain: {}\n    regpack_digest_sha256: \"{}\"\n    as_of_date: \"{}\"",
                e.jurisdiction, e.domain, e.digest, today
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let domains_block: String = compliance_domains
        .iter()
        .map(|d| format!("  - {d}"))
        .collect::<Vec<_>>()
        .join("\n");

    let corridors_block: String = corridors
        .iter()
        .map(|c| format!("  - {c}"))
        .collect::<Vec<_>>()
        .join("\n");

    let profile_display = PROFILES
        .iter()
        .find(|(id, _)| *id == profile)
        .map(|(_, desc)| *desc)
        .unwrap_or(profile);

    format!(
        r#"# {name} — Zone Manifest
#
# Profile: {profile_display}
# Generated by: mez zone init --jurisdiction {jurisdiction} --profile {profile}
# Date: {today}
#
# CAS digests are computed from real regpack content — not placeholders.
# Verify: mez lock {jurisdiction}/zone.yaml --check

zone_id: {zone_id}
jurisdiction_id: {jurisdiction}
zone_name: "{name}"

profile:
  profile_id: {profile_id}
  version: "0.4.44"

jurisdiction_stack:
{stack_block}

lawpack_domains:
{lawpack_block}

licensepack_domains:
{licensepack_block}

licensepack_refresh_policy:
  default:
    refresh_frequency: daily
    max_staleness_hours: 24
  financial:
    refresh_frequency: hourly
    max_staleness_hours: 4

regpacks:
{regpack_block}

compliance_domains:
{domains_block}

corridors:
{corridors_block}

corridor_peers: []

trust_anchors: []

key_rotation_policy:
  default:
    rotation_days: 90
    grace_days: 14

lockfile_path: stack.lock
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_jurisdiction_stack_simple() {
        let stack = build_jurisdiction_stack("pk");
        assert_eq!(stack, vec!["pk"]);
    }

    #[test]
    fn build_jurisdiction_stack_nested() {
        let stack = build_jurisdiction_stack("ae-dubai-difc");
        assert_eq!(stack, vec!["ae", "ae-dubai", "ae-dubai-difc"]);
    }

    #[test]
    fn build_jurisdiction_stack_two_level() {
        let stack = build_jurisdiction_stack("pk-sifc");
        assert_eq!(stack, vec!["pk", "pk-sifc"]);
    }

    #[test]
    fn select_compliance_domains_sovereign_govos() {
        let domains = select_compliance_domains("sovereign-govos");
        assert!(domains.contains(&"aml"));
        assert!(domains.contains(&"sanctions"));
        assert!(domains.contains(&"tax"));
        assert!(domains.len() >= 10);
    }

    #[test]
    fn select_compliance_domains_financial_center() {
        let domains = select_compliance_domains("digital-financial-center");
        assert!(domains.contains(&"securities"));
        assert!(domains.contains(&"intellectual_property"));
        assert!(domains.len() >= 12);
    }

    #[test]
    fn compute_regpack_digests_for_pk() {
        let entries = compute_regpack_digests("pk").unwrap();
        assert!(entries.len() >= 2, "Pakistan should have at least 2 regpack domains");
        for entry in &entries {
            assert_eq!(entry.digest.len(), 64, "digest must be 64 hex chars");
            assert!(!entry.json_bytes.is_empty());
        }
    }

    #[test]
    fn compute_regpack_digests_for_unknown_jurisdiction() {
        let entries = compute_regpack_digests("xx").unwrap();
        assert!(entries.is_empty(), "unknown jurisdiction should produce no entries");
    }

    #[test]
    fn compute_regpack_digests_for_ae() {
        let entries = compute_regpack_digests("ae").unwrap();
        assert!(entries.len() >= 2, "UAE should have at least 2 regpack domains");
    }

    #[test]
    fn generate_zone_yaml_contains_real_digests() {
        let entries = compute_regpack_digests("pk").unwrap();
        let yaml = generate_zone_yaml(
            "org.momentum.mez.zone.pk.sifc",
            "pk-sifc",
            "Pakistan SIFC",
            "org.momentum.mez.profile.sovereign-govos",
            &["pk".to_string(), "pk-sifc".to_string()],
            &["civil".to_string(), "financial".to_string()],
            &["financial", "corporate"],
            &entries,
            &["aml", "kyc", "sanctions"],
            &["org.momentum.mez.corridor.swift.iso20022-cross-border"],
            "sovereign-govos",
        );
        assert!(yaml.contains("regpack_digest_sha256:"));
        // Verify no zero-filled digests.
        assert!(!yaml.contains("0000000000000000000000000000000000000000000000000000000000000000"));
        // All digests should be exactly 64 hex chars.
        for entry in &entries {
            assert!(yaml.contains(&entry.digest));
        }
    }

    #[test]
    fn parse_profile_valid() {
        assert!(parse_profile("sovereign-govos").is_ok());
        assert!(parse_profile("digital-financial-center").is_ok());
        assert!(parse_profile("trade-hub").is_ok());
    }

    #[test]
    fn parse_profile_invalid() {
        assert!(parse_profile("nonexistent-profile").is_err());
    }

    #[test]
    fn run_zone_init_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();

        let args = ZoneInitArgs {
            jurisdiction: "pk".to_string(),
            profile: "sovereign-govos".to_string(),
            name: "Test Zone".to_string(),
            output: None,
            dry_run: true,
            force: false,
        };

        let result = run_zone_init(&args, repo_root);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_zone_init_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();

        let args = ZoneInitArgs {
            jurisdiction: "pk".to_string(),
            profile: "sovereign-govos".to_string(),
            name: "Pakistan Test Zone".to_string(),
            output: None,
            dry_run: false,
            force: false,
        };

        let result = run_zone_init(&args, repo_root);
        assert!(result.is_ok());

        // Verify zone.yaml was created.
        let zone_yaml_path = repo_root.join("jurisdictions").join("pk").join("zone.yaml");
        assert!(zone_yaml_path.exists(), "zone.yaml should exist");

        // Verify content.
        let content = std::fs::read_to_string(&zone_yaml_path).unwrap();
        assert!(content.contains("org.momentum.mez.zone.pk"));
        assert!(content.contains("sovereign-govos"));
        assert!(content.contains("regpack_digest_sha256:"));
    }

    #[test]
    fn run_zone_init_refuses_overwrite_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();

        let args = ZoneInitArgs {
            jurisdiction: "pk".to_string(),
            profile: "sovereign-govos".to_string(),
            name: "Test".to_string(),
            output: None,
            dry_run: false,
            force: false,
        };

        // First init succeeds.
        run_zone_init(&args, repo_root).unwrap();

        // Second init without --force fails.
        let result = run_zone_init(&args, repo_root);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("--force"));
    }

    #[test]
    fn run_zone_init_force_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();

        let args = ZoneInitArgs {
            jurisdiction: "ae".to_string(),
            profile: "digital-financial-center".to_string(),
            name: "DIFC Test".to_string(),
            output: None,
            dry_run: false,
            force: true,
        };

        run_zone_init(&args, repo_root).unwrap();
        // Second init with --force succeeds.
        let result = run_zone_init(&args, repo_root);
        assert!(result.is_ok());
    }

    #[test]
    fn select_corridors_by_profile() {
        let fin = select_corridors("digital-financial-center");
        assert!(fin.len() >= 2);
        assert!(fin.iter().any(|c| c.contains("swift")));

        let trade = select_corridors("trade-hub");
        assert!(trade.iter().any(|c| c.contains("trade")));

        let govos = select_corridors("sovereign-govos");
        assert!(govos.len() >= 1);
    }

    #[test]
    fn select_licensepack_domains_by_profile() {
        let fin = select_licensepack_domains("digital-financial-center");
        assert!(fin.contains(&"securities"));

        let govos = select_licensepack_domains("sovereign-govos");
        assert!(govos.contains(&"financial"));
    }
}
