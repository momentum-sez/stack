"""
PHOENIX v0.4.44 Comprehensive Test Suite

Tests for all PHOENIX components:
- Compliance Tensor (tensor algebra, commitment, slicing)
- ZK Proof Infrastructure (circuit registry, proof generation)
- Compliance Manifold (path planning, attestation gaps)
- Migration Protocol (saga state machine, compensation)
- Watcher Economy (bonds, slashing, reputation)

Run with: pytest tests/test_phoenix.py -v

Copyright (c) 2026 Momentum. All rights reserved.
"""

import hashlib
import json
import pytest
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from typing import List

# Import PHOENIX modules directly for testing
from tools.phoenix.tensor import (
    ComplianceDomain,
    ComplianceState,
    ComplianceTensorV2,
    TensorSlice,
    TensorCommitment,
    TensorCoord,
    TensorCell,
    AttestationRef,
    tensor_meet,
    tensor_join,
    portfolio_compliance,
)
from tools.phoenix.zkp import (
    ProofSystem,
    CircuitType,
    Circuit,
    CircuitRegistry,
    Witness,
    Proof,
    MockProver,
    MockVerifier,
    ProvingKey,
    VerificationKey,
    build_balance_sufficiency_circuit,
    build_sanctions_clearance_circuit,
    create_standard_registry,
)
from tools.phoenix.manifold import (
    ComplianceManifold,
    MigrationPath,
    AttestationRequirement,
    AttestationType,
    PathConstraint,
    JurisdictionNode,
    CorridorEdge,
    AttestationGap,
    create_uae_difc_jurisdiction,
    create_kz_aifc_jurisdiction,
    create_difc_aifc_corridor,
    create_standard_manifold,
)
from tools.phoenix.migration import (
    MigrationSaga,
    MigrationState,
    MigrationRequest,
    MigrationEvidence,
    CompensationAction,
    MigrationOrchestrator,
    StateTransition,
    LockEvidence,
)
from tools.phoenix.watcher import (
    WatcherBond,
    WatcherId,
    BondStatus,
    SlashingCondition,
    SlashingClaim,
    WatcherReputation,
    WatcherRegistry,
    ReputationMetrics,
    EquivocationDetector,
    EquivocationEvidence,
    SLASH_PERCENTAGES,
)


# =============================================================================
# COMPLIANCE TENSOR TESTS
# =============================================================================

class TestComplianceTensor:
    """Tests for Compliance Tensor implementation."""
    
    def test_tensor_creation(self):
        """Test basic tensor creation."""
        tensor = ComplianceTensorV2()
        assert len(tensor) == 0
    
    def test_tensor_set_get(self):
        """Test setting and getting compliance states."""
        tensor = ComplianceTensorV2()
        
        coord = tensor.set(
            asset_id="asset-123",
            jurisdiction_id="uae-difc",
            domain=ComplianceDomain.KYC,
            state=ComplianceState.COMPLIANT,
        )
        
        cell = tensor.get("asset-123", "uae-difc", ComplianceDomain.KYC)
        assert cell.state == ComplianceState.COMPLIANT
    
    def test_tensor_unknown_default(self):
        """Test that unknown coordinates return UNKNOWN state (fail-safe)."""
        tensor = ComplianceTensorV2()
        
        cell = tensor.get("nonexistent", "nowhere", ComplianceDomain.AML)
        assert cell.state == ComplianceState.UNKNOWN
    
    def test_compliance_state_lattice(self):
        """Test compliance state lattice operations."""
        # Meet (pessimistic) - should return lower state
        assert ComplianceState.COMPLIANT.meet(ComplianceState.PENDING) == ComplianceState.PENDING
        assert ComplianceState.COMPLIANT.meet(ComplianceState.NON_COMPLIANT) == ComplianceState.NON_COMPLIANT
        assert ComplianceState.PENDING.meet(ComplianceState.UNKNOWN) == ComplianceState.UNKNOWN
        
        # Join (optimistic) - should return higher state
        assert ComplianceState.PENDING.join(ComplianceState.COMPLIANT) == ComplianceState.COMPLIANT
        assert ComplianceState.UNKNOWN.join(ComplianceState.PENDING) == ComplianceState.PENDING
    
    def test_tensor_evaluate(self):
        """Test compliance evaluation across domains."""
        tensor = ComplianceTensorV2()
        
        # Set some compliance states
        tensor.set("asset-1", "uae-difc", ComplianceDomain.KYC, ComplianceState.COMPLIANT)
        tensor.set("asset-1", "uae-difc", ComplianceDomain.AML, ComplianceState.COMPLIANT)
        tensor.set("asset-1", "uae-difc", ComplianceDomain.SANCTIONS, ComplianceState.PENDING)
        
        # Evaluate - should be PENDING (pessimistic across domains)
        is_compliant, aggregate, issues = tensor.evaluate(
            "asset-1", "uae-difc",
            domains={ComplianceDomain.KYC, ComplianceDomain.AML, ComplianceDomain.SANCTIONS}
        )
        
        assert not is_compliant
        assert aggregate == ComplianceState.PENDING
        assert len(issues) == 1  # SANCTIONS is pending
    
    def test_tensor_slicing(self):
        """Test tensor slicing operations."""
        tensor = ComplianceTensorV2()
        
        # Populate tensor
        tensor.set("asset-1", "uae-difc", ComplianceDomain.KYC, ComplianceState.COMPLIANT)
        tensor.set("asset-1", "uae-difc", ComplianceDomain.AML, ComplianceState.COMPLIANT)
        tensor.set("asset-1", "kz-aifc", ComplianceDomain.KYC, ComplianceState.PENDING)
        tensor.set("asset-2", "uae-difc", ComplianceDomain.KYC, ComplianceState.COMPLIANT)
        
        # Slice by jurisdiction
        difc_slice = tensor.slice(jurisdiction_id="uae-difc")
        assert len(difc_slice.cells) == 3
        assert difc_slice.aggregate_state() == ComplianceState.COMPLIANT
        
        # Slice by asset
        asset1_slice = tensor.slice(asset_id="asset-1")
        assert len(asset1_slice.cells) == 3
        assert asset1_slice.aggregate_state() == ComplianceState.PENDING
    
    def test_tensor_commitment(self):
        """Test tensor commitment generation."""
        tensor = ComplianceTensorV2()
        
        tensor.set("asset-1", "uae-difc", ComplianceDomain.KYC, ComplianceState.COMPLIANT)
        tensor.set("asset-1", "uae-difc", ComplianceDomain.AML, ComplianceState.COMPLIANT)
        
        commitment = tensor.commit()
        
        assert len(commitment.root) == 64  # SHA256 hex
        assert commitment.cell_count == 2
        assert "asset-1" in commitment.asset_ids
        assert "uae-difc" in commitment.jurisdiction_ids
    
    def test_tensor_commitment_determinism(self):
        """Test that same tensor produces same commitment."""
        tensor1 = ComplianceTensorV2()
        tensor2 = ComplianceTensorV2()
        
        # Use fixed time quantum for determinism
        fixed_tq = 1000
        
        # Same data in different order
        tensor1.set("asset-1", "uae-difc", ComplianceDomain.KYC, ComplianceState.COMPLIANT, time_quantum=fixed_tq)
        tensor1.set("asset-1", "uae-difc", ComplianceDomain.AML, ComplianceState.PENDING, time_quantum=fixed_tq)
        
        tensor2.set("asset-1", "uae-difc", ComplianceDomain.AML, ComplianceState.PENDING, time_quantum=fixed_tq)
        tensor2.set("asset-1", "uae-difc", ComplianceDomain.KYC, ComplianceState.COMPLIANT, time_quantum=fixed_tq)
        
        assert tensor1.commit().root == tensor2.commit().root
    
    def test_tensor_merge(self):
        """Test merging two tensors."""
        tensor1 = ComplianceTensorV2()
        tensor2 = ComplianceTensorV2()
        
        tensor1.set("asset-1", "uae-difc", ComplianceDomain.KYC, ComplianceState.COMPLIANT)
        tensor2.set("asset-1", "kz-aifc", ComplianceDomain.KYC, ComplianceState.PENDING)
        
        tensor1.merge(tensor2)
        
        assert len(tensor1) == 2
        assert tensor1.get("asset-1", "kz-aifc", ComplianceDomain.KYC).state == ComplianceState.PENDING
    
    def test_tensor_meet_operation(self):
        """Test tensor meet (pessimistic intersection)."""
        t1 = ComplianceTensorV2()
        t2 = ComplianceTensorV2()
        
        t1.set("asset-1", "uae-difc", ComplianceDomain.KYC, ComplianceState.COMPLIANT)
        t2.set("asset-1", "uae-difc", ComplianceDomain.KYC, ComplianceState.PENDING)
        
        result = tensor_meet(t1, t2)
        
        # Meet should return PENDING (lower in lattice)
        cell = result.get("asset-1", "uae-difc", ComplianceDomain.KYC)
        assert cell.state == ComplianceState.PENDING
    
    def test_attestation_expiry(self):
        """Test attestation expiry detection."""
        past_date = (datetime.now(timezone.utc) - timedelta(days=1)).isoformat()
        future_date = (datetime.now(timezone.utc) + timedelta(days=1)).isoformat()
        
        expired_att = AttestationRef(
            attestation_id="att-1",
            attestation_type="kyc",
            issuer_did="did:example:issuer",
            issued_at="2024-01-01T00:00:00Z",
            expires_at=past_date,
            digest="abc123",
        )
        
        valid_att = AttestationRef(
            attestation_id="att-2",
            attestation_type="kyc",
            issuer_did="did:example:issuer",
            issued_at="2024-01-01T00:00:00Z",
            expires_at=future_date,
            digest="def456",
        )
        
        assert expired_att.is_expired()
        assert not valid_att.is_expired()


