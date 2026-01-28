"""
MASS Protocol Primitives Test Suite
====================================

Tests for the formal MASS Protocol implementation, validating:
- Canonical Digest Bridge (Theorem 3.2)
- Smart Asset tuple structure (Definition 11.1)
- Receipt Chain integrity (Lemma 12.1)
- Design Invariants I1-I5 (Definition 11.2)
- State Transition Determinism (Theorem 13.1)
- Agentic Determinism (Theorem 17.1)
- Offline Operation Capability (Theorem 16.1)
- Compliance Tensor (Definition 7.3)
- MMR operations (Theorem 12.2)

Reference: MASS Protocol Technical Specification v0.2
"""

import pytest
import json
import hashlib
from datetime import datetime, timezone, timedelta
from decimal import Decimal

# Import all primitives
from tools.mass_primitives import (
    # Digest and Canonicalization
    DigestAlgorithm,
    Digest,
    json_canonicalize,
    stack_digest,
    semantic_digest,
    
    # Artifacts
    ArtifactType,
    ArtifactRef,
    compute_artifact_closure,
    extract_artifact_refs,
    
    # Smart Asset Components
    GenesisDocument,
    RegistryCredential,
    JurisdictionalBinding,
    OperationalManifest,
    
    # Receipt Chain
    AssetReceipt,
    genesis_receipt_root,
    MerkleMountainRange,
    AssetCheckpoint,
    
    # State Machine
    TransitionKind,
    TransitionEnvelope,
    
    # Compliance
    ComplianceStatus,
    ComplianceConstraint,
    ComplianceTensor,
    
    # Agentic Execution
    AgenticTriggerType,
    AgenticTrigger,
    AgenticPolicy,
    STANDARD_POLICIES,
    
    # Smart Asset
    SmartAsset,
    
    # Theorems
    verify_offline_capability,
)


# =============================================================================
# CHAPTER 3: CANONICAL DIGEST BRIDGE TESTS
# =============================================================================

class TestJSONCanonicalization:
    """Test JCS (RFC 8785) compliance."""
    
    def test_key_ordering(self):
        """Keys must be sorted lexicographically."""
        obj = {"z": 1, "a": 2, "m": 3}
        canonical = json_canonicalize(obj)
        assert canonical == '{"a":2,"m":3,"z":1}'
    
    def test_nested_key_ordering(self):
        """Nested objects must also have sorted keys."""
        obj = {"b": {"y": 1, "x": 2}, "a": 3}
        canonical = json_canonicalize(obj)
        assert canonical == '{"a":3,"b":{"x":2,"y":1}}'
    
    def test_number_normalization(self):
        """Numbers should be normalized."""
        obj = {"val": 1.0}
        canonical = json_canonicalize(obj)
        # 1.0 should become 1
        assert canonical == '{"val":1}'
    
    def test_decimal_handling(self):
        """Decimal values should be handled correctly."""
        obj = {"amount": Decimal("100.50")}
        canonical = json_canonicalize(obj)
        assert '100.5' in canonical
    
    def test_whitespace_removal(self):
        """No insignificant whitespace."""
        obj = {"key": "value"}
        canonical = json_canonicalize(obj)
        assert ' ' not in canonical
        assert '\n' not in canonical
    
    def test_deterministic_output(self):
        """Same input always produces same output."""
        obj = {"a": [1, 2, 3], "b": {"c": True}}
        results = [json_canonicalize(obj) for _ in range(100)]
        assert len(set(results)) == 1


class TestDigestBridge:
    """
    Theorem 3.2 (Digest Bridge Security).
    
    The Canonical Digest Bridge preserves collision resistance.
    """
    
    def test_stack_digest_deterministic(self):
        """Same artifact always produces same digest."""
        artifact = {"type": "test", "value": 42}
        d1 = stack_digest(artifact)
        d2 = stack_digest(artifact)
        assert d1.bytes_hex == d2.bytes_hex
    
    def test_different_artifacts_different_digests(self):
        """Different artifacts produce different digests."""
        a1 = {"type": "test", "value": 1}
        a2 = {"type": "test", "value": 2}
        d1 = stack_digest(a1)
        d2 = stack_digest(a2)
        assert d1.bytes_hex != d2.bytes_hex
    
    def test_digest_length(self):
        """SHA256 digest is 64 hex characters."""
        artifact = {"data": "test"}
        d = stack_digest(artifact)
        assert len(d.bytes_hex) == 64
    
    def test_digest_lowercase(self):
        """Digest must be lowercase hex."""
        artifact = {"data": "test"}
        d = stack_digest(artifact)
        assert d.bytes_hex == d.bytes_hex.lower()
    
    def test_semantic_digest_vc_removes_proof(self):
        """VCs should be digested without proof field."""
        vc = {
            "@context": ["https://www.w3.org/2018/credentials/v1"],
            "type": ["VerifiableCredential"],
            "issuer": "did:key:z6Mk...",
            "credentialSubject": {"id": "test"},
            "proof": {"type": "Ed25519Signature2020", "proofValue": "z..."}
        }
        d = semantic_digest("vc", vc)
        
        # Verify proof was excluded
        vc_no_proof = {k: v for k, v in vc.items() if k != "proof"}
        expected = stack_digest(vc_no_proof)
        assert d.bytes_hex == expected.bytes_hex


# =============================================================================
# CHAPTER 4: ARTIFACT MODEL TESTS
# =============================================================================

class TestArtifactRef:
    """Test ArtifactRef structure."""
    
    def test_creation(self):
        """Valid ArtifactRef can be created."""
        ref = ArtifactRef(
            artifact_type=ArtifactType.VC,
            digest_sha256="a" * 64
        )
        assert ref.digest_sha256 == "a" * 64
    
    def test_lowercase_normalization(self):
        """Digest is normalized to lowercase."""
        ref = ArtifactRef(
            artifact_type=ArtifactType.VC,
            digest_sha256="A" * 64
        )
        assert ref.digest_sha256 == "a" * 64
    
    def test_invalid_digest_length_raises(self):
        """Invalid digest length raises ValueError."""
        with pytest.raises(ValueError):
            ArtifactRef(
                artifact_type=ArtifactType.VC,
                digest_sha256="abc"  # Too short
            )
    
    def test_to_dict_roundtrip(self):
        """to_dict and from_dict are inverses."""
        ref = ArtifactRef(
            artifact_type=ArtifactType.LAWPACK,
            digest_sha256="b" * 64,
            uri="ipfs://Qm...",
            media_type="application/zip"
        )
        d = ref.to_dict()
        ref2 = ArtifactRef.from_dict(d)
        assert ref.artifact_type == ref2.artifact_type
        assert ref.digest_sha256 == ref2.digest_sha256
        assert ref.uri == ref2.uri


