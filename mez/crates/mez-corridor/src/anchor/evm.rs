//! # EVM JSON-RPC Anchor Target
//!
//! Production L1 anchor target that records corridor checkpoint digests on
//! EVM-compatible chains (Ethereum, Arbitrum, Base, Polygon) via JSON-RPC.
//!
//! ## How It Works
//!
//! 1. The anchor target calls a smart contract's `recordDigest(bytes32)`
//!    function via `eth_sendTransaction`.
//! 2. The JSON-RPC endpoint (e.g., AWS Managed Blockchain, Alchemy, Infura)
//!    handles transaction signing. The `from` address must be unlocked or
//!    managed by the RPC provider's signing service.
//! 3. Status checks use `eth_getTransactionReceipt` and compare the current
//!    block height against the transaction's block to determine finality.
//!
//! ## Supported Chains
//!
//! Per `schemas/phoenix.anchor.schema.json`: ethereum, arbitrum, base, polygon.
//!
//! ## Security
//!
//! - The anchor target does NOT hold private keys. Transaction signing is
//!   delegated to the RPC endpoint's key management (HSM, KMS, or unlocked
//!   account).
//! - The `from` address must be funded with sufficient native token for gas.
//! - All RPC calls use HTTPS.

use super::{AnchorCommitment, AnchorError, AnchorReceipt, AnchorStatus, AnchorTarget};

/// Configuration for the EVM JSON-RPC anchor target.
#[derive(Debug, Clone)]
pub struct EvmAnchorConfig {
    /// JSON-RPC endpoint URL (must be HTTPS in production).
    pub rpc_url: String,
    /// Contract address for the digest recording contract (0x-prefixed, 40 hex chars).
    pub contract_address: String,
    /// Sender address whose transactions are signed by the RPC provider (0x-prefixed).
    pub from_address: String,
    /// Human-readable chain name (e.g., "ethereum", "arbitrum", "base", "polygon").
    pub chain_name: String,
    /// EVM chain ID (e.g., 1 for Ethereum mainnet, 42161 for Arbitrum One).
    pub chain_id: u64,
    /// Number of block confirmations required before reporting `Confirmed`.
    pub confirmations_for_confirmed: u64,
    /// Number of block confirmations required before reporting `Finalized`.
    pub confirmations_for_finalized: u64,
    /// Request timeout in seconds (default: 30).
    pub timeout_secs: u64,
}

impl EvmAnchorConfig {
    /// Create a configuration with sensible defaults for Ethereum mainnet.
    ///
    /// Defaults: 1 confirmation for Confirmed, 12 for Finalized, 30s timeout.
    pub fn new(
        rpc_url: impl Into<String>,
        contract_address: impl Into<String>,
        from_address: impl Into<String>,
        chain_name: impl Into<String>,
        chain_id: u64,
    ) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            contract_address: contract_address.into(),
            from_address: from_address.into(),
            chain_name: chain_name.into(),
            chain_id,
            confirmations_for_confirmed: 1,
            confirmations_for_finalized: 12,
            timeout_secs: 30,
        }
    }

    /// Set the finality thresholds.
    pub fn with_finality(mut self, confirmed: u64, finalized: u64) -> Self {
        self.confirmations_for_confirmed = confirmed;
        self.confirmations_for_finalized = finalized;
        self
    }
}

/// EVM JSON-RPC anchor target for production L1 settlement finality.
///
/// Connects to an EVM-compatible chain via JSON-RPC to anchor corridor
/// checkpoint digests on-chain. The RPC endpoint handles transaction
/// signing — this target does not hold private keys.
///
/// ## Contract Interface
///
/// The target contract must expose:
/// ```solidity
/// function recordDigest(bytes32 digest) external;
/// ```
///
/// The 4-byte function selector for `recordDigest(bytes32)` is `0x6b3ee21a`.
#[derive(Debug)]
pub struct EvmAnchorTarget {
    client: reqwest::Client,
    config: EvmAnchorConfig,
}

/// 4-byte function selector for `recordDigest(bytes32)`.
/// keccak256("recordDigest(bytes32)") = 0x6b3ee21a...
const RECORD_DIGEST_SELECTOR: &str = "6b3ee21a";