# =============================================================================
# ZK PROOF INFRASTRUCTURE TESTS
# =============================================================================

class TestZKProofInfrastructure:
    """Tests for ZK proof infrastructure."""
    
    def test_circuit_creation(self):
        """Test circuit creation."""
        circuit = build_balance_sufficiency_circuit()
        
        assert circuit.circuit_id == "zk.balance_sufficiency.v1"
        assert circuit.circuit_type == CircuitType.BALANCE_SUFFICIENCY
        assert circuit.proof_system == ProofSystem.GROTH16
        assert "threshold" in circuit.public_input_names
        assert "balance" in circuit.private_input_names
    
    def test_circuit_digest_determinism(self):
        """Test circuit digest is deterministic."""
        c1 = build_balance_sufficiency_circuit()
        c2 = build_balance_sufficiency_circuit()
        
        assert c1.digest == c2.digest
    
    def test_circuit_registry(self):
        """Test circuit registry operations."""
        registry = CircuitRegistry()
        
        circuit = build_sanctions_clearance_circuit()
        digest = registry.register(circuit)
        
        retrieved = registry.get_circuit(digest)
        assert retrieved is not None
        assert retrieved.circuit_id == circuit.circuit_id
    
    def test_standard_registry(self):
        """Test standard registry contains expected circuits."""
        registry = create_standard_registry()
        
        circuits = registry.list_circuits()
        assert len(circuits) >= 4  # balance, sanctions, kyc, tensor inclusion
        
        # Check specific circuit types exist
        types = {c.circuit_type for c in circuits}
        assert CircuitType.BALANCE_SUFFICIENCY in types
        assert CircuitType.SANCTIONS_CLEARANCE in types
    
    def test_mock_prover_verifier(self):
        """Test mock prover and verifier."""
        registry = create_standard_registry()
        circuit = build_balance_sufficiency_circuit()
        
        pk = registry.get_proving_key(circuit.digest)
        vk = registry.get_verification_key(circuit.digest)
        
        witness = Witness(
            circuit_id=circuit.circuit_id,
            private_inputs={"balance": 1000},
            public_inputs={"threshold": 500, "result_commitment": "abc"},
        )
        
        prover = MockProver()
        proof = prover.prove(circuit, pk, witness)
        
        assert proof.circuit_id == circuit.circuit_id
        assert len(proof.proof_data) > 0
        
        verifier = MockVerifier()
        assert verifier.verify(circuit, vk, proof)
    
    def test_proof_system_properties(self):
        """Test proof system property queries."""
        assert ProofSystem.GROTH16.requires_trusted_setup()
        assert ProofSystem.PLONK.requires_trusted_setup()
        assert not ProofSystem.STARK.requires_trusted_setup()
        assert ProofSystem.STARK.is_post_quantum()
        assert not ProofSystem.GROTH16.is_post_quantum()


# =============================================================================
# COMPLIANCE MANIFOLD TESTS
# =============================================================================

class TestComplianceManifold:
    """Tests for compliance manifold and path planning."""
    
    def test_manifold_creation(self):
        """Test manifold creation."""
        manifold = ComplianceManifold()
        
        assert len(manifold.list_jurisdictions()) == 0
        assert len(manifold.list_corridors()) == 0
    
    def test_add_jurisdiction(self):
        """Test adding jurisdictions."""
        manifold = ComplianceManifold()
        difc = create_uae_difc_jurisdiction()
        
        manifold.add_jurisdiction(difc)
        
        assert len(manifold.list_jurisdictions()) == 1
        assert manifold.get_jurisdiction("uae-difc") is not None
    
    def test_add_corridor(self):
        """Test adding corridors."""
        manifold = create_standard_manifold()
        
        corridors = manifold.list_corridors()
        assert len(corridors) >= 1
    
    def test_find_direct_path(self):
        """Test finding direct path between jurisdictions."""
        manifold = create_standard_manifold()
        
        path = manifold.find_path("uae-difc", "kz-aifc")
        
        assert path is not None
        assert path.source_jurisdiction == "uae-difc"
        assert path.target_jurisdiction == "kz-aifc"
        assert path.hop_count == 1
    
    def test_path_constraints(self):
        """Test path finding with constraints."""
        manifold = create_standard_manifold()
        
        # Very low cost constraint - should fail
        constraints = PathConstraint(max_total_cost_usd=Decimal("1"))
        path = manifold.find_path("uae-difc", "kz-aifc", constraints)
        
        assert path is None  # Too expensive
        
        # Reasonable constraint - should succeed
        constraints = PathConstraint(max_total_cost_usd=Decimal("100000"))
        path = manifold.find_path("uae-difc", "kz-aifc", constraints)
        
        assert path is not None
    
    def test_attestation_gap_analysis(self):
        """Test attestation gap analysis."""
        manifold = create_standard_manifold()
        path = manifold.find_path("uae-difc", "kz-aifc")
        
        # No attestations available
        gap = manifold.analyze_attestation_gap(path, [])
        
        assert not gap.is_satisfied
        assert gap.missing_count > 0
    
    def test_compliance_distance(self):
        """Test compliance distance calculation."""
        manifold = create_standard_manifold()
        
        distance = manifold.compliance_distance("uae-difc", "kz-aifc")
        
        assert distance is not None
        assert "hop_count" in distance
        assert "total_cost_usd" in distance
        assert "attestation_count" in distance
    
    def test_unreachable_jurisdiction(self):
        """Test handling of unreachable jurisdictions."""
        manifold = ComplianceManifold()
        manifold.add_jurisdiction(create_uae_difc_jurisdiction())
        # Don't add corridor or target jurisdiction
        
        path = manifold.find_path("uae-difc", "nonexistent")
        assert path is None


# =============================================================================
# MIGRATION PROTOCOL TESTS
# =============================================================================