class TestArtifactClosure:
    """Test artifact closure computation."""
    
    def test_extract_artifact_refs(self):
        """Extract all ArtifactRefs from nested structure."""
        artifact = {
            "top_ref": {
                "artifact_type": "vc",
                "digest_sha256": "a" * 64
            },
            "nested": {
                "another_ref": {
                    "artifact_type": "lawpack",
                    "digest_sha256": "b" * 64
                }
            },
            "list_refs": [
                {"artifact_type": "schema", "digest_sha256": "c" * 64}
            ]
        }
        
        refs = extract_artifact_refs(artifact)
        assert len(refs) == 3
        digests = {r.digest_sha256 for r in refs}
        assert "a" * 64 in digests
        assert "b" * 64 in digests
        assert "c" * 64 in digests


# =============================================================================
# CHAPTER 11: SMART ASSET TESTS
# =============================================================================

class TestGenesisDocument:
    """Test Genesis Document (G component)."""
    
    def test_asset_id_immutable(self):
        """
        I1 (Immutable Identity):
        asset_id = SHA256(JCS(G)) never changes.
        """
        genesis = GenesisDocument(
            asset_name="Test Asset",
            asset_class="equity",
            initial_bindings=["harbor:uae-adgm"],
            governance={"type": "single_owner", "owner": "did:key:z6Mk..."},
            created_at="2026-01-01T00:00:00Z"
        )
        
        # asset_id should be deterministic
        id1 = genesis.asset_id
        id2 = genesis.asset_id
        assert id1 == id2
        
        # And match SHA256(JCS(G))
        expected = stack_digest(genesis.to_dict()).bytes_hex
        assert genesis.asset_id == expected
    
    def test_different_genesis_different_id(self):
        """Different genesis documents have different IDs."""
        g1 = GenesisDocument(
            asset_name="Asset 1",
            asset_class="equity",
            initial_bindings=["harbor:uae-adgm"],
            governance={},
            created_at="2026-01-01T00:00:00Z"
        )
        g2 = GenesisDocument(
            asset_name="Asset 2",
            asset_class="equity",
            initial_bindings=["harbor:uae-adgm"],
            governance={},
            created_at="2026-01-01T00:00:00Z"
        )
        assert g1.asset_id != g2.asset_id


class TestJurisdictionalBinding:
    """Test jurisdictional bindings (I3 invariant)."""
    
    def test_binding_creation(self):
        """Binding can be created with required fields."""
        binding = JurisdictionalBinding(
            harbor_id="harbor:uae-adgm",
            lawpack_digest="d" * 64
        )
        assert binding.binding_status == "active"
    
    def test_binding_to_dict(self):
        """Binding serializes correctly."""
        binding = JurisdictionalBinding(
            harbor_id="harbor:kaz-aifc",
            lawpack_digest="e" * 64,
            regpack_digest="f" * 64
        )
        d = binding.to_dict()
        assert d["harbor_id"] == "harbor:kaz-aifc"
        assert d["lawpack_digest"] == "e" * 64


# =============================================================================
# CHAPTER 12: RECEIPT CHAIN TESTS
# =============================================================================

class TestReceiptChain:
    """
    Lemma 12.1 (Receipt Chain Integrity).
    
    Under SHA-256 collision resistance, tampering with any receipt
    in a chain is detectable.
    """
    
    def test_genesis_receipt_root(self):
        """Genesis root is deterministic."""
        asset_id = "a" * 64
        root1 = genesis_receipt_root(asset_id)
        root2 = genesis_receipt_root(asset_id)
        assert root1 == root2
        
        # Different asset_id produces different root
        root3 = genesis_receipt_root("b" * 64)
        assert root1 != root3
    
    def test_receipt_next_root_computed(self):
        """Receipt next_root is computed on creation."""
        asset_id = "a" * 64
        receipt = AssetReceipt(
            asset_id=asset_id,
            seq=0,
            prev_root=genesis_receipt_root(asset_id),
            transition_envelope_digest="b" * 64,
            transition_kind="mint",
            prev_state_root="c" * 64,
            new_state_root="d" * 64,
            harbor_ids=["harbor:test"],
            jurisdiction_scope="single",
            signatures=[]
        )
        
        assert receipt.next_root is not None
        assert len(receipt.next_root) == 64
    
    def test_receipt_chain_linkage(self):
        """Receipts chain correctly via prev_root/next_root."""
        asset_id = "a" * 64
        
        r0 = AssetReceipt(
            asset_id=asset_id,
            seq=0,
            prev_root=genesis_receipt_root(asset_id),
            transition_envelope_digest="b" * 64,
            transition_kind="mint",
            prev_state_root="0" * 64,
            new_state_root="c" * 64,
            harbor_ids=["harbor:test"],
            jurisdiction_scope="single",
            signatures=[]
        )
        
        r1 = AssetReceipt(
            asset_id=asset_id,
            seq=1,
            prev_root=r0.next_root,  # Links to previous
            transition_envelope_digest="d" * 64,
            transition_kind="transfer",
            prev_state_root=r0.new_state_root,
            new_state_root="e" * 64,
            harbor_ids=["harbor:test"],
            jurisdiction_scope="single",
            signatures=[]
        )
        
        assert r1.prev_root == r0.next_root


class TestMerkleMountainRange:
    """
    Theorem 12.2 (MMR Efficiency).
    
    For n receipts:
    1. Append requires at most ⌊log_2 n⌋ + 1 hash operations
    2. Inclusion proof size is at most ⌊log_2 n⌋ + 1 hashes
    3. Root can be computed from peaks in O(log n) time
    """
    
    def test_mmr_empty_root(self):
        """Empty MMR has zero root."""
        mmr = MerkleMountainRange()
        assert mmr.root == "0" * 64
    
    def test_mmr_single_element(self):
        """Single element MMR has element as root."""
        mmr = MerkleMountainRange()
        leaf = "a" * 64
        mmr.append(leaf)
        # Single leaf is its own root
        assert mmr.root == leaf
    
    def test_mmr_multiple_elements(self):
        """MMR with multiple elements computes correct root."""
        mmr = MerkleMountainRange()
        mmr.append("a" * 64)
        mmr.append("b" * 64)
        
        # After 2 elements, they merge into single peak
        assert mmr.leaf_count == 2
        assert len([p for p in mmr.peaks if p is not None]) == 1
    
    def test_mmr_deterministic(self):
        """Same sequence produces same root."""
        leaves = ["a" * 64, "b" * 64, "c" * 64, "d" * 64]
        
        mmr1 = MerkleMountainRange()
        for leaf in leaves:
            mmr1.append(leaf)
        
        mmr2 = MerkleMountainRange()
        for leaf in leaves:
            mmr2.append(leaf)
        
        assert mmr1.root == mmr2.root
    
    def test_mmr_different_sequences(self):
        """Different sequences produce different roots."""
        mmr1 = MerkleMountainRange()
        mmr1.append("a" * 64)
        mmr1.append("b" * 64)
        
        mmr2 = MerkleMountainRange()
        mmr2.append("b" * 64)
        mmr2.append("a" * 64)
        
        assert mmr1.root != mmr2.root


# =============================================================================
# CHAPTER 13: STATE MACHINE TESTS
# =============================================================================

