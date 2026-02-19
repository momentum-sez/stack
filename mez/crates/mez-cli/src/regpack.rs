//! # Regpack Subcommand
//!
//! Build and verify content-addressed regpack artifacts.
//!
//! Wraps the `mez-pack` regpack module to provide CLI access to regpack
//! build and verify operations. All digests flow through `CanonicalBytes`
//! and `Sha256Accumulator` per the CAS integrity invariant (I-CANON).
//!
//! ## Commands
//!
//! - `mez regpack build --jurisdiction <jid> --output <path>` — Build a
//!   regpack, write JSON + `.digest` file.
//! - `mez regpack verify <path>` — Recompute digest from file, compare to
//!   `.digest` sidecar.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};

use mez_core::CanonicalBytes;
use mez_core::digest::Sha256Accumulator;

/// Arguments for the `mez regpack` subcommand.
#[derive(Args, Debug)]
pub struct RegpackArgs {
    #[command(subcommand)]
    pub command: RegpackCommand,
}

/// Regpack subcommands.
#[derive(Subcommand, Debug)]
pub enum RegpackCommand {
    /// Build a regpack for a jurisdiction and write to output path.
    Build {
        /// Jurisdiction identifier (e.g., "pk-sifc", "pk").
        #[arg(long)]
        jurisdiction: String,
        /// Output directory for the regpack JSON and digest files.
        #[arg(long, short, default_value = ".")]
        output: PathBuf,
    },