impl EvmAnchorTarget {
    /// Create a new EVM anchor target from configuration.
    pub fn new(config: EvmAnchorConfig) -> Result<Self, AnchorError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| AnchorError::ChainUnavailable {
                chain_id: format!("{}: failed to build HTTP client: {e}", config.chain_name),
            })?;

        // Validate contract address format.
        if !is_valid_eth_address(&config.contract_address) {
            return Err(AnchorError::Rejected(format!(
                "invalid contract address: {}",
                config.contract_address
            )));
        }

        // Validate sender address format.
        if !is_valid_eth_address(&config.from_address) {
            return Err(AnchorError::Rejected(format!(
                "invalid from address: {}",
                config.from_address
            )));
        }

        Ok(Self { client, config })
    }

    /// Send a JSON-RPC request and return the result field.
    async fn rpc_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, AnchorError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });

        let resp = self
            .client
            .post(&self.config.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AnchorError::ChainUnavailable {
                        chain_id: format!("{}: request timed out", self.config.chain_name),
                    }
                } else {
                    AnchorError::ChainUnavailable {
                        chain_id: format!("{}: {e}", self.config.chain_name),
                    }
                }
            })?;

        if !resp.status().is_success() {
            return Err(AnchorError::ChainUnavailable {
                chain_id: format!(
                    "{}: HTTP {}",
                    self.config.chain_name,
                    resp.status()
                ),
            });
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| {
            AnchorError::ChainUnavailable {
                chain_id: format!("{}: invalid JSON response: {e}", self.config.chain_name),
            }
        })?;

        // Check for JSON-RPC error.
        if let Some(error) = json.get("error") {
            let msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown RPC error");
            return Err(AnchorError::TransactionFailed {
                chain_id: self.config.chain_name.clone(),
                reason: msg.to_string(),
            });
        }

        json.get("result")
            .cloned()
            .ok_or_else(|| AnchorError::ChainUnavailable {
                chain_id: format!(
                    "{}: JSON-RPC response missing 'result' field",
                    self.config.chain_name
                ),
            })
    }

    /// Encode the `recordDigest(bytes32)` calldata from a checkpoint digest.
    fn encode_calldata(digest: &mez_core::ContentDigest) -> String {
        // ABI encoding: 4-byte selector + 32-byte digest (zero-padded on left).
        // ContentDigest is already 32 bytes (SHA-256), so no padding needed.
        format!("0x{RECORD_DIGEST_SELECTOR}{}", digest.to_hex())
    }

    /// Send the anchor transaction via `eth_sendTransaction`.
    async fn send_anchor_tx(
        &self,
        commitment: &AnchorCommitment,
    ) -> Result<String, AnchorError> {
        let data = Self::encode_calldata(&commitment.checkpoint_digest);

        let tx = serde_json::json!({
            "from": self.config.from_address,
            "to": self.config.contract_address,
            "data": data,
        });

        let result = self
            .rpc_call("eth_sendTransaction", serde_json::json!([tx]))
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| AnchorError::TransactionFailed {
                chain_id: self.config.chain_name.clone(),
                reason: "eth_sendTransaction returned non-string result".to_string(),
            })
    }

    /// Get the transaction receipt and current block number to determine status.
    async fn get_tx_status(&self, tx_hash: &str) -> Result<AnchorStatus, AnchorError> {
        // Get transaction receipt.
        let receipt = self
            .rpc_call(
                "eth_getTransactionReceipt",
                serde_json::json!([tx_hash]),
            )
            .await?;

        // Null receipt means transaction is still pending.
        if receipt.is_null() {
            return Ok(AnchorStatus::Pending);
        }

        // Check if transaction succeeded (status 0x1) or failed (status 0x0).
        let status_hex = receipt
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("0x0");
        if status_hex == "0x0" {
            return Ok(AnchorStatus::Failed);
        }

        // Get block number from receipt.
        let tx_block = receipt
            .get("blockNumber")
            .and_then(|b| b.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0);

        // Get current block number.
        let current_block_val = self
            .rpc_call("eth_blockNumber", serde_json::json!([]))
            .await?;
        let current_block = current_block_val
            .as_str()
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0);

        let confirmations = current_block.saturating_sub(tx_block);

        if confirmations >= self.config.confirmations_for_finalized {
            Ok(AnchorStatus::Finalized)
        } else if confirmations >= self.config.confirmations_for_confirmed {
            Ok(AnchorStatus::Confirmed)
        } else {
            Ok(AnchorStatus::Pending)
        }
    }
}