class TestTransitionEnvelope:
    """Test TransitionEnvelope structure."""
    
    def test_envelope_digest_deterministic(self):
        """
        Theorem 13.1 (State Transition Determinism).
        
        Same envelope always produces same digest.
        """
        envelope = TransitionEnvelope(
            asset_id="a" * 64,
            seq=0,
            kind=TransitionKind.MINT,
            effective_time="2026-01-01T00:00:00Z",
            params={"to": "did:key:z6Mk...", "amount": 1000}
        )
        
        d1 = envelope.digest
        d2 = envelope.digest
        assert d1 == d2
    
    def test_different_envelopes_different_digests(self):
        """Different envelopes have different digests."""
        e1 = TransitionEnvelope(
            asset_id="a" * 64,
            seq=0,
            kind=TransitionKind.MINT,
            effective_time="2026-01-01T00:00:00Z"
        )
        e2 = TransitionEnvelope(
            asset_id="a" * 64,
            seq=1,  # Different seq
            kind=TransitionKind.MINT,
            effective_time="2026-01-01T00:00:00Z"
        )
        assert e1.digest != e2.digest


# =============================================================================
# CHAPTER 14: COMPLIANCE TENSOR TESTS
# =============================================================================

class TestComplianceTensor:
    """
    Definition 7.3 (Compliance Tensor).
    
    T: OpType × BindingID × BindingID → ComplianceStatus
    """
    
    def test_default_permitted(self):
        """Default status is Permitted."""
        tensor = ComplianceTensor()
        constraint = tensor.get("transfer", "binding:a", "binding:b")
        assert constraint.status == ComplianceStatus.PERMITTED
    
    def test_set_and_get(self):
        """Can set and retrieve constraints."""
        tensor = ComplianceTensor()
        tensor.set("transfer", "binding:a", "binding:b",
                   ComplianceConstraint(ComplianceStatus.PROHIBITED, reason_code=1))
        
        result = tensor.get("transfer", "binding:a", "binding:b")
        assert result.status == ComplianceStatus.PROHIBITED
        assert result.reason_code == 1
    
    def test_evaluate_permitted(self):
        """Permitted operations pass evaluation."""
        tensor = ComplianceTensor()
        permitted, reason = tensor.evaluate("transfer", "a", "b", {})
        assert permitted is True
        assert reason is None
    
    def test_evaluate_prohibited(self):
        """Prohibited operations fail evaluation."""
        tensor = ComplianceTensor()
        tensor.set("transfer", "a", "b",
                   ComplianceConstraint(ComplianceStatus.PROHIBITED, reason_code=42))
        
        permitted, reason = tensor.evaluate("transfer", "a", "b", {})
        assert permitted is False
        assert "42" in reason
    
    def test_evaluate_requires_attestation(self):
        """RequiresAttestation checks context."""
        tensor = ComplianceTensor()
        tensor.set("transfer", "a", "b",
                   ComplianceConstraint(
                       ComplianceStatus.REQUIRES_ATTESTATION,
                       attestation_types=["KYC"],
                       min_attestation_count=1
                   ))
        
        # Without attestation
        permitted, _ = tensor.evaluate("transfer", "a", "b", {})
        assert permitted is False
        
        # With attestation
        permitted, _ = tensor.evaluate("transfer", "a", "b", {
            "attestations": [{"type": "KYC"}]
        })
        assert permitted is True
    
    def test_tensor_root_deterministic(self):
        """Tensor root is deterministic."""
        tensor = ComplianceTensor()
        tensor.set("a", "b", "c", ComplianceConstraint(ComplianceStatus.PERMITTED))
        tensor.set("d", "e", "f", ComplianceConstraint(ComplianceStatus.PROHIBITED))
        
        root1 = tensor.root
        root2 = tensor.root
        assert root1 == root2


# =============================================================================
# CHAPTER 17: AGENTIC EXECUTION TESTS
# =============================================================================

class TestAgenticExecution:
    """
    Theorem 17.1 (Agentic Determinism).
    
    Given identical trigger events and environment state,
    agentic execution is deterministic.
    """
    
    def test_trigger_creation(self):
        """Agentic trigger can be created."""
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            data={"affected_parties": ["did:key:z6Mk..."]}
        )
        assert trigger.trigger_type == AgenticTriggerType.SANCTIONS_LIST_UPDATE
    
    def test_policy_condition_evaluation(self):
        """Policy condition is evaluated deterministically."""
        policy = AgenticPolicy(
            policy_id="test",
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            condition={"type": "threshold", "field": "receipts_since_last", "threshold": 100},
            action=TransitionKind.UPDATE_MANIFEST
        )
        
        # Below threshold
        trigger_low = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"receipts_since_last": 50}
        )
        assert policy.evaluate_condition(trigger_low, {}) is False
        
        # Above threshold
        trigger_high = AgenticTrigger(
            trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
            data={"receipts_since_last": 150}
        )
        assert policy.evaluate_condition(trigger_high, {}) is True
    
    def test_policy_determinism(self):
        """Same inputs produce same evaluation result."""
        policy = STANDARD_POLICIES["sanctions_auto_halt"]
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
            data={"affected_parties": ["self", "other"]}
        )
        
        results = [policy.evaluate_condition(trigger, {}) for _ in range(100)]
        assert len(set(results)) == 1  # All same
    
    def test_disabled_policy_never_triggers(self):
        """Disabled policies don't evaluate."""
        policy = AgenticPolicy(
            policy_id="disabled",
            trigger_type=AgenticTriggerType.RULING_RECEIVED,
            action=TransitionKind.ARBITRATION_ENFORCE,
            enabled=False
        )
        
        trigger = AgenticTrigger(
            trigger_type=AgenticTriggerType.RULING_RECEIVED,
            data={}
        )
        assert policy.evaluate_condition(trigger, {}) is False


# =============================================================================
# SMART ASSET INTEGRATION TESTS
# =============================================================================

