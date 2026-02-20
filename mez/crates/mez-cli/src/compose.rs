//! # Zone Compose CLI — Synthetic Zone Generator
//!
//! Provides the `mez zone compose` subcommand for generating synthetic zones
//! from composition specifications.
//!
//! ## Usage
//!
//! ```bash
//! # Compose a synthetic zone from a composition spec:
//! mez zone compose --spec composition.yaml --output jurisdictions/synth-atlantic-fintech/
//!
//! # Validate a composition without generating files:
//! mez zone compose --spec composition.yaml --validate-only
//! ```

use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

use mez_corridor::composition::{
    RegulatoryDomain, RegulatoryLayer, ZoneComposition, ZoneType, validate_composition,
};

/// Zone subcommand arguments.
#[derive(Args, Debug)]
pub struct ZoneArgs {
    #[command(subcommand)]
    pub command: ZoneCommand,
}

/// Available zone subcommands.
#[derive(Subcommand, Debug)]
pub enum ZoneCommand {
    /// Compose a synthetic zone from a composition specification.
    Compose {
        /// Path to the composition specification YAML file.
        #[arg(long)]
        spec: std::path::PathBuf,

        /// Output directory for generated zone.yaml and profile.yaml.
        /// Defaults to jurisdictions/{jurisdiction_id}/.
        #[arg(long)]
        output: Option<std::path::PathBuf>,

        /// Only validate the composition without generating files.
        #[arg(long)]
        validate_only: bool,
    },
}

/// Composition spec as parsed from YAML input file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionSpec {
    pub zone_name: String,
    pub zone_id: String,
    pub jurisdiction_id: String,
    pub primary_jurisdiction: String,
    pub layers: Vec<LayerSpec>,
}

/// A single layer in a composition spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerSpec {
    pub domain: String,
    pub source: String,
    #[serde(default)]
    pub source_profile_module: Option<String>,
}

/// Execute the zone subcommand.
pub fn run_zone(args: &ZoneArgs, repo_root: &Path) -> Result<u8> {
    match &args.command {
        ZoneCommand::Compose {
            spec,
            output,
            validate_only,
        } => run_compose(spec, output.as_deref(), *validate_only, repo_root),
    }
}

fn parse_domain(s: &str) -> Result<RegulatoryDomain> {
    match s {
        "corporate_formation" => Ok(RegulatoryDomain::CorporateFormation),
        "civic_code" => Ok(RegulatoryDomain::CivicCode),
        "digital_assets" => Ok(RegulatoryDomain::DigitalAssets),
        "arbitration" => Ok(RegulatoryDomain::Arbitration),
        "tax" => Ok(RegulatoryDomain::Tax),
        "aml_cft" => Ok(RegulatoryDomain::AmlCft),
        "data_privacy" => Ok(RegulatoryDomain::DataPrivacy),
        "licensing" => Ok(RegulatoryDomain::Licensing),
        "payment_rails" => Ok(RegulatoryDomain::PaymentRails),
        "securities" => Ok(RegulatoryDomain::Securities),
        _ => bail!("unknown regulatory domain: '{s}'"),
    }
}

