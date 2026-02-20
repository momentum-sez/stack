//! # Regpack CLI — Build, store, and verify regulatory packs.
//!
//! Provides the `mez regpack build` subcommand for building content-addressed
//! regpack artifacts from jurisdiction-specific regulatory data.
//!
//! ## Usage
//!
//! ```bash
//! # Build and store the Pakistan financial regpack:
//! mez regpack build --jurisdiction pk --domain financial
//!
//! # Build all available regpacks for any supported jurisdiction:
//! mez regpack build --jurisdiction ae --all-domains --store
//! mez regpack build --jurisdiction sg --all-domains --store
//! mez regpack build --jurisdiction hk --all-domains --store
//! mez regpack build --jurisdiction ky --all-domains --store
//!
//! # List available jurisdictions:
//! mez regpack list
//! ```

use std::path::Path;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use mez_pack::regpack;

/// Regpack subcommand arguments.
#[derive(Args, Debug)]
pub struct RegpackArgs {
    #[command(subcommand)]
    pub command: RegpackCommand,
}

/// Available regpack subcommands.
#[derive(Subcommand, Debug)]
pub enum RegpackCommand {
    /// Build a content-addressed regpack artifact for a jurisdiction and domain.
    Build {
        /// Jurisdiction ID (e.g., pk, ae, sg, hk, ky).
        #[arg(long)]
        jurisdiction: String,

        /// Compliance domain (e.g., financial, sanctions).
        /// Required unless --all-domains is set.
        #[arg(long, required_unless_present = "all_domains")]
        domain: Option<String>,

        /// Build all available domains for the jurisdiction.
        #[arg(long)]
        all_domains: bool,

        /// Store the built regpack in the CAS directory.
        /// If not set, only prints the computed digest.
        #[arg(long)]
        store: bool,
    },

    /// List all jurisdictions with available regpack content.
    List,
}

/// Execute the regpack subcommand.
pub fn run_regpack(args: &RegpackArgs, repo_root: &Path) -> Result<u8> {
    match &args.command {
        RegpackCommand::Build {
            jurisdiction,
            domain,
            all_domains,
            store,
        } => run_build(jurisdiction, domain.as_deref(), *all_domains, *store, repo_root),
        RegpackCommand::List => run_list(),
    }
}

fn run_list() -> Result<u8> {
    let jurisdictions = regpack::available_jurisdictions();
    println!("Available jurisdictions with regpack content:");
    println!();
    for j in &jurisdictions {
        println!(
            "  {:<6} {} (domains: {})",
            j.jurisdiction_id,
            j.jurisdiction_name,
            j.available_domains.join(", ")
        );
    }
    println!();
    println!("Total: {} jurisdictions", jurisdictions.len());
    Ok(0)
}