class TestMigrationProtocol:
    """Tests for migration saga state machine."""
    
    def test_saga_creation(self):
        """Test saga creation."""
        request = MigrationRequest(
            asset_id="asset-123",
            asset_genesis_digest="abc" * 21 + "a",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
        )
        
        saga = MigrationSaga(request)
        
        assert saga.state == MigrationState.INITIATED
        assert not saga.is_complete
    
    def test_saga_state_transitions(self):
        """Test valid state transitions."""
        request = MigrationRequest(
            asset_id="asset-123",
            asset_genesis_digest="abc" * 21 + "a",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
        )
        
        saga = MigrationSaga(request)
        
        # Valid transitions
        assert saga.advance_to(MigrationState.COMPLIANCE_CHECK, "Checking compliance")
        assert saga.state == MigrationState.COMPLIANCE_CHECK
        
        assert saga.advance_to(MigrationState.ATTESTATION_GATHERING, "Gathering attestations")
        assert saga.state == MigrationState.ATTESTATION_GATHERING
    
    def test_invalid_state_transitions(self):
        """Test invalid state transitions are rejected."""
        request = MigrationRequest(
            asset_id="asset-123",
            asset_genesis_digest="abc" * 21 + "a",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
        )
        
        saga = MigrationSaga(request)
        
        # Cannot jump to TRANSIT from INITIATED
        assert not saga.advance_to(MigrationState.TRANSIT, "Skipping states")
        assert saga.state == MigrationState.INITIATED
    
    def test_saga_compensation(self):
        """Test saga compensation on failure."""
        request = MigrationRequest(
            asset_id="asset-123",
            asset_genesis_digest="abc" * 21 + "a",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
        )
        
        saga = MigrationSaga(request)
        saga.advance_to(MigrationState.COMPLIANCE_CHECK)
        saga.advance_to(MigrationState.ATTESTATION_GATHERING)
        
        # Compensate
        assert saga.compensate("Attestation gathering failed")
        assert saga.state == MigrationState.COMPENSATED
        assert saga.is_complete
        assert not saga.is_successful
    
    def test_saga_cancellation(self):
        """Test saga cancellation in early states."""
        request = MigrationRequest(
            asset_id="asset-123",
            asset_genesis_digest="abc" * 21 + "a",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
        )
        
        saga = MigrationSaga(request)
        
        # Can cancel in INITIATED
        assert saga.cancel("User requested cancellation")
        assert saga.state == MigrationState.CANCELLED
    
    def test_saga_completion(self):
        """Test saga successful completion."""
        request = MigrationRequest(
            asset_id="asset-123",
            asset_genesis_digest="abc" * 21 + "a",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
        )
        
        saga = MigrationSaga(request)
        
        # Progress through all states
        saga.advance_to(MigrationState.COMPLIANCE_CHECK)
        saga.advance_to(MigrationState.ATTESTATION_GATHERING)
        saga.advance_to(MigrationState.SOURCE_LOCK)
        saga.advance_to(MigrationState.TRANSIT)
        saga.advance_to(MigrationState.DESTINATION_VERIFICATION)
        saga.advance_to(MigrationState.DESTINATION_UNLOCK)
        
        # Complete
        assert saga.complete()
        assert saga.state == MigrationState.COMPLETED
        assert saga.is_successful
    
    def test_saga_evidence_collection(self):
        """Test evidence collection during saga."""
        request = MigrationRequest(
            asset_id="asset-123",
            asset_genesis_digest="abc" * 21 + "a",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
        )
        
        saga = MigrationSaga(request)
        
        # Add attestation
        attestation = AttestationRef(
            attestation_id="att-1",
            attestation_type="kyc",
            issuer_did="did:example:issuer",
            issued_at=datetime.now(timezone.utc).isoformat(),
            digest="abc123",
        )
        saga.add_attestation(attestation)
        
        assert len(saga.evidence.collected_attestations) == 1
    
    def test_migration_orchestrator(self):
        """Test migration orchestrator."""
        orchestrator = MigrationOrchestrator()
        
        request = MigrationRequest(
            asset_id="asset-123",
            asset_genesis_digest="abc" * 21 + "a",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
        )
        
        saga = orchestrator.create_migration(request)
        
        assert orchestrator.get_saga(saga.migration_id) is not None
        assert len(orchestrator.list_active_migrations()) == 1


# =============================================================================
# WATCHER ECONOMY TESTS
# =============================================================================

class TestWatcherEconomy:
    """Tests for watcher economy and accountability."""
    
    def test_watcher_registration(self):
        """Test watcher registration."""
        registry = WatcherRegistry()
        
        watcher_id = WatcherId(
            did="did:example:watcher1",
            public_key_hex="ab" * 32,
        )
        
        assert registry.register_watcher(watcher_id)
        assert registry.get_watcher("did:example:watcher1") is not None
    
    def test_bond_posting(self):
        """Test bond posting."""
        registry = WatcherRegistry()
        
        watcher_id = WatcherId(
            did="did:example:watcher1",
            public_key_hex="ab" * 32,
        )
        registry.register_watcher(watcher_id)
        
        bond = WatcherBond(
            bond_id="bond-1",
            watcher_id=watcher_id,
            collateral_amount=Decimal("10000"),
            collateral_currency="USDC",
            collateral_address="0x" + "ab" * 20,
        )
        
        assert registry.post_bond(bond)
        assert registry.get_bond("bond-1") is not None
    
    def test_bond_activation(self):
        """Test bond activation."""
        registry = WatcherRegistry()
        
        watcher_id = WatcherId(
            did="did:example:watcher1",
            public_key_hex="ab" * 32,
        )
        registry.register_watcher(watcher_id)
        
        bond = WatcherBond(
            bond_id="bond-1",
            watcher_id=watcher_id,
            collateral_amount=Decimal("10000"),
            collateral_currency="USDC",
            collateral_address="0x" + "ab" * 20,
        )
        registry.post_bond(bond)
        
        assert registry.activate_bond("bond-1")
        assert registry.get_bond("bond-1").is_active
    
    def test_slashing_claim(self):
        """Test slashing claim filing and execution."""
        registry = WatcherRegistry()
        
        watcher_id = WatcherId(
            did="did:example:watcher1",
            public_key_hex="ab" * 32,
        )
        registry.register_watcher(watcher_id)
        
        bond = WatcherBond(
            bond_id="bond-1",
            watcher_id=watcher_id,
            collateral_amount=Decimal("10000"),
            collateral_currency="USDC",
            collateral_address="0x" + "ab" * 20,
        )
        registry.post_bond(bond)
        registry.activate_bond("bond-1")
        
        # File slashing claim
        evidence = EquivocationEvidence(
            attestation_1={"digest": "abc"},
            attestation_2={"digest": "def"},
        )
        
        claim = SlashingClaim(
            claim_id="claim-1",
            watcher_id=watcher_id,
            condition=SlashingCondition.FALSE_ATTESTATION,
            evidence=evidence,
            claimant_did="did:example:claimant",
            claimed_slash_amount=Decimal("5000"),
        )
        
        assert registry.file_slashing_claim(claim)
    
    def test_slash_percentages(self):
        """Test slash percentages are correct."""
        assert SLASH_PERCENTAGES[SlashingCondition.EQUIVOCATION] == Decimal("1.00")
        assert SLASH_PERCENTAGES[SlashingCondition.AVAILABILITY_FAILURE] == Decimal("0.01")
        assert SLASH_PERCENTAGES[SlashingCondition.FALSE_ATTESTATION] == Decimal("0.50")
    
    def test_reputation_scoring(self):
        """Test reputation score computation."""
        watcher_id = WatcherId(
            did="did:example:watcher1",
            public_key_hex="ab" * 32,
        )
        
        metrics = ReputationMetrics(
            required_attestations=100,
            delivered_attestations=95,
            on_time_attestations=90,
            challenged_attestations=5,
            successful_challenges=1,
            failed_challenges=4,
            continuous_active_days=180,
        )
        
        reputation = WatcherReputation(
            watcher_id=watcher_id,
            metrics=metrics,
        )
        
        score = reputation.compute_score()
        
        assert 0 <= score <= 100
        assert reputation.tier in ["novice", "standard", "trusted", "elite"]
    
    def test_watcher_selection(self):
        """Test watcher selection for jurisdiction."""
        registry = WatcherRegistry()
        
        # Register and bond multiple watchers
        for i in range(3):
            watcher_id = WatcherId(
                did=f"did:example:watcher{i}",
                public_key_hex=f"{i:02d}" * 32,
            )
            registry.register_watcher(watcher_id)
            
            bond = WatcherBond(
                bond_id=f"bond-{i}",
                watcher_id=watcher_id,
                collateral_amount=Decimal("10000") * (i + 1),
                collateral_currency="USDC",
                collateral_address=f"0x{i:02d}" * 20,
                scope_jurisdictions=frozenset(["uae-difc"]),
            )
            registry.post_bond(bond)
            registry.activate_bond(f"bond-{i}")
        
        # Select watchers
        selected = registry.select_watchers(
            jurisdiction_id="uae-difc",
            min_count=2,
        )
        
        assert len(selected) >= 2
    
    def test_equivocation_detection(self):
        """Test equivocation detection."""
        detector = EquivocationDetector()
        
        # First attestation
        result1 = detector.record_attestation(
            watcher_did="did:example:watcher1",
            corridor_id="corridor-1",
            height=100,
            attestation_digest="abc123",
            attestation_data={"state": "A"},
        )
        assert result1 is None  # No equivocation
        
        # Same height, different digest = equivocation
        result2 = detector.record_attestation(
            watcher_did="did:example:watcher1",
            corridor_id="corridor-1",
            height=100,
            attestation_digest="def456",
            attestation_data={"state": "B"},
        )
        assert result2 is not None
        assert isinstance(result2, EquivocationEvidence)


# =============================================================================
# INTEGRATION TESTS
# =============================================================================

