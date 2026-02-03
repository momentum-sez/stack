"""
PHOENIX L1 Anchor Layer

Settlement finality through Ethereum and L2 checkpointing. This module provides
the infrastructure for anchoring MSEZ corridor checkpoints to L1 blockchains,
creating immutable proof of state that can be verified by external parties.

Supported Chains:
    - Ethereum Mainnet: Maximum security, highest cost
    - Arbitrum One: Lower cost, fast finality, inherits Ethereum security
    - Base: Coinbase ecosystem integration
    - Polygon PoS: Low cost, emerging markets

Architecture:
    
    ┌─────────────────────────────────────────────────────────────────────┐
    │                    MSEZ CORRIDOR STATE                               │
    │  Checkpoints, Receipts, Watcher Attestations                        │
    └──────────────────────────────┬──────────────────────────────────────┘
                                   │
                                   ▼
    ┌─────────────────────────────────────────────────────────────────────┐
    │                    ANCHOR MANAGER                                    │
    │  Batching, Gas Optimization, Chain Selection                         │
    └──────────────────────────────┬──────────────────────────────────────┘
                                   │
          ┌────────────────────────┼────────────────────────┐
          ▼                        ▼                        ▼
    ┌───────────┐            ┌───────────┐            ┌───────────┐
    │  Ethereum │            │  Arbitrum │            │   Base    │
    │  Adapter  │            │  Adapter  │            │  Adapter  │
    └───────────┘            └───────────┘            └───────────┘

Copyright (c) 2026 Momentum. All rights reserved.
Contact: engineering@momentum.inc
"""

from __future__ import annotations

import hashlib
import json
import secrets
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from enum import Enum
from typing import Any, Dict, List, Optional, Protocol, Tuple


# =============================================================================
# CHAIN DEFINITIONS
# =============================================================================

class Chain(Enum):
    """Supported blockchain networks for anchoring."""
    ETHEREUM = "ethereum"
    ARBITRUM = "arbitrum"
    BASE = "base"
    POLYGON = "polygon"
    
    @property
    def chain_id(self) -> int:
        """Return the chain ID."""
        return {
            Chain.ETHEREUM: 1,
            Chain.ARBITRUM: 42161,
            Chain.BASE: 8453,
            Chain.POLYGON: 137,
        }[self]
    
    @property
    def is_l2(self) -> bool:
        """Check if this is an L2 chain."""
        return self in {Chain.ARBITRUM, Chain.BASE, Chain.POLYGON}
    
    @property
    def finality_blocks(self) -> int:
        """Number of blocks for finality."""
        return {
            Chain.ETHEREUM: 64,  # ~13 minutes
            Chain.ARBITRUM: 1,   # Instant with L1 finality
            Chain.BASE: 1,       # Instant with L1 finality
            Chain.POLYGON: 256,  # ~9 minutes
        }[self]
    
    @property
    def average_block_time_seconds(self) -> float:
        """Average time between blocks."""
        return {
            Chain.ETHEREUM: 12.0,
            Chain.ARBITRUM: 0.25,
            Chain.BASE: 2.0,
            Chain.POLYGON: 2.0,
        }[self]


class AnchorStatus(Enum):
    """Status of an anchor transaction."""
    PENDING = "pending"          # Transaction submitted
    CONFIRMED = "confirmed"      # Transaction confirmed but not final
    FINALIZED = "finalized"      # Transaction final
    FAILED = "failed"            # Transaction failed
    CHALLENGED = "challenged"    # Anchor challenged during dispute period


# =============================================================================
# CHECKPOINT DATA
# =============================================================================