fn run_build(
    jurisdiction: &str,
    domain: Option<&str>,
    all_domains: bool,
    store: bool,
    repo_root: &Path,
) -> Result<u8> {
    let domains: Vec<&str> = if all_domains {
        regpack::domains_for_jurisdiction(jurisdiction)
            .ok_or_else(|| {
                let available = regpack::available_jurisdictions()
                    .iter()
                    .map(|j| j.jurisdiction_id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                anyhow::anyhow!(
                    "No regpack content available for jurisdiction '{jurisdiction}'. \
                     Available: {available}"
                )
            })?
    } else {
        vec![domain.context("--domain is required when --all-domains is not set")?]
    };

    let cas_dir = repo_root.join("dist").join("artifacts").join("regpack");

    for d in &domains {
        let (digest_hex, json_bytes) = build_regpack_for(jurisdiction, d)?;

        println!("  jurisdiction: {jurisdiction}");
        println!("  domain:       {d}");
        println!("  digest:       {digest_hex}");

        if store {
            std::fs::create_dir_all(&cas_dir)
                .with_context(|| format!("failed to create CAS directory: {}", cas_dir.display()))?;

            let artifact_path = cas_dir.join(format!("{digest_hex}.json"));
            std::fs::write(&artifact_path, &json_bytes)
                .with_context(|| format!("failed to write artifact: {}", artifact_path.display()))?;

            println!("  stored:       {}", artifact_path.display());
        }

        println!();
    }

    Ok(0)
}

/// Build a regpack for a specific jurisdiction and domain, returning
/// the digest hex and the serialized JSON bytes.
///
/// Delegates to the multi-jurisdiction dispatch in `mez_pack::regpack`.
fn build_regpack_for(jurisdiction: &str, domain: &str) -> Result<(String, Vec<u8>)> {
    regpack::build_regpack_artifact(jurisdiction, domain)
        .map_err(|e| anyhow::anyhow!("failed to build regpack for {jurisdiction}/{domain}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_pk_financial_returns_digest() {
        let (hex, bytes) = build_regpack_for("pk", "financial").unwrap();
        assert_eq!(hex.len(), 64, "digest must be 64 hex chars");
        assert!(!bytes.is_empty());
        let _: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    }

    #[test]
    fn build_pk_sanctions_returns_digest() {
        let (hex, bytes) = build_regpack_for("pk", "sanctions").unwrap();
        assert_eq!(hex.len(), 64, "digest must be 64 hex chars");
        assert!(!bytes.is_empty());
        let _: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    }

    #[test]
    fn build_ae_financial_returns_digest() {
        let (hex, bytes) = build_regpack_for("ae", "financial").unwrap();
        assert_eq!(hex.len(), 64, "digest must be 64 hex chars");
        assert!(!bytes.is_empty());
        let _: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    }

    #[test]
    fn build_sg_financial_returns_digest() {
        let (hex, bytes) = build_regpack_for("sg", "financial").unwrap();
        assert_eq!(hex.len(), 64, "digest must be 64 hex chars");
        assert!(!bytes.is_empty());
        let _: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    }

    #[test]
    fn build_hk_financial_returns_digest() {
        let (hex, bytes) = build_regpack_for("hk", "financial").unwrap();
        assert_eq!(hex.len(), 64, "digest must be 64 hex chars");
        assert!(!bytes.is_empty());
        let _: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    }

    #[test]
    fn build_ky_financial_returns_digest() {
        let (hex, bytes) = build_regpack_for("ky", "financial").unwrap();
        assert_eq!(hex.len(), 64, "digest must be 64 hex chars");
        assert!(!bytes.is_empty());
        let _: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    }

    #[test]
    fn build_unknown_jurisdiction_errors() {
        let result = build_regpack_for("xx", "financial");
        assert!(result.is_err());
    }

    #[test]
    fn build_unknown_domain_errors() {
        let result = build_regpack_for("pk", "unknown-domain");
        assert!(result.is_err());
    }

    #[test]
    fn pk_financial_and_sanctions_digests_differ() {
        let (fin_hex, _) = build_regpack_for("pk", "financial").unwrap();
        let (san_hex, _) = build_regpack_for("pk", "sanctions").unwrap();
        assert_ne!(fin_hex, san_hex);
    }

    #[test]
    fn all_jurisdictions_build_financial_regpack() {
        for j in regpack::available_jurisdictions() {
            let result = build_regpack_for(j.jurisdiction_id, "financial");
            assert!(
                result.is_ok(),
                "failed to build financial regpack for {}: {:?}",
                j.jurisdiction_id,
                result.err()
            );
        }
    }

    #[test]
    fn all_jurisdictions_build_sanctions_regpack() {
        for j in regpack::available_jurisdictions() {
            let result = build_regpack_for(j.jurisdiction_id, "sanctions");
            assert!(
                result.is_ok(),
                "failed to build sanctions regpack for {}: {:?}",
                j.jurisdiction_id,
                result.err()
            );
        }
    }
}
