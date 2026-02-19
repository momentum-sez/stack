//! # Zone Subcommand
//!
//! Zone bootstrap and management operations. Reduces time-to-deploy by
//! generating a complete zone directory from a template, including zone
//! manifest, lockfile, and operator keypair.
//!
//! ## Commands
//!
//! - `mez zone init --jurisdiction <jid> --profile <profile>` — Bootstrap
//!   a new zone directory with zone.yaml, operator keys, and lockfile.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use rand_core::OsRng;

use mez_crypto::SigningKey;

/// Arguments for the `mez zone` subcommand.
#[derive(Args, Debug)]
pub struct ZoneArgs {
    #[command(subcommand)]
    pub command: ZoneCommand,
}

/// Zone subcommands.
#[derive(Subcommand, Debug)]
pub enum ZoneCommand {
    /// Initialize a new zone directory from a template.
    Init {
        /// Jurisdiction identifier (e.g., "pk", "pk-sifc", "ae-difc").
        #[arg(long)]
        jurisdiction: String,

        /// Zone deployment profile.
        #[arg(long, default_value = "sandbox")]
        profile: ZoneProfile,

        /// Zone name (human-readable). Defaults to "<jurisdiction> Zone".
        #[arg(long)]
        name: Option<String>,

        /// Output directory for the zone files.
        #[arg(long, short, default_value = ".")]
        output: PathBuf,
    },
}

/// Zone deployment profiles.
///
/// Each profile selects a different set of modules and configuration defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ZoneProfile {
    /// Sovereign government operating system deployment (full stack).
    SovereignGovos,
    /// Corridor node — minimal footprint for inter-zone operations.
    CorridorNode,
    /// Development sandbox with relaxed constraints.
    Sandbox,
}

impl ZoneProfile {
    /// Map to the canonical profile ID used in zone.yaml.
    fn profile_id(self) -> &'static str {
        match self {
            Self::SovereignGovos => "org.momentum.mez.profile.charter-city",
            Self::CorridorNode => "org.momentum.mez.profile.minimal-mvp",
            Self::Sandbox => "org.momentum.mez.profile.digital-financial-center",
        }
    }

    /// Default lawpack domains for this profile.
    fn lawpack_domains(self) -> &'static [&'static str] {
        match self {
            Self::SovereignGovos => &["civil", "financial", "tax", "aml"],
            Self::CorridorNode => &["financial"],
            Self::Sandbox => &["civil", "financial"],
        }
    }

    /// Default compliance domains for this profile.
    fn compliance_domains(self) -> &'static [&'static str] {
        match self {
            Self::SovereignGovos => &[
                "aml",
                "kyc",
                "sanctions",
                "tax",
                "securities",
                "corporate",
                "licensing",
                "data_privacy",
                "consumer_protection",
                "environmental",
            ],
            Self::CorridorNode => &["aml", "kyc", "sanctions"],
            Self::Sandbox => &[
                "aml",
                "kyc",
                "sanctions",
                "tax",
                "corporate",
                "licensing",
            ],
        }
    }

    /// Default corridors for this profile.
    fn corridors(self) -> &'static [&'static str] {
        match self {
            Self::SovereignGovos | Self::Sandbox => {
                &["org.momentum.mez.corridor.swift.iso20022-cross-border"]
            }
            Self::CorridorNode => &["org.momentum.mez.corridor.swift.iso20022-cross-border"],
        }
    }
}

/// Execute the zone subcommand.
pub fn run_zone(args: &ZoneArgs, repo_root: &Path) -> Result<u8> {
    match &args.command {
        ZoneCommand::Init {
            jurisdiction,
            profile,
            name,
            output,
        } => {
            let resolved_output = crate::resolve_path(output, repo_root);
            cmd_init(jurisdiction, *profile, name.as_deref(), &resolved_output)
        }
    }
}

