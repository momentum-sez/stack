//! # Zone Deployment Manifests with Corridor Peering Configuration
//!
//! Implements [`ZoneManifest`] — a full deployment descriptor for a single zone —
//! and [`NetworkTopology`] — the multi-zone network graph that connects zones
//! via corridors and generates deployment artifacts (e.g., docker-compose).
//!
//! ## Zone Manifest
//!
//! A `ZoneManifest` captures everything needed to deploy and operate a single
//! zone: jurisdiction identity, profile, lawpack domains, corridor subscriptions,
//! peer configurations for inter-zone receipt exchange, trust anchors, key
//! rotation policies, compliance domains, and optional deployment configuration.
//!
//! ```text
//! ZoneManifest
//! ├── zone_id / jurisdiction_id / zone_name
//! ├── profile (ZoneProfile)
//! ├── jurisdiction_stack
//! ├── lawpack_domains
//! ├── corridors (corridor IDs this zone participates in)
//! ├── corridor_peers (CorridorPeerConfig — remote zone endpoints)
//! ├── trust_anchors
//! ├── key_rotation_policy (KeyRotationPolicy)
//! ├── compliance_domains
//! └── deployment (DeploymentConfig — optional container config)
//! ```
//!
//! ## Network Topology
//!
//! A `NetworkTopology` aggregates multiple `ZoneManifest`s into a network graph.
//! It computes corridor counts and can generate a multi-service docker-compose
//! for the entire network (one mez-api service per zone, shared Postgres and
//! Prometheus infrastructure).
//!
//! ## Parsing
//!
//! [`ZoneManifest::from_zone_yaml`] parses the `zone.yaml` format used in
//! `jurisdictions/*/zone.yaml` and `docs/examples/trade/src/zones/*/zone.yaml`.
//! It handles the nested `key_rotation_policy.default` structure and the
//! `profile.profile_id` / `profile.version` nesting.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::{PackError, PackResult};

// ---------------------------------------------------------------------------
// Supporting Types
// ---------------------------------------------------------------------------

/// Zone profile: identifies the deployment profile template and its version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneProfile {
    /// Profile identifier (e.g., `org.momentum.mez.profile.sovereign-govos`).
    pub profile_id: String,
    /// Spec version this profile targets (e.g., `0.4.44`).
    pub version: String,
}

/// Configuration for a corridor peer — a remote zone that this zone can
/// exchange receipts with over an inter-zone corridor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorridorPeerConfig {
    /// Zone ID of the peer zone.
    pub zone_id: String,
    /// Jurisdiction ID of the peer zone.
    pub jurisdiction_id: String,
    /// Endpoint URL for the peer zone's corridor API.
    pub endpoint_url: String,
}

/// Key rotation policy governing cryptographic key lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRotationPolicy {
    /// Number of days between key rotations.
    pub rotation_days: u32,
    /// Grace period (days) during which the old key remains valid.
    pub grace_days: u32,
}

impl KeyRotationPolicy {
    /// Validate the policy. Returns a list of error messages (empty = valid).
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.rotation_days == 0 {
            errors.push("rotation_days must be > 0".to_string());
        }
        if self.grace_days >= self.rotation_days {
            errors.push(format!(
                "grace_days ({}) must be less than rotation_days ({})",
                self.grace_days, self.rotation_days
            ));
        }
        errors
    }
}

/// Deployment configuration for containerized zone operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Docker image for the mez-api binary.
    pub docker_image: String,
    /// Number of replicas.
    pub replicas: u32,
    /// Environment variables passed to the container.
    pub env_vars: BTreeMap<String, String>,
    /// HTTP path for liveness/readiness health checks.
    pub health_check_path: String,
    /// HTTP path for Prometheus metrics scraping.
    pub metrics_path: String,
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            docker_image: "mez-api:latest".to_string(),
            replicas: 1,
            env_vars: BTreeMap::new(),
            health_check_path: "/health/liveness".to_string(),
            metrics_path: "/metrics".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// ZoneManifest
// ---------------------------------------------------------------------------

