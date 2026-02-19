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
//! # Build and store the Pakistan sanctions regpack:
//! mez regpack build --jurisdiction pk --domain sanctions
//!
//! # Build all available regpacks for a jurisdiction:
//! mez regpack build --jurisdiction pk --all-domains
//! ```

use std::path::Path;

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};

use mez_pack::regpack::pakistan::{build_pakistan_regpack, build_pakistan_sanctions_regpack};

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
        /// Jurisdiction ID (e.g., pk, ae-difc).
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
    }
}

fn run_build(
    jurisdiction: &str,
    domain: Option<&str>,
    all_domains: bool,
    store: bool,
    repo_root: &Path,
) -> Result<u8> {
    let domains: Vec<&str> = if all_domains {
        match jurisdiction {
            "pk" => vec!["financial", "sanctions"],
            _ => bail!("No regpack content available for jurisdiction '{jurisdiction}'"),
        }
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
fn build_regpack_for(jurisdiction: &str, domain: &str) -> Result<(String, Vec<u8>)> {
    match (jurisdiction, domain) {
        ("pk", "financial") => {
            let (regpack, metadata, sanctions, deadlines, reporting, wht) =
                build_pakistan_regpack().context("failed to build Pakistan financial regpack")?;

            let digest_hex = regpack
                .digest
                .as_ref()
                .context("regpack has no digest")?
                .to_hex();

            // Build the CAS-storable JSON artifact.
            let artifact = serde_json::json!({
                "regpack_id": metadata.regpack_id,
                "jurisdiction_id": metadata.jurisdiction_id,
                "domain": metadata.domain,
                "as_of_date": metadata.as_of_date,
                "snapshot_type": metadata.snapshot_type,
                "sources": metadata.sources,
                "includes": metadata.includes,
                "created_at": metadata.created_at,
                "expires_at": metadata.expires_at,
                "digest_sha256": digest_hex,
                "sanctions_snapshot": sanctions,
                "regulators": mez_pack::regpack::pakistan::pakistan_regulators(),
                "compliance_deadlines": deadlines,
                "reporting_requirements": reporting,
                "withholding_tax_rates": wht,
            });

            let json_bytes = serde_json::to_vec_pretty(&artifact)
                .context("failed to serialize regpack artifact")?;

            Ok((digest_hex, json_bytes))
        }
        ("pk", "sanctions") => {
            let (regpack, metadata, sanctions) =
                build_pakistan_sanctions_regpack()
                    .context("failed to build Pakistan sanctions regpack")?;

            let digest_hex = regpack
                .digest
                .as_ref()
                .context("regpack has no digest")?
                .to_hex();

            let artifact = serde_json::json!({
                "regpack_id": metadata.regpack_id,
                "jurisdiction_id": metadata.jurisdiction_id,
                "domain": metadata.domain,
                "as_of_date": metadata.as_of_date,
                "snapshot_type": metadata.snapshot_type,
                "sources": metadata.sources,
                "includes": metadata.includes,
                "created_at": metadata.created_at,
                "expires_at": metadata.expires_at,
                "digest_sha256": digest_hex,
                "sanctions_snapshot": sanctions,
            });

            let json_bytes = serde_json::to_vec_pretty(&artifact)
                .context("failed to serialize sanctions regpack artifact")?;

            Ok((digest_hex, json_bytes))
        }
        _ => bail!(
            "No regpack content available for jurisdiction '{jurisdiction}' domain '{domain}'"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_pk_financial_returns_digest() {
        let (hex, bytes) = build_regpack_for("pk", "financial").unwrap();
        assert_eq!(hex.len(), 64, "digest must be 64 hex chars");
        assert!(!bytes.is_empty());
        // Verify it's valid JSON.
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
}