/// Initialize a new zone directory.
fn cmd_init(
    jurisdiction: &str,
    profile: ZoneProfile,
    name: Option<&str>,
    output_dir: &Path,
) -> Result<u8> {
    // Validate jurisdiction ID.
    if jurisdiction.is_empty() {
        bail!("jurisdiction must not be empty");
    }
    if jurisdiction.len() > 64 {
        bail!("jurisdiction must not exceed 64 characters");
    }

    let default_name = format!("{} Zone", jurisdiction.to_uppercase());
    let zone_name = name.unwrap_or(&default_name);
    let zone_id = format!("org.momentum.mez.zone.{jurisdiction}");

    // Create the zone directory.
    let zone_dir = output_dir.join(jurisdiction);
    std::fs::create_dir_all(&zone_dir).with_context(|| {
        format!(
            "failed to create zone directory: {}",
            zone_dir.display()
        )
    })?;

    // Generate Ed25519 keypair for zone operator.
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();
    let sk_hex = sk
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let vk_hex = vk.to_hex();

    // Write keypair files.
    let key_dir = zone_dir.join("keys");
    std::fs::create_dir_all(&key_dir).with_context(|| {
        format!("failed to create keys directory: {}", key_dir.display())
    })?;

    let sk_path = key_dir.join("zone-operator.key");
    let vk_path = key_dir.join("zone-operator.pub");
    std::fs::write(&sk_path, &sk_hex)
        .with_context(|| format!("failed to write private key: {}", sk_path.display()))?;
    std::fs::write(&vk_path, &vk_hex)
        .with_context(|| format!("failed to write public key: {}", vk_path.display()))?;

    // Generate zone.yaml from template.
    let lawpack_domains: Vec<String> = profile
        .lawpack_domains()
        .iter()
        .map(|d| format!("  - {d}"))
        .collect();

    let compliance_domains: Vec<String> = profile
        .compliance_domains()
        .iter()
        .map(|d| format!("  - {d}"))
        .collect();

    let corridors: Vec<String> = profile
        .corridors()
        .iter()
        .map(|c| format!("  - {c}"))
        .collect();

    let zone_yaml = format!(
        r#"# Zone Manifest — {zone_name}
#
# Generated by: mez zone init --jurisdiction {jurisdiction} --profile {profile_label}
# Profile: {profile_id}
#
# Lock via:
#   mez lock {jurisdiction}/zone.yaml
#
# Deploy via:
#   ZONE_SIGNING_KEY_HEX=$(cat {jurisdiction}/keys/zone-operator.key) \
#   ./deploy/scripts/deploy-zone.sh {profile_label} {zone_id} {jurisdiction}

zone_id: {zone_id}
jurisdiction_id: {jurisdiction}
zone_name: "{zone_name}"

profile:
  profile_id: {profile_id}
  version: "0.4.44"

jurisdiction_stack:
  - {jurisdiction}

lawpack_domains:
{lawpack_domains}

overlays: []
params_overrides: {{}}

corridors:
{corridors}

domains:
{compliance_domains}

corridor_peers: []
trust_anchors: []

key_rotation_policy:
  default:
    rotation_days: 90
    grace_days: 14

lockfile_path: stack.lock
"#,
        zone_name = zone_name,
        jurisdiction = jurisdiction,
        profile_label = match profile {
            ZoneProfile::SovereignGovos => "sovereign-govos",
            ZoneProfile::CorridorNode => "corridor-node",
            ZoneProfile::Sandbox => "sandbox",
        },
        profile_id = profile.profile_id(),
        zone_id = zone_id,
        lawpack_domains = lawpack_domains.join("\n"),
        compliance_domains = compliance_domains.join("\n"),
        corridors = corridors.join("\n"),
    );

    let zone_yaml_path = zone_dir.join("zone.yaml");
    std::fs::write(&zone_yaml_path, &zone_yaml)
        .with_context(|| format!("failed to write zone.yaml: {}", zone_yaml_path.display()))?;

    // Write README with next steps.
    let readme = format!(
        r#"# {zone_name}

Zone bootstrapped by `mez zone init`.

## Files

- `zone.yaml` — Zone manifest
- `keys/zone-operator.key` — Ed25519 private key (KEEP SECRET)
- `keys/zone-operator.pub` — Ed25519 public key

## Next Steps

1. Review and customize `zone.yaml` for your deployment.
2. Generate the lockfile:
   ```
   mez lock {jurisdiction}/zone.yaml
   ```
3. Set the zone signing key in your environment:
   ```
   export ZONE_SIGNING_KEY_HEX=$(cat {jurisdiction}/keys/zone-operator.key)
   ```
4. Deploy the zone:
   ```
   ./deploy/scripts/deploy-zone.sh {profile_label} {zone_id} {jurisdiction}
   ```

## Security

- The private key (`zone-operator.key`) must be kept secret.
- In production, use a hardware security module (HSM) or KMS.
- Rotate keys per the `key_rotation_policy` in `zone.yaml`.
"#,
        zone_name = zone_name,
        jurisdiction = jurisdiction,
        profile_label = match profile {
            ZoneProfile::SovereignGovos => "sovereign-govos",
            ZoneProfile::CorridorNode => "corridor-node",
            ZoneProfile::Sandbox => "sandbox",
        },
        zone_id = zone_id,
    );

    let readme_path = zone_dir.join("README.md");
    std::fs::write(&readme_path, &readme)
        .with_context(|| format!("failed to write README: {}", readme_path.display()))?;

    println!("OK: initialized zone '{zone_id}'");
    println!("  Directory:   {}", zone_dir.display());
    println!("  Zone YAML:   {}", zone_yaml_path.display());
    println!("  Private key: {}", sk_path.display());
    println!("  Public key:  {}", vk_path.display());
    println!("  Public key (hex): {vk_hex}");
    println!();
    println!("Next: mez lock {}/zone.yaml", zone_dir.display());

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_sandbox_creates_zone_directory() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_init("test-zone", ZoneProfile::Sandbox, None, dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        let zone_dir = dir.path().join("test-zone");
        assert!(zone_dir.join("zone.yaml").exists());
        assert!(zone_dir.join("keys/zone-operator.key").exists());
        assert!(zone_dir.join("keys/zone-operator.pub").exists());
        assert!(zone_dir.join("README.md").exists());
    }

    #[test]
    fn init_sovereign_govos_creates_zone() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_init("pk", ZoneProfile::SovereignGovos, Some("Pakistan SIFC"), dir.path());
        assert!(result.is_ok());

        let content = std::fs::read_to_string(dir.path().join("pk/zone.yaml")).unwrap();
        assert!(content.contains("Pakistan SIFC"));
        assert!(content.contains("pk"));
        assert!(content.contains("charter-city"));
        assert!(content.contains("aml"));
        assert!(content.contains("tax"));
    }

    #[test]
    fn init_corridor_node_has_minimal_domains() {
        let dir = tempfile::tempdir().unwrap();
        cmd_init("relay", ZoneProfile::CorridorNode, None, dir.path()).unwrap();

        let content = std::fs::read_to_string(dir.path().join("relay/zone.yaml")).unwrap();
        assert!(content.contains("minimal-mvp"));
        // Corridor node has only financial lawpack domain.
        assert!(content.contains("financial"));
    }

    #[test]
    fn init_generates_valid_keypair() {
        let dir = tempfile::tempdir().unwrap();
        cmd_init("keys-test", ZoneProfile::Sandbox, None, dir.path()).unwrap();

        let sk_hex = std::fs::read_to_string(
            dir.path().join("keys-test/keys/zone-operator.key"),
        )
        .unwrap();
        assert_eq!(sk_hex.len(), 64);
        assert!(sk_hex.chars().all(|c| c.is_ascii_hexdigit()));

        let vk_hex = std::fs::read_to_string(
            dir.path().join("keys-test/keys/zone-operator.pub"),
        )
        .unwrap();
        assert_eq!(vk_hex.len(), 64);
        assert!(vk_hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn init_empty_jurisdiction_fails() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_init("", ZoneProfile::Sandbox, None, dir.path());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("must not be empty"));
    }

    #[test]
    fn init_long_jurisdiction_fails() {
        let dir = tempfile::tempdir().unwrap();
        let long_jid = "x".repeat(65);
        let result = cmd_init(&long_jid, ZoneProfile::Sandbox, None, dir.path());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("64 characters"));
    }

    #[test]
    fn zone_yaml_is_valid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        cmd_init("yaml-test", ZoneProfile::Sandbox, None, dir.path()).unwrap();

        let content = std::fs::read_to_string(dir.path().join("yaml-test/zone.yaml")).unwrap();
        let value: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        assert_eq!(
            value["zone_id"].as_str().unwrap(),
            "org.momentum.mez.zone.yaml-test"
        );
        assert_eq!(value["jurisdiction_id"].as_str().unwrap(), "yaml-test");
    }

    #[test]
    fn init_different_zones_get_different_keys() {
        let dir = tempfile::tempdir().unwrap();
        cmd_init("zone-a", ZoneProfile::Sandbox, None, dir.path()).unwrap();
        cmd_init("zone-b", ZoneProfile::Sandbox, None, dir.path()).unwrap();

        let key_a = std::fs::read_to_string(
            dir.path().join("zone-a/keys/zone-operator.key"),
        )
        .unwrap();
        let key_b = std::fs::read_to_string(
            dir.path().join("zone-b/keys/zone-operator.key"),
        )
        .unwrap();
        assert_ne!(key_a, key_b, "Different zones should get different keys");
    }

    #[test]
    fn profile_ids_are_correct() {
        assert!(ZoneProfile::SovereignGovos.profile_id().contains("charter-city"));
        assert!(ZoneProfile::CorridorNode.profile_id().contains("minimal-mvp"));
        assert!(ZoneProfile::Sandbox.profile_id().contains("digital-financial-center"));
    }

    #[test]
    fn zone_yaml_contains_lockfile_path() {
        let dir = tempfile::tempdir().unwrap();
        cmd_init("lock-test", ZoneProfile::Sandbox, None, dir.path()).unwrap();

        let content = std::fs::read_to_string(dir.path().join("lock-test/zone.yaml")).unwrap();
        assert!(content.contains("lockfile_path: stack.lock"));
    }
}