class TestPhoenixIntegration:
    """Integration tests combining multiple PHOENIX components."""
    
    def test_full_migration_flow(self):
        """Test complete migration flow with all components."""
        # Setup manifold
        manifold = create_standard_manifold()
        
        # Create migration request
        request = MigrationRequest(
            asset_id="asset-integration-test",
            asset_genesis_digest="abc" * 21 + "a",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            asset_value_usd=Decimal("100000"),
        )
        
        # Find migration path
        path = manifold.find_path(
            request.source_jurisdiction,
            request.target_jurisdiction,
            asset_value_usd=request.asset_value_usd,
        )
        assert path is not None
        request.migration_path = path
        
        # Create saga
        saga = MigrationSaga(request)
        
        # Create compliance tensor
        tensor = ComplianceTensorV2()
        tensor.set(
            request.asset_id,
            request.source_jurisdiction,
            ComplianceDomain.KYC,
            ComplianceState.COMPLIANT,
        )
        tensor.set(
            request.asset_id,
            request.source_jurisdiction,
            ComplianceDomain.AML,
            ComplianceState.COMPLIANT,
        )
        
        # Record tensor commitment
        saga.set_source_tensor_commitment(tensor.commit())
        
        # Progress saga
        saga.advance_to(MigrationState.COMPLIANCE_CHECK, "Compliance verified")
        saga.advance_to(MigrationState.ATTESTATION_GATHERING, "Gathering attestations")
        
        # Analyze attestation gap
        gap = manifold.analyze_attestation_gap(path, [])
        assert gap.missing_count > 0  # Would need attestations
        
        # Complete migration (simulated)
        saga.advance_to(MigrationState.SOURCE_LOCK)
        saga.advance_to(MigrationState.TRANSIT)
        saga.advance_to(MigrationState.DESTINATION_VERIFICATION)
        saga.advance_to(MigrationState.DESTINATION_UNLOCK)
        saga.complete()
        
        assert saga.is_successful
        assert saga.evidence.source_tensor_commitment is not None
    
    def test_watcher_attestation_with_tensor_update(self):
        """Test watcher attestation updating compliance tensor."""
        # Setup watcher
        registry = WatcherRegistry()
        watcher_id = WatcherId(
            did="did:example:watcher-integration",
            public_key_hex="ab" * 32,
        )
        registry.register_watcher(watcher_id)
        
        bond = WatcherBond(
            bond_id="bond-integration",
            watcher_id=watcher_id,
            collateral_amount=Decimal("50000"),
            collateral_currency="USDC",
            collateral_address="0x" + "ab" * 20,
        )
        registry.post_bond(bond)
        registry.activate_bond("bond-integration")
        
        # Create tensor
        tensor = ComplianceTensorV2()
        
        # Watcher attests to KYC compliance
        attestation = AttestationRef(
            attestation_id="att-kyc-1",
            attestation_type="kyc_verification",
            issuer_did=watcher_id.did,
            issued_at=datetime.now(timezone.utc).isoformat(),
            expires_at=(datetime.now(timezone.utc) + timedelta(days=365)).isoformat(),
            digest=hashlib.sha256(b"kyc-data").hexdigest(),
        )
        
        # Update tensor with attested state
        tensor.set(
            asset_id="asset-attested",
            jurisdiction_id="uae-difc",
            domain=ComplianceDomain.KYC,
            state=ComplianceState.COMPLIANT,
            attestations=[attestation],
        )
        
        # Record attestation in registry
        registry.record_attestation(
            watcher_did=watcher_id.did,
            value_usd=Decimal("100000"),
            on_time=True,
        )
        
        # Verify
        cell = tensor.get("asset-attested", "uae-difc", ComplianceDomain.KYC)
        assert cell.state == ComplianceState.COMPLIANT
        assert len(cell.attestations) == 1
        
        active_bond = registry.get_active_bond(watcher_id.did)
        assert active_bond.attestation_count == 1


# =============================================================================
# EDGE CASE TESTS
# =============================================================================

class TestPhoenixEdgeCases:
    """Tests for edge cases and error handling."""
    
    def test_empty_tensor_commitment(self):
        """Test commitment of empty tensor."""
        tensor = ComplianceTensorV2()
        commitment = tensor.commit()
        
        assert commitment.root == "0" * 64
        assert commitment.cell_count == 0
    
    def test_tensor_with_stale_attestations(self):
        """Test tensor evaluation with expired attestations."""
        past = (datetime.now(timezone.utc) - timedelta(days=1)).isoformat()
        
        attestation = AttestationRef(
            attestation_id="att-expired",
            attestation_type="kyc",
            issuer_did="did:example:issuer",
            issued_at="2024-01-01T00:00:00Z",
            expires_at=past,
            digest="abc",
        )
        
        tensor = ComplianceTensorV2()
        tensor.set(
            "asset-1",
            "uae-difc",
            ComplianceDomain.KYC,
            ComplianceState.COMPLIANT,
            attestations=[attestation],
        )
        
        # Evaluation should flag expired attestation
        is_compliant, state, issues = tensor.evaluate("asset-1", "uae-difc")
        
        assert not is_compliant
        assert "expired" in str(issues).lower()
    
    def test_migration_timeout(self):
        """Test migration saga timeout handling."""
        request = MigrationRequest(
            asset_id="asset-timeout",
            asset_genesis_digest="abc" * 21 + "a",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
        )
        
        saga = MigrationSaga(request)
        
        # Manually set entered time to past
        saga._state_entered_at = datetime.now(timezone.utc) - timedelta(hours=100)
        
        assert saga.is_timed_out
    
    def test_bond_insufficient_for_attestation(self):
        """Test bond cannot attest beyond its limit."""
        watcher_id = WatcherId(
            did="did:example:small-bond",
            public_key_hex="ab" * 32,
        )
        
        bond = WatcherBond(
            bond_id="small-bond",
            watcher_id=watcher_id,
            collateral_amount=Decimal("100"),  # Small bond
            collateral_currency="USDC",
            collateral_address="0x" + "ab" * 20,
            status=BondStatus.ACTIVE,
        )
        
        # Max attestation is 10x collateral = $1000
        assert bond.can_attest(Decimal("500"))
        assert not bond.can_attest(Decimal("5000"))  # Too large
    
    def test_slashing_drains_bond(self):
        """Test that slashing can fully drain a bond."""
        watcher_id = WatcherId(
            did="did:example:slashed",
            public_key_hex="ab" * 32,
        )
        
        bond = WatcherBond(
            bond_id="drain-bond",
            watcher_id=watcher_id,
            collateral_amount=Decimal("1000"),
            collateral_currency="USDC",
            collateral_address="0x" + "ab" * 20,
            status=BondStatus.ACTIVE,
        )
        
        # Slash 100%
        slashed = bond.slash(Decimal("1000"), "equivocation")
        
        assert slashed == Decimal("1000")
        assert bond.available_collateral == Decimal("0")
        assert bond.status == BondStatus.FULLY_SLASHED
    
    def test_excluded_jurisdictions(self):
        """Test path finding excludes specified jurisdictions."""
        manifold = create_standard_manifold()
        
        # Add intermediate jurisdiction
        intermediate = JurisdictionNode(
            jurisdiction_id="intermediate",
            name="Intermediate Zone",
            country_code="XX",
        )
        manifold.add_jurisdiction(intermediate)
        
        # Exclude target
        constraints = PathConstraint(
            excluded_jurisdictions=frozenset(["kz-aifc"])
        )
        
        path = manifold.find_path("uae-difc", "kz-aifc", constraints)
        assert path is None  # Target is excluded


# =============================================================================
# L1 ANCHOR TESTS
# =============================================================================