@dataclass
class CorridorCheckpoint:
    """
    A checkpoint of corridor state to be anchored.
    
    Checkpoints capture the state of a corridor at a specific height,
    including the Merkle root of all receipts.
    """
    corridor_id: str
    checkpoint_height: int
    receipt_merkle_root: str  # 32-byte hex
    state_root: str  # 32-byte hex
    timestamp: str
    watcher_signatures: List[bytes]
    
    # Metadata
    receipt_count: int = 0
    previous_checkpoint_digest: Optional[str] = None
    
    @property
    def digest(self) -> str:
        """Canonical digest of the checkpoint."""
        content = {
            "corridor_id": self.corridor_id,
            "checkpoint_height": self.checkpoint_height,
            "receipt_merkle_root": self.receipt_merkle_root,
            "state_root": self.state_root,
            "timestamp": self.timestamp,
            "previous_checkpoint_digest": self.previous_checkpoint_digest,
        }
        canonical = json.dumps(content, sort_keys=True, separators=(",", ":"))
        return hashlib.sha256(canonical.encode()).hexdigest()
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "corridor_id": self.corridor_id,
            "checkpoint_height": self.checkpoint_height,
            "receipt_merkle_root": self.receipt_merkle_root,
            "state_root": self.state_root,
            "timestamp": self.timestamp,
            "watcher_signature_count": len(self.watcher_signatures),
            "receipt_count": self.receipt_count,
            "previous_checkpoint_digest": self.previous_checkpoint_digest,
            "digest": self.digest,
        }


# =============================================================================
# ANCHOR RECORD
# =============================================================================

@dataclass
class AnchorRecord:
    """
    Record of a checkpoint anchored to L1.
    
    The anchor record provides proof that a checkpoint was committed
    to a specific blockchain at a specific block.
    """
    anchor_id: str
    checkpoint: CorridorCheckpoint
    chain: Chain
    
    # Transaction details
    tx_hash: str
    block_number: int
    block_hash: str
    
    # Contract details
    contract_address: str
    log_index: int
    
    # Status
    status: AnchorStatus = AnchorStatus.PENDING
    confirmations: int = 0
    
    # Timing
    submitted_at: str = ""
    confirmed_at: Optional[str] = None
    finalized_at: Optional[str] = None
    
    # Cost
    gas_used: int = 0
    gas_price_gwei: Decimal = Decimal("0")
    cost_eth: Decimal = Decimal("0")
    
    def __post_init__(self):
        if not self.submitted_at:
            self.submitted_at = datetime.now(timezone.utc).isoformat()
    
    @property
    def is_final(self) -> bool:
        """Check if anchor is finalized."""
        return self.status == AnchorStatus.FINALIZED
    
    @property
    def explorer_url(self) -> str:
        """Get block explorer URL for the transaction."""
        base_urls = {
            Chain.ETHEREUM: "https://etherscan.io/tx/",
            Chain.ARBITRUM: "https://arbiscan.io/tx/",
            Chain.BASE: "https://basescan.org/tx/",
            Chain.POLYGON: "https://polygonscan.com/tx/",
        }
        return base_urls.get(self.chain, "") + self.tx_hash
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "anchor_id": self.anchor_id,
            "checkpoint_digest": self.checkpoint.digest,
            "chain": self.chain.value,
            "chain_id": self.chain.chain_id,
            "tx_hash": self.tx_hash,
            "block_number": self.block_number,
            "block_hash": self.block_hash,
            "contract_address": self.contract_address,
            "status": self.status.value,
            "confirmations": self.confirmations,
            "is_final": self.is_final,
            "submitted_at": self.submitted_at,
            "confirmed_at": self.confirmed_at,
            "finalized_at": self.finalized_at,
            "gas_used": self.gas_used,
            "cost_eth": str(self.cost_eth),
            "explorer_url": self.explorer_url,
        }


# =============================================================================
# INCLUSION PROOF
# =============================================================================