fn run_compose(
    spec_path: &Path,
    output_dir: Option<&Path>,
    validate_only: bool,
    repo_root: &Path,
) -> Result<u8> {
    let spec_resolved = if spec_path.is_absolute() {
        spec_path.to_path_buf()
    } else {
        let repo_relative = repo_root.join(spec_path);
        if repo_relative.exists() {
            repo_relative
        } else {
            spec_path.to_path_buf()
        }
    };

    let spec_content = std::fs::read_to_string(&spec_resolved)
        .with_context(|| format!("reading composition spec: {}", spec_resolved.display()))?;

    let spec: CompositionSpec = serde_yaml::from_str(&spec_content)
        .with_context(|| "parsing composition spec YAML")?;

    // Convert spec layers to composition layers.
    let layers: Vec<RegulatoryLayer> = spec
        .layers
        .iter()
        .map(|l| {
            let domain = parse_domain(&l.domain)?;
            Ok(RegulatoryLayer {
                domain,
                source_jurisdiction: l.source.clone(),
                source_profile_module: l.source_profile_module.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let composition = ZoneComposition {
        zone_id: spec.zone_id.clone(),
        zone_name: spec.zone_name.clone(),
        zone_type: ZoneType::Synthetic,
        layers,
        primary_jurisdiction: spec.primary_jurisdiction.clone(),
        jurisdiction_id: spec.jurisdiction_id.clone(),
    };

    // Validate composition.
    match validate_composition(&composition) {
        Ok(()) => {
            println!("  composition: VALID");
            println!("  zone_name:   {}", composition.zone_name);
            println!("  zone_type:   synthetic");
            println!("  layers:      {}", composition.layers.len());
            for layer in &composition.layers {
                println!(
                    "    - {} <- {}",
                    layer.domain, layer.source_jurisdiction
                );
            }
        }
        Err(errors) => {
            eprintln!("  composition: INVALID");
            for err in &errors {
                eprintln!("    - {err}");
            }
            return Ok(1);
        }
    }

    if validate_only {
        println!("  (validate-only mode — no files generated)");
        return Ok(0);
    }

    // Validate that all source jurisdiction zone.yaml files exist.
    for layer in &composition.layers {
        let source_zone = repo_root
            .join("jurisdictions")
            .join(&layer.source_jurisdiction)
            .join("zone.yaml");
        if !source_zone.exists() {
            bail!(
                "source jurisdiction '{}' has no zone.yaml at {}",
                layer.source_jurisdiction,
                source_zone.display()
            );
        }
    }

    // Collect regpack references and compliance domains from source zones.
    let mut all_regpacks: Vec<serde_yaml::Value> = Vec::new();
    let mut all_compliance_domains: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    let mut all_licensepack_domains: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();

    for layer in &composition.layers {
        let source_zone_path = repo_root
            .join("jurisdictions")
            .join(&layer.source_jurisdiction)
            .join("zone.yaml");
        let source_content = std::fs::read_to_string(&source_zone_path)
            .with_context(|| {
                format!(
                    "reading source zone: {}",
                    source_zone_path.display()
                )
            })?;
        let source: serde_yaml::Value = serde_yaml::from_str(&source_content)?;

        // Collect regpacks.
        if let Some(regpacks) = source.get("regpacks").and_then(|v| v.as_sequence()) {
            for rp in regpacks {
                // Deduplicate by domain+jurisdiction_id.
                let domain = rp.get("domain").and_then(|v| v.as_str()).unwrap_or("");
                let jid = rp
                    .get("jurisdiction_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let key = format!("{domain}:{jid}");
                let already = all_regpacks.iter().any(|existing| {
                    let ed = existing.get("domain").and_then(|v| v.as_str()).unwrap_or("");
                    let ej = existing
                        .get("jurisdiction_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    format!("{ed}:{ej}") == key
                });
                if !already {
                    all_regpacks.push(rp.clone());
                }
            }
        }

        // Collect compliance domains.
        let domains_key = if source.get("compliance_domains").is_some() {
            "compliance_domains"
        } else {
            "domains"
        };
        if let Some(domains) = source.get(domains_key).and_then(|v| v.as_sequence()) {
            for d in domains {
                if let Some(s) = d.as_str() {
                    all_compliance_domains.insert(s.to_string());
                }
            }
        }

        // Collect licensepack domains.
        if let Some(lp) = source
            .get("licensepack_domains")
            .and_then(|v| v.as_sequence())
        {
            for d in lp {
                if let Some(s) = d.as_str() {
                    all_licensepack_domains.insert(s.to_string());
                }
            }
        }
    }

    // Generate zone.yaml.
    let output = if let Some(dir) = output_dir {
        dir.to_path_buf()
    } else {
        repo_root
            .join("jurisdictions")
            .join(&spec.jurisdiction_id)
    };

    std::fs::create_dir_all(&output)
        .with_context(|| format!("creating output directory: {}", output.display()))?;

    let zone_yaml = format!(
        "# Zone Manifest: {zone_name} (Synthetic)
#
# Synthetic zone composed from regulatory primitives across multiple
# jurisdictions. Primary jurisdiction: {primary_jurisdiction}.
# Generated by: mez zone compose
#
# zone_type: synthetic

zone_id: {zone_id}
jurisdiction_id: {jurisdiction_id}
zone_name: {zone_name}
zone_type: synthetic

profile:
  profile_id: org.momentum.mez.profile.synthetic-{jurisdiction_id}
  version: \"0.4.44\"

primary_jurisdiction: {primary_jurisdiction}

composition:
{composition_block}
jurisdiction_stack:
  - {jurisdiction_id}

lawpack_domains:
  - civil
  - financial

licensepack_domains:
{licensepack_block}
licensepack_refresh_policy:
  default:
    frequency: daily
    max_staleness_hours: 24
  financial:
    frequency: hourly
    max_staleness_hours: 4

regpacks:
{regpacks_block}
compliance_domains:
{domains_block}
corridors:
  - org.momentum.mez.corridor.swift.iso20022-cross-border
  - org.momentum.mez.corridor.stablecoin.regulated-stablecoin

trust_anchors: []

key_management:
  rotation_interval_days: 90
  grace_period_days: 14

lockfile_path: stack.lock
",
        zone_name = spec.zone_name,
        primary_jurisdiction = spec.primary_jurisdiction,
        zone_id = spec.zone_id,
        jurisdiction_id = spec.jurisdiction_id,
        composition_block = composition
            .layers
            .iter()
            .map(|l| format!(
                "  - domain: {}\n    source_jurisdiction: {}",
                l.domain, l.source_jurisdiction
            ))
            .collect::<Vec<_>>()
            .join("\n"),
        licensepack_block = all_licensepack_domains
            .iter()
            .map(|d| format!("  - {d}"))
            .collect::<Vec<_>>()
            .join("\n"),
        regpacks_block = format_regpacks(&all_regpacks),
        domains_block = all_compliance_domains
            .iter()
            .map(|d| format!("  - {d}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    let zone_yaml_path = output.join("zone.yaml");
    std::fs::write(&zone_yaml_path, &zone_yaml)
        .with_context(|| format!("writing zone.yaml: {}", zone_yaml_path.display()))?;
    println!("  wrote: {}", zone_yaml_path.display());

    // Generate profile.yaml.
    let profile_yaml = format!(
        "# Profile: {zone_name} (Synthetic)
#
# Auto-generated synthetic zone profile.
# Each regulatory domain is sourced from a different jurisdiction.

profile_id: org.momentum.mez.profile.synthetic-{jurisdiction_id}
profile_name: {zone_name}
version: \"0.4.44\"
zone_type: synthetic

composition:
{composition_profile_block}
modules:
  - id: org.momentum.mez.legal.core
    variant: synthetic-composed
  - id: org.momentum.mez.reg.aml-cft
    variant: risk-based
  - id: org.momentum.mez.fin.payments-adapter
    variant: iso20022-mapping
  - id: org.momentum.mez.corridor.swift
    variant: iso20022-cross-border

corridors:
  - org.momentum.mez.corridor.swift.iso20022-cross-border
  - org.momentum.mez.corridor.stablecoin.regulated-stablecoin
",
        zone_name = spec.zone_name,
        jurisdiction_id = spec.jurisdiction_id,
        composition_profile_block = composition
            .layers
            .iter()
            .map(|l| format!(
                "  - domain: {}\n    source: {}",
                l.domain, l.source_jurisdiction
            ))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    let profile_dir = repo_root
        .join("profiles")
        .join(format!("synthetic-{}", spec.jurisdiction_id));
    std::fs::create_dir_all(&profile_dir)
        .with_context(|| format!("creating profile directory: {}", profile_dir.display()))?;

    let profile_yaml_path = profile_dir.join("profile.yaml");
    std::fs::write(&profile_yaml_path, &profile_yaml)
        .with_context(|| format!("writing profile.yaml: {}", profile_yaml_path.display()))?;
    println!("  wrote: {}", profile_yaml_path.display());

    Ok(0)
}

/// Format regpack references for YAML output.
fn format_regpacks(regpacks: &[serde_yaml::Value]) -> String {
    regpacks
        .iter()
        .map(|rp| {
            let domain = rp.get("domain").and_then(|v| v.as_str()).unwrap_or("unknown");
            let jid = rp
                .get("jurisdiction_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let digest = rp
                .get("regpack_digest_sha256")
                .and_then(|v| v.as_str());
            let as_of = rp
                .get("as_of_date")
                .and_then(|v| v.as_str())
                .unwrap_or("2026-01-15");

            if let Some(d) = digest {
                format!(
                    "  - domain: {domain}\n    jurisdiction_id: {jid}\n    regpack_digest_sha256: \"{d}\"\n    as_of_date: \"{as_of}\""
                )
            } else {
                format!(
                    "  - domain: {domain}\n    jurisdiction_id: {jid}\n    as_of_date: \"{as_of}\""
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_domain_all_variants() {
        assert_eq!(
            parse_domain("corporate_formation").unwrap(),
            RegulatoryDomain::CorporateFormation
        );
        assert_eq!(parse_domain("aml_cft").unwrap(), RegulatoryDomain::AmlCft);
        assert_eq!(parse_domain("tax").unwrap(), RegulatoryDomain::Tax);
        assert!(parse_domain("unknown_domain").is_err());
    }

    #[test]
    fn composition_spec_deserialization() {
        let yaml = r#"
zone_name: Test Zone
zone_id: org.momentum.mez.zone.synthetic.test
jurisdiction_id: synth-test
primary_jurisdiction: us
layers:
  - domain: corporate_formation
    source: us-de
  - domain: aml_cft
    source: ae
"#;
        let spec: CompositionSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(spec.zone_name, "Test Zone");
        assert_eq!(spec.layers.len(), 2);
        assert_eq!(spec.layers[0].domain, "corporate_formation");
        assert_eq!(spec.layers[0].source, "us-de");
    }

    #[test]
    fn format_regpacks_with_digest() {
        let rp = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
domain: financial
jurisdiction_id: ae
regpack_digest_sha256: "abc123"
as_of_date: "2026-01-15"
"#,
        )
        .unwrap();
        let result = format_regpacks(&[rp]);
        assert!(result.contains("abc123"));
        assert!(result.contains("financial"));
    }

    #[test]
    fn format_regpacks_without_digest() {
        let rp = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
domain: sanctions
jurisdiction_id: hk
as_of_date: "2026-01-15"
"#,
        )
        .unwrap();
        let result = format_regpacks(&[rp]);
        assert!(result.contains("sanctions"));
        assert!(!result.contains("regpack_digest_sha256"));
    }
}
