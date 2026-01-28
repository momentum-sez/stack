"""Comprehensive tests for RegPack and Arbitration systems (v0.4.41).

These tests validate the existing regpack and arbitration modules against their
actual implementation APIs.
"""

import json
from datetime import datetime, date, timedelta
from decimal import Decimal
from pathlib import Path

import pytest

import sys
REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT))


# ─────────────────────────────────────────────────────────────────────────────
# RegPack Tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRegPackStructure:
    """Test RegPack data structure and serialization."""
    
    def test_regpack_metadata_creation(self):
        from tools.regpack import RegPackMetadata
        
        metadata = RegPackMetadata(
            regpack_id="regpack:test-jurisdiction-2026q1",
            jurisdiction_id="test-jurisdiction",
            domain="financial",
            as_of_date=date(2026, 1, 15),
            snapshot_type="quarterly",
        )
        
        data = metadata.to_dict()
        
        assert data["type"] == "MSEZRegPackMetadata"
        assert data["regpack_id"] == "regpack:test-jurisdiction-2026q1"
        assert data["jurisdiction_id"] == "test-jurisdiction"
        assert data["domain"] == "financial"
    
    def test_regpack_manager_creation(self):
        from tools.regpack import RegPackManager
        
        manager = RegPackManager(
            jurisdiction_id="uae-adgm",
            domain="financial",
        )
        
        assert manager.jurisdiction_id == "uae-adgm"
        assert manager.domain == "financial"


class TestSanctionsEntry:
    """Test sanctions entry structure."""
    
    def test_sanctions_entry_creation(self):
        from tools.regpack import SanctionsEntry
        
        entry = SanctionsEntry(
            entry_id="OFAC-12345",
            entry_type="individual",
            source_lists=["ofac_sdn"],
            primary_name="JOHN DOE",
            aliases=[{"name": "DOE JOHN", "type": "alias"}],
            programs=["SDGT", "IRAN"],
        )
        
        data = entry.to_dict()
        
        assert data["entry_id"] == "OFAC-12345"
        assert data["entry_type"] == "individual"
        assert data["primary_name"] == "JOHN DOE"
        assert "SDGT" in data["programs"]


class TestSanctionsChecker:
    """Test sanctions checking functionality."""
    
    def test_sanctions_checker_creation(self):
        from tools.regpack import SanctionsEntry, SanctionsChecker
        
        entries = [
            SanctionsEntry("1", "individual", ["ofac_sdn"], "JOHN DOE", 
                          aliases=[{"name": "JOHNNY DOE", "type": "alias"}]),
            SanctionsEntry("2", "entity", ["ofac_sdn"], "ACME CORP",
                          aliases=[{"name": "ACME CORPORATION", "type": "alias"}]),
        ]
        
        checker = SanctionsChecker(entries=entries, snapshot_id="test-snapshot")
        
        assert checker.snapshot_id == "test-snapshot"
        assert len(checker.entries) == 2
    
    def test_sanctions_check_entity(self):
        from tools.regpack import SanctionsEntry, SanctionsChecker
        
        entries = [
            SanctionsEntry("1", "individual", ["ofac_sdn"], "JOHN DOE"),
            SanctionsEntry("2", "entity", ["ofac_sdn"], "BLOCKED CORP"),
        ]
        
        checker = SanctionsChecker(entries=entries, snapshot_id="test")
        
        # Should find match
        result = checker.check_entity("JOHN DOE")
        assert result.matched is True
        
        # Should not find unknown person
        result = checker.check_entity("COMPLETELY UNKNOWN XYZ999")
        assert result.matched is False


class TestLicenseType:
    """Test license type functionality."""
    
    def test_license_type_creation(self):
        from tools.regpack import LicenseType
        
        lt = LicenseType(
            license_type_id="cat3a",
            name="Category 3A",
            regulator_id="test-regulator",
        )
        
        data = lt.to_dict()
        
        assert data["license_type_id"] == "cat3a"
        assert data["name"] == "Category 3A"


class TestRegulatorProfile:
    """Test regulator profile functionality."""
    
    def test_regulator_profile_creation(self):
        from tools.regpack import RegulatorProfile
        
        profile = RegulatorProfile(
            regulator_id="adgm-fsra",
            name="ADGM Financial Services Regulatory Authority",
            jurisdiction_id="uae-adgm",
            parent_authority="adgm-ra",
            scope={
                "asset_classes": ["securities", "derivatives"],
                "activities": ["dealing", "custody"],
            },
        )
        
        data = profile.to_dict()
        
        assert data["regulator_id"] == "adgm-fsra"
        assert data["jurisdiction_id"] == "uae-adgm"
        assert "securities" in data["scope"]["asset_classes"]