@dataclass
class InclusionProof:
    """
    Proof that a specific receipt is included in an anchored checkpoint.
    
    The proof consists of the Merkle path from the receipt to the root
    that was anchored on-chain.
    """
    receipt_digest: str
    checkpoint_digest: str
    anchor_id: str
    
    # Merkle proof
    merkle_path: List[str]  # Sibling hashes
    merkle_indices: List[int]  # 0 = left, 1 = right
    
    # Verification data
    root: str
    leaf_index: int
    
    def verify(self) -> bool:
        """
        Verify the inclusion proof.
        
        Recomputes the Merkle root from the receipt and path,
        then checks it matches the anchored root.
        """
        current = self.receipt_digest
        
        for sibling, index in zip(self.merkle_path, self.merkle_indices):
            if index == 0:  # Current is left child
                combined = current + sibling
            else:  # Current is right child
                combined = sibling + current
            current = hashlib.sha256(combined.encode()).hexdigest()
        
        return current == self.root
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "receipt_digest": self.receipt_digest,
            "checkpoint_digest": self.checkpoint_digest,
            "anchor_id": self.anchor_id,
            "merkle_path": self.merkle_path,
            "root": self.root,
            "leaf_index": self.leaf_index,
            "is_valid": self.verify(),
        }


# =============================================================================
# CHAIN ADAPTER INTERFACE
# =============================================================================

class ChainAdapter(Protocol):
    """
    Protocol for blockchain-specific adapters.
    
    Each supported chain implements this interface to provide
    chain-specific transaction submission and verification.
    """
    
    @property
    def chain(self) -> Chain:
        """The chain this adapter handles."""
        ...
    
    def submit_checkpoint(
        self,
        checkpoint: CorridorCheckpoint,
        contract_address: str,
    ) -> str:
        """
        Submit a checkpoint to the chain.
        
        Returns the transaction hash.
        """
        ...
    
    def get_transaction_status(
        self,
        tx_hash: str,
    ) -> Tuple[AnchorStatus, int]:
        """
        Get the status and confirmation count of a transaction.
        
        Returns (status, confirmations).
        """
        ...
    
    def verify_inclusion(
        self,
        checkpoint_digest: str,
        contract_address: str,
        block_number: int,
    ) -> bool:
        """
        Verify that a checkpoint was anchored at the given block.
        """
        ...
    
    def estimate_gas(
        self,
        checkpoint: CorridorCheckpoint,
        contract_address: str,
    ) -> int:
        """
        Estimate gas for anchoring a checkpoint.
        """
        ...
    
    def get_current_gas_price(self) -> Decimal:
        """
        Get current gas price in Gwei.
        """
        ...


# =============================================================================
# MOCK CHAIN ADAPTER
# =============================================================================

class MockChainAdapter:
    """
    Mock chain adapter for testing.
    
    Simulates blockchain interactions without actual network calls.
    """
    
    def __init__(self, chain: Chain):
        self._chain = chain
        self._submitted: Dict[str, CorridorCheckpoint] = {}
        self._block_number = 1000000
        self._gas_price = Decimal("20")  # Gwei
    
    @property
    def chain(self) -> Chain:
        return self._chain
    
    def submit_checkpoint(
        self,
        checkpoint: CorridorCheckpoint,
        contract_address: str,
    ) -> str:
        """Submit checkpoint and return mock tx hash."""
        tx_hash = "0x" + secrets.token_hex(32)
        self._submitted[tx_hash] = checkpoint
        self._block_number += 1
        return tx_hash
    
    def get_transaction_status(
        self,
        tx_hash: str,
    ) -> Tuple[AnchorStatus, int]:
        """Get mock transaction status."""
        if tx_hash not in self._submitted:
            return AnchorStatus.FAILED, 0
        
        # Simulate confirmations based on chain
        confirmations = self._chain.finality_blocks + 10
        if confirmations >= self._chain.finality_blocks:
            return AnchorStatus.FINALIZED, confirmations
        return AnchorStatus.CONFIRMED, confirmations
    
    def verify_inclusion(
        self,
        checkpoint_digest: str,
        contract_address: str,
        block_number: int,
    ) -> bool:
        """Verify checkpoint inclusion (mock always returns True)."""
        return True
    
    def estimate_gas(
        self,
        checkpoint: CorridorCheckpoint,
        contract_address: str,
    ) -> int:
        """Estimate gas (mock returns fixed value)."""
        base_gas = 50000
        sig_gas = len(checkpoint.watcher_signatures) * 5000
        return base_gas + sig_gas
    
    def get_current_gas_price(self) -> Decimal:
        """Get current gas price."""
        return self._gas_price
    
    def get_block_number(self) -> int:
        """Get current block number."""
        return self._block_number