class TestL1Anchor:
    """Tests for L1 anchoring infrastructure."""
    
    def test_chain_properties(self):
        """Test chain enum properties."""
        from tools.phoenix.anchor import Chain
        
        assert Chain.ETHEREUM.chain_id == 1
        assert Chain.ARBITRUM.chain_id == 42161
        assert Chain.ARBITRUM.is_l2
        assert not Chain.ETHEREUM.is_l2
        assert Chain.ETHEREUM.finality_blocks == 64
    
    def test_checkpoint_creation(self):
        """Test checkpoint creation and digest."""
        from tools.phoenix.anchor import CorridorCheckpoint
        
        checkpoint = CorridorCheckpoint(
            corridor_id="test-corridor",
            checkpoint_height=100,
            receipt_merkle_root="a" * 64,
            state_root="b" * 64,
            timestamp="2026-01-29T00:00:00Z",
            watcher_signatures=[b"sig1", b"sig2"],
            receipt_count=50,
        )
        
        assert len(checkpoint.digest) == 64
        assert checkpoint.receipt_count == 50
    
    def test_checkpoint_digest_determinism(self):
        """Test checkpoint digest is deterministic."""
        from tools.phoenix.anchor import CorridorCheckpoint
        
        c1 = CorridorCheckpoint(
            corridor_id="test-corridor",
            checkpoint_height=100,
            receipt_merkle_root="a" * 64,
            state_root="b" * 64,
            timestamp="2026-01-29T00:00:00Z",
            watcher_signatures=[],
        )
        
        c2 = CorridorCheckpoint(
            corridor_id="test-corridor",
            checkpoint_height=100,
            receipt_merkle_root="a" * 64,
            state_root="b" * 64,
            timestamp="2026-01-29T00:00:00Z",
            watcher_signatures=[],
        )
        
        assert c1.digest == c2.digest
    
    def test_mock_anchor_manager(self):
        """Test mock anchor manager."""
        from tools.phoenix.anchor import (
            create_mock_anchor_manager, CorridorCheckpoint, Chain, AnchorStatus
        )
        
        manager = create_mock_anchor_manager()
        
        checkpoint = CorridorCheckpoint(
            corridor_id="test-corridor",
            checkpoint_height=100,
            receipt_merkle_root="a" * 64,
            state_root="b" * 64,
            timestamp="2026-01-29T00:00:00Z",
            watcher_signatures=[b"sig1"],
        )
        
        anchor = manager.anchor_checkpoint(checkpoint, Chain.ARBITRUM)
        
        assert anchor.anchor_id is not None
        assert anchor.chain == Chain.ARBITRUM
        assert anchor.status == AnchorStatus.FINALIZED
        assert "arbiscan.io" in anchor.explorer_url
    
    def test_anchor_cost_comparison(self):
        """Test comparing anchor costs across chains."""
        from tools.phoenix.anchor import create_mock_anchor_manager, CorridorCheckpoint
        
        manager = create_mock_anchor_manager()
        
        checkpoint = CorridorCheckpoint(
            corridor_id="test-corridor",
            checkpoint_height=100,
            receipt_merkle_root="a" * 64,
            state_root="b" * 64,
            timestamp="2026-01-29T00:00:00Z",
            watcher_signatures=[b"sig1", b"sig2", b"sig3"],
        )
        
        costs = manager.compare_chain_costs(checkpoint)
        
        assert len(costs) == 4  # All 4 chains
        assert all("cost_eth" in c for c in costs)
    
    def test_anchor_retrieval(self):
        """Test anchor retrieval by checkpoint."""
        from tools.phoenix.anchor import (
            create_mock_anchor_manager, CorridorCheckpoint, Chain
        )
        
        manager = create_mock_anchor_manager()
        
        checkpoint = CorridorCheckpoint(
            corridor_id="test-corridor",
            checkpoint_height=100,
            receipt_merkle_root="a" * 64,
            state_root="b" * 64,
            timestamp="2026-01-29T00:00:00Z",
            watcher_signatures=[],
        )
        
        anchor = manager.anchor_checkpoint(checkpoint, Chain.BASE)
        
        retrieved = manager.get_anchor_for_checkpoint(checkpoint.digest)
        assert retrieved is not None
        assert retrieved.anchor_id == anchor.anchor_id
    
    def test_inclusion_proof_verification(self):
        """Test inclusion proof verification."""
        from tools.phoenix.anchor import InclusionProof
        import hashlib
        
        # Create a simple two-leaf Merkle tree
        leaf1 = "a" * 64
        leaf2 = "b" * 64
        root = hashlib.sha256((leaf1 + leaf2).encode()).hexdigest()
        
        # Proof for leaf1 (index 0)
        proof = InclusionProof(
            receipt_digest=leaf1,
            checkpoint_digest="checkpoint-123",
            anchor_id="anchor-456",
            merkle_path=[leaf2],
            merkle_indices=[0],  # leaf1 is left child
            root=root,
            leaf_index=0,
        )
        
        assert proof.verify()
    
    def test_cross_chain_verification(self):
        """Test cross-chain verification."""
        from tools.phoenix.anchor import (
            create_mock_anchor_manager, CorridorCheckpoint, Chain,
            CrossChainVerifier
        )
        
        manager = create_mock_anchor_manager()
        
        checkpoint = CorridorCheckpoint(
            corridor_id="test-corridor",
            checkpoint_height=100,
            receipt_merkle_root="a" * 64,
            state_root="b" * 64,
            timestamp="2026-01-29T00:00:00Z",
            watcher_signatures=[],
        )
        
        # Anchor to multiple chains
        manager.anchor_checkpoint(checkpoint, Chain.ETHEREUM)
        manager.anchor_checkpoint(checkpoint, Chain.ARBITRUM)
        
        verifier = CrossChainVerifier(manager)
        result = verifier.verify_across_chains(
            checkpoint,
            chains=[Chain.ETHEREUM, Chain.ARBITRUM]
        )
        
        assert result.verification_count == 2
        assert result.all_verified


# =============================================================================
# CORRIDOR BRIDGE TESTS
# =============================================================================

class TestCorridorBridge:
    """Tests for corridor bridge protocol."""
    
    def test_bridge_creation(self):
        """Test bridge creation."""
        from tools.phoenix.bridge import create_bridge_with_manifold
        
        bridge = create_bridge_with_manifold()
        assert bridge is not None
    
    def test_bridge_execution_success(self):
        """Test successful bridge execution."""
        from tools.phoenix.bridge import (
            create_bridge_with_manifold, BridgeRequest, BridgePhase
        )
        from decimal import Decimal
        
        bridge = create_bridge_with_manifold()
        
        request = BridgeRequest(
            bridge_id="test-bridge-001",
            asset_id="asset-123",
            asset_genesis_digest="a" * 64,
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            amount=Decimal("100000"),
            currency="USD",
        )
        
        execution = bridge.execute(request)
        
        assert execution.phase == BridgePhase.COMPLETED
        assert execution.is_successful
        assert len(execution.hops) == 1
    
    def test_bridge_hop_receipts(self):
        """Test bridge generates receipts at each hop."""
        from tools.phoenix.bridge import (
            create_bridge_with_manifold, BridgeRequest
        )
        from decimal import Decimal
        
        bridge = create_bridge_with_manifold()
        
        request = BridgeRequest(
            bridge_id="test-bridge-002",
            asset_id="asset-456",
            asset_genesis_digest="b" * 64,
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            amount=Decimal("50000"),
            currency="USD",
        )
        
        execution = bridge.execute(request)
        
        # Check each hop has receipts
        for hop in execution.hops:
            assert hop.prepare_receipt is not None
            assert hop.commit_receipt is not None
            assert len(hop.prepare_receipt.digest) == 64
            assert len(hop.commit_receipt.digest) == 64
    
    def test_bridge_no_path(self):
        """Test bridge fails gracefully when no path exists."""
        from tools.phoenix.bridge import (
            CorridorBridge, BridgeRequest, BridgePhase
        )
        from tools.phoenix.manifold import ComplianceManifold
        from decimal import Decimal
        
        # Empty manifold - no paths
        manifold = ComplianceManifold()
        bridge = CorridorBridge(manifold)
        
        request = BridgeRequest(
            bridge_id="test-bridge-003",
            asset_id="asset-789",
            asset_genesis_digest="c" * 64,
            source_jurisdiction="nowhere",
            target_jurisdiction="elsewhere",
            amount=Decimal("1000"),
            currency="USD",
        )
        
        execution = bridge.execute(request)
        
        assert execution.phase == BridgePhase.FAILED
        assert "No path found" in execution.fatal_error
    
    def test_bridge_fee_constraint(self):
        """Test bridge respects fee constraints."""
        from tools.phoenix.bridge import (
            create_bridge_with_manifold, BridgeRequest, BridgePhase
        )
        from decimal import Decimal
        
        bridge = create_bridge_with_manifold()
        
        # Request with very low fee tolerance
        request = BridgeRequest(
            bridge_id="test-bridge-004",
            asset_id="asset-abc",
            asset_genesis_digest="d" * 64,
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            amount=Decimal("1000"),  # Low amount
            currency="USD",
            max_fee_bps=1,  # 0.01% - impossibly low
        )
        
        execution = bridge.execute(request)
        
        assert execution.phase == BridgePhase.FAILED
        assert "fees" in execution.fatal_error.lower()
    
    def test_bridge_statistics(self):
        """Test bridge statistics."""
        from tools.phoenix.bridge import (
            create_bridge_with_manifold, BridgeRequest
        )
        from decimal import Decimal
        
        bridge = create_bridge_with_manifold()
        
        # Execute a few bridges with sufficient fee tolerance
        for i in range(3):
            request = BridgeRequest(
                bridge_id=f"stats-test-{i}",
                asset_id=f"asset-{i}",
                asset_genesis_digest=f"{i+1}" * 64,  # Avoid empty string
                source_jurisdiction="uae-difc",
                target_jurisdiction="kz-aifc",
                amount=Decimal("10000") * (i + 1),
                currency="USD",
                max_fee_bps=1000,  # 10% tolerance for fees
            )
            bridge.execute(request)
        
        stats = bridge.get_statistics()
        
        assert stats["total_bridges"] == 3
        assert stats["completed"] == 3
    
    def test_receipt_chain(self):
        """Test bridge receipt chain."""
        from tools.phoenix.bridge import BridgeReceiptChain, PrepareReceipt, CommitReceipt
        from decimal import Decimal
        from datetime import datetime, timezone, timedelta
        
        chain = BridgeReceiptChain()
        
        # Add prepare receipt
        prep = PrepareReceipt(
            receipt_id="prep-001",
            hop_index=0,
            corridor_id="corridor-1",
            asset_id="asset-1",
            lock_id="lock-001",
            locked_amount=Decimal("1000"),
            lock_expiry=(datetime.now(timezone.utc) + timedelta(hours=1)).isoformat(),
            source_signature=b"sig",
            corridor_signature=b"sig",
            compliance_tensor_slice={},
        )
        chain.add_prepare_receipt("bridge-001", prep)
        
        # Add commit receipt
        commit = CommitReceipt(
            receipt_id="commit-001",
            hop_index=0,
            corridor_id="corridor-1",
            asset_id="asset-1",
            prepare_receipt_digest=prep.digest,
            transfer_amount=Decimal("1000"),
            settlement_tx_id="0x" + "a" * 64,
            settlement_block=1000000,
            corridor_signature=b"sig",
            target_signature=b"sig",
        )
        chain.add_commit_receipt("bridge-001", commit)
        
        # Get receipts
        receipts = chain.get_bridge_receipts("bridge-001")
        assert len(receipts["prepare_receipts"]) == 1
        assert len(receipts["commit_receipts"]) == 1
        
        # Compute Merkle root
        root = chain.compute_merkle_root("bridge-001")
        assert len(root) == 64