class TestSmartAsset:
    """Integration tests for complete Smart Asset tuple."""
    
    @pytest.fixture
    def sample_asset(self) -> SmartAsset:
        """Create a sample Smart Asset for testing."""
        genesis = GenesisDocument(
            asset_name="Test Token",
            asset_class="utility",
            initial_bindings=["harbor:uae-adgm"],
            governance={"type": "threshold", "threshold": 2},
            created_at="2026-01-01T00:00:00Z"
        )
        
        binding = JurisdictionalBinding(
            harbor_id="harbor:uae-adgm",
            lawpack_digest="l" * 64
        )
        
        registry = RegistryCredential(
            asset_id=genesis.asset_id,
            bindings=[binding],
            registry_vc_digest="r" * 64,
            effective_from="2026-01-01T00:00:00Z"
        )
        
        manifest = OperationalManifest(
            asset_id=genesis.asset_id,
            version=1,
            config={},
            quorum_threshold=1,
            authorized_governors=["did:key:z6Mk..."]
        )
        
        return SmartAsset(
            genesis=genesis,
            registry=registry,
            manifest=manifest,
            state={"balances": {}, "nonce": -1}
        )
    
    def test_asset_id_from_genesis(self, sample_asset):
        """Asset ID derived from genesis document."""
        assert sample_asset.asset_id == sample_asset.genesis.asset_id
    
    def test_transition_creates_receipt(self, sample_asset):
        """Transition creates valid receipt."""
        envelope = TransitionEnvelope(
            asset_id=sample_asset.asset_id,
            seq=0,
            kind=TransitionKind.MINT,
            effective_time="2026-01-01T00:00:00Z",
            params={"to": "did:key:holder", "amount": 1000}
        )
        
        receipt = sample_asset.transition(envelope)
        
        assert receipt.seq == 0
        assert receipt.asset_id == sample_asset.asset_id
        assert len(sample_asset.receipts) == 1
    
    def test_receipt_chain_valid(self, sample_asset):
        """Receipt chain maintains integrity."""
        # Perform multiple transitions
        for i in range(5):
            envelope = TransitionEnvelope(
                asset_id=sample_asset.asset_id,
                seq=i,
                kind=TransitionKind.MINT,
                effective_time=f"2026-01-0{i+1}T00:00:00Z",
                params={"to": f"holder_{i}", "amount": 100}
            )
            sample_asset.transition(envelope)
        
        # Verify chain integrity (Lemma 12.1)
        assert sample_asset.verify_receipt_chain() is True
    
    def test_invalid_sequence_rejected(self, sample_asset):
        """Invalid sequence number is rejected."""
        envelope = TransitionEnvelope(
            asset_id=sample_asset.asset_id,
            seq=5,  # Wrong - should be 0
            kind=TransitionKind.MINT,
            effective_time="2026-01-01T00:00:00Z"
        )
        
        with pytest.raises(ValueError, match="I2 violation"):
            sample_asset.transition(envelope)
    
    def test_mmr_updated_on_transition(self, sample_asset):
        """MMR is updated with each receipt."""
        assert sample_asset.mmr.leaf_count == 0
        
        envelope = TransitionEnvelope(
            asset_id=sample_asset.asset_id,
            seq=0,
            kind=TransitionKind.MINT,
            effective_time="2026-01-01T00:00:00Z",
            params={"to": "holder", "amount": 100}
        )
        sample_asset.transition(envelope)
        
        assert sample_asset.mmr.leaf_count == 1
    
    def test_checkpoint_creation(self, sample_asset):
        """Checkpoint captures current state."""
        # Do some transitions
        for i in range(3):
            envelope = TransitionEnvelope(
                asset_id=sample_asset.asset_id,
                seq=i,
                kind=TransitionKind.MINT,
                effective_time=f"2026-01-0{i+1}T00:00:00Z",
                params={"to": f"holder_{i}", "amount": 100}
            )
            sample_asset.transition(envelope)
        
        checkpoint = sample_asset.create_checkpoint(
            witnesses=["did:key:watcher1", "did:key:watcher2"],
            signatures=[{"sig": "..."}]
        )
        
        assert checkpoint.asset_id == sample_asset.asset_id
        assert checkpoint.receipt_count == 3
        assert checkpoint.mmr_root == sample_asset.mmr.root
        assert checkpoint.receipt_chain_head == sample_asset.receipt_chain_head


# =============================================================================
# THEOREM 16.1: OFFLINE OPERATION TESTS
# =============================================================================

class TestOfflineOperation:
    """
    Theorem 16.1 (Offline Operation Capability).
    
    A Smart Asset can continue valid operations without blockchain connectivity.
    """
    
    def test_offline_capable_with_valid_credentials(self):
        """Asset with valid cached credentials can operate offline."""
        genesis = GenesisDocument(
            asset_name="Offline Asset",
            asset_class="utility",
            initial_bindings=["harbor:test"],
            governance={},
            created_at="2026-01-01T00:00:00Z"
        )
        
        binding = JurisdictionalBinding(
            harbor_id="harbor:test",
            lawpack_digest="l" * 64
        )
        
        registry = RegistryCredential(
            asset_id=genesis.asset_id,
            bindings=[binding],
            registry_vc_digest="r" * 64,
            effective_from="2026-01-01T00:00:00Z"
        )
        
        manifest = OperationalManifest(
            asset_id=genesis.asset_id,
            version=1,
            config={},
            quorum_threshold=1,
            authorized_governors=[]
        )
        
        asset = SmartAsset(
            genesis=genesis,
            registry=registry,
            manifest=manifest
        )
        
        # Valid credentials (not yet expired)
        future = (datetime.now(timezone.utc) + timedelta(days=30)).isoformat()
        cached_credentials = [{"valid_until": future}]
        cached_attestations = [{"valid_until": future}]
        
        capable, _ = verify_offline_capability(asset, cached_credentials, cached_attestations)
        assert capable is True
    
    def test_offline_fails_with_expired_credentials(self):
        """Asset with expired credentials cannot operate offline."""
        genesis = GenesisDocument(
            asset_name="Offline Asset",
            asset_class="utility",
            initial_bindings=["harbor:test"],
            governance={},
            created_at="2026-01-01T00:00:00Z"
        )
        
        binding = JurisdictionalBinding(
            harbor_id="harbor:test",
            lawpack_digest="l" * 64
        )
        
        registry = RegistryCredential(
            asset_id=genesis.asset_id,
            bindings=[binding],
            registry_vc_digest="r" * 64,
            effective_from="2026-01-01T00:00:00Z"
        )
        
        manifest = OperationalManifest(
            asset_id=genesis.asset_id,
            version=1,
            config={},
            quorum_threshold=1,
            authorized_governors=[]
        )
        
        asset = SmartAsset(
            genesis=genesis,
            registry=registry,
            manifest=manifest
        )
        
        # Expired credentials
        past = (datetime.now(timezone.utc) - timedelta(days=1)).isoformat()
        cached_credentials = [{"valid_until": past}]
        cached_attestations = []
        
        capable, _ = verify_offline_capability(asset, cached_credentials, cached_attestations)
        assert capable is False


# =============================================================================
# DESIGN INVARIANTS TESTS
# =============================================================================