# ─────────────────────────────────────────────────────────────────────────────
# Arbitration Tests
# ─────────────────────────────────────────────────────────────────────────────

class TestArbitrationParty:
    """Test arbitration party structure."""
    
    def test_party_creation(self):
        from tools.arbitration import Party
        
        party = Party(
            party_id="did:key:z6MkClaimant",
            legal_name="Claimant Corp",
            jurisdiction_id="uae-adgm",
        )
        
        data = party.to_dict()
        
        assert data["party_id"] == "did:key:z6MkClaimant"
        assert data["legal_name"] == "Claimant Corp"
        assert data["jurisdiction_id"] == "uae-adgm"


class TestMoney:
    """Test Money class."""
    
    def test_money_creation(self):
        from tools.arbitration import Money
        
        money = Money(amount=Decimal("250000"), currency="USD")
        
        data = money.to_dict()
        
        assert data["amount"] == 250000.0
        assert data["currency"] == "USD"
    
    def test_money_from_dict(self):
        from tools.arbitration import Money
        
        money = Money.from_dict({"amount": 100000, "currency": "EUR"})
        
        assert money.amount == Decimal("100000")
        assert money.currency == "EUR"


class TestClaim:
    """Test Claim class."""
    
    def test_claim_creation(self):
        from tools.arbitration import Claim, Money
        
        claim = Claim(
            claim_id="claim-001",
            claim_type="breach_of_contract",
            description="Failure to deliver conforming goods",
            amount=Money(Decimal("250000"), "USD"),
        )
        
        data = claim.to_dict()
        
        assert data["claim_id"] == "claim-001"
        assert data["claim_type"] == "breach_of_contract"
        assert data["amount"]["amount"] == 250000.0


class TestDisputeRequest:
    """Test dispute filing workflow."""
    
    def test_dispute_request_creation(self):
        from tools.arbitration import DisputeRequest, Party, Claim, Money
        
        claimant = Party(
            party_id="did:key:z6MkClaimant",
            legal_name="Claimant Corp",
        )
        
        respondent = Party(
            party_id="did:key:z6MkRespondent",
            legal_name="Respondent Corp",
        )
        
        claims = [
            Claim(
                claim_id="claim-001",
                claim_type="breach_of_contract",
                description="Failure to deliver conforming goods",
                amount=Money(Decimal("250000"), "USD"),
            ),
        ]
        
        dispute = DisputeRequest(
            dispute_id="dr-2026-001",
            institution_id="difc-lcia",
            corridor_id="corridor:uae-kaz-trade",
            claimant=claimant,
            respondent=respondent,
            dispute_type="breach_of_contract",
            claims=claims,
        )
        
        data = dispute.to_dict()
        
        assert data["type"] == "MSEZDisputeRequest"
        assert data["claimant"]["party_id"] == "did:key:z6MkClaimant"
        assert len(data["claims"]) == 1
        assert data["claims"][0]["claim_type"] == "breach_of_contract"


class TestOrder:
    """Test arbitration Order structure."""
    
    def test_order_creation(self):
        from tools.arbitration import Order, Money
        
        order = Order(
            order_id="order-001",
            order_type="monetary_damages",
            obligor="did:respondent",
            obligee="did:claimant",
            amount=Money(Decimal("175000"), "USD"),
            due_date="2026-10-15",
            enforcement_method="smart_asset_state_transition",
        )
        
        data = order.to_dict()
        
        assert data["order_id"] == "order-001"
        assert data["order_type"] == "monetary_damages"
        assert data["obligor"] == "did:respondent"
        assert data["amount"]["amount"] == 175000.0


class TestRuling:
    """Test ruling structure."""
    
    def test_ruling_creation(self):
        from tools.arbitration import Ruling, Order, Money
        
        orders = [
            Order(
                order_id="order-001",
                order_type="monetary_damages",
                obligor="did:respondent",
                obligee="did:claimant",
                amount=Money(Decimal("175000"), "USD"),
            ),
        ]
        
        ruling = Ruling(
            ruling_type="final_award",
            disposition="in_favor_of_claimant",
            orders=orders,
        )
        
        data = ruling.to_dict()
        
        assert data["ruling_type"] == "final_award"
        assert data["disposition"] == "in_favor_of_claimant"
        assert len(data["orders"]) == 1