/// Full deployment descriptor for a single zone.
///
/// Captures the zone's identity, jurisdiction stack, corridor configuration,
/// compliance domains, key management policy, and optional deployment config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneManifest {
    /// Unique zone identifier (e.g., `org.momentum.mez.zone.pk-sifc`).
    pub zone_id: String,
    /// Jurisdiction identifier (e.g., `pk`, `ae-dubai-difc`).
    pub jurisdiction_id: String,
    /// Human-readable zone name.
    pub zone_name: String,
    /// Zone profile template and version.
    pub profile: ZoneProfile,
    /// Jurisdiction stack — layered legal/regulatory frameworks (top = most specific).
    #[serde(default)]
    pub jurisdiction_stack: Vec<String>,
    /// Lawpack domains to include when generating the zone lock.
    #[serde(default)]
    pub lawpack_domains: Vec<String>,
    /// Corridor IDs this zone participates in.
    #[serde(default)]
    pub corridors: Vec<String>,
    /// Corridor peer configurations for inter-zone receipt exchange.
    #[serde(default)]
    pub corridor_peers: Vec<CorridorPeerConfig>,
    /// Trust anchor identifiers.
    #[serde(default)]
    pub trust_anchors: Vec<String>,
    /// Key rotation policy.
    pub key_rotation_policy: KeyRotationPolicy,
    /// Compliance domains enforced by this zone.
    #[serde(default)]
    pub compliance_domains: Vec<String>,
    /// Optional deployment configuration for containerized operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment: Option<DeploymentConfig>,
}

impl ZoneManifest {
    /// Parse a `ZoneManifest` from a zone.yaml string.
    ///
    /// Handles the zone.yaml format used across the repository:
    /// - `profile` is nested as `{ profile_id, version }`
    /// - `key_rotation_policy` is nested under a `default` key
    /// - `corridor_peers` defaults to empty if absent
    /// - `compliance_domains` may appear as `domains` in some manifests
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] if the YAML cannot be parsed or required fields
    /// are missing.
    pub fn from_zone_yaml(yaml_str: &str) -> PackResult<Self> {
        let value: serde_json::Value =
            serde_yaml::from_str(yaml_str).map_err(|e| PackError::SchemaViolation {
                message: format!("failed to parse zone YAML: {e}"),
            })?;

        let obj = value
            .as_object()
            .ok_or_else(|| PackError::SchemaViolation {
                message: "zone manifest must be a YAML mapping".to_string(),
            })?;

        // Required: zone_id
        let zone_id = obj
            .get("zone_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PackError::SchemaViolation {
                message: "missing required field: zone_id".to_string(),
            })?
            .to_string();

        // Required: jurisdiction_id
        let jurisdiction_id = obj
            .get("jurisdiction_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PackError::SchemaViolation {
                message: "missing required field: jurisdiction_id".to_string(),
            })?
            .to_string();

        // Optional: zone_name (fallback to zone_id)
        let zone_name = obj
            .get("zone_name")
            .and_then(|v| v.as_str())
            .unwrap_or(&zone_id)
            .to_string();

        // Required: profile (nested object with profile_id, version)
        let profile = match obj.get("profile") {
            Some(p) if p.is_object() => {
                let profile_id = p
                    .get("profile_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let version = p
                    .get("version")
                    .and_then(|v| match v {
                        serde_json::Value::String(s) => Some(s.as_str()),
                        serde_json::Value::Number(_) => None, // handled below
                        _ => None,
                    })
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        // Version may be a number in YAML (e.g., 0.4.44 parsed as string,
                        // but some formatters emit it differently).
                        p.get("version")
                            .map(|v| match v {
                                serde_json::Value::Number(n) => n.to_string(),
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            })
                            .unwrap_or_default()
                    });
                ZoneProfile {
                    profile_id,
                    version,
                }
            }
            Some(p) if p.is_string() => ZoneProfile {
                profile_id: p.as_str().unwrap_or("").to_string(),
                version: String::new(),
            },
            _ => {
                return Err(PackError::SchemaViolation {
                    message: "missing required field: profile".to_string(),
                })
            }
        };

        // Optional: jurisdiction_stack
        let jurisdiction_stack = extract_string_array(obj, "jurisdiction_stack");

        // Optional: lawpack_domains
        let lawpack_domains = extract_string_array(obj, "lawpack_domains");

        // Optional: corridors
        let corridors = extract_string_array(obj, "corridors");

        // Optional: corridor_peers
        let corridor_peers = match obj.get("corridor_peers") {
            Some(serde_json::Value::Array(arr)) => {
                let mut peers = Vec::new();
                for (i, v) in arr.iter().enumerate() {
                    match serde_json::from_value::<CorridorPeerConfig>(v.clone()) {
                        Ok(peer) => peers.push(peer),
                        Err(e) => {
                            return Err(PackError::SchemaViolation {
                                message: format!(
                                    "corridor_peers[{i}]: invalid peer config: {e}"
                                ),
                            });
                        }
                    }
                }
                peers
            }
            _ => Vec::new(),
        };

        // Optional: trust_anchors
        let trust_anchors = extract_string_array(obj, "trust_anchors");

        // Required: key_rotation_policy (nested under "default")
        let key_rotation_policy = match obj.get("key_rotation_policy") {
            Some(krp) => {
                // The zone.yaml format nests under "default":
                //   key_rotation_policy:
                //     default:
                //       rotation_days: 90
                //       grace_days: 14
                let inner = krp.get("default").unwrap_or(krp);
                let rotation_days = inner
                    .get("rotation_days")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(90) as u32;
                let grace_days = inner
                    .get("grace_days")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(14) as u32;
                KeyRotationPolicy {
                    rotation_days,
                    grace_days,
                }
            }
            None => KeyRotationPolicy {
                rotation_days: 90,
                grace_days: 14,
            },
        };

        // Optional: compliance_domains (may also appear as "domains")
        let compliance_domains = {
            let mut domains = extract_string_array(obj, "compliance_domains");
            if domains.is_empty() {
                domains = extract_string_array(obj, "domains");
            }
            domains
        };

        // Optional: deployment
        let deployment = match obj.get("deployment") {
            Some(v) => Some(serde_json::from_value::<DeploymentConfig>(v.clone()).map_err(
                |e| PackError::SchemaViolation {
                    message: format!("invalid deployment config: {e}"),
                },
            )?),
            None => None,
        };

        Ok(ZoneManifest {
            zone_id,
            jurisdiction_id,
            zone_name,
            profile,
            jurisdiction_stack,
            lawpack_domains,
            corridors,
            corridor_peers,
            trust_anchors,
            key_rotation_policy,
            compliance_domains,
            deployment,
        })
    }
}