    /// Verify a regpack file against its digest sidecar.
    Verify {
        /// Path to the regpack JSON file.
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
}

/// Execute the regpack subcommand.
pub fn run_regpack(args: &RegpackArgs, repo_root: &Path) -> Result<u8> {
    match &args.command {
        RegpackCommand::Build {
            jurisdiction,
            output,
        } => {
            let resolved_output = crate::resolve_path(output, repo_root);
            cmd_build(jurisdiction, &resolved_output)
        }
        RegpackCommand::Verify { file } => {
            let resolved_file = crate::resolve_path(file, repo_root);
            cmd_verify(&resolved_file)
        }
    }
}

/// Build a regpack for the given jurisdiction.
///
/// Currently supports `pk` and `pk-sifc` (both map to Pakistan regpack).
/// Writes `<jurisdiction>.regpack.json` and `<jurisdiction>.regpack.digest`
/// to the output directory.
fn cmd_build(jurisdiction: &str, output_dir: &Path) -> Result<u8> {
    // Normalize jurisdiction to determine builder.
    let jid = jurisdiction.to_lowercase();

    match jid.as_str() {
        "pk" | "pk-sifc" => cmd_build_pakistan(output_dir),
        _ => {
            bail!(
                "unsupported jurisdiction for regpack build: {jurisdiction}. \
                 Supported: pk, pk-sifc"
            );
        }
    }
}

/// Build the Pakistan regpack, serialize via CanonicalBytes, compute digest.
fn cmd_build_pakistan(output_dir: &Path) -> Result<u8> {
    use mez_pack::regpack::pakistan::build_pakistan_regpack;

    let (regpack, metadata, sanctions, deadlines, reporting, wht_rates) =
        build_pakistan_regpack().map_err(|e| anyhow::anyhow!("regpack build failed: {e}"))?;

    // Assemble the full regpack document for serialization.
    let regpack_doc = serde_json::json!({
        "regpack": {
            "jurisdiction": regpack.jurisdiction.as_str(),
            "name": regpack.name,
            "version": regpack.version,
            "digest": regpack.digest.as_ref().map(|d| d.to_hex()),
        },
        "metadata": metadata,
        "sanctions": sanctions,
        "deadlines": deadlines,
        "reporting_requirements": reporting,
        "withholding_tax_rates": wht_rates,
    });

    // Canonicalize and compute digest via CanonicalBytes.
    let canonical = CanonicalBytes::from_value(regpack_doc.clone())
        .context("failed to canonicalize regpack")?;

    let mut acc = Sha256Accumulator::new();
    acc.update(b"mez-regpack-artifact-v1\0");
    acc.update(canonical.as_bytes());
    let digest_hex = acc.finalize_hex();

    // Write output files.
    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    let json_path = output_dir.join("pk.regpack.json");
    let digest_path = output_dir.join("pk.regpack.digest");

    let json_pretty = serde_json::to_string_pretty(&regpack_doc)
        .context("failed to serialize regpack JSON")?;
    std::fs::write(&json_path, &json_pretty)
        .with_context(|| format!("failed to write regpack: {}", json_path.display()))?;

    std::fs::write(&digest_path, &digest_hex)
        .with_context(|| format!("failed to write digest: {}", digest_path.display()))?;

    // Also report the internal regpack digest (from compute_regpack_digest).
    let internal_digest = regpack
        .digest
        .as_ref()
        .map(|d| d.to_hex())
        .unwrap_or_else(|| "none".to_string());

    println!("OK: built Pakistan regpack");
    println!("  Regpack:  {}", json_path.display());
    println!("  Digest:   {}", digest_path.display());
    println!("  Artifact digest (SHA-256): {digest_hex}");
    println!("  Internal regpack digest:   {internal_digest}");

    Ok(0)
}

/// Verify a regpack file against its `.digest` sidecar.
///
/// Recomputes the artifact digest from the JSON file using the same
/// `CanonicalBytes` + `Sha256Accumulator` pipeline as `build`, then
/// compares to the contents of the `.digest` file.
fn cmd_verify(file_path: &Path) -> Result<u8> {
    if !file_path.exists() {
        bail!("regpack file not found: {}", file_path.display());
    }

    // Derive the digest sidecar path.
    let digest_path = derive_digest_path(file_path);
    if !digest_path.exists() {
        bail!(
            "digest sidecar not found: {}. Run `mez regpack build` first.",
            digest_path.display()
        );
    }

    // Read and parse the regpack JSON.
    let content = std::fs::read_to_string(file_path)
        .with_context(|| format!("failed to read regpack: {}", file_path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse regpack JSON: {}", file_path.display()))?;

    // Recompute digest.
    let canonical =
        CanonicalBytes::from_value(value).context("failed to canonicalize regpack")?;

    let mut acc = Sha256Accumulator::new();
    acc.update(b"mez-regpack-artifact-v1\0");
    acc.update(canonical.as_bytes());
    let computed = acc.finalize_hex();

    // Read expected digest.
    let expected = std::fs::read_to_string(&digest_path)
        .with_context(|| format!("failed to read digest: {}", digest_path.display()))?;
    let expected = expected.trim();

    if computed == expected {
        println!("OK: regpack integrity verified");
        println!("  File:   {}", file_path.display());
        println!("  Digest: {computed}");
        Ok(0)
    } else {
        println!("FAIL: regpack digest mismatch");
        println!("  File:     {}", file_path.display());
        println!("  Expected: {expected}");
        println!("  Computed: {computed}");
        Ok(1)
    }
}

/// Derive the `.digest` sidecar path from a regpack JSON path.
///
/// `foo/pk.regpack.json` → `foo/pk.regpack.digest`
fn derive_digest_path(json_path: &Path) -> PathBuf {
    let stem = json_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("regpack");
    let parent = json_path.parent().unwrap_or(Path::new("."));
    parent.join(format!("{stem}.digest"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_pakistan_regpack_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_build("pk", dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        assert!(dir.path().join("pk.regpack.json").exists());
        assert!(dir.path().join("pk.regpack.digest").exists());

        // Digest file should be 64 hex chars.
        let digest = std::fs::read_to_string(dir.path().join("pk.regpack.digest")).unwrap();
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn build_pk_sifc_maps_to_pakistan() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_build("pk-sifc", dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert!(dir.path().join("pk.regpack.json").exists());
    }

    #[test]
    fn build_unsupported_jurisdiction_fails() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_build("xx-unknown", dir.path());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("unsupported jurisdiction"));
    }

    #[test]
    fn build_then_verify_roundtrip() {
        let dir = tempfile::tempdir().unwrap();

        // Build.
        let build_result = cmd_build("pk", dir.path());
        assert!(build_result.is_ok());

        // Verify.
        let verify_result = cmd_verify(&dir.path().join("pk.regpack.json"));
        assert!(verify_result.is_ok());
        assert_eq!(verify_result.unwrap(), 0);
    }

    #[test]
    fn verify_detects_tampered_file() {
        let dir = tempfile::tempdir().unwrap();

        // Build.
        cmd_build("pk", dir.path()).unwrap();

        // Tamper with the JSON file by modifying actual content.
        let json_path = dir.path().join("pk.regpack.json");
        let content = std::fs::read_to_string(&json_path).unwrap();
        let tampered = content.replace("Pakistan", "TAMPERED");
        std::fs::write(&json_path, tampered).unwrap();

        // Verify should detect mismatch (exit code 1, not error).
        let result = cmd_verify(&json_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn verify_missing_file_fails() {
        let result = cmd_verify(Path::new("/tmp/mez-nonexistent-regpack.json"));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("not found"));
    }

    #[test]
    fn verify_missing_digest_sidecar_fails() {
        let dir = tempfile::tempdir().unwrap();
        let json_path = dir.path().join("test.regpack.json");
        std::fs::write(&json_path, r#"{"key":"value"}"#).unwrap();

        let result = cmd_verify(&json_path);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("digest sidecar not found"));
    }

    #[test]
    fn derive_digest_path_standard() {
        let input = PathBuf::from("/out/pk.regpack.json");
        let expected = PathBuf::from("/out/pk.regpack.digest");
        assert_eq!(derive_digest_path(&input), expected);
    }

    #[test]
    fn derive_digest_path_no_extension() {
        let input = PathBuf::from("/out/regpack");
        let expected = PathBuf::from("/out/regpack.digest");
        assert_eq!(derive_digest_path(&input), expected);
    }

    #[test]
    fn build_produces_deterministic_digest() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();

        cmd_build("pk", dir1.path()).unwrap();
        cmd_build("pk", dir2.path()).unwrap();

        let d1 = std::fs::read_to_string(dir1.path().join("pk.regpack.digest")).unwrap();
        let d2 = std::fs::read_to_string(dir2.path().join("pk.regpack.digest")).unwrap();
        assert_eq!(d1, d2, "Regpack build should be deterministic");
    }

    #[test]
    fn regpack_json_is_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        cmd_build("pk", dir.path()).unwrap();

        let content = std::fs::read_to_string(dir.path().join("pk.regpack.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.get("regpack").is_some());
        assert!(parsed.get("metadata").is_some());
        assert!(parsed.get("sanctions").is_some());
    }

    #[test]
    fn regpack_contains_internal_digest() {
        let dir = tempfile::tempdir().unwrap();
        cmd_build("pk", dir.path()).unwrap();

        let content = std::fs::read_to_string(dir.path().join("pk.regpack.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        let digest = parsed["regpack"]["digest"].as_str().unwrap();
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