class TestArbitrationRulingVC:
    """Test ruling VC structure."""
    
    def test_ruling_vc_creation(self):
        from tools.arbitration import (
            ArbitrationRulingVC, Ruling, Order, Money, Party
        )
        
        claimant = Party("did:claimant", "Claimant Corp")
        respondent = Party("did:respondent", "Respondent Corp")
        
        orders = [
            Order(
                order_id="order-001",
                order_type="monetary_damages",
                obligor=respondent.party_id,
                obligee=claimant.party_id,
                amount=Money(Decimal("175000"), "USD"),
            ),
        ]
        
        ruling = Ruling(
            ruling_type="final_award",
            disposition="in_favor_of_claimant",
            orders=orders,
        )
        
        vc = ArbitrationRulingVC(
            dispute_id="dr-2026-001",
            institution_id="difc-lcia",
            case_reference="DIFC-LCIA-2026-001",
            corridor_id="corridor:uae-kaz-trade",
            claimant=claimant,
            respondent=respondent,
            ruling=ruling,
        )
        
        data = vc.to_vc()
        
        assert "VerifiableCredential" in data["type"]
        assert data["credentialSubject"]["dispute_id"] == "dr-2026-001"
        assert data["credentialSubject"]["ruling"]["ruling_type"] == "final_award"


class TestEnforcementReceipt:
    """Test enforcement receipt structure."""
    
    def test_enforcement_receipt_creation(self):
        from tools.arbitration import EnforcementReceipt
        
        receipt = EnforcementReceipt(
            enforcement_id="enf-001",
            ruling_vc_digest="a" * 64,
            order_id="order-001",
            corridor_id="corridor:test",
            transition_type="arbitration.award.enforce",
        )
        
        data = receipt.to_dict()
        
        assert data["type"] == "MSEZArbitrationEnforcementReceipt"
        assert data["ruling_vc_digest"] == "a" * 64
        assert data["order_id"] == "order-001"


class TestArbitrationManager:
    """Test ArbitrationManager functionality."""
    
    def test_manager_creation(self):
        from tools.arbitration import ArbitrationManager
        
        manager = ArbitrationManager(institution_id="difc-lcia")
        
        assert manager.institution_id == "difc-lcia"
        assert manager.institution["name"] == "DIFC-LCIA Arbitration Centre"
    
    def test_manager_create_dispute(self):
        from tools.arbitration import ArbitrationManager, Party, Claim, Money
        
        manager = ArbitrationManager(institution_id="difc-lcia")
        
        claimant = Party("did:claimant", "A Corp")
        respondent = Party("did:respondent", "B Corp")
        claims = [Claim("c1", "breach_of_contract", "Breach", Money(Decimal("50000"), "USD"))]
        
        dispute = manager.create_dispute_request(
            corridor_id="corridor:test",
            claimant=claimant,
            respondent=respondent,
            dispute_type="breach_of_contract",
            claims=claims,
        )
        
        assert dispute.corridor_id == "corridor:test"
        assert dispute.institution_id == "difc-lcia"
        assert dispute.claimant.party_id == "did:claimant"
    
    def test_manager_unknown_institution_raises(self):
        from tools.arbitration import ArbitrationManager
        
        with pytest.raises(ValueError, match="Unknown institution"):
            ArbitrationManager(institution_id="unknown-institution")


# ─────────────────────────────────────────────────────────────────────────────
# Integration Tests
# ─────────────────────────────────────────────────────────────────────────────

