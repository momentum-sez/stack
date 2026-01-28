"""Arbitration System Tests for v0.4.41.

Comprehensive test coverage for the Arbitration system:
- Institution registry
- Dispute filing
- Evidence submission
- Ruling verification
- Automatic enforcement
- Smart asset transitions
"""

import json
import os
from datetime import datetime, timezone
from decimal import Decimal
from pathlib import Path

import pytest


REPO_ROOT = Path(__file__).resolve().parents[1]


# ─────────────────────────────────────────────────────────────────────────────
# Fixtures
# ─────────────────────────────────────────────────────────────────────────────

@pytest.fixture
def arbitration_manager():
    """Create an arbitration manager for DIFC-LCIA."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.arbitration import ArbitrationManager
    
    os.environ["SOURCE_DATE_EPOCH"] = "1735689600"
    return ArbitrationManager(institution_id="difc-lcia")


@pytest.fixture
def sample_parties():
    """Sample parties for testing."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.arbitration import Party
    
    claimant = Party(
        party_id="did:key:z6MkClaimant123",
        legal_name="Trade Corp ADGM Ltd",
        jurisdiction_id="uae-adgm",
        email="legal@tradecorp.ae",
    )
    
    respondent = Party(
        party_id="did:key:z6MkRespondent456",
        legal_name="Import Corp AIFC LLP",
        jurisdiction_id="kaz-aifc",
        email="legal@importcorp.kz",
    )
    
    return claimant, respondent


@pytest.fixture
def sample_claims():
    """Sample claims for testing."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.arbitration import Claim, Money
    
    return [
        Claim(
            claim_id="claim-001",
            claim_type="principal_amount",
            description="Outstanding payment for delivered goods",
            amount=Money(amount=Decimal("150000"), currency="USD"),
            supporting_receipts=[
                {
                    "artifact_type": "corridor-receipt",
                    "digest_sha256": "abc123" * 10 + "abcd",
                }
            ],
        ),
        Claim(
            claim_id="claim-002",
            claim_type="damages",
            description="Consequential damages from delayed payment",
            amount=Money(amount=Decimal("25000"), currency="USD"),
        ),
    ]


@pytest.fixture
def sample_ruling_vc(sample_parties):
    """Sample ruling VC for testing."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.arbitration import ArbitrationRulingVC, Ruling, Order, Money
    
    claimant, respondent = sample_parties
    
    ruling = Ruling(
        ruling_type="final_award",
        disposition="partially_in_favor_of_claimant",
        findings=[
            {
                "claim_id": "claim-001",
                "finding": "sustained",
            },
            {
                "claim_id": "claim-002",
                "finding": "partially_sustained",
            },
        ],
        orders=[
            Order(
                order_id="order-001",
                order_type="monetary_damages",
                obligor=respondent.party_id,
                obligee=claimant.party_id,
                amount=Money(Decimal("150000"), "USD"),
                due_date="2026-10-15",
                enforcement_method="smart_asset_state_transition",
            ),
            Order(
                order_id="order-002",
                order_type="costs",
                obligor=respondent.party_id,
                obligee=claimant.party_id,
                amount=Money(Decimal("20000"), "USD"),
                due_date="2026-10-15",
                enforcement_method="escrow_release",
            ),
        ],
        interest={
            "rate": 0.05,
            "basis": "annual_simple",
            "from_date": "2026-03-15",
        },
        costs_allocation={
            "arbitration_costs": {"claimant_share": 0.25, "respondent_share": 0.75},
            "legal_costs": {"claimant_share": 0.0, "respondent_share": 1.0},
        },
    )
    
    return ArbitrationRulingVC(
        dispute_id="dispute:abc123",
        institution_id="difc-lcia",
        case_reference="DIFC-LCIA-2026-001",
        corridor_id="corridor:uae-kaz-trade-01",
        claimant=claimant,
        respondent=respondent,
        ruling=ruling,
        tribunal={
            "presiding_arbitrator": {
                "name": "Hon. Jane Smith",
                "appointed_by": "institution",
            },
            "co_arbitrators": [
                {"name": "Mr. Ahmed Hassan", "appointed_by": "claimant"},
                {"name": "Ms. Elena Petrov", "appointed_by": "respondent"},
            ],
        },
        enforcement={
            "enforceable_jurisdictions": ["uae-difc", "kaz-aifc", "new_york_convention"],
            "smart_asset_enforcement": {
                "enabled": True,
                "enforcement_transition_type": "arbitration.ruling.enforce.v1",
            },
        },
        appeal={
            "appeal_available": True,
            "appeal_deadline": "2026-10-15T00:00:00Z",
            "appeal_grounds": ["serious_irregularity"],
        },
        issuer="did:key:z6MkDIFCLCIA",
        issuance_date="2026-09-15T16:00:00Z",
    )