class TestDesignInvariants:
    """
    Definition 11.2 (Smart Asset Invariants).
    
    Test all five design invariants I1-I5.
    """
    
    def test_i1_immutable_identity(self):
        """I1: Asset ID never changes."""
        genesis = GenesisDocument(
            asset_name="Invariant Test",
            asset_class="equity",
            initial_bindings=["harbor:test"],
            governance={}
        )
        
        id_before = genesis.asset_id
        
        # Even if we modify the object (which we shouldn't)
        # the asset_id computation should be stable
        id_after = genesis.asset_id
        
        assert id_before == id_after
    
    def test_i3_no_bindings_raises(self):
        """I3: At least one active binding required."""
        genesis = GenesisDocument(
            asset_name="No Bindings",
            asset_class="equity",
            initial_bindings=[],
            governance={}
        )
        
        # No active bindings
        registry = RegistryCredential(
            asset_id=genesis.asset_id,
            bindings=[],  # Empty!
            registry_vc_digest="r" * 64,
            effective_from="2026-01-01T00:00:00Z"
        )
        
        manifest = OperationalManifest(
            asset_id=genesis.asset_id,
            version=1,
            config={},
            quorum_threshold=1,
            authorized_governors=[]
        )
        
        with pytest.raises(ValueError, match="I3 violation"):
            SmartAsset(genesis=genesis, registry=registry, manifest=manifest)
    
    def test_i3_all_bindings_inactive_raises(self):
        """I3: Inactive bindings don't count."""
        genesis = GenesisDocument(
            asset_name="Inactive Bindings",
            asset_class="equity",
            initial_bindings=["harbor:test"],
            governance={}
        )
        
        # Binding exists but is terminated
        binding = JurisdictionalBinding(
            harbor_id="harbor:test",
            lawpack_digest="l" * 64,
            binding_status="terminated"  # Not active!
        )
        
        registry = RegistryCredential(
            asset_id=genesis.asset_id,
            bindings=[binding],
            registry_vc_digest="r" * 64,
            effective_from="2026-01-01T00:00:00Z"
        )
        
        manifest = OperationalManifest(
            asset_id=genesis.asset_id,
            version=1,
            config={},
            quorum_threshold=1,
            authorized_governors=[]
        )
        
        with pytest.raises(ValueError, match="I3 violation"):
            SmartAsset(genesis=genesis, registry=registry, manifest=manifest)


# =============================================================================
# Protocol 14.1 Tests: Cross-Jurisdiction Transfer
# =============================================================================

class TestProtocol141_CrossJurisdictionTransfer:
    """Test Protocol 14.1 - Cross-Jurisdiction Transfer."""
    
    def test_cross_jurisdiction_transfer_request_creation(self):
        from tools.mass_primitives import CrossJurisdictionTransferRequest
        
        req = CrossJurisdictionTransferRequest(
            asset_id="a" * 64,
            from_binding_id="binding:uae-adgm",
            to_binding_id="binding:kaz-aifc",
            amount=1000000,
            corridor_id="corridor:uae-kaz-trade",
        )
        
        d = req.to_dict()
        assert d["asset_id"] == "a" * 64
        assert d["from_binding_id"] == "binding:uae-adgm"
        assert d["amount"] == 1000000


# =============================================================================
# Protocol 16.1 Tests: Fork Detection and Resolution
# =============================================================================

class TestProtocol161_ForkResolution:
    """Test Protocol 16.1 - Fork Detection and Resolution."""
    
    def test_no_fork_with_single_chain(self):
        from tools.mass_primitives import detect_fork, AssetReceipt
        
        chain = [
            AssetReceipt(
                asset_id="a" * 64,
                seq=0,
                prev_root="genesis",
                transition_envelope_digest="b" * 64,
                transition_kind="transfer",
                prev_state_root="",
                new_state_root="c" * 64,
                harbor_ids=["harbor:test"],
                jurisdiction_scope="single",
                signatures=[],
            )
        ]
        
        result = detect_fork([chain])
        assert result is None
    
    def test_fork_detection_with_divergent_chains(self):
        from tools.mass_primitives import detect_fork, AssetReceipt
        
        # Force different next_root by changing new_state_root
        chain1 = [AssetReceipt(
            asset_id="a" * 64,
            seq=0,
            prev_root="genesis",
            transition_envelope_digest="b" * 64,
            transition_kind="transfer",
            prev_state_root="",
            new_state_root="c" * 64,
            harbor_ids=["harbor:test"],
            jurisdiction_scope="single",
            signatures=[],
        )]
        
        chain2 = [AssetReceipt(
            asset_id="a" * 64,
            seq=0,
            prev_root="genesis",
            transition_envelope_digest="e" * 64,  # Different envelope
            transition_kind="transfer",
            prev_state_root="",
            new_state_root="f" * 64,  # Different state
            harbor_ids=["harbor:test"],
            jurisdiction_scope="single",
            signatures=[],
        )]
        
        result = detect_fork([chain1, chain2])
        assert result is not None
        assert result.fork_sequence == 0
    
    def test_fork_resolution_longest_chain(self):
        from tools.mass_primitives import (
            ForkDetection, ForkResolutionStrategy, resolve_fork
        )
        
        fork = ForkDetection(
            fork_sequence=5,
            prev_root="prev" * 16,
            branches=[
                {"root": "a" * 64, "witness_count": 2},
                {"root": "b" * 64, "witness_count": 5},
            ],
        )
        
        canonical = resolve_fork(fork, ForkResolutionStrategy.LONGEST_CHAIN)
        assert canonical == 1


# =============================================================================
# Protocol 18.1 Tests: Artifact Graph Verification
# =============================================================================

class TestProtocol181_ArtifactGraphVerification:
    """Test Protocol 18.1 - Artifact Graph Verification."""
    
    def test_empty_artifact_graph(self):
        from tools.mass_primitives import verify_artifact_graph
        
        root = {"type": "simple", "value": 42}
        
        def resolve_fn(digest):
            return None
        
        report = verify_artifact_graph(root, resolve_fn, strict=True)
        assert report.passed is True
        assert report.total_artifacts == 0
    
    def test_artifact_graph_with_missing_ref_strict(self):
        from tools.mass_primitives import verify_artifact_graph
        
        root = {
            "type": "complex",
            "ref": {
                "artifact_type": "lawpack",
                "digest_sha256": "a" * 64,
            }
        }
        
        def resolve_fn(digest):
            return None
        
        report = verify_artifact_graph(root, resolve_fn, strict=True)
        assert report.passed is False
        assert "a" * 64 in report.missing_artifacts


# =============================================================================
# Theorem 29.1 Tests: Identity Immutability
# =============================================================================

class TestTheorem291_IdentityImmutability:
    """Test Theorem 29.1 - Identity Immutability."""
    
    def test_valid_identity(self):
        from tools.mass_primitives import GenesisDocument, verify_identity_immutability
        
        genesis = GenesisDocument(
            asset_name="Test Asset",
            asset_class="equity",
            initial_bindings=["harbor:test"],
            governance={},
        )
        
        correct_id = genesis.asset_id  # Property
        valid, error = verify_identity_immutability(genesis, correct_id)
        
        assert valid is True
        assert error == ""
    
    def test_invalid_identity(self):
        from tools.mass_primitives import GenesisDocument, verify_identity_immutability
        
        genesis = GenesisDocument(
            asset_name="Test Asset",
            asset_class="equity",
            initial_bindings=["harbor:test"],
            governance={},
        )
        
        wrong_id = "wrong" * 16
        valid, error = verify_identity_immutability(genesis, wrong_id)
        
        assert valid is False
        assert "mismatch" in error.lower()


# =============================================================================
# Theorem 29.2 Tests: Receipt Chain Non-Repudiation
# =============================================================================