impl AnchorTarget for EvmAnchorTarget {
    fn anchor(&self, commitment: AnchorCommitment) -> Result<AnchorReceipt, AnchorError> {
        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            AnchorError::ChainUnavailable {
                chain_id: format!(
                    "{}: no async runtime available",
                    self.config.chain_name
                ),
            }
        })?;

        rt.block_on(async {
            // Send the anchor transaction.
            let tx_hash = self.send_anchor_tx(&commitment).await?;

            // Get the receipt to find the block number.
            // The transaction may still be pending, so we check status.
            let receipt_result = self
                .rpc_call(
                    "eth_getTransactionReceipt",
                    serde_json::json!([&tx_hash]),
                )
                .await?;

            let (block_number, status) = if receipt_result.is_null() {
                // Transaction submitted but not yet mined.
                (0, AnchorStatus::Pending)
            } else {
                let block = receipt_result
                    .get("blockNumber")
                    .and_then(|b| b.as_str())
                    .and_then(|s| {
                        u64::from_str_radix(s.trim_start_matches("0x"), 16).ok()
                    })
                    .unwrap_or(0);

                let tx_status = receipt_result
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("0x0");

                if tx_status == "0x0" {
                    (block, AnchorStatus::Failed)
                } else {
                    (block, AnchorStatus::Confirmed)
                }
            };

            Ok(AnchorReceipt {
                commitment,
                chain_id: self.config.chain_name.clone(),
                transaction_id: tx_hash,
                block_number,
                status,
            })
        })
    }

    fn check_status(&self, transaction_id: &str) -> Result<AnchorStatus, AnchorError> {
        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            AnchorError::ChainUnavailable {
                chain_id: format!(
                    "{}: no async runtime available",
                    self.config.chain_name
                ),
            }
        })?;

        rt.block_on(self.get_tx_status(transaction_id))
    }

    fn chain_id(&self) -> &str {
        &self.config.chain_name
    }
}

/// Validate that a string is a well-formed Ethereum address (0x + 40 hex chars).
fn is_valid_eth_address(addr: &str) -> bool {
    addr.len() == 42
        && addr.starts_with("0x")
        && addr[2..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_eth_addresses() {
        assert!(is_valid_eth_address(
            "0x0000000000000000000000000000000000000000"
        ));
        assert!(is_valid_eth_address(
            "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
        ));
        assert!(is_valid_eth_address(
            "0xAbCdEf0123456789AbCdEf0123456789AbCdEf01"
        ));
    }

    #[test]
    fn invalid_eth_addresses() {
        assert!(!is_valid_eth_address(""));
        assert!(!is_valid_eth_address("0x"));
        assert!(!is_valid_eth_address("0x123"));
        assert!(!is_valid_eth_address("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef00"));
        assert!(!is_valid_eth_address(
            "0xGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG"
        ));
    }

    #[test]
    fn encode_calldata_format() {
        let canonical =
            mez_core::CanonicalBytes::new(&serde_json::json!({"test": true}))
                .expect("canonical");
        let digest = mez_core::sha256_digest(&canonical);
        let calldata = EvmAnchorTarget::encode_calldata(&digest);

        // 0x + 8 hex (selector) + 64 hex (digest) = 74 chars
        assert_eq!(calldata.len(), 74);
        assert!(calldata.starts_with("0x6b3ee21a"));
    }

    #[test]
    fn config_defaults() {
        let config = EvmAnchorConfig::new(
            "https://eth-mainnet.example.com",
            "0x0000000000000000000000000000000000000001",
            "0x0000000000000000000000000000000000000002",
            "ethereum",
            1,
        );
        assert_eq!(config.confirmations_for_confirmed, 1);
        assert_eq!(config.confirmations_for_finalized, 12);
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.chain_id, 1);
    }

    #[test]
    fn config_with_finality() {
        let config = EvmAnchorConfig::new(
            "https://arb.example.com",
            "0x0000000000000000000000000000000000000001",
            "0x0000000000000000000000000000000000000002",
            "arbitrum",
            42161,
        )
        .with_finality(1, 64);

        assert_eq!(config.confirmations_for_confirmed, 1);
        assert_eq!(config.confirmations_for_finalized, 64);
    }

    #[test]
    fn evm_anchor_rejects_invalid_contract_address() {
        let config = EvmAnchorConfig::new(
            "https://rpc.example.com",
            "not-an-address",
            "0x0000000000000000000000000000000000000002",
            "ethereum",
            1,
        );
        let result = EvmAnchorTarget::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn evm_anchor_rejects_invalid_from_address() {
        let config = EvmAnchorConfig::new(
            "https://rpc.example.com",
            "0x0000000000000000000000000000000000000001",
            "bad-addr",
            "ethereum",
            1,
        );
        let result = EvmAnchorTarget::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn evm_anchor_builds_with_valid_config() {
        let config = EvmAnchorConfig::new(
            "https://rpc.example.com",
            "0x0000000000000000000000000000000000000001",
            "0x0000000000000000000000000000000000000002",
            "ethereum",
            1,
        );
        let target = EvmAnchorTarget::new(config).expect("should build");
        assert_eq!(target.chain_id(), "ethereum");
    }

    #[test]
    fn evm_anchor_is_debug() {
        let config = EvmAnchorConfig::new(
            "https://rpc.example.com",
            "0x0000000000000000000000000000000000000001",
            "0x0000000000000000000000000000000000000002",
            "test",
            1,
        );
        let target = EvmAnchorTarget::new(config).expect("should build");
        let debug = format!("{target:?}");
        assert!(debug.contains("EvmAnchorTarget"));
    }
}