class TestEndToEndScenarios:
    """End-to-end scenario tests."""
    
    def test_trade_dispute_full_lifecycle(self):
        """Test complete dispute lifecycle from filing to enforcement."""
        from tools.regpack import SanctionsEntry, SanctionsChecker
        from tools.arbitration import (
            ArbitrationManager, Party, Claim, Money,
            Ruling, Order, EnforcementReceipt
        )
        
        # Step 1: Create sanctions check (respondent is NOT sanctioned)
        checker = SanctionsChecker(entries=[], snapshot_id="test")
        
        # Step 2: File dispute via manager
        manager = ArbitrationManager(institution_id="difc-lcia")
        
        claimant = Party("did:claimant", "Exporter Corp")
        respondent = Party("did:respondent", "Importer Corp")
        
        # Verify respondent is not sanctioned
        result = checker.check_entity(respondent.legal_name)
        assert result.matched is False
        
        claims = [
            Claim("c1", "payment_default", "Failed to pay", Money(Decimal("100000"), "USD")),
        ]
        
        dispute = manager.create_dispute_request(
            corridor_id="corridor:uae-kaz-trade",
            claimant=claimant,
            respondent=respondent,
            dispute_type="payment_default",
            claims=claims,
        )
        
        # Step 3: Create ruling
        ruling = Ruling(
            ruling_type="final_award",
            disposition="in_favor_of_claimant",
            orders=[
                Order(
                    order_id="o1",
                    order_type="monetary_damages",
                    obligor=respondent.party_id,
                    obligee=claimant.party_id,
                    amount=Money(Decimal("100000"), "USD"),
                    enforcement_method="smart_asset_state_transition",
                ),
            ],
        )
        
        # Step 4: Enforce ruling
        receipt = EnforcementReceipt(
            enforcement_id="enf-2026-001",
            ruling_vc_digest="a" * 64,
            order_id="o1",
            corridor_id=dispute.corridor_id,
            transition_type="arbitration.award.enforce",
        )
        
        # Verify lifecycle
        assert dispute.dispute_type == "payment_default"
        assert ruling.disposition == "in_favor_of_claimant"
        assert receipt.order_id == "o1"
    
    def test_sanctions_blocked_party_detection(self):
        """Test that sanctioned parties are detected."""
        from tools.regpack import SanctionsEntry, SanctionsChecker
        
        # Create checker with a blocked party
        entries = [
            SanctionsEntry(
                entry_id="OFAC-99999",
                entry_type="entity",
                source_lists=["ofac_sdn"],
                primary_name="BLOCKED CORP",
                aliases=[{"name": "BLOCKED CORPORATION", "type": "alias"}],
                programs=["SDGT"],
            ),
        ]
        
        checker = SanctionsChecker(entries=entries, snapshot_id="test")
        
        # Check if party is sanctioned
        result = checker.check_entity("BLOCKED CORP")
        assert result.matched is True
        
        # A non-sanctioned party should not match
        clean_result = checker.check_entity("COMPLETELY CLEAN CORP XYZ999")
        assert clean_result.matched is False


class TestArbitrationInstitutionRegistry:
    """Test institution registry."""
    
    def test_difc_lcia_available(self):
        from tools.arbitration import ARBITRATION_INSTITUTIONS
        
        assert "difc-lcia" in ARBITRATION_INSTITUTIONS
        inst = ARBITRATION_INSTITUTIONS["difc-lcia"]
        assert inst["jurisdiction_id"] == "uae-difc"
        assert inst["procedural_options"]["emergency_arbitrator"] is True
    
    def test_siac_available(self):
        from tools.arbitration import ARBITRATION_INSTITUTIONS
        
        assert "siac" in ARBITRATION_INSTITUTIONS
        inst = ARBITRATION_INSTITUTIONS["siac"]
        assert inst["jurisdiction_id"] == "sg"
    
    def test_aifc_iac_available(self):
        from tools.arbitration import ARBITRATION_INSTITUTIONS
        
        assert "aifc-iac" in ARBITRATION_INSTITUTIONS
        inst = ARBITRATION_INSTITUTIONS["aifc-iac"]
        assert inst["jurisdiction_id"] == "kaz-aifc"


class TestDisputeTypes:
    """Test dispute type validation."""
    
    def test_all_dispute_types_supported(self):
        """Test DISPUTE_TYPES matches spec Definition 26.4 ClaimType enum."""
        from tools.arbitration import DISPUTE_TYPES, CLAIM_TYPES
        
        # Per MASS Protocol v0.2 Definition 26.4 - ClaimType enum
        expected = [
            "breach_of_contract",
            "non_conforming_goods",
            "payment_default",
            "delivery_failure",
            "quality_defect",
            "documentary_discrepancy",
            "force_majeure",
            "fraudulent_misrepresentation",
        ]
        
        for dt in expected:
            assert dt in DISPUTE_TYPES, f"Missing spec ClaimType: {dt}"
        
        # CLAIM_TYPES should be alias
        assert CLAIM_TYPES == DISPUTE_TYPES
    
    def test_all_order_types_supported(self):
        from tools.arbitration import ORDER_TYPES
        
        expected = [
            "monetary_damages",
            "specific_performance",
            "declaratory",
            "injunction",
            "costs",
            "interest",
        ]
        
        for ot in expected:
            assert ot in ORDER_TYPES