class TestTheorem292_ReceiptChainNonRepudiation:
    """Test Theorem 29.2 - Receipt Chain Non-Repudiation."""
    
    def test_valid_receipt_chain(self):
        from tools.mass_primitives import (
            AssetReceipt, genesis_receipt_root, verify_receipt_chain_integrity
        )
        
        asset_id = "a" * 64
        genesis_root = genesis_receipt_root(asset_id)
        
        chain = [
            AssetReceipt(
                asset_id=asset_id,
                seq=0,
                prev_root=genesis_root,
                transition_envelope_digest="c" * 64,
                transition_kind="mint",
                prev_state_root="",
                new_state_root="d" * 64,
                harbor_ids=["harbor:test"],
                jurisdiction_scope="single",
                signatures=[],
            )
        ]
        
        valid, errors = verify_receipt_chain_integrity(asset_id, chain)
        assert valid is True
        assert len(errors) == 0
    
    def test_invalid_receipt_chain_wrong_prev_root(self):
        from tools.mass_primitives import (
            AssetReceipt, verify_receipt_chain_integrity
        )
        
        asset_id = "a" * 64
        
        chain = [
            AssetReceipt(
                asset_id=asset_id,
                seq=0,
                prev_root="wrong_root",
                transition_envelope_digest="c" * 64,
                transition_kind="mint",
                prev_state_root="",
                new_state_root="d" * 64,
                harbor_ids=["harbor:test"],
                jurisdiction_scope="single",
                signatures=[],
            )
        ]
        
        valid, errors = verify_receipt_chain_integrity(asset_id, chain)
        assert valid is False
        assert len(errors) > 0


# =============================================================================
# COMPREHENSIVE THEOREM VERIFICATION TESTS
# =============================================================================

class TestTheorem291_IdentityImmutability:
    """Tests for Theorem 29.1 (Identity Immutability)."""
    
    def test_genesis_identity_is_sha256_jcs(self):
        """Verify asset_id = SHA256(JCS(G)) per Theorem 29.1."""
        from tools.mass_primitives import GenesisDocument, stack_digest, json_canonicalize
        import hashlib
        
        genesis = GenesisDocument(
            asset_name="TestToken",
            asset_class="security",
            initial_bindings=["uae-difc"],
            governance={"quorum": 1, "governors": ["did:key:z6MkTest..."]},
            metadata={"description": "Test asset"},
            created_at="2026-01-28T00:00:00Z",
        )
        
        # Compute asset_id using our implementation (property)
        computed_id = genesis.asset_id
        
        # Manually verify using SHA256(JCS(G))
        genesis_dict = genesis.to_dict()
        jcs_bytes = json_canonicalize(genesis_dict).encode('utf-8')
        expected_id = hashlib.sha256(jcs_bytes).hexdigest()
        
        assert computed_id == expected_id
    
    def test_different_genesis_different_id(self):
        """Verify different genesis documents produce different IDs."""
        from tools.mass_primitives import GenesisDocument
        
        genesis1 = GenesisDocument(
            asset_name="TestToken1",
            asset_class="security",
            initial_bindings=["uae-difc"],
            governance={"quorum": 1, "governors": ["did:key:z6MkTest1..."]},
            created_at="2026-01-28T00:00:00Z",
        )
        
        genesis2 = GenesisDocument(
            asset_name="TestToken2",  # Different name
            asset_class="security",
            initial_bindings=["uae-difc"],
            governance={"quorum": 1, "governors": ["did:key:z6MkTest2..."]},  # Different governor
            created_at="2026-01-28T00:00:00Z",
        )
        
        assert genesis1.asset_id != genesis2.asset_id


class TestTheorem292_ReceiptChainNonRepudiation:
    """Tests for Theorem 29.2 (Receipt Chain Non-Repudiation)."""
    
    def test_chain_linkage_prevents_tampering(self):
        """Verify tampering with any receipt breaks chain integrity."""
        from tools.mass_primitives import AssetReceipt, genesis_receipt_root, verify_receipt_chain_integrity
        
        asset_id = "sha256:deadbeef"
        
        # Create a valid chain
        receipt0 = AssetReceipt(
            asset_id=asset_id,
            seq=0,
            prev_root=genesis_receipt_root(asset_id),
            transition_envelope_digest="sha256:env0",
            transition_kind="transfer",
            prev_state_root="sha256:s0",
            new_state_root="sha256:s1",
            harbor_ids=["uae-difc"],
            jurisdiction_scope="single",
            signatures=[{"signer": "did:key:z6Mk...", "sig": "..."}],
        )
        
        receipt1 = AssetReceipt(
            asset_id=asset_id,
            seq=1,
            prev_root=receipt0.next_root,
            transition_envelope_digest="sha256:env1",
            transition_kind="transfer",
            prev_state_root="sha256:s1",
            new_state_root="sha256:s2",
            harbor_ids=["uae-difc"],
            jurisdiction_scope="single",
            signatures=[{"signer": "did:key:z6Mk...", "sig": "..."}],
        )
        
        # Valid chain should verify
        valid, errors = verify_receipt_chain_integrity(asset_id, [receipt0, receipt1])
        assert valid
        assert len(errors) == 0
        
        # Tampered chain (wrong prev_root) should fail
        tampered = AssetReceipt(
            asset_id=asset_id,
            seq=1,
            prev_root="sha256:wrong",  # Tampering!
            transition_envelope_digest="sha256:env1",
            transition_kind="transfer",
            prev_state_root="sha256:s1",
            new_state_root="sha256:s2",
            harbor_ids=["uae-difc"],
            jurisdiction_scope="single",
            signatures=[{"signer": "did:key:z6Mk...", "sig": "..."}],
        )
        
        valid, errors = verify_receipt_chain_integrity(asset_id, [receipt0, tampered])
        assert not valid
        assert len(errors) > 0


class TestTheorem131_StateDeterminism:
    """Tests for Theorem 13.1 (State Transition Determinism)."""
    
    def test_transition_envelope_produces_deterministic_digest(self):
        """Verify identical envelopes produce identical digests."""
        from tools.mass_primitives import TransitionEnvelope, TransitionKind, stack_digest
        from datetime import datetime, timezone
        
        effective = datetime.now(timezone.utc).isoformat()
        
        # Create two identical envelopes
        env1 = TransitionEnvelope(
            asset_id="sha256:test",
            seq=0,
            kind=TransitionKind.TRANSFER,
            effective_time=effective,
            params={"from": "a", "to": "b", "amount": 100},
        )
        
        env2 = TransitionEnvelope(
            asset_id="sha256:test",
            seq=0,
            kind=TransitionKind.TRANSFER,
            effective_time=effective,
            params={"from": "a", "to": "b", "amount": 100},
        )
        
        # Both should produce identical digests
        digest1 = stack_digest(env1.to_dict()).bytes_hex
        digest2 = stack_digest(env2.to_dict()).bytes_hex
        
        assert digest1 == digest2