# =============================================================================
# ANCHOR MANAGER
# =============================================================================

class AnchorManager:
    """
    Manager for L1 anchoring operations.
    
    The manager handles:
    - Chain selection based on cost/security tradeoffs
    - Batching multiple checkpoints
    - Retry logic for failed transactions
    - Cross-chain verification
    """
    
    def __init__(
        self,
        adapters: Optional[Dict[Chain, ChainAdapter]] = None,
        default_chain: Chain = Chain.ARBITRUM,
    ):
        self._adapters: Dict[Chain, ChainAdapter] = adapters or {}
        self._default_chain = default_chain
        self._anchors: Dict[str, AnchorRecord] = {}
        self._checkpoint_anchors: Dict[str, str] = {}  # checkpoint_digest -> anchor_id
        
        # Default contract addresses (would be configured per deployment)
        self._contracts: Dict[Chain, str] = {
            Chain.ETHEREUM: "0x" + "00" * 19 + "01",
            Chain.ARBITRUM: "0x" + "00" * 19 + "02",
            Chain.BASE: "0x" + "00" * 19 + "03",
            Chain.POLYGON: "0x" + "00" * 19 + "04",
        }
    
    def add_adapter(self, adapter: ChainAdapter) -> None:
        """Add a chain adapter."""
        self._adapters[adapter.chain] = adapter
    
    def set_contract(self, chain: Chain, address: str) -> None:
        """Set contract address for a chain."""
        self._contracts[chain] = address
    
    def anchor_checkpoint(
        self,
        checkpoint: CorridorCheckpoint,
        chain: Optional[Chain] = None,
    ) -> AnchorRecord:
        """
        Anchor a checkpoint to a blockchain.
        
        Args:
            checkpoint: The checkpoint to anchor
            chain: Optional chain override (defaults to default_chain)
            
        Returns:
            AnchorRecord with transaction details
        """
        chain = chain or self._default_chain
        
        if chain not in self._adapters:
            raise ValueError(f"No adapter configured for {chain.value}")
        
        adapter = self._adapters[chain]
        contract = self._contracts.get(chain, "")
        
        # Submit to chain
        tx_hash = adapter.submit_checkpoint(checkpoint, contract)
        
        # Get initial status
        status, confirmations = adapter.get_transaction_status(tx_hash)
        
        # Create anchor record
        anchor_id = f"anchor-{secrets.token_hex(8)}"
        
        # Get block info (mock for now)
        if isinstance(adapter, MockChainAdapter):
            block_number = adapter.get_block_number()
            block_hash = "0x" + hashlib.sha256(str(block_number).encode()).hexdigest()
        else:
            block_number = 0
            block_hash = "0x" + "00" * 32
        
        anchor = AnchorRecord(
            anchor_id=anchor_id,
            checkpoint=checkpoint,
            chain=chain,
            tx_hash=tx_hash,
            block_number=block_number,
            block_hash=block_hash,
            contract_address=contract,
            log_index=0,
            status=status,
            confirmations=confirmations,
            gas_used=adapter.estimate_gas(checkpoint, contract),
            gas_price_gwei=adapter.get_current_gas_price(),
        )
        
        # Calculate cost
        anchor.cost_eth = (
            Decimal(anchor.gas_used) *
            anchor.gas_price_gwei /
            Decimal("1000000000")
        )
        
        # Update status
        if status == AnchorStatus.FINALIZED:
            anchor.finalized_at = datetime.now(timezone.utc).isoformat()
        elif status == AnchorStatus.CONFIRMED:
            anchor.confirmed_at = datetime.now(timezone.utc).isoformat()
        
        # Store
        self._anchors[anchor_id] = anchor
        self._checkpoint_anchors[checkpoint.digest] = anchor_id
        
        return anchor
    
    def get_anchor(self, anchor_id: str) -> Optional[AnchorRecord]:
        """Get anchor record by ID."""
        return self._anchors.get(anchor_id)
    
    def get_anchor_for_checkpoint(
        self,
        checkpoint_digest: str,
    ) -> Optional[AnchorRecord]:
        """Get anchor record for a checkpoint."""
        anchor_id = self._checkpoint_anchors.get(checkpoint_digest)
        if anchor_id:
            return self._anchors.get(anchor_id)
        return None
    
    def refresh_anchor_status(self, anchor_id: str) -> Optional[AnchorRecord]:
        """Refresh the status of an anchor."""
        anchor = self._anchors.get(anchor_id)
        if not anchor:
            return None
        
        adapter = self._adapters.get(anchor.chain)
        if not adapter:
            return anchor
        
        status, confirmations = adapter.get_transaction_status(anchor.tx_hash)
        
        anchor.status = status
        anchor.confirmations = confirmations
        
        if status == AnchorStatus.CONFIRMED and not anchor.confirmed_at:
            anchor.confirmed_at = datetime.now(timezone.utc).isoformat()
        elif status == AnchorStatus.FINALIZED and not anchor.finalized_at:
            anchor.finalized_at = datetime.now(timezone.utc).isoformat()
        
        return anchor
    
    def verify_checkpoint_inclusion(
        self,
        checkpoint: CorridorCheckpoint,
        chain: Optional[Chain] = None,
    ) -> bool:
        """
        Verify that a checkpoint is anchored on-chain.
        """
        anchor = self.get_anchor_for_checkpoint(checkpoint.digest)
        if not anchor:
            return False
        
        chain = chain or anchor.chain
        adapter = self._adapters.get(chain)
        if not adapter:
            return False
        
        return adapter.verify_inclusion(
            checkpoint.digest,
            anchor.contract_address,
            anchor.block_number,
        )
    
    def generate_inclusion_proof(
        self,
        receipt_digest: str,
        checkpoint: CorridorCheckpoint,
        merkle_path: List[str],
        merkle_indices: List[int],
        leaf_index: int,
    ) -> Optional[InclusionProof]:
        """
        Generate an inclusion proof for a receipt.
        
        Args:
            receipt_digest: Digest of the receipt to prove
            checkpoint: The checkpoint containing the receipt
            merkle_path: Merkle path siblings
            merkle_indices: Path direction indicators
            leaf_index: Index of receipt in the tree
            
        Returns:
            InclusionProof if checkpoint is anchored, None otherwise
        """
        anchor = self.get_anchor_for_checkpoint(checkpoint.digest)
        if not anchor:
            return None
        
        proof = InclusionProof(
            receipt_digest=receipt_digest,
            checkpoint_digest=checkpoint.digest,
            anchor_id=anchor.anchor_id,
            merkle_path=merkle_path,
            merkle_indices=merkle_indices,
            root=checkpoint.receipt_merkle_root,
            leaf_index=leaf_index,
        )
        
        return proof
    
    def estimate_anchor_cost(
        self,
        checkpoint: CorridorCheckpoint,
        chain: Optional[Chain] = None,
    ) -> Dict[str, Any]:
        """
        Estimate the cost of anchoring a checkpoint.
        """
        chain = chain or self._default_chain
        adapter = self._adapters.get(chain)
        
        if not adapter:
            return {"error": f"No adapter for {chain.value}"}
        
        contract = self._contracts.get(chain, "")
        gas = adapter.estimate_gas(checkpoint, contract)
        gas_price = adapter.get_current_gas_price()
        cost_eth = Decimal(gas) * gas_price / Decimal("1000000000")
        
        return {
            "chain": chain.value,
            "gas_estimate": gas,
            "gas_price_gwei": str(gas_price),
            "cost_eth": str(cost_eth),
            "finality_blocks": chain.finality_blocks,
            "finality_time_seconds": chain.finality_blocks * chain.average_block_time_seconds,
        }
    
    def compare_chain_costs(
        self,
        checkpoint: CorridorCheckpoint,
    ) -> List[Dict[str, Any]]:
        """
        Compare anchoring costs across all configured chains.
        """
        results = []
        for chain in self._adapters.keys():
            estimate = self.estimate_anchor_cost(checkpoint, chain)
            if "error" not in estimate:
                results.append(estimate)
        
        # Sort by cost
        results.sort(key=lambda x: Decimal(x["cost_eth"]))
        return results
    
    def list_anchors(
        self,
        corridor_id: Optional[str] = None,
        chain: Optional[Chain] = None,
        status: Optional[AnchorStatus] = None,
    ) -> List[AnchorRecord]:
        """List anchors with optional filters."""
        anchors = list(self._anchors.values())
        
        if corridor_id:
            anchors = [a for a in anchors if a.checkpoint.corridor_id == corridor_id]
        if chain:
            anchors = [a for a in anchors if a.chain == chain]
        if status:
            anchors = [a for a in anchors if a.status == status]
        
        # Sort by block number descending
        anchors.sort(key=lambda a: a.block_number, reverse=True)
        return anchors
    
    def get_statistics(self) -> Dict[str, Any]:
        """Get anchoring statistics."""
        by_chain: Dict[str, int] = {}
        by_status: Dict[str, int] = {}
        total_cost = Decimal("0")
        
        for anchor in self._anchors.values():
            chain = anchor.chain.value
            by_chain[chain] = by_chain.get(chain, 0) + 1
            
            status = anchor.status.value
            by_status[status] = by_status.get(status, 0) + 1
            
            total_cost += anchor.cost_eth
        
        return {
            "total_anchors": len(self._anchors),
            "by_chain": by_chain,
            "by_status": by_status,
            "total_cost_eth": str(total_cost),
            "configured_chains": [c.value for c in self._adapters.keys()],
        }


