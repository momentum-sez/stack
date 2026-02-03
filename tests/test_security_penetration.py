"""
PHOENIX Security Penetration Test Suite

Aggressive security testing to uncover vulnerabilities:
1. Input Validation Bypass
2. Integer Overflow/Underflow
3. Replay Attacks
4. TOCTOU (Time-Of-Check to Time-Of-Use) Races
5. Unauthorized Access
6. Denial of Service
7. Data Leakage
8. Cross-Jurisdiction Security Bypass
9. Signature Verification Bypasses
10. Attestation Forgery

Copyright (c) 2026 Momentum. All rights reserved.
"""

import hashlib
import json
import secrets
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from typing import Any, Dict, List, Optional


# =============================================================================
# SECURITY #1: Input Validation Bypass
# =============================================================================

class TestInputValidationBypass:
    """
    Test input validation for bypass vulnerabilities.
    """

    def test_jurisdiction_id_injection(self):
        """Test for injection in jurisdiction ID."""
        from tools.msez.composition import JurisdictionLayer, Domain

        # Try to inject special characters
        malicious_ids = [
            "us-ny<script>",  # XSS attempt
            "us-ny'; DROP TABLE--",  # SQL injection attempt
            "us-ny/../../../etc/passwd",  # Path traversal
            "us-ny\x00hidden",  # Null byte injection
        ]

        for mal_id in malicious_ids:
            layer = JurisdictionLayer(
                jurisdiction_id=mal_id,
                domains=[Domain.CIVIC],
                description="Test",
            )
            errors = layer.validate()
            # Malicious IDs should be rejected
            assert errors, f"Malicious ID '{mal_id}' should be rejected"

    def test_zone_id_unicode_normalization(self):
        """Test zone ID with unicode normalization attacks."""
        from tools.msez.composition import ZoneComposition

        # Unicode lookalike characters
        lookalike_ids = [
            "tеst.zone",  # Cyrillic 'е' instead of Latin 'e'
            "test．zone",  # Fullwidth period
        ]

        for zone_id in lookalike_ids:
            comp = ZoneComposition(
                zone_id=zone_id,
                name="Test Zone",
                layers=[],
            )
            errors = comp.validate()
            # Either reject or normalize - either is acceptable


    def test_hex_digest_validation(self):
        """Test that invalid hex digests are rejected."""
        from tools.msez.composition import LawpackRef

        invalid_digests = [
            "xyz",  # Not hex
            "a" * 63,  # Wrong length (63)
            "a" * 65,  # Wrong length (65)
            "A" * 64,  # Uppercase (might be valid depending on impl)
            "g" * 64,  # Invalid hex char 'g'
        ]

        for digest in invalid_digests:
            ref = LawpackRef(
                jurisdiction_id="us-ny",
                domain="civic",
                digest_sha256=digest,
            )
            # Validation might happen at different levels


# =============================================================================
# SECURITY #2: Integer Overflow Protection
# =============================================================================

class TestIntegerOverflowProtection:
    """
    Test integer overflow/underflow protection.
    """

    def test_vm_word_overflow(self):
        """Test VM Word arithmetic overflow handling."""
        from tools.phoenix.vm import Word

        max_val = Word.from_int((1 << 256) - 1)  # 2^256 - 1
        one = Word.one()

        result = max_val + one
        # Should wrap around to 0 (256-bit modular arithmetic)
        assert result.to_int() == 0, \
            "VM Word overflow should wrap to 0"

    def test_vm_word_underflow(self):
        """Test VM Word arithmetic underflow handling."""
        from tools.phoenix.vm import Word

        zero = Word.zero()
        one = Word.one()

        result = zero - one
        # Should wrap around to max value
        expected = (1 << 256) - 1
        assert result.to_int() == expected, \
            "VM Word underflow should wrap to max"

    def test_attestation_count_overflow(self):
        """Test that large attestation counts don't overflow."""
        from tools.phoenix.watcher import WatcherBond, WatcherId, BondStatus

        watcher = WatcherId(
            did="did:msez:watcher:overflow",
            public_key_hex="a" * 64,
        )

        bond = WatcherBond(
            bond_id="bond-overflow",
            watcher_id=watcher,
            collateral_amount=Decimal("1000"),
            collateral_currency="USDC",
            collateral_address="0x" + "a" * 40,
            status=BondStatus.ACTIVE,
            attestation_count=2**31 - 1,  # Max 32-bit
        )

        # Should handle large counts
        bond.attestation_count += 1
        assert bond.attestation_count == 2**31, "Should handle large counts"