class TestTheorem161_OfflineOperation:
    """Tests for Theorem 16.1 (Offline Operation Capability)."""
    
    def test_asset_operations_without_network(self):
        """Verify assets can process operations without network connectivity."""
        from tools.mass_primitives import (
            SmartAsset, GenesisDocument, RegistryCredential, 
            OperationalManifest, JurisdictionalBinding, verify_offline_capability
        )
        
        genesis = GenesisDocument(
            asset_name="OfflineTestAsset",
            asset_class="security",
            initial_bindings=["uae-difc"],
            governance={"quorum": 1, "governors": ["did:key:z6MkIssuer..."]},
            created_at="2026-01-28T00:00:00Z",
        )
        
        binding = JurisdictionalBinding(
            harbor_id="uae-difc",
            lawpack_digest="sha256:lawpack123",
            regpack_digest="sha256:regpack123",
            binding_status="active",
        )
        
        registry = RegistryCredential(
            asset_id=genesis.asset_id,
            bindings=[binding],
            registry_vc_digest="sha256:registrycred123",
            effective_from="2026-01-28T00:00:00Z",
        )
        
        manifest = OperationalManifest(
            asset_id=genesis.asset_id,
            version=1,
            config={"state_root": "sha256:initialstate"},
            quorum_threshold=1,
            authorized_governors=["did:key:z6MkIssuer..."],
        )
        
        asset = SmartAsset(
            genesis=genesis,
            registry=registry,
            manifest=manifest,
        )
        
        # Verify offline capability
        capable, reason = verify_offline_capability(asset)
        assert capable, f"Asset should be offline-capable: {reason}"


class TestProtocol161_ForkResolution:
    """Tests for Protocol 16.1 (Fork Resolution)."""
    
    def test_fork_detection(self):
        """Verify fork detection identifies divergent chains."""
        from tools.mass_primitives import (
            AssetReceipt, genesis_receipt_root, detect_fork, ForkDetection
        )
        
        asset_id = "sha256:forktest"
        genesis = genesis_receipt_root(asset_id)
        
        # Create two divergent chains at seq 1
        receipt0 = AssetReceipt(
            asset_id=asset_id,
            seq=0,
            prev_root=genesis,
            transition_envelope_digest="sha256:common",
            transition_kind="transfer",
            prev_state_root="sha256:s0",
            new_state_root="sha256:s1",
            harbor_ids=["uae-difc"],
            jurisdiction_scope="single",
            signatures=[{"signer": "did:key:z6Mk...", "sig": "..."}],
        )
        
        # Branch A continues from receipt0
        receipt1a = AssetReceipt(
            asset_id=asset_id,
            seq=1,
            prev_root=receipt0.next_root,
            transition_envelope_digest="sha256:branchA",
            transition_kind="transfer",
            prev_state_root="sha256:s1",
            new_state_root="sha256:s2a",
            harbor_ids=["uae-difc"],
            jurisdiction_scope="single",
            signatures=[{"signer": "did:key:z6Mk...", "sig": "..."}],
        )
        
        # Branch B also continues from receipt0 (fork!)
        receipt1b = AssetReceipt(
            asset_id=asset_id,
            seq=1,
            prev_root=receipt0.next_root,
            transition_envelope_digest="sha256:branchB",
            transition_kind="transfer",
            prev_state_root="sha256:s1",
            new_state_root="sha256:s2b",
            harbor_ids=["uae-difc"],
            jurisdiction_scope="single",
            signatures=[{"signer": "did:key:z6Mk...", "sig": "..."}],
        )
        
        chain_a = [receipt0, receipt1a]
        chain_b = [receipt0, receipt1b]
        
        fork = detect_fork([chain_a, chain_b])
        assert fork is not None
        assert fork.fork_sequence == 1


class TestProtocol181_ArtifactGraphVerification:
    """Tests for Protocol 18.1 (Artifact Graph Verification)."""
    
    def test_artifact_graph_strict_mode(self):
        """Verify strict mode fails on missing artifacts."""
        from tools.mass_primitives import verify_artifact_graph
        
        # Use a properly formatted 64-char hex digest
        missing_digest = "0" * 64
        
        root = {
            "type": "document",
            "refs": [{"artifact_type": "blob", "digest_sha256": missing_digest}]
        }
        
        # Resolver that returns None for the referenced artifact
        def resolver(digest):
            if digest == missing_digest:
                return None
            return b'test content'
        
        report = verify_artifact_graph(root, resolver, strict=True)
        
        # In strict mode, missing artifacts should cause failure
        assert not report.passed
        assert missing_digest in report.missing_artifacts
    
    def test_artifact_graph_valid_closure(self):
        """Verify valid artifact closures pass verification."""
        from tools.mass_primitives import verify_artifact_graph
        import hashlib
        
        # Create a content and its digest
        content = b'valid artifact content'
        digest = hashlib.sha256(content).hexdigest()
        
        root = {
            "type": "document",
            "refs": [{"artifact_type": "blob", "digest_sha256": digest}]
        }
        
        # Resolver that returns the correct content
        def resolver(d):
            if d == digest:
                return content
            return None
        
        report = verify_artifact_graph(root, resolver, strict=True)
        
        assert report.passed
        assert len(report.missing_artifacts) == 0
        assert len(report.invalid_artifacts) == 0


class TestDefinition142_QuorumThreshold:
    """Tests for Definition 14.2 (Multi-Jurisdiction Quorum)."""
    
    def test_unanimous_quorum(self):
        """Test unanimous quorum requires all to pass."""
        results = [True, True, True]
        assert all(results) == True
        
        results = [True, False, True]
        assert all(results) == False
    
    def test_majority_quorum(self):
        """Test majority quorum requires > 50%."""
        def majority(results):
            return sum(results) > len(results) / 2
        
        assert majority([True, True, False]) == True
        assert majority([True, False, False]) == False
    
    def test_threshold_quorum(self):
        """Test threshold quorum requires min of max."""
        def threshold(results, min_required):
            return sum(results) >= min_required
        
        assert threshold([True, True, False, False], 2) == True
        assert threshold([True, False, False, False], 2) == False


# =============================================================================
# CONSTRUCTION 3.1 - CANONICAL DIGEST BRIDGE TESTS
# =============================================================================

class TestConstruction31_CanonicalDigestBridge:
    """Tests for Construction 3.1 (Canonical Digest Bridge)."""
    
    def test_stack_digest_deterministic(self):
        """Same artifact always produces same digest."""
        artifact = {"name": "test", "value": 42, "nested": {"a": 1, "b": 2}}
        
        digest1 = stack_digest(artifact)
        digest2 = stack_digest(artifact)
        
        assert digest1.bytes_hex == digest2.bytes_hex
    
    def test_stack_digest_key_order_independent(self):
        """JCS ensures key order doesn't affect digest."""
        artifact1 = {"z": 1, "a": 2, "m": 3}
        artifact2 = {"a": 2, "m": 3, "z": 1}
        
        assert stack_digest(artifact1).bytes_hex == stack_digest(artifact2).bytes_hex
    
    def test_stack_digest_nested_objects(self):
        """Nested objects are properly canonicalized."""
        artifact = {
            "outer": {
                "inner": {
                    "deep": {"value": "test"}
                }
            }
        }
        
        digest = stack_digest(artifact)
        assert len(digest.bytes_hex) == 64
        assert digest.alg == DigestAlgorithm.SHA256
    
    def test_stack_digest_different_artifacts_different_digests(self):
        """Different artifacts produce different digests."""
        artifact1 = {"id": 1}
        artifact2 = {"id": 2}
        
        assert stack_digest(artifact1).bytes_hex != stack_digest(artifact2).bytes_hex