# =============================================================================
# Escrow Tests (v0.4.41 Requirement)
# =============================================================================

class TestEscrow:
    """Test escrow management."""
    
    def test_escrow_creation(self):
        """Can create an escrow."""
        from tools.arbitration import Escrow, Money
        from decimal import Decimal
        
        escrow = Escrow(
            escrow_id="escrow:test123",
            dispute_id="disp-2026-001234",
            escrow_type="filing_fee",
            amount=Money(Decimal("3000"), "USD"),
            depositor="did:key:claimant",
        )
        assert escrow.status == "pending"
    
    def test_escrow_deposit(self):
        """Can deposit to escrow."""
        from tools.arbitration import Escrow, Money
        from decimal import Decimal
        
        escrow = Escrow(
            escrow_id="escrow:test123",
            dispute_id="disp-2026-001234",
            escrow_type="filing_fee",
            amount=Money(Decimal("3000"), "USD"),
        )
        tx = escrow.deposit(transaction_proof="0xabc123")
        assert escrow.status == "funded"
        assert len(escrow.transactions) == 1
        assert tx.transaction_type == "deposit"
    
    def test_escrow_release(self):
        """Can release escrow funds."""
        from tools.arbitration import Escrow, Money
        from decimal import Decimal
        
        escrow = Escrow(
            escrow_id="escrow:test123",
            dispute_id="disp-2026-001234",
            escrow_type="award_escrow",
            amount=Money(Decimal("175000"), "USD"),
            status="funded",
        )
        tx = escrow.release(
            recipient="did:key:beneficiary",
            reason="Ruling enforced",
            ruling_ref={"artifact_type": "vc", "digest_sha256": "a" * 64}
        )
        assert escrow.status == "fully_released"
        assert tx.recipient == "did:key:beneficiary"
    
    def test_escrow_forfeit(self):
        """Can forfeit escrow."""
        from tools.arbitration import Escrow, Money
        from decimal import Decimal
        
        escrow = Escrow(
            escrow_id="escrow:test123",
            dispute_id="disp-2026-001234",
            escrow_type="security_deposit",
            amount=Money(Decimal("50000"), "USD"),
            status="funded",
        )
        tx = escrow.forfeit(
            reason="Adverse ruling",
            ruling_ref={"artifact_type": "vc", "digest_sha256": "b" * 64}
        )
        assert escrow.status == "forfeited"


# =============================================================================
# Settlement Tests (v0.4.41 Requirement)
# =============================================================================

class TestSettlement:
    """Test settlement agreement handling."""
    
    def test_settlement_creation(self):
        """Can create a settlement."""
        from tools.arbitration import Settlement, SettlementTerms
        
        settlement = Settlement(
            settlement_id=Settlement.create_settlement_id(),
            dispute_id="disp-2026-001234",
            parties=[
                {"did": "did:key:claimant", "role": "claimant"},
                {"did": "did:key:respondent", "role": "respondent"},
            ],
            terms=SettlementTerms(
                monetary_terms={
                    "payments": [{
                        "from": "did:key:respondent",
                        "to": "did:key:claimant",
                        "amount": {"amount": 100000, "currency": "USD"},
                        "due_date": "2026-03-01T00:00:00Z"
                    }]
                },
                mutual_release=True,
            ),
            effective_date="2026-02-01T00:00:00Z",
        )
        assert settlement.dispute_id == "disp-2026-001234"
        assert len(settlement.parties) == 2
    
    def test_settlement_terms_to_dict(self):
        """Settlement terms serialize correctly."""
        from tools.arbitration import SettlementTerms
        
        terms = SettlementTerms(
            mutual_release=True,
            non_admission=True,
            confidentiality_clause={"scope": "all_terms", "duration_years": 5}
        )
        d = terms.to_dict()
        assert d["mutual_release"] is True
        assert d["confidentiality_clause"]["duration_years"] == 5


# =============================================================================
# Evidence Package Tests (v0.4.41 Requirement)
# =============================================================================