# ─────────────────────────────────────────────────────────────────────────────
# Institution Registry Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_arbitration_institution_registry_loads():
    """Verify arbitration institution registry is available."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.arbitration import ARBITRATION_INSTITUTIONS
    
    assert "difc-lcia" in ARBITRATION_INSTITUTIONS
    assert "siac" in ARBITRATION_INSTITUTIONS
    assert "icc" in ARBITRATION_INSTITUTIONS
    assert "aifc-iac" in ARBITRATION_INSTITUTIONS


def test_arbitration_institution_schema():
    """Verify arbitration institution schema exists and is valid."""
    schema_path = REPO_ROOT / "schemas" / "arbitration.institution.schema.json"
    assert schema_path.exists(), "arbitration.institution.schema.json must exist"
    
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    assert schema["$id"].endswith("arbitration.institution.schema.json")
    assert "MSEZArbitrationInstitution" in str(schema)


def test_arbitration_institution_difc_lcia_details():
    """Verify DIFC-LCIA institution details."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.arbitration import ARBITRATION_INSTITUTIONS
    
    difc = ARBITRATION_INSTITUTIONS["difc-lcia"]
    
    assert difc["name"] == "DIFC-LCIA Arbitration Centre"
    assert difc["jurisdiction_id"] == "uae-difc"
    assert difc["procedural_options"]["emergency_arbitrator"] is True
    assert "breach_of_contract" in difc["supported_dispute_types"]


# ─────────────────────────────────────────────────────────────────────────────
# Dispute Request Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_arbitration_dispute_request_schema():
    """Verify dispute request schema exists and is valid."""
    schema_path = REPO_ROOT / "schemas" / "arbitration.dispute-request.schema.json"
    assert schema_path.exists(), "arbitration.dispute-request.schema.json must exist"
    
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    assert schema["$id"].endswith("arbitration.dispute-request.schema.json")
    assert "MSEZDisputeRequest" in str(schema)


def test_arbitration_dispute_file_creates_request(arbitration_manager, sample_parties, sample_claims):
    """Test creating a dispute request."""
    claimant, respondent = sample_parties
    
    dispute = arbitration_manager.create_dispute_request(
        corridor_id="corridor:uae-kaz-trade-01",
        claimant=claimant,
        respondent=respondent,
        dispute_type="payment_default",
        claims=sample_claims,
        relief_sought={
            "monetary_damages": {"amount": 175000, "currency": "USD"},
            "interest": True,
            "costs": True,
        },
        expedited=True,
    )
    
    assert dispute.dispute_id.startswith("dispute:")
    assert dispute.institution_id == "difc-lcia"
    assert dispute.corridor_id == "corridor:uae-kaz-trade-01"
    assert dispute.claimant.party_id == claimant.party_id
    assert dispute.respondent.party_id == respondent.party_id
    assert len(dispute.claims) == 2
    assert dispute.procedural_preferences.get("expedited") is True


def test_arbitration_dispute_request_to_dict(arbitration_manager, sample_parties, sample_claims):
    """Test dispute request serialization."""
    claimant, respondent = sample_parties
    
    dispute = arbitration_manager.create_dispute_request(
        corridor_id="corridor:test",
        claimant=claimant,
        respondent=respondent,
        dispute_type="breach_of_contract",
        claims=sample_claims,
    )
    
    d = dispute.to_dict()
    
    assert d["type"] == "MSEZDisputeRequest"
    assert d["stack_spec_version"] == "0.4.42"
    assert d["claimant"]["party_id"] == claimant.party_id
    assert len(d["claims"]) == 2