/// Extract an array of strings from a JSON object field.
fn extract_string_array(
    obj: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Vec<String> {
    match obj.get(field) {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// NetworkTopology
// ---------------------------------------------------------------------------

/// Multi-zone network topology.
///
/// Aggregates multiple [`ZoneManifest`]s and provides network-level operations:
/// corridor counting, docker-compose generation, and topology queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTopology {
    /// All zones in the network.
    pub zones: Vec<ZoneManifest>,
    /// Total number of unique corridors (computed).
    pub corridor_count: usize,
    /// Network identifier.
    pub network_id: String,
    /// Spec version.
    pub spec_version: String,
}

impl NetworkTopology {
    /// Create a new empty network topology.
    pub fn new(network_id: &str) -> Self {
        Self {
            zones: Vec::new(),
            corridor_count: 0,
            network_id: network_id.to_string(),
            spec_version: "0.4.44".to_string(),
        }
    }

    /// Add a zone to the network and recompute corridor count.
    pub fn add_zone(&mut self, manifest: ZoneManifest) {
        self.zones.push(manifest);
        self.corridor_count = self.compute_corridor_count();
    }

    /// Number of zones in the network.
    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    /// Number of unique corridors across all zones.
    pub fn corridor_count(&self) -> usize {
        self.compute_corridor_count()
    }

    /// Compute the number of unique corridor IDs across all zones.
    fn compute_corridor_count(&self) -> usize {
        let unique: BTreeSet<&str> = self
            .zones
            .iter()
            .flat_map(|z| z.corridors.iter().map(|c| c.as_str()))
            .collect();
        unique.len()
    }

    /// Generate a multi-service docker-compose.yaml string for the entire network.
    ///
    /// Produces one `mez-api-<zone_suffix>` service per zone, plus shared
    /// Postgres and Prometheus infrastructure services. Each zone service
    /// gets a unique port mapping starting from 8080.
    ///
    /// The generated YAML follows the same architectural pattern as
    /// `deploy/docker/docker-compose.yaml` (single Rust binary + Postgres +
    /// Prometheus).
    pub fn generate_docker_compose_network(&self) -> String {
        let mut out = String::new();

        // Header
        out.push_str(&format!(
            "# Auto-generated docker-compose for network: {}\n",
            self.network_id
        ));
        out.push_str(&format!("# Zones: {}\n", self.zones.len()));
        out.push_str(&format!(
            "# Corridors: {}\n\n",
            self.compute_corridor_count()
        ));
        out.push_str(&format!("name: {}\n\n", self.network_id));
        out.push_str("services:\n");

        // Zone services
        for (i, zone) in self.zones.iter().enumerate() {
            let suffix = zone_service_suffix(&zone.zone_id);
            let port = 8080 + i as u16;
            let deployment = zone.deployment.as_ref();
            let image = deployment
                .map(|d| d.docker_image.as_str())
                .unwrap_or("mez-api:latest");
            let health_path = deployment
                .map(|d| d.health_check_path.as_str())
                .unwrap_or("/health/liveness");
            let replicas = deployment.map(|d| d.replicas).unwrap_or(1);

            out.push_str(&format!("\n  mez-api-{}:\n", suffix));
            out.push_str(&format!("    image: {}\n", image));
            out.push_str(&format!("    container_name: mez-api-{}\n", suffix));
            out.push_str("    restart: unless-stopped\n");
            out.push_str(&format!("    ports:\n      - \"{}:{}\"\n", port, port));
            out.push_str("    environment:\n");
            out.push_str(&format!("      MEZ_ZONE_ID: \"{}\"\n", zone.zone_id));
            out.push_str(&format!(
                "      MEZ_JURISDICTION: \"{}\"\n",
                zone.jurisdiction_id
            ));
            out.push_str(&format!("      MEZ_PORT: \"{}\"\n", port));
            out.push_str(
                "      DATABASE_URL: \"postgresql://mez:${POSTGRES_PASSWORD}@postgres:5432/mez\"\n",
            );

            // Include any custom env vars from deployment config
            if let Some(deploy) = deployment {
                for (k, v) in &deploy.env_vars {
                    out.push_str(&format!("      {}: \"{}\"\n", k, v));
                }
            }

            if replicas > 1 {
                out.push_str(&format!("    deploy:\n      replicas: {}\n", replicas));
            }

            out.push_str("    depends_on:\n");
            out.push_str("      postgres:\n");
            out.push_str("        condition: service_healthy\n");
            out.push_str("    networks:\n");
            out.push_str("      - mez-internal\n");
            out.push_str("    healthcheck:\n");
            out.push_str(&format!(
                "      test: [\"CMD\", \"curl\", \"-f\", \"http://localhost:{}{}\"]",
                port, health_path
            ));
            out.push('\n');
            out.push_str("      interval: 30s\n");
            out.push_str("      timeout: 5s\n");
            out.push_str("      retries: 3\n");
            out.push_str("      start_period: 15s\n");
        }

        // Shared Postgres
        out.push_str("\n  postgres:\n");
        out.push_str("    image: postgres:16-alpine\n");
        out.push_str("    container_name: mez-postgres\n");
        out.push_str("    restart: unless-stopped\n");
        out.push_str("    environment:\n");
        out.push_str("      POSTGRES_USER: mez\n");
        out.push_str(
            "      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:?POSTGRES_PASSWORD must be set}\n",
        );
        out.push_str("      POSTGRES_DB: mez\n");
        out.push_str("    ports:\n");
        out.push_str("      - \"5432:5432\"\n");
        out.push_str("    networks:\n");
        out.push_str("      - mez-internal\n");
        out.push_str("    healthcheck:\n");
        out.push_str(
            "      test: [\"CMD-SHELL\", \"pg_isready -U mez -d mez\"]\n",
        );
        out.push_str("      interval: 10s\n");
        out.push_str("      timeout: 5s\n");
        out.push_str("      retries: 5\n");
        out.push_str("      start_period: 10s\n");

        // Shared Prometheus
        out.push_str("\n  prometheus:\n");
        out.push_str("    image: prom/prometheus:v2.51.0\n");
        out.push_str("    container_name: mez-prometheus\n");
        out.push_str("    restart: unless-stopped\n");
        out.push_str("    ports:\n");
        out.push_str("      - \"9090:9090\"\n");
        out.push_str("    networks:\n");
        out.push_str("      - mez-internal\n");
        out.push_str("    depends_on:\n");
        for zone in &self.zones {
            let suffix = zone_service_suffix(&zone.zone_id);
            out.push_str(&format!("      - mez-api-{}\n", suffix));
        }

        // Networks
        out.push_str("\nnetworks:\n");
        out.push_str("  mez-internal:\n");
        out.push_str("    driver: bridge\n");
        out.push_str(&format!("    name: {}-internal\n", self.network_id));

        // Volumes
        out.push_str("\nvolumes:\n");
        out.push_str("  postgres-data:\n");
        out.push_str("  prometheus-data:\n");

        out
    }
}

/// Derive a docker-compose service suffix from a zone ID.
///
/// Takes the last dot-separated segment and replaces dots with dashes.
/// e.g., `org.momentum.mez.zone.pk-sifc` -> `pk-sifc`
fn zone_service_suffix(zone_id: &str) -> String {
    zone_id
        .rsplit('.')
        .next()
        .unwrap_or(zone_id)
        .replace('.', "-")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Test 1: Parse a minimal zone.yaml
    // -----------------------------------------------------------------------

    #[test]
    fn parse_minimal_zone_yaml() {
        let yaml = r#"
zone_id: org.momentum.mez.zone.example
jurisdiction_id: ex
zone_name: Example Zone
profile:
  profile_id: org.momentum.mez.profile.digital-financial-center
  version: "0.4.44"
key_rotation_policy:
  default:
    rotation_days: 90
    grace_days: 14
"#;
        let manifest = ZoneManifest::from_zone_yaml(yaml).unwrap();
        assert_eq!(manifest.zone_id, "org.momentum.mez.zone.example");
        assert_eq!(manifest.jurisdiction_id, "ex");
        assert_eq!(manifest.zone_name, "Example Zone");
        assert_eq!(
            manifest.profile.profile_id,
            "org.momentum.mez.profile.digital-financial-center"
        );
        assert_eq!(manifest.profile.version, "0.4.44");
        assert_eq!(manifest.key_rotation_policy.rotation_days, 90);
        assert_eq!(manifest.key_rotation_policy.grace_days, 14);
        assert!(manifest.corridors.is_empty());
        assert!(manifest.corridor_peers.is_empty());
        assert!(manifest.deployment.is_none());
    }

    // -----------------------------------------------------------------------
    // Test 2: Parse the pk-sifc zone.yaml format
    // -----------------------------------------------------------------------

    #[test]
    fn parse_pk_sifc_zone_yaml() {
        let yaml = r#"
zone_id: org.momentum.mez.zone.pk-sifc
jurisdiction_id: pk
zone_name: Pakistan SIFC Economic Zone
profile:
  profile_id: org.momentum.mez.profile.sovereign-govos
  version: "0.4.44"
jurisdiction_stack:
  - pk
lawpack_domains:
  - civil
  - financial
  - tax
  - aml
domains:
  - aml
  - kyc
  - sanctions
  - tax
corridors:
  - org.momentum.mez.corridor.swift.iso20022-cross-border
corridor_peers: []
trust_anchors: []
key_rotation_policy:
  default:
    rotation_days: 90
    grace_days: 14
"#;
        let manifest = ZoneManifest::from_zone_yaml(yaml).unwrap();
        assert_eq!(manifest.zone_id, "org.momentum.mez.zone.pk-sifc");
        assert_eq!(manifest.jurisdiction_id, "pk");
        assert_eq!(manifest.zone_name, "Pakistan SIFC Economic Zone");
        assert_eq!(
            manifest.profile.profile_id,
            "org.momentum.mez.profile.sovereign-govos"
        );
        assert_eq!(manifest.jurisdiction_stack, vec!["pk"]);
        assert_eq!(manifest.lawpack_domains.len(), 4);
        assert!(manifest.lawpack_domains.contains(&"aml".to_string()));
        assert_eq!(manifest.corridors.len(), 1);
        assert!(manifest.corridor_peers.is_empty());
        // compliance_domains populated from "domains" field
        assert_eq!(manifest.compliance_domains.len(), 4);
        assert!(manifest.compliance_domains.contains(&"sanctions".to_string()));
    }

    // -----------------------------------------------------------------------
    // Test 3: NetworkTopology with 3 zones
    // -----------------------------------------------------------------------

    #[test]
    fn network_topology_three_zones() {
        let mut topo = NetworkTopology::new("mez-test-network");

        let zone_a = ZoneManifest {
            zone_id: "org.momentum.mez.zone.alpha".to_string(),
            jurisdiction_id: "pk".to_string(),
            zone_name: "Alpha Zone".to_string(),
            profile: ZoneProfile {
                profile_id: "test".to_string(),
                version: "0.4.44".to_string(),
            },
            jurisdiction_stack: vec!["pk".to_string()],
            lawpack_domains: vec!["civil".to_string()],
            corridors: vec!["corridor-swift".to_string(), "corridor-stablecoin".to_string()],
            corridor_peers: vec![],
            trust_anchors: vec![],
            key_rotation_policy: KeyRotationPolicy {
                rotation_days: 90,
                grace_days: 14,
            },
            compliance_domains: vec!["aml".to_string()],
            deployment: None,
        };

        let zone_b = ZoneManifest {
            zone_id: "org.momentum.mez.zone.beta".to_string(),
            jurisdiction_id: "ae".to_string(),
            zone_name: "Beta Zone".to_string(),
            profile: ZoneProfile {
                profile_id: "test".to_string(),
                version: "0.4.44".to_string(),
            },
            jurisdiction_stack: vec!["ae".to_string()],
            lawpack_domains: vec!["financial".to_string()],
            corridors: vec!["corridor-swift".to_string()],
            corridor_peers: vec![CorridorPeerConfig {
                zone_id: "org.momentum.mez.zone.alpha".to_string(),
                jurisdiction_id: "pk".to_string(),
                endpoint_url: "https://alpha.example.com/corridor".to_string(),
            }],
            trust_anchors: vec![],
            key_rotation_policy: KeyRotationPolicy {
                rotation_days: 60,
                grace_days: 7,
            },
            compliance_domains: vec!["aml".to_string(), "kyc".to_string()],
            deployment: None,
        };

        let zone_c = ZoneManifest {
            zone_id: "org.momentum.mez.zone.gamma".to_string(),
            jurisdiction_id: "sg".to_string(),
            zone_name: "Gamma Zone".to_string(),
            profile: ZoneProfile {
                profile_id: "test".to_string(),
                version: "0.4.44".to_string(),
            },
            jurisdiction_stack: vec!["sg".to_string()],
            lawpack_domains: vec![],
            corridors: vec!["corridor-stablecoin".to_string(), "corridor-raast".to_string()],
            corridor_peers: vec![],
            trust_anchors: vec![],
            key_rotation_policy: KeyRotationPolicy {
                rotation_days: 30,
                grace_days: 5,
            },
            compliance_domains: vec![],
            deployment: None,
        };

        topo.add_zone(zone_a);
        topo.add_zone(zone_b);
        topo.add_zone(zone_c);

        assert_eq!(topo.zone_count(), 3);
        // Unique corridors: corridor-swift, corridor-stablecoin, corridor-raast
        assert_eq!(topo.corridor_count(), 3);
        assert_eq!(topo.network_id, "mez-test-network");
        assert_eq!(topo.spec_version, "0.4.44");
    }

    // -----------------------------------------------------------------------
    // Test 4: Docker compose generation produces valid YAML
    // -----------------------------------------------------------------------

    #[test]
    fn docker_compose_generation_produces_valid_yaml() {
        let mut topo = NetworkTopology::new("mez-multi");

        topo.add_zone(ZoneManifest {
            zone_id: "org.momentum.mez.zone.pk-sifc".to_string(),
            jurisdiction_id: "pk".to_string(),
            zone_name: "PK SIFC".to_string(),
            profile: ZoneProfile {
                profile_id: "sovereign-govos".to_string(),
                version: "0.4.44".to_string(),
            },
            jurisdiction_stack: vec!["pk".to_string()],
            lawpack_domains: vec!["civil".to_string()],
            corridors: vec!["swift-corridor".to_string()],
            corridor_peers: vec![],
            trust_anchors: vec![],
            key_rotation_policy: KeyRotationPolicy {
                rotation_days: 90,
                grace_days: 14,
            },
            compliance_domains: vec![],
            deployment: Some(DeploymentConfig {
                docker_image: "mez-api:0.4.44".to_string(),
                replicas: 2,
                env_vars: BTreeMap::from([
                    ("MEZ_LOG_LEVEL".to_string(), "debug".to_string()),
                ]),
                health_check_path: "/health/liveness".to_string(),
                metrics_path: "/metrics".to_string(),
            }),
        });

        topo.add_zone(ZoneManifest {
            zone_id: "org.momentum.mez.zone.ae-difc".to_string(),
            jurisdiction_id: "ae-dubai-difc".to_string(),
            zone_name: "DIFC Zone".to_string(),
            profile: ZoneProfile {
                profile_id: "trade-playbook".to_string(),
                version: "0.4.44".to_string(),
            },
            jurisdiction_stack: vec![
                "ae".to_string(),
                "ae-dubai".to_string(),
                "ae-dubai-difc".to_string(),
            ],
            lawpack_domains: vec!["financial".to_string()],
            corridors: vec!["swift-corridor".to_string()],
            corridor_peers: vec![],
            trust_anchors: vec![],
            key_rotation_policy: KeyRotationPolicy {
                rotation_days: 90,
                grace_days: 14,
            },
            compliance_domains: vec![],
            deployment: None,
        });

        let compose = topo.generate_docker_compose_network();

        // Verify it contains expected structural elements
        assert!(compose.contains("name: mez-multi"));
        assert!(compose.contains("services:"));
        assert!(compose.contains("mez-api-pk-sifc:"));
        assert!(compose.contains("mez-api-ae-difc:"));
        assert!(compose.contains("postgres:"));
        assert!(compose.contains("prometheus:"));
        assert!(compose.contains("networks:"));
        assert!(compose.contains("volumes:"));
        // Zone-specific ports
        assert!(compose.contains("\"8080:8080\""));
        assert!(compose.contains("\"8081:8081\""));
        // Zone IDs in environment
        assert!(compose.contains("org.momentum.mez.zone.pk-sifc"));
        assert!(compose.contains("org.momentum.mez.zone.ae-difc"));
        // Custom image
        assert!(compose.contains("image: mez-api:0.4.44"));
        // Custom env var
        assert!(compose.contains("MEZ_LOG_LEVEL: \"debug\""));
        // Replicas
        assert!(compose.contains("replicas: 2"));
        // Default image for second zone
        assert!(compose.contains("image: mez-api:latest"));

        // Verify the YAML can be parsed back by serde_yaml
        let parsed: serde_yaml::Value = serde_yaml::from_str(&compose)
            .expect("generated docker-compose must be valid YAML");
        assert!(parsed.get("services").is_some());
        assert!(parsed.get("networks").is_some());
        assert!(parsed.get("volumes").is_some());
    }

    // -----------------------------------------------------------------------
    // Test 5: Corridor peer config serialization roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn corridor_peer_config_serialization_roundtrip() {
        let peer = CorridorPeerConfig {
            zone_id: "org.momentum.mez.zone.pk-sifc".to_string(),
            jurisdiction_id: "pk".to_string(),
            endpoint_url: "https://pk-sifc.example.com/corridor/v1".to_string(),
        };

        let json = serde_json::to_string(&peer).unwrap();
        let deserialized: CorridorPeerConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(peer, deserialized);
        assert!(json.contains("pk-sifc"));
        assert!(json.contains("endpoint_url"));

        // Also test YAML roundtrip
        let yaml = serde_yaml::to_string(&peer).unwrap();
        let from_yaml: CorridorPeerConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(peer, from_yaml);
    }

    // -----------------------------------------------------------------------
    // Test 6: ZoneProfile serialization
    // -----------------------------------------------------------------------

    #[test]
    fn zone_profile_serialization() {
        let profile = ZoneProfile {
            profile_id: "org.momentum.mez.profile.sovereign-govos".to_string(),
            version: "0.4.44".to_string(),
        };

        let json = serde_json::to_string_pretty(&profile).unwrap();
        assert!(json.contains("sovereign-govos"));
        assert!(json.contains("0.4.44"));

        let deserialized: ZoneProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, deserialized);
    }

    // -----------------------------------------------------------------------
    // Test 7: DeploymentConfig defaults
    // -----------------------------------------------------------------------

    #[test]
    fn deployment_config_defaults() {
        let config = DeploymentConfig::default();

        assert_eq!(config.docker_image, "mez-api:latest");
        assert_eq!(config.replicas, 1);
        assert!(config.env_vars.is_empty());
        assert_eq!(config.health_check_path, "/health/liveness");
        assert_eq!(config.metrics_path, "/metrics");

        // Verify serialization roundtrip preserves defaults
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: DeploymentConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }

    // -----------------------------------------------------------------------
    // Test 8: KeyRotationPolicy validation
    // -----------------------------------------------------------------------

    #[test]
    fn key_rotation_policy_validation() {
        // Valid policy
        let valid = KeyRotationPolicy {
            rotation_days: 90,
            grace_days: 14,
        };
        assert!(valid.validate().is_empty());

        // Invalid: rotation_days == 0
        let zero_rotation = KeyRotationPolicy {
            rotation_days: 0,
            grace_days: 0,
        };
        let errors = zero_rotation.validate();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("rotation_days must be > 0")));

        // Invalid: grace_days >= rotation_days
        let grace_too_large = KeyRotationPolicy {
            rotation_days: 30,
            grace_days: 30,
        };
        let errors = grace_too_large.validate();
        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.contains("grace_days") && e.contains("less than")));

        // Edge case: grace_days > rotation_days
        let grace_exceeds = KeyRotationPolicy {
            rotation_days: 10,
            grace_days: 20,
        };
        let errors = grace_exceeds.validate();
        assert!(!errors.is_empty());
    }

    // -----------------------------------------------------------------------
    // Additional tests beyond the required 8
    // -----------------------------------------------------------------------

    #[test]
    fn zone_manifest_missing_zone_id_rejected() {
        let yaml = r#"
jurisdiction_id: pk
zone_name: Test
profile:
  profile_id: test
  version: "1.0"
"#;
        let result = ZoneManifest::from_zone_yaml(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("zone_id"));
    }

    #[test]
    fn zone_manifest_missing_jurisdiction_id_rejected() {
        let yaml = r#"
zone_id: test.zone
zone_name: Test
profile:
  profile_id: test
  version: "1.0"
"#;
        let result = ZoneManifest::from_zone_yaml(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("jurisdiction_id"));
    }

    #[test]
    fn zone_manifest_missing_profile_rejected() {
        let yaml = r#"
zone_id: test.zone
jurisdiction_id: pk
zone_name: Test
"#;
        let result = ZoneManifest::from_zone_yaml(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("profile"));
    }

    #[test]
    fn zone_manifest_defaults_zone_name_to_zone_id() {
        let yaml = r#"
zone_id: org.momentum.mez.zone.fallback
jurisdiction_id: ex
profile:
  profile_id: test
  version: "1.0"
"#;
        let manifest = ZoneManifest::from_zone_yaml(yaml).unwrap();
        assert_eq!(manifest.zone_name, "org.momentum.mez.zone.fallback");
    }

    #[test]
    fn zone_manifest_defaults_key_rotation_when_missing() {
        let yaml = r#"
zone_id: test.zone
jurisdiction_id: ex
zone_name: Test
profile:
  profile_id: test
  version: "1.0"
"#;
        let manifest = ZoneManifest::from_zone_yaml(yaml).unwrap();
        assert_eq!(manifest.key_rotation_policy.rotation_days, 90);
        assert_eq!(manifest.key_rotation_policy.grace_days, 14);
    }

    #[test]
    fn network_topology_empty() {
        let topo = NetworkTopology::new("empty-net");
        assert_eq!(topo.zone_count(), 0);
        assert_eq!(topo.corridor_count(), 0);
        assert_eq!(topo.network_id, "empty-net");
    }

    #[test]
    fn zone_service_suffix_extraction() {
        assert_eq!(
            zone_service_suffix("org.momentum.mez.zone.pk-sifc"),
            "pk-sifc"
        );
        assert_eq!(
            zone_service_suffix("org.momentum.mez.zone.ae-difc"),
            "ae-difc"
        );
        assert_eq!(zone_service_suffix("simple"), "simple");
    }

    #[test]
    fn zone_manifest_serde_roundtrip() {
        let manifest = ZoneManifest {
            zone_id: "test.zone".to_string(),
            jurisdiction_id: "pk".to_string(),
            zone_name: "Test Zone".to_string(),
            profile: ZoneProfile {
                profile_id: "test-profile".to_string(),
                version: "1.0".to_string(),
            },
            jurisdiction_stack: vec!["pk".to_string()],
            lawpack_domains: vec!["civil".to_string()],
            corridors: vec!["corridor-a".to_string()],
            corridor_peers: vec![CorridorPeerConfig {
                zone_id: "peer.zone".to_string(),
                jurisdiction_id: "ae".to_string(),
                endpoint_url: "https://peer.example.com".to_string(),
            }],
            trust_anchors: vec!["anchor-1".to_string()],
            key_rotation_policy: KeyRotationPolicy {
                rotation_days: 60,
                grace_days: 7,
            },
            compliance_domains: vec!["aml".to_string(), "kyc".to_string()],
            deployment: Some(DeploymentConfig::default()),
        };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let deserialized: ZoneManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(manifest, deserialized);
    }

    #[test]
    fn zone_manifest_with_corridor_peers_from_yaml() {
        let yaml = r#"
zone_id: org.momentum.mez.zone.beta
jurisdiction_id: ae
zone_name: Beta Zone
profile:
  profile_id: test
  version: "1.0"
corridor_peers:
  - zone_id: org.momentum.mez.zone.alpha
    jurisdiction_id: pk
    endpoint_url: https://alpha.example.com/corridor
  - zone_id: org.momentum.mez.zone.gamma
    jurisdiction_id: sg
    endpoint_url: https://gamma.example.com/corridor
key_rotation_policy:
  default:
    rotation_days: 60
    grace_days: 7
"#;
        let manifest = ZoneManifest::from_zone_yaml(yaml).unwrap();
        assert_eq!(manifest.corridor_peers.len(), 2);
        assert_eq!(
            manifest.corridor_peers[0].zone_id,
            "org.momentum.mez.zone.alpha"
        );
        assert_eq!(manifest.corridor_peers[0].jurisdiction_id, "pk");
        assert_eq!(
            manifest.corridor_peers[1].endpoint_url,
            "https://gamma.example.com/corridor"
        );
    }

    #[test]
    fn network_topology_corridor_count_deduplicates() {
        let mut topo = NetworkTopology::new("dedup-test");

        let make_zone = |id: &str, corridors: Vec<String>| ZoneManifest {
            zone_id: id.to_string(),
            jurisdiction_id: "ex".to_string(),
            zone_name: id.to_string(),
            profile: ZoneProfile {
                profile_id: "test".to_string(),
                version: "1.0".to_string(),
            },
            jurisdiction_stack: vec![],
            lawpack_domains: vec![],
            corridors,
            corridor_peers: vec![],
            trust_anchors: vec![],
            key_rotation_policy: KeyRotationPolicy {
                rotation_days: 90,
                grace_days: 14,
            },
            compliance_domains: vec![],
            deployment: None,
        };

        topo.add_zone(make_zone(
            "zone-a",
            vec!["corridor-1".to_string(), "corridor-2".to_string()],
        ));
        topo.add_zone(make_zone(
            "zone-b",
            vec!["corridor-2".to_string(), "corridor-3".to_string()],
        ));

        // corridor-1, corridor-2, corridor-3 = 3 unique
        assert_eq!(topo.corridor_count(), 3);
    }
}