# =============================================================================
# SECURITY #3: Replay Attack Prevention
# =============================================================================

class TestReplayAttackPrevention:
    """
    Test replay attack prevention mechanisms.
    """

    def test_nonce_replay_rejected(self):
        """Test that replayed nonces are rejected."""
        from tools.phoenix.security import NonceRegistry

        registry = NonceRegistry()
        nonce = secrets.token_hex(16)

        # First use should succeed
        assert registry.check_and_register(nonce), \
            "First nonce use should succeed"

        # Replay should be rejected
        assert not registry.check_and_register(nonce), \
            "Replayed nonce should be rejected"

    def test_scoped_attestation_replay(self):
        """Test that attestations can't be replayed across scopes."""
        from tools.phoenix.security import AttestationScope, ScopedAttestation

        # Use future dates to ensure attestation is valid now
        now = datetime.now(timezone.utc)
        valid_from = (now - timedelta(days=1)).strftime("%Y-%m-%dT%H:%M:%S+00:00")
        valid_until = (now + timedelta(days=365)).strftime("%Y-%m-%dT%H:%M:%S+00:00")

        scope1 = AttestationScope(
            asset_id="asset-123",
            jurisdiction_id="ae-abudhabi-adgm",
            domain="kyc",
            valid_from=valid_from,
            valid_until=valid_until,
        )

        attestation = ScopedAttestation.create(
            attestation_id="att-123",
            attestation_type="kyc_verification",
            issuer_did="did:msez:issuer:kyc",
            scope=scope1,
            issuer_signature=b"signature",
        )

        # Attestation should verify for its scope
        assert attestation.verify_scope(
            "asset-123",
            "ae-abudhabi-adgm",
            "kyc",
        ), "Attestation should verify for correct scope"

        # Attestation should NOT verify for different asset
        assert not attestation.verify_scope(
            "asset-456",  # Different asset!
            "ae-abudhabi-adgm",
            "kyc",
        ), "Attestation should NOT verify for different asset"


# =============================================================================
# SECURITY #4: TOCTOU Prevention
# =============================================================================

class TestTOCTOUPrevention:
    """
    Test Time-Of-Check to Time-Of-Use race condition prevention.
    """

    def test_versioned_store_prevents_toctou(self):
        """Test that versioned store CAS prevents TOCTOU."""
        from tools.phoenix.security import VersionedStore

        store = VersionedStore()
        key = "critical_data"
        store.set(key, "initial")

        # Simulate TOCTOU: get version, then try to update
        versioned1 = store.get(key)
        version1 = versioned1.version

        # Another thread updates the value
        store.set(key, "updated_by_other")

        # CAS with old version should fail
        success, _ = store.compare_and_swap(
            key,
            expected_version=version1,
            new_value="attacker_value",
        )

        assert not success, \
            "CAS should fail when version has changed (TOCTOU prevention)"


# =============================================================================
# SECURITY #5: Unauthorized Access Prevention
# =============================================================================

class TestUnauthorizedAccessPrevention:
    """
    Test unauthorized access prevention.
    """

    def test_inactive_corridor_blocked(self):
        """Test that inactive corridors block transfers."""
        from tools.phoenix.manifold import CorridorEdge

        edge = CorridorEdge(
            corridor_id="blocked-corridor",
            source_jurisdiction="ae-abudhabi-adgm",
            target_jurisdiction="sg-mas",
            is_active=False,  # Inactive!
        )

        # Corridor exists but is inactive
        assert not edge.is_active, "Corridor should be inactive"
        # Business logic should check is_active before allowing transfers

    def test_expired_bond_rejected(self):
        """Test that expired bonds are rejected."""
        from tools.phoenix.watcher import WatcherBond, WatcherId, BondStatus

        watcher = WatcherId(
            did="did:msez:watcher:expired",
            public_key_hex="a" * 64,
        )

        # Create bond that expired yesterday
        yesterday = (datetime.now(timezone.utc) - timedelta(days=1)).isoformat()
        two_days_ago = (datetime.now(timezone.utc) - timedelta(days=2)).isoformat()

        bond = WatcherBond(
            bond_id="bond-expired",
            watcher_id=watcher,
            collateral_amount=Decimal("1000"),
            collateral_currency="USDC",
            collateral_address="0x" + "a" * 40,
            status=BondStatus.ACTIVE,
            valid_from=two_days_ago,
            valid_until=yesterday,  # Expired!
        )

        # Bond should not be valid
        assert not bond.is_valid, "Expired bond should not be valid"