# =============================================================================
# CROSS-CHAIN VERIFICATION
# =============================================================================

@dataclass
class CrossChainVerification:
    """
    Verification result across multiple chains.
    
    For critical checkpoints, anchoring to multiple chains
    provides defense in depth against chain-specific issues.
    """
    checkpoint_digest: str
    verifications: Dict[str, bool]  # chain -> verified
    
    @property
    def all_verified(self) -> bool:
        return all(self.verifications.values())
    
    @property
    def any_verified(self) -> bool:
        return any(self.verifications.values())
    
    @property
    def verification_count(self) -> int:
        return sum(1 for v in self.verifications.values() if v)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "checkpoint_digest": self.checkpoint_digest,
            "verifications": self.verifications,
            "all_verified": self.all_verified,
            "any_verified": self.any_verified,
            "verification_count": self.verification_count,
        }


class CrossChainVerifier:
    """
    Verifies checkpoints across multiple chains.
    """
    
    def __init__(self, anchor_manager: AnchorManager):
        self._manager = anchor_manager
    
    def verify_across_chains(
        self,
        checkpoint: CorridorCheckpoint,
        chains: Optional[List[Chain]] = None,
    ) -> CrossChainVerification:
        """
        Verify a checkpoint across multiple chains.
        """
        chains = chains or list(self._manager._adapters.keys())
        
        verifications: Dict[str, bool] = {}
        for chain in chains:
            verified = self._manager.verify_checkpoint_inclusion(checkpoint, chain)
            verifications[chain.value] = verified
        
        return CrossChainVerification(
            checkpoint_digest=checkpoint.digest,
            verifications=verifications,
        )


# =============================================================================
# FACTORY FUNCTIONS
# =============================================================================

def create_mock_anchor_manager() -> AnchorManager:
    """Create an anchor manager with mock adapters for all chains."""
    manager = AnchorManager()
    
    for chain in Chain:
        adapter = MockChainAdapter(chain)
        manager.add_adapter(adapter)
    
    return manager