# =============================================================================
# FULL SYSTEM INTEGRATION TEST
# =============================================================================

class TestPhoenixFullSystem:
    """Full system integration tests combining all PHOENIX components."""
    
    def test_complete_cross_jurisdictional_migration(self):
        """Test complete migration with all components."""
        from tools.phoenix.tensor import ComplianceTensorV2, ComplianceDomain, ComplianceState, AttestationRef
        from tools.phoenix.manifold import create_standard_manifold
        from tools.phoenix.migration import MigrationSaga, MigrationRequest, MigrationState
        from tools.phoenix.watcher import WatcherRegistry, WatcherId, WatcherBond
        from tools.phoenix.anchor import create_mock_anchor_manager, CorridorCheckpoint
        from tools.phoenix.bridge import create_bridge_with_manifold, BridgeRequest
        from decimal import Decimal
        from datetime import datetime, timezone, timedelta
        import hashlib
        
        # 1. Setup watcher infrastructure
        registry = WatcherRegistry()
        watcher_id = WatcherId(
            did="did:example:elite-watcher",
            public_key_hex="ab" * 32,
        )
        registry.register_watcher(watcher_id)
        
        bond = WatcherBond(
            bond_id="bond-elite",
            watcher_id=watcher_id,
            collateral_amount=Decimal("100000"),
            collateral_currency="USDC",
            collateral_address="0x" + "ab" * 20,
        )
        registry.post_bond(bond)
        registry.activate_bond("bond-elite")
        
        # 2. Create compliance tensor for asset
        tensor = ComplianceTensorV2()
        asset_id = "asset-full-integration"
        
        # Watcher provides attestations
        attestation = AttestationRef(
            attestation_id="att-full-integration",
            attestation_type="kyc_verification",
            issuer_did=watcher_id.did,
            issued_at=datetime.now(timezone.utc).isoformat(),
            expires_at=(datetime.now(timezone.utc) + timedelta(days=365)).isoformat(),
            digest=hashlib.sha256(b"full-integration-kyc").hexdigest(),
        )
        
        tensor.set(
            asset_id, "uae-difc", ComplianceDomain.KYC,
            ComplianceState.COMPLIANT, attestations=[attestation]
        )
        tensor.set(
            asset_id, "uae-difc", ComplianceDomain.AML,
            ComplianceState.COMPLIANT, attestations=[attestation]
        )
        tensor.set(
            asset_id, "uae-difc", ComplianceDomain.SANCTIONS,
            ComplianceState.COMPLIANT, attestations=[attestation]
        )
        
        # Record attestation in registry
        registry.record_attestation(watcher_id.did, Decimal("500000"), on_time=True)
        
        # 3. Execute bridge transfer
        bridge = create_bridge_with_manifold()
        bridge_request = BridgeRequest(
            bridge_id="full-integration-bridge",
            asset_id=asset_id,
            asset_genesis_digest="full" * 16,
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            amount=Decimal("500000"),
            currency="USD",
            max_fee_bps=1000,  # 10% tolerance for test
        )
        
        execution = bridge.execute(bridge_request, existing_attestations=[attestation])
        assert execution.is_successful, f"Bridge failed: {execution.fatal_error}"
        
        # 4. Anchor checkpoint to L1
        anchor_manager = create_mock_anchor_manager()
        
        checkpoint = CorridorCheckpoint(
            corridor_id="corridor-difc-aifc",
            checkpoint_height=1,
            receipt_merkle_root=execution.hops[0].commit_receipt.digest if execution.hops else "0" * 64,
            state_root=tensor.commit().root,
            timestamp=datetime.now(timezone.utc).isoformat(),
            watcher_signatures=[b"watcher-sig"],
            receipt_count=1,
        )
        
        from tools.phoenix.anchor import Chain
        anchor = anchor_manager.anchor_checkpoint(checkpoint, Chain.ARBITRUM)
        assert anchor.is_final
        
        # 5. Update tensor for destination jurisdiction
        tensor.set(
            asset_id, "kz-aifc", ComplianceDomain.KYC,
            ComplianceState.COMPLIANT, attestations=[attestation]
        )
        tensor.set(
            asset_id, "kz-aifc", ComplianceDomain.AML,
            ComplianceState.COMPLIANT, attestations=[attestation]
        )
        tensor.set(
            asset_id, "kz-aifc", ComplianceDomain.SANCTIONS,
            ComplianceState.COMPLIANT, attestations=[attestation]
        )
        
        # 6. Verify final compliance for specific domains
        is_compliant, state, issues = tensor.evaluate(
            asset_id, "kz-aifc",
            domains={ComplianceDomain.KYC, ComplianceDomain.AML, ComplianceDomain.SANCTIONS}
        )
        assert is_compliant, f"Compliance issues: {issues}"
        assert state == ComplianceState.COMPLIANT
        
        # 7. Verify all components integrated correctly
        assert registry.get_active_bond(watcher_id.did) is not None
        assert anchor_manager.get_anchor_for_checkpoint(checkpoint.digest) is not None
        assert bridge.get_execution("full-integration-bridge").is_successful


# =============================================================================
# HARDENING MODULE TESTS
# =============================================================================