# =============================================================================
# DEFINITION 11.1 - SMART ASSET IDENTITY TESTS
# =============================================================================

class TestDefinition111_SmartAssetIdentity:
    """Tests for Definition 11.1 - Smart Asset canonical identity."""
    
    def test_asset_id_is_sha256_of_jcs_genesis(self):
        """Verify asset_id = SHA256(JCS(G))."""
        genesis = GenesisDocument(
            asset_name="Identity Test Asset",
            asset_class="token",
            initial_bindings=["test-harbor"],
            governance={"quorum": 1},
            created_at="2026-01-01T00:00:00Z",
        )
        
        # Use to_dict() to get the canonical representation
        genesis_data = genesis.to_dict()
        expected_digest = stack_digest(genesis_data)
        
        assert genesis.asset_id == expected_digest.bytes_hex
    
    def test_immutable_identity_invariant(self):
        """I1: asset_id never changes regardless of state."""
        genesis = GenesisDocument(
            asset_name="Immutable Test",
            asset_class="security",
            initial_bindings=["harbor:test"],
            governance={},
            created_at="2026-01-01T00:00:00Z",
        )
        
        original_id = genesis.asset_id
        
        # Create SmartAsset and modify state
        binding = JurisdictionalBinding(
            harbor_id="harbor:test",
            lawpack_digest="l" * 64,
        )
        registry = RegistryCredential(
            asset_id=genesis.asset_id,
            bindings=[binding],
            registry_vc_digest="r" * 64,
            effective_from="2026-01-01T00:00:00Z",
        )
        manifest = OperationalManifest(
            asset_id=genesis.asset_id,
            version=1,
            config={},
            quorum_threshold=1,
            authorized_governors=[],
        )
        
        asset = SmartAsset(
            genesis=genesis,
            registry=registry,
            manifest=manifest,
        )
        
        # Add receipts (state changes)
        asset.receipts.append(AssetReceipt(
            asset_id=genesis.asset_id,
            seq=0,
            prev_root=genesis_receipt_root(genesis.asset_id),
            transition_envelope_digest="t" * 64,
            transition_kind="test.transition.v1",
            prev_state_root="p" * 64,
            new_state_root="n" * 64,
            harbor_ids=["harbor:test"],
            jurisdiction_scope="single",
            signatures=[],
        ))
        
        # Identity must remain unchanged
        assert asset.asset_id == original_id


# =============================================================================
# DEFINITION 12.2 - RECEIPT CHAIN LINKAGE TESTS
# =============================================================================

class TestDefinition122_ReceiptChainLinkage:
    """Tests for Definition 12.2 (Receipt Chain Linkage)."""
    
    def test_genesis_root_formula(self):
        """H0 = SHA256('SMART_ASSET_GENESIS' || asset_id)."""
        asset_id = "a" * 64
        
        genesis_root = genesis_receipt_root(asset_id)
        
        # Manually compute expected
        import hashlib
        expected = hashlib.sha256(f"SMART_ASSET_GENESIS{asset_id}".encode()).hexdigest()
        
        assert genesis_root == expected
    
    def test_chain_invariant_prev_root_equals_previous_next_root(self):
        """∀n > 0: Receipt_n.prev_root = H_{n-1}."""
        asset_id = "b" * 64
        genesis_root = genesis_receipt_root(asset_id)
        
        receipt0 = AssetReceipt(
            asset_id=asset_id,
            seq=0,
            prev_root=genesis_root,
            transition_envelope_digest="t0" + "0" * 62,
            transition_kind="test.v1",
            prev_state_root="p0" + "0" * 62,
            new_state_root="n0" + "0" * 62,
            harbor_ids=["harbor:test"],
            jurisdiction_scope="single",
            signatures=[],
        )
        
        # Receipt 1's prev_root should be receipt 0's next_root
        receipt1 = AssetReceipt(
            asset_id=asset_id,
            seq=1,
            prev_root=receipt0.next_root,  # Chain invariant
            transition_envelope_digest="t1" + "0" * 62,
            transition_kind="test.v1",
            prev_state_root="n0" + "0" * 62,
            new_state_root="n1" + "0" * 62,
            harbor_ids=["harbor:test"],
            jurisdiction_scope="single",
            signatures=[],
        )
        
        assert receipt1.prev_root == receipt0.next_root


# =============================================================================
# LEMMA 12.1 - RECEIPT CHAIN INTEGRITY TESTS
# =============================================================================

class TestLemma121_ReceiptChainIntegrity:
    """Tests for Lemma 12.1 (Receipt Chain Integrity)."""
    
    def test_tampering_detected_modified_receipt(self):
        """Modifying a receipt breaks chain verification."""
        genesis = GenesisDocument(
            asset_name="Tamper Test",
            asset_class="token",
            initial_bindings=["test"],
            governance={},
            created_at="2026-01-01T00:00:00Z",
        )
        
        binding = JurisdictionalBinding(
            harbor_id="test",
            lawpack_digest="l" * 64,
        )
        registry = RegistryCredential(
            asset_id=genesis.asset_id,
            bindings=[binding],
            registry_vc_digest="r" * 64,
            effective_from="2026-01-01T00:00:00Z",
        )
        manifest = OperationalManifest(
            asset_id=genesis.asset_id,
            version=1,
            config={},
            quorum_threshold=1,
            authorized_governors=[],
        )
        
        asset = SmartAsset(genesis=genesis, registry=registry, manifest=manifest)
        
        # Add valid receipts
        for i in range(3):
            prev = genesis_receipt_root(genesis.asset_id) if i == 0 else asset.receipts[-1].next_root
            asset.receipts.append(AssetReceipt(
                asset_id=genesis.asset_id,
                seq=i,
                prev_root=prev,
                transition_envelope_digest=f"t{i}" + "0" * 62,
                transition_kind="test.v1",
                prev_state_root=f"p{i}" + "0" * 62,
                new_state_root=f"n{i}" + "0" * 62,
                harbor_ids=["test"],
                jurisdiction_scope="single",
                signatures=[],
            ))
        
        # Verify valid chain
        assert asset.verify_receipt_chain() is True
        
        # Tamper with middle receipt (break chain)
        original_prev_root = asset.receipts[1].prev_root
        asset.receipts[1].prev_root = "x" * 64  # Invalid
        
        # Chain verification should fail
        assert asset.verify_receipt_chain() is False
        
        # Restore
        asset.receipts[1].prev_root = original_prev_root
        assert asset.verify_receipt_chain() is True