class TestEvidencePackage:
    """Test evidence package handling."""
    
    def test_evidence_item_creation(self):
        """Can create evidence items."""
        from tools.arbitration import EvidenceItem, EvidencePackage
        
        item = EvidenceItem(
            evidence_id=EvidencePackage.create_evidence_id(),
            evidence_type="SmartAssetReceipt",
            description="Receipt showing delivery confirmation",
            artifact_ref={"artifact_type": "smart-asset-receipt", "digest_sha256": "c" * 64},
            relevance="Proves delivery was completed on time",
        )
        assert item.evidence_type == "SmartAssetReceipt"
    
    def test_evidence_package_creation(self):
        """Can create evidence package."""
        from tools.arbitration import EvidenceItem, EvidencePackage
        
        item = EvidenceItem(
            evidence_id=EvidencePackage.create_evidence_id(),
            evidence_type="ContractDocument",
            description="Original sale agreement",
            artifact_ref={"artifact_type": "blob", "digest_sha256": "d" * 64},
        )
        
        package = EvidencePackage(
            evidence_package_id=EvidencePackage.create_evidence_package_id(),
            dispute_id="disp-2026-001234",
            submitting_party="did:key:claimant",
            evidence_items=[item],
            witness_bundle_ref={"artifact_type": "witness-bundle", "digest_sha256": "e" * 64},
        )
        assert len(package.evidence_items) == 1
    
    def test_authenticity_attestation(self):
        """Can add authenticity attestation to evidence."""
        from tools.arbitration import AuthenticityAttestation, EvidenceItem, EvidencePackage
        
        attestation = AuthenticityAttestation(
            attestation_type="SmartAssetCheckpointInclusion",
            proof_ref={"artifact_type": "checkpoint", "digest_sha256": "f" * 64}
        )
        
        item = EvidenceItem(
            evidence_id=EvidencePackage.create_evidence_id(),
            evidence_type="SmartAssetReceipt",
            description="Receipt with checkpoint proof",
            artifact_ref={"artifact_type": "smart-asset-receipt", "digest_sha256": "g" * 64},
            authenticity_attestation=attestation,
        )
        
        d = item.to_dict()
        assert d["authenticity_attestation"]["attestation_type"] == "SmartAssetCheckpointInclusion"
    
    def test_invalid_evidence_type_raises(self):
        """Invalid evidence type raises error."""
        from tools.arbitration import EvidenceItem
        
        with pytest.raises(ValueError):
            EvidenceItem(
                evidence_id="evidence:test",
                evidence_type="InvalidType",
                description="Test",
                artifact_ref={},
            )


# =============================================================================
# Definition 26.7 - Arbitration Transition Types Tests
# =============================================================================

class TestArbitrationTransitionTypes:
    """Tests for Definition 26.7 Arbitration-Related Transitions."""
    
    def test_all_spec_transition_types_present(self):
        """Verify all Definition 26.7 transition types are implemented."""
        from tools.arbitration import ARBITRATION_TRANSITION_TYPES
        
        # Per MASS Protocol v0.2 Definition 26.7
        required_transitions = [
            "arbitration.dispute.file.v1",       # DisputeFile
            "arbitration.dispute.respond.v1",    # DisputeRespond
            "arbitration.ruling.receive.v1",     # ArbitrationRulingReceive
            "arbitration.ruling.enforce.v1",     # ArbitrationEnforce
            "arbitration.appeal.file.v1",        # ArbitrationAppeal
            "arbitration.settlement.agree.v1",   # DisputeSettle
            "arbitration.escrow.release.v1",     # EscrowRelease
            "arbitration.escrow.forfeit.v1",     # EscrowForfeit
        ]
        
        for transition in required_transitions:
            assert transition in ARBITRATION_TRANSITION_TYPES, \
                f"Missing Definition 26.7 transition: {transition}"
    
    def test_escrow_release_transition_params(self):
        """Verify EscrowRelease has required params per Definition 26.7."""
        from tools.arbitration import ARBITRATION_TRANSITION_TYPES
        
        release = ARBITRATION_TRANSITION_TYPES.get("arbitration.escrow.release.v1")
        assert release is not None
        assert "params" in release
        
        # Per Definition 26.7: escrow_id, release_condition, beneficiary, amount
        params = release["params"]
        assert "escrow_id" in params
        assert "release_condition" in params
        assert "beneficiary" in params
        assert "amount" in params
    
    def test_escrow_forfeit_transition_params(self):
        """Verify EscrowForfeit has required params per Definition 26.7."""
        from tools.arbitration import ARBITRATION_TRANSITION_TYPES
        
        forfeit = ARBITRATION_TRANSITION_TYPES.get("arbitration.escrow.forfeit.v1")
        assert forfeit is not None
        assert "params" in forfeit
        
        # Per Definition 26.7: escrow_id, forfeit_reason, ruling_vc_ref
        params = forfeit["params"]
        assert "escrow_id" in params
        assert "forfeit_reason" in params
        assert "ruling_vc_ref" in params
    
    def test_transition_type_count(self):
        """Verify total transition type count."""
        from tools.arbitration import ARBITRATION_TRANSITION_TYPES
        
        # Definition 26.7 specifies 8 arbitration transitions
        # Plus we have evidence.submit which is useful operational extension
        assert len(ARBITRATION_TRANSITION_TYPES) >= 8