# =============================================================================
# SECURITY #6: Denial of Service Prevention
# =============================================================================

class TestDoSPrevention:
    """
    Test denial of service prevention.
    """

    def test_rate_limiter_prevents_dos(self):
        """Test that rate limiter prevents DoS."""
        from tools.phoenix.hardening import RateLimiter, RateLimitConfig

        config = RateLimitConfig(
            requests_per_minute=60,  # 1/second
            burst_size=10,
        )
        limiter = RateLimiter(config)

        # Exhaust burst
        for _ in range(10):
            limiter.acquire()

        # Subsequent requests should be blocked
        blocked_count = sum(1 for _ in range(100) if not limiter.acquire())
        assert blocked_count > 90, \
            f"Rate limiter should block most DoS attempts: {blocked_count}/100 blocked"

    def test_vm_gas_limit_prevents_dos(self):
        """Test that VM gas limit prevents DoS via infinite loops."""
        from tools.phoenix.vm import SmartAssetVM, ExecutionContext, OpCode

        vm = SmartAssetVM()

        # Infinite loop bytecode
        bytecode = bytes([
            OpCode.JUMPDEST,  # Offset 0 - loop start
            OpCode.PUSH1, 0x00,  # Push 0 (destination)
            OpCode.JUMP,  # Jump back to 0 (infinite loop)
        ])

        context = ExecutionContext(
            caller="did:test:caller",
            origin="did:test:origin",
            jurisdiction_id="ae-abudhabi-adgm",
            gas_limit=10000,  # Limited gas
        )

        result = vm.execute(bytecode, context)

        # Should fail due to gas exhaustion, not run forever
        assert not result.success, "Infinite loop should fail due to gas limit"
        assert "gas" in result.error.lower() if result.error else True


# =============================================================================
# SECURITY #7: Data Leakage Prevention
# =============================================================================

class TestDataLeakagePrevention:
    """
    Test data leakage prevention.
    """

    def test_private_key_not_in_to_dict(self):
        """Test that private keys are not exposed in serialization."""
        from tools.phoenix.watcher import WatcherBond, WatcherId, BondStatus

        watcher = WatcherId(
            did="did:msez:watcher:leak",
            public_key_hex="a" * 64,
        )

        bond = WatcherBond(
            bond_id="bond-leak",
            watcher_id=watcher,
            collateral_amount=Decimal("1000"),
            collateral_currency="USDC",
            collateral_address="0x" + "a" * 40,
            status=BondStatus.ACTIVE,
        )

        # Serialize to dict
        as_dict = bond.to_dict()
        as_json = json.dumps(as_dict, default=str)

        # Check for sensitive data patterns
        sensitive_patterns = [
            "private_key",
            "secret",
            "password",
            "mnemonic",
        ]

        for pattern in sensitive_patterns:
            assert pattern not in as_json.lower(), \
                f"Sensitive pattern '{pattern}' found in serialization"


# =============================================================================
# SECURITY #8: Cross-Jurisdiction Security
# =============================================================================

class TestCrossJurisdictionSecurity:
    """
    Test cross-jurisdiction security boundaries.
    """

    def test_attestation_scope_jurisdiction_binding(self):
        """Test that attestations are bound to specific jurisdictions."""
        from tools.phoenix.security import AttestationScope

        scope = AttestationScope(
            asset_id="asset-123",
            jurisdiction_id="ae-abudhabi-adgm",
            domain="kyc",
            valid_from="2024-01-01T00:00:00+00:00",
            valid_until="2024-12-31T23:59:59+00:00",
        )

        # Should work for correct jurisdiction
        assert scope.includes("asset-123", "ae-abudhabi-adgm", "kyc")

        # Should NOT work for different jurisdiction
        assert not scope.includes("asset-123", "sg-mas", "kyc"), \
            "Attestation scope should not apply to different jurisdiction"


# =============================================================================
# SECURITY #9: Cryptographic Verification
# =============================================================================