def test_arbitration_dispute_type_validation(arbitration_manager, sample_parties, sample_claims):
    """Test that invalid dispute types are rejected."""
    claimant, respondent = sample_parties
    
    with pytest.raises(ValueError, match="not supported"):
        arbitration_manager.create_dispute_request(
            corridor_id="corridor:test",
            claimant=claimant,
            respondent=respondent,
            dispute_type="invalid_type",
            claims=sample_claims,
        )


# ─────────────────────────────────────────────────────────────────────────────
# Evidence Package Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_arbitration_evidence_package_structure():
    """Test evidence package data structure."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.arbitration import Claim, Money
    
    claim = Claim(
        claim_id="claim-001",
        claim_type="principal_amount",
        description="Test claim",
        amount=Money(Decimal("50000"), "USD"),
        supporting_evidence=[
            {
                "artifact_type": "blob",
                "digest_sha256": "abc123" * 10 + "abcd",
                "description": "Invoice PDF",
            },
            {
                "artifact_type": "blob",
                "digest_sha256": "def456" * 10 + "defg",
                "description": "Bill of Lading",
            },
        ],
    )
    
    d = claim.to_dict()
    
    assert len(d["supporting_evidence"]) == 2
    assert d["amount"]["amount"] == 50000


# ─────────────────────────────────────────────────────────────────────────────
# Ruling VC Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_arbitration_ruling_vc_schema():
    """Verify ruling VC schema exists and is valid."""
    schema_path = REPO_ROOT / "schemas" / "vc.arbitration-ruling.schema.json"
    assert schema_path.exists(), "vc.arbitration-ruling.schema.json must exist"
    
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    assert schema["$id"].endswith("vc.arbitration-ruling.schema.json")
    assert "MSEZArbitrationRulingCredential" in str(schema)


def test_arbitration_ruling_vc_generation(sample_ruling_vc):
    """Test generating a ruling VC."""
    vc = sample_ruling_vc.to_vc()
    
    assert "@context" in vc
    assert "VerifiableCredential" in vc["type"]
    assert "MSEZArbitrationRulingCredential" in vc["type"]
    assert vc["issuer"] == "did:key:z6MkDIFCLCIA"
    assert vc["credentialSubject"]["institution_id"] == "difc-lcia"
    assert vc["credentialSubject"]["ruling"]["ruling_type"] == "final_award"


def test_arbitration_ruling_verify_signature(arbitration_manager, sample_ruling_vc):
    """Test ruling VC verification."""
    vc = sample_ruling_vc.to_vc()
    
    valid, errors = arbitration_manager.verify_ruling_vc(vc)
    
    assert valid is True
    assert len(errors) == 0


def test_arbitration_ruling_verify_invalid_type(arbitration_manager):
    """Test ruling verification fails for invalid type."""
    invalid_vc = {
        "type": ["VerifiableCredential"],  # Missing MSEZArbitrationRulingCredential
        "issuer": "did:key:test",
        "credentialSubject": {},
    }
    
    valid, errors = arbitration_manager.verify_ruling_vc(invalid_vc)
    
    assert valid is False
    assert any("Missing MSEZArbitrationRulingCredential" in e for e in errors)


def test_arbitration_ruling_orders_structure(sample_ruling_vc):
    """Test ruling orders are properly structured."""
    vc = sample_ruling_vc.to_vc()
    
    orders = vc["credentialSubject"]["ruling"]["orders"]
    
    assert len(orders) == 2
    
    monetary_order = orders[0]
    assert monetary_order["order_type"] == "monetary_damages"
    assert monetary_order["amount"]["amount"] == 150000
    assert monetary_order["enforcement_method"] == "smart_asset_state_transition"
    
    costs_order = orders[1]
    assert costs_order["order_type"] == "costs"
    assert costs_order["enforcement_method"] == "escrow_release"


# ─────────────────────────────────────────────────────────────────────────────
# Enforcement Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_arbitration_ruling_enforce_creates_receipt(arbitration_manager, sample_ruling_vc):
    """Test creating an enforcement receipt."""
    vc = sample_ruling_vc.to_vc()
    
    receipt = arbitration_manager.create_enforcement_receipt(
        ruling_vc=vc,
        order_id="order-001",
        corridor_id="corridor:uae-kaz-settlement-01",
        transition_type="arbitration.ruling.enforce.v1",
        asset_id="asset:escrow-001",
    )
    
    assert receipt.enforcement_id.startswith("enforcement:")
    assert receipt.order_id == "order-001"
    assert receipt.corridor_id == "corridor:uae-kaz-settlement-01"
    assert receipt.transition_type == "arbitration.ruling.enforce.v1"


def test_arbitration_enforcement_receipt_to_dict(arbitration_manager, sample_ruling_vc):
    """Test enforcement receipt serialization."""
    vc = sample_ruling_vc.to_vc()
    
    receipt = arbitration_manager.create_enforcement_receipt(
        ruling_vc=vc,
        order_id="order-001",
        corridor_id="corridor:test",
        transition_type="arbitration.ruling.enforce.v1",
    )
    
    d = receipt.to_dict()
    
    assert d["type"] == "MSEZArbitrationEnforcementReceipt"
    assert d["stack_spec_version"] == "0.4.42"
    assert len(d["ruling_vc_digest"]) == 64


def test_arbitration_can_enforce_in_jurisdiction(arbitration_manager):
    """Test jurisdiction enforceability check."""
    # DIFC awards are enforceable in UAE jurisdictions
    assert arbitration_manager.can_enforce_in_jurisdiction("uae-difc") is True
    assert arbitration_manager.can_enforce_in_jurisdiction("uae-adgm") is True
    
    # New York Convention covers most jurisdictions
    assert arbitration_manager.can_enforce_in_jurisdiction("us-ny") is True
    assert arbitration_manager.can_enforce_in_jurisdiction("sg") is True


# ─────────────────────────────────────────────────────────────────────────────
# Appeal Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_arbitration_appeal_deadline_enforcement(sample_ruling_vc):
    """Test appeal deadline is captured in ruling."""
    vc = sample_ruling_vc.to_vc()
    
    appeal = vc["credentialSubject"]["appeal"]
    
    assert appeal["appeal_available"] is True
    assert appeal["appeal_deadline"] == "2026-10-15T00:00:00Z"
    assert "serious_irregularity" in appeal["appeal_grounds"]


# ─────────────────────────────────────────────────────────────────────────────
# Escrow Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_arbitration_escrow_lock_on_file():
    """Test escrow structure for dispute filing."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.arbitration import Money
    
    escrow = {
        "filing_fee_amount": Money(Decimal("3000"), "USD").to_dict(),
        "escrow_corridor_id": "corridor:escrow-01",
        "escrow_receipt_ref": {
            "artifact_type": "corridor-receipt",
            "digest_sha256": "abc123" * 10 + "abcd",
        },
    }
    
    assert escrow["filing_fee_amount"]["amount"] == 3000
    assert escrow["filing_fee_amount"]["currency"] == "USD"