# =============================================================================
# Definition 26.9 - πruling Circuit Tests
# =============================================================================

class TestPiRulingCircuit:
    """Tests for Definition 26.9 πruling Circuit Specification."""
    
    def test_circuit_schema_exists(self):
        """Verify circuit schema file exists."""
        from pathlib import Path
        schema_path = Path(__file__).parent.parent / "schemas" / "circuit.pi-ruling.schema.json"
        assert schema_path.exists()
    
    def test_circuit_public_inputs(self):
        """Verify public inputs match Definition 26.9."""
        import json
        from pathlib import Path
        
        schema_path = Path(__file__).parent.parent / "schemas" / "circuit.pi-ruling.schema.json"
        with open(schema_path) as f:
            schema = json.load(f)
        
        public_inputs = schema["properties"]["public_inputs"]["properties"]
        
        # Per Definition 26.9 Public Inputs
        assert "ruling_vc_digest" in public_inputs
        assert "asset_id" in public_inputs
        assert "order_digest" in public_inputs
        assert "enforcement_timestamp" in public_inputs
    
    def test_circuit_private_inputs(self):
        """Verify private inputs match Definition 26.9."""
        import json
        from pathlib import Path
        
        schema_path = Path(__file__).parent.parent / "schemas" / "circuit.pi-ruling.schema.json"
        with open(schema_path) as f:
            schema = json.load(f)
        
        private_inputs = schema["properties"]["private_inputs"]["properties"]
        
        # Per Definition 26.9 Private Inputs
        assert "ruling_vc" in private_inputs
        assert "institution_credential" in private_inputs
        assert "appeal_status" in private_inputs
    
    def test_circuit_constraint_count(self):
        """Verify constraint count matches spec (~35,000)."""
        import json
        from pathlib import Path
        
        schema_path = Path(__file__).parent.parent / "schemas" / "circuit.pi-ruling.schema.json"
        with open(schema_path) as f:
            schema = json.load(f)
        
        assert schema["properties"]["constraint_count"]["const"] == 35000


# =============================================================================
# Definition 26.5 - Evidence Package Tests
# =============================================================================

class TestEvidencePackage:
    """Tests for Definition 26.5 Evidence Package."""
    
    def test_evidence_types_match_spec(self):
        """Verify EVIDENCE_TYPES matches Definition 26.5 EvidenceType enum."""
        from tools.arbitration import EVIDENCE_TYPES
        
        # Per Definition 26.5 EvidenceType enum
        expected = [
            "SmartAssetReceipt",
            "CorridorReceipt",
            "ComplianceEvidence",
            "ExpertReport",
            "WitnessStatement",
            "ContractDocument",
            "CommunicationRecord",
            "PaymentRecord",
            "ShippingDocument",
            "InspectionReport",
        ]
        
        for et in expected:
            assert et in EVIDENCE_TYPES, f"Missing EvidenceType: {et}"
    
    def test_authenticity_types_match_spec(self):
        """Verify AUTHENTICITY_TYPES matches Definition 26.5 AttestationType enum."""
        from tools.arbitration import AUTHENTICITY_TYPES
        
        # Per Definition 26.5 AttestationType enum
        expected = [
            "CorridorCheckpointInclusion",
            "SmartAssetCheckpointInclusion",
            "NotarizedDocument",
            "ExpertCertification",
            "ChainOfCustody",
        ]
        
        for at in expected:
            assert at in AUTHENTICITY_TYPES, f"Missing AttestationType: {at}"