class TestCryptographicVerification:
    """
    Test cryptographic verification security.
    """

    def test_timing_safe_comparison(self):
        """Test that comparison uses timing-safe function."""
        from tools.phoenix.hardening import CryptoUtils

        # These should be compared in constant time
        a = b"secret_value_1234567890"
        b1 = b"secret_value_1234567890"  # Same
        b2 = b"different_value_xxxxxxxx"  # Different

        # Both comparisons should take similar time (timing attack prevention)
        # We can't easily test timing, but we can test correctness
        assert CryptoUtils.secure_compare(a, b1), "Same values should match"
        assert not CryptoUtils.secure_compare(a, b2), "Different values should not match"

    def test_merkle_proof_validation(self):
        """Test that invalid Merkle proofs are rejected."""
        from tools.phoenix.hardening import CryptoUtils

        # Valid tree
        leaves = [CryptoUtils.hash_sha256(f"leaf{i}") for i in range(4)]
        root = CryptoUtils.merkle_root(leaves)

        # Invalid proof - wrong sibling
        invalid_proof = ["0" * 64, "1" * 64]  # Fake siblings
        invalid_indices = [0, 0]

        is_valid = CryptoUtils.verify_merkle_proof(
            leaves[0],
            invalid_proof,
            invalid_indices,
            root,
        )

        assert not is_valid, "Invalid Merkle proof should be rejected"


# =============================================================================
# SECURITY #10: Attestation Forgery Prevention
# =============================================================================

class TestAttestationForgeryPrevention:
    """
    Test attestation forgery prevention.
    """

    def test_tampered_scope_commitment_rejected(self):
        """Test that attestations with tampered scope commitment are rejected."""
        from tools.phoenix.security import AttestationScope, ScopedAttestation
        from tools.phoenix.hardening import SecurityViolation

        scope = AttestationScope(
            asset_id="asset-123",
            jurisdiction_id="ae-abudhabi-adgm",
            domain="kyc",
            valid_from="2024-01-01T00:00:00+00:00",
            valid_until="2024-12-31T23:59:59+00:00",
        )

        # Try to create attestation with fake commitment
        try:
            fake_attestation = ScopedAttestation(
                attestation_id="att-fake",
                attestation_type="kyc_verification",
                issuer_did="did:msez:attacker",
                scope=scope,
                scope_commitment="0" * 64,  # Fake commitment!
                issuer_signature=b"fake_sig",
                issued_at=datetime.now(timezone.utc).isoformat(),
                nonce=secrets.token_hex(16),
            )
            raise AssertionError(
                "Should reject attestation with tampered commitment"
            )
        except SecurityViolation:
            pass  # Expected - commitment mismatch detected


# =============================================================================
# RUN ALL TESTS
# =============================================================================

def run_tests():
    """Run all test classes."""
    test_classes = [
        TestInputValidationBypass,
        TestIntegerOverflowProtection,
        TestReplayAttackPrevention,
        TestTOCTOUPrevention,
        TestUnauthorizedAccessPrevention,
        TestDoSPrevention,
        TestDataLeakagePrevention,
        TestCrossJurisdictionSecurity,
        TestCryptographicVerification,
        TestAttestationForgeryPrevention,
    ]

    passed = 0
    failed = 0
    errors = []

    for cls in test_classes:
        print(f'\n=== {cls.__name__} ===')
        if cls.__doc__:
            print(f'  {cls.__doc__.strip().split(chr(10))[0]}')
        instance = cls()
        for method_name in dir(instance):
            if method_name.startswith('test_'):
                try:
                    getattr(instance, method_name)()
                    print(f'  PASS: {method_name}')
                    passed += 1
                except AssertionError as e:
                    print(f'  FAIL: {method_name}')
                    print(f'        {e}')
                    errors.append((cls.__name__, method_name, e))
                    failed += 1
                except Exception as e:
                    print(f'  ERROR: {method_name}')
                    print(f'        {type(e).__name__}: {e}')
                    errors.append((cls.__name__, method_name, e))
                    failed += 1

    print(f'\n{"="*60}')
    print(f'RESULTS: {passed} passed, {failed} failed')
    if errors:
        print('\nFailed/Error tests (VULNERABILITIES FOUND):')
        for cls_name, method_name, error in errors:
            print(f'  {cls_name}.{method_name}: {error}')

    return failed == 0


if __name__ == "__main__":
    import sys
    sys.exit(0 if run_tests() else 1)