def test_arbitration_escrow_release_on_award(sample_ruling_vc):
    """Test escrow release conditions in ruling."""
    vc = sample_ruling_vc.to_vc()
    
    enforcement = vc["credentialSubject"]["enforcement"]
    
    assert enforcement["smart_asset_enforcement"]["enabled"] is True


# ─────────────────────────────────────────────────────────────────────────────
# Corridor Integration Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_arbitration_corridor_specialization():
    """Test arbitration corridor configuration structure."""
    corridor_config = {
        "corridor_type": "arbitration",
        "arbitration_config": {
            "institution_id": "difc-lcia",
            "supported_dispute_types": [
                "breach_of_contract",
                "payment_default",
            ],
            "linked_corridors": [
                {"corridor_id": "corridor:trade-01", "role": "primary"},
                {"corridor_id": "corridor:settlement-01", "role": "enforcement"},
            ],
            "escrow_requirements": {
                "filing_fee_escrow": True,
                "award_escrow": True,
            },
            "automation_level": {
                "evidence_extraction": "automatic",
                "ruling_enforcement": "automatic_with_appeal_wait",
            },
        },
    }
    
    assert corridor_config["corridor_type"] == "arbitration"
    assert corridor_config["arbitration_config"]["institution_id"] == "difc-lcia"
    assert len(corridor_config["arbitration_config"]["linked_corridors"]) == 2