class TestHardeningModule:
    """Tests for the hardening module."""
    
    def test_string_validation(self):
        """Test string validation with various inputs."""
        from tools.phoenix.hardening import Validators
        
        # Valid asset ID
        result = Validators.validate_asset_id("valid-asset-123")
        assert result.is_valid
        
        # Invalid: too long
        result = Validators.validate_asset_id("x" * 200)
        assert not result.is_valid
        
        # Invalid: special characters
        result = Validators.validate_asset_id("asset@#$%")
        assert not result.is_valid
    
    def test_digest_validation(self):
        """Test SHA256 digest validation."""
        from tools.phoenix.hardening import Validators
        
        # Valid digest
        result = Validators.validate_digest("a" * 64)
        assert result.is_valid
        
        # Invalid: wrong length
        result = Validators.validate_digest("a" * 63)
        assert not result.is_valid
        
        # Invalid: non-hex characters
        result = Validators.validate_digest("g" * 64)
        assert not result.is_valid
    
    def test_amount_validation(self):
        """Test monetary amount validation."""
        from tools.phoenix.hardening import Validators
        from decimal import Decimal
        
        # Valid amount
        result = Validators.validate_amount("1000.50")
        assert result.is_valid
        assert result.sanitized_value == Decimal("1000.50")
        
        # Invalid: negative
        result = Validators.validate_amount("-100")
        assert not result.is_valid
        
        # Invalid: exceeds max
        result = Validators.validate_amount("999999999999999")
        assert not result.is_valid
    
    def test_timestamp_validation(self):
        """Test timestamp validation."""
        from tools.phoenix.hardening import Validators
        from datetime import datetime, timezone, timedelta
        
        # Valid recent timestamp
        now = datetime.now(timezone.utc)
        result = Validators.validate_timestamp(now.isoformat())
        assert result.is_valid
        
        # Invalid: too old
        old = now - timedelta(days=400)
        result = Validators.validate_timestamp(old.isoformat(), max_age_days=365)
        assert not result.is_valid
    
    def test_thread_safe_dict(self):
        """Test thread-safe dictionary operations."""
        from tools.phoenix.hardening import ThreadSafeDict
        import threading
        
        d = ThreadSafeDict()
        results = []
        
        def writer(key, value):
            d[key] = value
            results.append((key, d.get(key)))
        
        threads = [
            threading.Thread(target=writer, args=(f"key{i}", f"value{i}"))
            for i in range(10)
        ]
        
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        
        assert len(d) == 10
        assert len(results) == 10
    
    def test_atomic_counter(self):
        """Test atomic counter operations."""
        from tools.phoenix.hardening import AtomicCounter
        import threading
        
        counter = AtomicCounter(0)
        
        def incrementer():
            for _ in range(100):
                counter.increment()
        
        threads = [threading.Thread(target=incrementer) for _ in range(10)]
        
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        
        assert counter.get() == 1000
    
    def test_crypto_utils_merkle(self):
        """Test Merkle root computation."""
        from tools.phoenix.hardening import CryptoUtils
        
        # Single leaf
        root1 = CryptoUtils.merkle_root(["a" * 64])
        assert root1 == "a" * 64
        
        # Two leaves
        root2 = CryptoUtils.merkle_root(["a" * 64, "b" * 64])
        assert len(root2) == 64
        
        # Verify proof
        leaves = ["a" * 64, "b" * 64, "c" * 64, "d" * 64]
        root = CryptoUtils.merkle_root(leaves)
        
        # Proof for leaf 0
        import hashlib
        sibling_01 = leaves[1]
        parent_01 = hashlib.sha256((leaves[0] + sibling_01).encode()).hexdigest()
        sibling_23 = hashlib.sha256((leaves[2] + leaves[3]).encode()).hexdigest()
        
        valid = CryptoUtils.verify_merkle_proof(
            leaves[0],
            [sibling_01, sibling_23],
            [0, 0],
            root
        )
        assert valid
    
    def test_economic_guard(self):
        """Test economic attack prevention."""
        from tools.phoenix.hardening import EconomicGuard, EconomicAttackDetected
        from decimal import Decimal
        
        # Valid attestation
        EconomicGuard.check_attestation_limit(
            Decimal("10000"),  # Bond
            Decimal("50000"),  # Attestation (5x, under 10x limit)
        )
        
        # Invalid: exceeds limit
        import pytest
        with pytest.raises(EconomicAttackDetected):
            EconomicGuard.check_attestation_limit(
                Decimal("10000"),   # Bond
                Decimal("200000"),  # Attestation (20x, over 10x limit)
            )


# =============================================================================
# SECURITY MODULE TESTS
# =============================================================================

class TestSecurityModule:
    """Tests for the security module."""
    
    def test_attestation_scope(self):
        """Test attestation scope binding."""
        from tools.phoenix.security import AttestationScope
        from datetime import datetime, timezone, timedelta
        
        now = datetime.now(timezone.utc)
        scope = AttestationScope(
            asset_id="asset-123",
            jurisdiction_id="uae-difc",
            domain="kyc",
            valid_from=now.isoformat(),
            valid_until=(now + timedelta(days=365)).isoformat(),
        )
        
        # Check includes
        assert scope.includes("asset-123", "uae-difc", "kyc")
        assert not scope.includes("asset-456", "uae-difc", "kyc")
        assert not scope.includes("asset-123", "kz-aifc", "kyc")
        
        # Check time validity
        assert scope.is_valid_at(now + timedelta(days=100))
        assert not scope.is_valid_at(now + timedelta(days=400))
    
    def test_scoped_attestation(self):
        """Test scoped attestation creation and verification."""
        from tools.phoenix.security import AttestationScope, ScopedAttestation
        from datetime import datetime, timezone, timedelta
        
        now = datetime.now(timezone.utc)
        scope = AttestationScope(
            asset_id="asset-123",
            jurisdiction_id="uae-difc",
            domain="kyc",
            valid_from=now.isoformat(),
            valid_until=(now + timedelta(days=365)).isoformat(),
        )
        
        attestation = ScopedAttestation.create(
            attestation_id="att-001",
            attestation_type="kyc_verification",
            issuer_did="did:example:issuer",
            scope=scope,
            issuer_signature=b"signature" * 8,
        )
        
        # Verify scope
        assert attestation.verify_scope("asset-123", "uae-difc", "kyc")
        assert not attestation.verify_scope("asset-456", "uae-difc", "kyc")
    
    def test_nonce_registry_replay_prevention(self):
        """Test nonce registry prevents replay attacks."""
        from tools.phoenix.security import NonceRegistry
        
        registry = NonceRegistry()
        
        # First use succeeds
        assert registry.check_and_register("nonce-001")
        
        # Replay fails
        assert not registry.check_and_register("nonce-001")
        
        # Different nonce succeeds
        assert registry.check_and_register("nonce-002")
    
    def test_versioned_store_cas(self):
        """Test versioned store compare-and-swap."""
        from tools.phoenix.security import VersionedStore
        
        store = VersionedStore()
        
        # Initial set
        v1 = store.set("key1", {"value": 1})
        assert v1.version == 1
        
        # CAS with correct version succeeds
        success, v2 = store.compare_and_swap("key1", 1, {"value": 2})
        assert success
        assert v2.value == {"value": 2}
        
        # CAS with old version fails
        success, current = store.compare_and_swap("key1", 1, {"value": 3})
        assert not success
        assert current.value == {"value": 2}
    
    def test_time_lock_manager(self):
        """Test time-locked operations."""
        from tools.phoenix.security import TimeLockManager, TimeLockState
        from datetime import datetime, timezone
        import hashlib
        
        manager = TimeLockManager()
        
        # Announce operation
        operation_data = b"withdraw:1000:0xabc"
        commitment = hashlib.sha256(operation_data).hexdigest()
        
        lock = manager.announce(
            operation_type="test",
            operator_did="did:test:operator",
            operation_commitment=commitment,
            delay_hours=0,  # Immediate for testing
            expiry_hours=1,
        )
        
        assert lock.state == TimeLockState.PENDING
        
        # Execute with correct data
        success, msg = manager.execute(lock.lock_id, operation_data)
        assert success, msg
        
        # Cannot execute again
        success, msg = manager.execute(lock.lock_id, operation_data)
        assert not success
    
    def test_audit_logger_chain_integrity(self):
        """Test audit log chain integrity."""
        from tools.phoenix.security import AuditLogger, AuditEventType
        
        logger = AuditLogger()
        
        # Log several events
        for i in range(5):
            logger.log(
                event_type=AuditEventType.STATE_CREATED,
                actor_did=f"did:test:actor{i}",
                resource_type="test",
                resource_id=f"res-{i}",
                action="create",
                outcome="success",
            )
        
        # Verify chain
        valid, idx = logger.verify_chain()
        assert valid
        assert idx is None
    
    def test_secure_withdrawal_manager(self):
        """Test secure withdrawal with time lock."""
        from tools.phoenix.security import SecureWithdrawalManager, AuditLogger
        from decimal import Decimal
        
        audit = AuditLogger()
        manager = SecureWithdrawalManager(audit)
        
        # Request withdrawal
        success, result = manager.request_withdrawal(
            watcher_did="did:test:watcher",
            bond_id="bond-001",
            amount=Decimal("5000"),
            destination_address="0x" + "ab" * 20,
            current_collateral=Decimal("10000"),
            active_attestation_value=Decimal("20000"),
        )
        
        assert success
        assert result.state == "pending"
        
        # Cannot execute immediately (time locked)
        success, msg = manager.execute_withdrawal(
            result.request_id,
            current_collateral=Decimal("10000"),
            active_slashing_claims=0,
        )
        assert not success
        assert "not yet unlocked" in msg.lower()


# =============================================================================
# VM MODULE TESTS
# =============================================================================