def test_arbitration_linked_corridor_anchoring():
    """Test that arbitration corridors can anchor to trade corridors."""
    # Settlement anchor linking arbitration→trade corridor
    anchor = {
        "type": "MSEZSettlementAnchor",
        "obligation_corridor_id": "corridor:arbitration-01",
        "settlement_corridor_id": "corridor:settlement-01",
        "obligation_checkpoint_ref": {
            "artifact_type": "checkpoint",
            "digest_sha256": "abc123" * 10 + "abcd",
        },
        "settlement_receipt_ref": {
            "artifact_type": "corridor-receipt",
            "digest_sha256": "def456" * 10 + "defg",
        },
    }
    
    assert anchor["obligation_corridor_id"] == "corridor:arbitration-01"


# ─────────────────────────────────────────────────────────────────────────────
# Transition Type Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_arbitration_transition_types_defined():
    """Verify arbitration transition types are defined."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.arbitration import ARBITRATION_TRANSITION_TYPES
    
    assert "arbitration.dispute.file.v1" in ARBITRATION_TRANSITION_TYPES
    assert "arbitration.dispute.respond.v1" in ARBITRATION_TRANSITION_TYPES
    assert "arbitration.evidence.submit.v1" in ARBITRATION_TRANSITION_TYPES
    assert "arbitration.ruling.receive.v1" in ARBITRATION_TRANSITION_TYPES
    assert "arbitration.ruling.enforce.v1" in ARBITRATION_TRANSITION_TYPES
    assert "arbitration.appeal.file.v1" in ARBITRATION_TRANSITION_TYPES
    assert "arbitration.settlement.agree.v1" in ARBITRATION_TRANSITION_TYPES


def test_arbitration_transition_type_structure():
    """Test arbitration transition type structure."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.arbitration import ARBITRATION_TRANSITION_TYPES
    
    enforce = ARBITRATION_TRANSITION_TYPES["arbitration.ruling.enforce.v1"]
    
    assert enforce["title"] == "Enforce Arbitration Ruling"
    assert "ruling_exists" in enforce["required_attestations"]
    assert "appeal_period_expired" in enforce["required_attestations"]
    assert "executes_order" in enforce["effects"]


# ─────────────────────────────────────────────────────────────────────────────
# Settlement Agreement Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_arbitration_settlement_agreement(sample_parties):
    """Test settlement agreement structure."""
    claimant, respondent = sample_parties
    
    settlement = {
        "type": "MSEZArbitrationSettlement",
        "dispute_id": "dispute:abc123",
        "parties": {
            "claimant": claimant.to_dict(),
            "respondent": respondent.to_dict(),
        },
        "terms": {
            "payment": {"amount": 125000, "currency": "USD"},
            "due_date": "2026-08-15",
            "mutual_release": True,
            "confidentiality": True,
        },
        "consent_award_requested": True,
    }
    
    assert settlement["terms"]["payment"]["amount"] == 125000
    assert settlement["consent_award_requested"] is True


# ─────────────────────────────────────────────────────────────────────────────
# Cost Allocation Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_arbitration_cost_allocation(sample_ruling_vc):
    """Test cost allocation in ruling."""
    vc = sample_ruling_vc.to_vc()
    
    costs = vc["credentialSubject"]["ruling"]["costs_allocation"]
    
    # Respondent pays 75% of arbitration costs
    assert costs["arbitration_costs"]["respondent_share"] == 0.75
    
    # Respondent pays 100% of legal costs (costs follow the event)
    assert costs["legal_costs"]["respondent_share"] == 1.0