class TestVMModule:
    """Tests for the Smart Asset VM module."""
    
    def test_word_arithmetic(self):
        """Test 256-bit word arithmetic."""
        from tools.phoenix.vm import Word
        
        # Basic arithmetic
        a = Word.from_int(100)
        b = Word.from_int(50)
        
        assert (a + b).to_int() == 150
        assert (a - b).to_int() == 50
        assert (a * b).to_int() == 5000
        assert (a / b).to_int() == 2
        assert (a % b).to_int() == 0
    
    def test_word_overflow(self):
        """Test word overflow wraps correctly."""
        from tools.phoenix.vm import Word
        
        max_val = (1 << 256) - 1
        w = Word.from_int(max_val)
        result = w + Word.one()
        
        assert result.to_int() == 0  # Wraps to 0
    
    def test_word_negative(self):
        """Test two's complement negative numbers."""
        from tools.phoenix.vm import Word
        
        w = Word.from_int(-1)
        
        # As unsigned should be max value
        assert w.to_int(signed=False) == (1 << 256) - 1
        
        # As signed should be -1
        assert w.to_int(signed=True) == -1
    
    def test_vm_basic_execution(self):
        """Test basic VM execution."""
        from tools.phoenix.vm import SmartAssetVM, ExecutionContext, Assembler
        
        vm = SmartAssetVM()
        
        # Simple program: PUSH 42, PUSH 0, SSTORE, HALT
        bytecode = Assembler.assemble([
            ('PUSH1', 42),
            ('PUSH1', 0),
            ('SSTORE',),
            ('HALT',),
        ])
        
        context = ExecutionContext(
            caller="did:test:caller",
            origin="did:test:origin",
            jurisdiction_id="uae-difc",
        )
        
        result = vm.execute(bytecode, context)
        
        assert result.success
        assert result.gas_used > 0
    
    def test_vm_arithmetic_operations(self):
        """Test VM arithmetic operations."""
        from tools.phoenix.vm import SmartAssetVM, ExecutionContext, Assembler
        
        vm = SmartAssetVM()
        
        # Calculate: (10 + 5) * 2 = 30, store at slot 0
        bytecode = Assembler.assemble([
            ('PUSH1', 10),
            ('PUSH1', 5),
            ('ADD',),
            ('PUSH1', 2),
            ('MUL',),
            ('PUSH1', 0),  # Storage slot
            ('SSTORE',),
            ('HALT',),
        ])
        
        context = ExecutionContext(
            caller="did:test:calc",
            origin="did:test:calc",
            jurisdiction_id="test",
        )
        
        result = vm.execute(bytecode, context)
        
        assert result.success
    
    def test_vm_stack_overflow(self):
        """Test VM detects stack overflow."""
        from tools.phoenix.vm import SmartAssetVM, ExecutionContext
        
        vm = SmartAssetVM()
        
        # Push 257 values (max is 256)
        bytecode = bytes([0x01, 0x42] * 257)  # PUSH1 0x42 repeated
        
        context = ExecutionContext(
            caller="did:test:overflow",
            origin="did:test:overflow",
            jurisdiction_id="test",
            gas_limit=10000000,
        )
        
        result = vm.execute(bytecode, context)
        
        assert not result.success
        assert "overflow" in result.error.lower()
    
    def test_vm_out_of_gas(self):
        """Test VM detects out of gas."""
        from tools.phoenix.vm import SmartAssetVM, ExecutionContext, Assembler
        
        vm = SmartAssetVM()
        
        # SSTORE costs 20000 gas
        bytecode = Assembler.assemble([
            ('PUSH1', 42),
            ('PUSH1', 0),
            ('SSTORE',),
        ])
        
        context = ExecutionContext(
            caller="did:test:gas",
            origin="did:test:gas",
            jurisdiction_id="test",
            gas_limit=100,  # Too low
        )
        
        result = vm.execute(bytecode, context)
        
        assert not result.success
        assert "gas" in result.error.lower()
    
    def test_vm_invalid_jump(self):
        """Test VM rejects invalid jump destinations."""
        from tools.phoenix.vm import SmartAssetVM, ExecutionContext, Assembler
        
        vm = SmartAssetVM()
        
        # Jump to invalid destination
        bytecode = Assembler.assemble([
            ('PUSH1', 0xFF),  # Invalid destination
            ('JUMP',),
        ])
        
        context = ExecutionContext(
            caller="did:test:jump",
            origin="did:test:jump",
            jurisdiction_id="test",
        )
        
        result = vm.execute(bytecode, context)
        
        assert not result.success
        assert "jump" in result.error.lower()
    
    def test_vm_compliance_coprocessor(self):
        """Test VM compliance coprocessor operations."""
        from tools.phoenix.vm import ComplianceCoprocessor
        
        coproc = ComplianceCoprocessor()
        
        # Set compliance state
        from tools.phoenix.tensor import ComplianceDomain, ComplianceState
        success = coproc.tensor_set(
            "asset-123",
            "uae-difc",
            ComplianceDomain.KYC.value,
            ComplianceState.COMPLIANT.value,
        )
        assert success
        
        # Get compliance state
        state_code, has_expired = coproc.tensor_get(
            "asset-123",
            "uae-difc",
            ComplianceDomain.KYC.value,
        )
        assert state_code == ComplianceState.COMPLIANT.value
    
    def test_vm_migration_coprocessor(self):
        """Test VM migration coprocessor operations."""
        from tools.phoenix.vm import MigrationCoprocessor
        
        coproc = MigrationCoprocessor()
        
        # Lock asset
        lock_id = coproc.lock(
            "asset-123",
            "uae-difc",
            amount=1000,
            lock_duration_seconds=3600,
        )
        assert lock_id is not None
        
        # Begin transit
        transit_id = coproc.transit_begin(lock_id, "kz-aifc")
        assert transit_id is not None
        
        # End transit
        success = coproc.transit_end(transit_id)
        assert success
        
        # Settle
        success = coproc.settle(transit_id)
        assert success
    
    def test_assembler_disassembler(self):
        """Test bytecode assembler and disassembler."""
        from tools.phoenix.vm import Assembler
        
        instructions = [
            ('PUSH1', 0x42),
            ('PUSH1', 0x00),
            ('SSTORE',),
            ('HALT',),
        ]
        
        # Assemble
        bytecode = Assembler.assemble(instructions)
        assert len(bytecode) == 6  # PUSH1 x + PUSH1 0 + SSTORE + HALT
        
        # Disassemble
        disasm = Assembler.disassemble(bytecode)
        assert len(disasm) == 4
        assert disasm[0][1] == "PUSH1"
        assert disasm[0][2] == 0x42
        assert disasm[3][1] == "HALT"


# =============================================================================
# INTEGRATED SECURITY TESTS
# =============================================================================

class TestIntegratedSecurity:
    """Integration tests for security features."""
    
    def test_end_to_end_secure_migration(self):
        """Test complete migration with all security features."""
        from tools.phoenix.security import (
            NonceRegistry, AuditLogger, AuditEventType,
            AttestationScope, ScopedAttestation
        )
        from tools.phoenix.vm import SmartAssetVM, ExecutionContext, MigrationCoprocessor
        from datetime import datetime, timezone, timedelta
        
        # Setup security infrastructure
        nonce_registry = NonceRegistry()
        audit_logger = AuditLogger()
        
        # Create scoped attestation
        now = datetime.now(timezone.utc)
        scope = AttestationScope(
            asset_id="secure-asset-001",
            jurisdiction_id="uae-difc",
            domain="kyc",
            valid_from=now.isoformat(),
            valid_until=(now + timedelta(days=365)).isoformat(),
        )
        
        attestation = ScopedAttestation.create(
            attestation_id="att-secure-001",
            attestation_type="kyc_verification",
            issuer_did="did:example:trusted-issuer",
            scope=scope,
            issuer_signature=b"sig" * 22,  # 66 bytes
        )
        
        # Verify attestation scope
        assert attestation.verify_scope("secure-asset-001", "uae-difc", "kyc")
        
        # Check nonce is fresh
        assert nonce_registry.check_and_register(attestation.nonce)
        
        # Log migration start
        audit_logger.log(
            event_type=AuditEventType.MIGRATION_STARTED,
            actor_did="did:test:migrator",
            resource_type="asset",
            resource_id="secure-asset-001",
            action="migrate",
            outcome="success",
            details={"attestation_id": attestation.attestation_id},
        )
        
        # Execute migration via coprocessor
        migration = MigrationCoprocessor()
        lock_id = migration.lock("secure-asset-001", "uae-difc", 1000000, 3600)
        transit_id = migration.transit_begin(lock_id, "kz-aifc")
        migration.transit_end(transit_id)
        migration.settle(transit_id)
        
        # Log migration completion
        audit_logger.log(
            event_type=AuditEventType.MIGRATION_COMPLETED,
            actor_did="did:test:migrator",
            resource_type="asset",
            resource_id="secure-asset-001",
            action="migrate",
            outcome="success",
            details={"transit_id": transit_id},
        )
        
        # Verify audit chain
        valid, idx = audit_logger.verify_chain()
        assert valid
        
        # Verify replay protection
        assert not nonce_registry.check_and_register(attestation.nonce)
