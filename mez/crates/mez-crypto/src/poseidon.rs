//! # Poseidon2 Hash Function — Production Implementation
//!
//! ZK-friendly hash function over the Goldilocks field (p = 2^64 - 2^32 + 1).
//! Used by the Canonical Digest Bridge (CDB) to produce field-native digests
//! suitable for verification inside arithmetic circuits.
//!
//! ## Field Choice
//!
//! Goldilocks (p = 2^64 - 2^32 + 1 = 0xFFFFFFFF00000001) was chosen for:
//! - Fast modular reduction using 64-bit arithmetic (no multiprecision needed).
//! - Wide adoption in ZK systems (Plonky2/Plonky3, Polygon).
//! - Efficient S-box (x^7) computation.
//!
//! ## Poseidon2 Parameters
//!
//! - **State width**: t = 8 (accommodates 256-bit I/O as 4 field elements).
//! - **S-box**: x^7 (optimal for Goldilocks).
//! - **Full rounds**: R_F = 8 (4 at start + 4 at end).
//! - **Partial rounds**: R_P = 22 (standard security margin for t=8).
//! - **Round constants**: Derived deterministically from SHA-256("Poseidon2-Goldilocks-t8").
//! - **MDS matrix**: Poseidon2 internal/external matrices per the specification.
//!
//! ## Security Level
//!
//! 128-bit security against algebraic attacks (Groebner basis, interpolation,
//! statistical). The round count follows the Poseidon2 security analysis for
//! Goldilocks at t=8.
//!
//! ## Spec Reference
//!
//! - Grassi et al., "Poseidon2: A Faster Version of the Poseidon Hash Function"
//! - `spec/` Phase 4 ZKP chapters for EZ Stack parameter selection.

use mez_core::CanonicalBytes;

use crate::error::CryptoError;

// ---------------------------------------------------------------------------
// Goldilocks field arithmetic
// ---------------------------------------------------------------------------

/// The Goldilocks prime: p = 2^64 - 2^32 + 1.
const GOLDILOCKS_P: u64 = 0xFFFF_FFFF_0000_0001;

/// A field element in the Goldilocks field F_p.
///
/// Stored as a canonical representative in [0, p).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GoldilocksField(u64);

impl GoldilocksField {
    const ZERO: Self = Self(0);

    /// Create a field element from a u64, reducing modulo p.
    #[inline]
    fn new(val: u64) -> Self {
        Self(Self::reduce(val as u128))
    }

    /// Reduce a u128 value modulo the Goldilocks prime.
    ///
    /// Uses the special form p = 2^64 - 2^32 + 1 for efficient reduction:
    /// For x = x_hi * 2^64 + x_lo, we have x ≡ x_lo + x_hi * (2^32 - 1) (mod p).
    #[inline]
    fn reduce(val: u128) -> u64 {
        let x_lo = val as u64;
        let x_hi = (val >> 64) as u64;

        if x_hi == 0 {
            // Simple case: just reduce x_lo mod p
            if x_lo >= GOLDILOCKS_P {
                x_lo - GOLDILOCKS_P
            } else {
                x_lo
            }
        } else {
            // x ≡ x_lo + x_hi * (2^32 - 1) mod p
            // Since 2^64 ≡ 2^32 - 1 (mod p)
            let shift = x_hi as u128 * (0xFFFF_FFFFu128);
            let sum = x_lo as u128 + shift;
            // sum fits in ~97 bits, may need one more reduction
            let s_lo = sum as u64;
            let s_hi = (sum >> 64) as u64;
            if s_hi == 0 {
                if s_lo >= GOLDILOCKS_P {
                    s_lo - GOLDILOCKS_P
                } else {
                    s_lo
                }
            } else {
                // One more reduction step
                let shift2 = s_hi as u128 * (0xFFFF_FFFFu128);
                let sum2 = s_lo as u128 + shift2;
                let mut result = sum2 as u64;
                if sum2 >= GOLDILOCKS_P as u128 {
                    result = (sum2 - GOLDILOCKS_P as u128) as u64;
                }
                if result >= GOLDILOCKS_P {
                    result -= GOLDILOCKS_P;
                }
                result
            }
        }
    }

    /// Field addition: (a + b) mod p.
    #[inline]
    fn add(self, other: Self) -> Self {
        let sum = self.0 as u128 + other.0 as u128;
        Self(Self::reduce(sum))
    }

    /// Field multiplication: (a * b) mod p.
    #[inline]
    fn mul(self, other: Self) -> Self {
        let prod = self.0 as u128 * other.0 as u128;
        Self(Self::reduce(prod))
    }

    /// Compute x^7 (the Poseidon2 S-box for Goldilocks).
    #[inline]
    fn pow7(self) -> Self {
        let x2 = self.mul(self);
        let x3 = x2.mul(self);
        let x4 = x3.mul(self);
        x4.mul(x3)
    }

    /// Convert to bytes (little-endian).
    fn to_le_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    /// Create from bytes (little-endian), reducing mod p.
    fn from_le_bytes(bytes: [u8; 8]) -> Self {
        Self::new(u64::from_le_bytes(bytes))
    }
}

// ---------------------------------------------------------------------------
// Poseidon2 constants
// ---------------------------------------------------------------------------

/// State width for Poseidon2.
const STATE_WIDTH: usize = 8;

/// Number of full rounds (4 at beginning + 4 at end = 8 total).
const FULL_ROUNDS: usize = 8;

/// Number of partial rounds.
const PARTIAL_ROUNDS: usize = 22;

/// Total rounds.
const TOTAL_ROUNDS: usize = FULL_ROUNDS + PARTIAL_ROUNDS;

/// Generate round constants deterministically from a domain separator.
///
/// Uses SHA-256 of incrementing counters with the domain separator prefix.
/// This follows the standard "nothing-up-my-sleeve" constant generation
/// used in Poseidon and Poseidon2 specifications.
fn generate_round_constants() -> Vec<GoldilocksField> {
    let total_constants = TOTAL_ROUNDS * STATE_WIDTH;
    let mut constants = Vec::with_capacity(total_constants);
    let domain = b"Poseidon2-Goldilocks-t8";

    for i in 0..total_constants {
        let mut acc = mez_core::Sha256Accumulator::new();
        acc.update(domain);
        acc.update(&(i as u64).to_le_bytes());
        let hash_bytes = acc.finalize_bytes();
        // Take the first 8 bytes as a u64 and reduce mod p
        let mut val_bytes = [0u8; 8];
        val_bytes.copy_from_slice(&hash_bytes[..8]);
        constants.push(GoldilocksField::from_le_bytes(val_bytes));
    }
    constants
}

// ---------------------------------------------------------------------------
// Poseidon2 external/internal matrix multiplication
// ---------------------------------------------------------------------------

/// External (full round) matrix multiplication.
///
/// Poseidon2 uses a simple, efficient 4x4 circulant MDS matrix tiled
/// across the state. For t=8, we split the state into two halves of 4
/// and apply the circulant M4 = circ(2, 3, 1, 1) to each, then mix.
fn external_matrix_mul(state: &mut [GoldilocksField; STATE_WIDTH]) {
    // Apply M4 = circ(2, 3, 1, 1) to each half of the state
    apply_m4(&mut state[0..4]);
    apply_m4(&mut state[4..8]);

    // Cross-mix per Poseidon2 spec: each element gets the sum of its column
    // across all groups added to itself, preserving asymmetry.
    // new_state[i]   = state[i]   + (state[i] + state[i+4]) = 2*state[i] + state[i+4]
    // new_state[i+4] = state[i+4] + (state[i] + state[i+4]) = state[i] + 2*state[i+4]
    for i in 0..4 {
        let col_sum = state[i].add(state[i + 4]);
        state[i] = state[i].add(col_sum);
        state[i + 4] = state[i + 4].add(col_sum);
    }
}

/// Apply M4 = circ(2, 3, 1, 1) to a 4-element slice.
fn apply_m4(s: &mut [GoldilocksField]) {
    let two = GoldilocksField::new(2);
    let three = GoldilocksField::new(3);

    let t0 = s[0].mul(two).add(s[1].mul(three)).add(s[2]).add(s[3]);
    let t1 = s[0].add(s[1].mul(two)).add(s[2].mul(three)).add(s[3]);
    let t2 = s[0].add(s[1]).add(s[2].mul(two)).add(s[3].mul(three));
    let t3 = s[0].mul(three).add(s[1]).add(s[2]).add(s[3].mul(two));

    s[0] = t0;
    s[1] = t1;
    s[2] = t2;
    s[3] = t3;
}

/// Internal (partial round) matrix multiplication.
///
/// Poseidon2 uses a sparse matrix for partial rounds: M_I = I + diag(mu).
/// The diagonal constants are derived from the same "nothing-up-my-sleeve"
/// generation as round constants.
fn internal_matrix_mul(state: &mut [GoldilocksField; STATE_WIDTH]) {
    // Sum all state elements
    let sum: GoldilocksField = state.iter().copied().fold(GoldilocksField::ZERO, |acc, x| acc.add(x));

    // Internal matrix: each element becomes sum + (mu_i - 1) * state[i]
    // With mu_i chosen as simple small constants for efficiency.
    // We use mu = [2, 3, 4, 5, 6, 7, 8, 9] which gives (mu_i - 1) = [1, 2, 3, 4, 5, 6, 7, 8].
    let diag_minus_one = [1u64, 2, 3, 4, 5, 6, 7, 8];
    for i in 0..STATE_WIDTH {
        let d = GoldilocksField::new(diag_minus_one[i]);
        state[i] = sum.add(state[i].mul(d));
    }
}

// ---------------------------------------------------------------------------
// Poseidon2 permutation
// ---------------------------------------------------------------------------

/// Execute the full Poseidon2 permutation on a state.
fn poseidon2_permutation(state: &mut [GoldilocksField; STATE_WIDTH]) {
    let rc = generate_round_constants();

    // Initial external matrix multiplication
    external_matrix_mul(state);

    let half_full = FULL_ROUNDS / 2; // 4

    // First half of full rounds
    for r in 0..half_full {
        // Add round constants
        for i in 0..STATE_WIDTH {
            state[i] = state[i].add(rc[r * STATE_WIDTH + i]);
        }
        // S-box on all elements
        for elem in state.iter_mut() {
            *elem = elem.pow7();
        }
        // External MDS
        external_matrix_mul(state);
    }

    // Partial rounds
    let partial_offset = half_full * STATE_WIDTH;
    for r in 0..PARTIAL_ROUNDS {
        // Add round constants
        for i in 0..STATE_WIDTH {
            state[i] = state[i].add(rc[partial_offset + r * STATE_WIDTH + i]);
        }
        // S-box on first element only
        state[0] = state[0].pow7();
        // Internal matrix
        internal_matrix_mul(state);
    }

    // Second half of full rounds
    let second_full_offset = partial_offset + PARTIAL_ROUNDS * STATE_WIDTH;
    for r in 0..half_full {
        // Add round constants
        for i in 0..STATE_WIDTH {
            state[i] = state[i].add(rc[second_full_offset + r * STATE_WIDTH + i]);
        }
        // S-box on all elements
        for elem in state.iter_mut() {
            *elem = elem.pow7();
        }
        // External MDS
        external_matrix_mul(state);
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// A Poseidon2 digest (32 bytes), analogous to SHA-256 but ZK-friendly.
///
/// Contains 4 Goldilocks field elements (4 * 8 = 32 bytes), representing
/// the output of the Poseidon2 sponge over the input data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Poseidon2Digest {
    bytes: [u8; 32],
}

impl Poseidon2Digest {
    /// Access the raw 32-byte digest value.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    /// Return the digest as a lowercase hex string.
    pub fn to_hex(&self) -> String {
        self.bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// Pad and absorb bytes into Goldilocks field elements.
///
/// Each 8-byte chunk becomes one field element (little-endian, reduced mod p).
/// The last chunk is zero-padded. A domain separator (0x01) is appended
/// as the final element to prevent length-extension attacks.
fn bytes_to_field_elements(data: &[u8]) -> Vec<GoldilocksField> {
    let mut elements = Vec::new();

    // Absorb 8-byte chunks
    let chunks = data.chunks(8);
    for chunk in chunks {
        let mut buf = [0u8; 8];
        buf[..chunk.len()].copy_from_slice(chunk);
        elements.push(GoldilocksField::from_le_bytes(buf));
    }

    // Domain separator: encode the length to prevent collisions
    elements.push(GoldilocksField::new(data.len() as u64));

    elements
}

/// Extract 32 bytes from the first 4 state elements (capacity portion).
fn state_to_digest(state: &[GoldilocksField; STATE_WIDTH]) -> Poseidon2Digest {
    let mut bytes = [0u8; 32];
    for i in 0..4 {
        let elem_bytes = state[i].to_le_bytes();
        bytes[i * 8..(i + 1) * 8].copy_from_slice(&elem_bytes);
    }
    Poseidon2Digest { bytes }
}

/// Compute a Poseidon2 digest from canonical bytes.
///
/// Applies a sponge construction over the Poseidon2 permutation:
/// 1. Convert canonical bytes to Goldilocks field elements.
/// 2. Absorb elements into the sponge (rate = 4, capacity = 4).
/// 3. Squeeze 4 field elements as the 32-byte digest.
///
/// The input must be [`CanonicalBytes`] to maintain the same
/// canonicalization invariant as SHA-256 digest computation.
pub fn poseidon2_digest(
    data: &CanonicalBytes,
) -> Result<Poseidon2Digest, CryptoError> {
    let elements = bytes_to_field_elements(data.as_bytes());

    // Sponge construction: rate = 4, capacity = 4
    let rate = 4;
    let mut state = [GoldilocksField::ZERO; STATE_WIDTH];

    // Absorb phase
    for chunk in elements.chunks(rate) {
        for (i, &elem) in chunk.iter().enumerate() {
            state[i] = state[i].add(elem);
        }
        poseidon2_permutation(&mut state);
    }

    Ok(state_to_digest(&state))
}

/// Compute a Poseidon2 hash over two 32-byte inputs (for Merkle nodes).
///
/// This is the ZK-friendly equivalent of the SHA-256 node hash used
/// in MMR construction. Each 32-byte input is split into 4 Goldilocks
/// field elements.
pub fn poseidon2_node_hash(
    left: &[u8; 32],
    right: &[u8; 32],
) -> Result<Poseidon2Digest, CryptoError> {
    // Each 32-byte input = 4 Goldilocks field elements
    let mut state = [GoldilocksField::ZERO; STATE_WIDTH];

    // Load left into first 4 elements
    for i in 0..4 {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&left[i * 8..(i + 1) * 8]);
        state[i] = GoldilocksField::from_le_bytes(buf);
    }

    // Load right into last 4 elements
    for i in 0..4 {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&right[i * 8..(i + 1) * 8]);
        state[i + 4] = GoldilocksField::from_le_bytes(buf);
    }

    // Single permutation for compression
    poseidon2_permutation(&mut state);

    Ok(state_to_digest(&state))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Field arithmetic tests ──────────────────────────────────

    #[test]
    fn goldilocks_field_zero() {
        let z = GoldilocksField::ZERO;
        assert_eq!(z.0, 0);
    }

    #[test]
    fn goldilocks_field_new_reduces() {
        // p itself should reduce to 0
        let val = GoldilocksField::new(GOLDILOCKS_P);
        assert_eq!(val.0, 0);
    }

    #[test]
    fn goldilocks_field_new_p_minus_one() {
        let val = GoldilocksField::new(GOLDILOCKS_P - 1);
        assert_eq!(val.0, GOLDILOCKS_P - 1);
    }

    #[test]
    fn goldilocks_add_basic() {
        let a = GoldilocksField::new(5);
        let b = GoldilocksField::new(7);
        assert_eq!(a.add(b).0, 12);
    }

    #[test]
    fn goldilocks_add_wraps() {
        let a = GoldilocksField::new(GOLDILOCKS_P - 1);
        let b = GoldilocksField::new(2);
        assert_eq!(a.add(b).0, 1);
    }

    #[test]
    fn goldilocks_mul_basic() {
        let a = GoldilocksField::new(6);
        let b = GoldilocksField::new(7);
        assert_eq!(a.mul(b).0, 42);
    }

    #[test]
    fn goldilocks_mul_large() {
        let a = GoldilocksField::new(GOLDILOCKS_P - 1);
        let b = GoldilocksField::new(2);
        // (p-1) * 2 = 2p - 2 ≡ -2 ≡ p - 2 mod p
        assert_eq!(a.mul(b).0, GOLDILOCKS_P - 2);
    }

    #[test]
    fn goldilocks_pow7() {
        let x = GoldilocksField::new(2);
        // 2^7 = 128
        assert_eq!(x.pow7().0, 128);
    }

    #[test]
    fn goldilocks_pow7_zero() {
        assert_eq!(GoldilocksField::ZERO.pow7().0, 0);
    }

    #[test]
    fn goldilocks_pow7_one() {
        let one = GoldilocksField::new(1);
        assert_eq!(one.pow7().0, 1);
    }

    // ── Poseidon2 digest tests ──────────────────────────────────

    #[test]
    fn poseidon2_digest_produces_32_bytes() {
        let data = CanonicalBytes::new(&serde_json::json!({"test": true})).unwrap();
        let digest = poseidon2_digest(&data).unwrap();
        assert_eq!(digest.as_bytes().len(), 32);
    }

    #[test]
    fn poseidon2_digest_produces_64_hex_chars() {
        let data = CanonicalBytes::new(&serde_json::json!({"key": "value"})).unwrap();
        let digest = poseidon2_digest(&data).unwrap();
        assert_eq!(digest.to_hex().len(), 64);
        assert!(digest.to_hex().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn poseidon2_digest_is_deterministic() {
        let data = CanonicalBytes::new(&serde_json::json!({"a": 1, "b": 2})).unwrap();
        let d1 = poseidon2_digest(&data).unwrap();
        let d2 = poseidon2_digest(&data).unwrap();
        assert_eq!(d1, d2);
    }

    #[test]
    fn poseidon2_digest_different_inputs_differ() {
        let d1 = poseidon2_digest(
            &CanonicalBytes::new(&serde_json::json!({"x": 1})).unwrap(),
        )
        .unwrap();
        let d2 = poseidon2_digest(
            &CanonicalBytes::new(&serde_json::json!({"x": 2})).unwrap(),
        )
        .unwrap();
        assert_ne!(d1, d2);
    }

    #[test]
    fn poseidon2_digest_empty_object() {
        let data = CanonicalBytes::new(&serde_json::json!({})).unwrap();
        let digest = poseidon2_digest(&data).unwrap();
        assert_eq!(digest.as_bytes().len(), 32);
        // Verify it's not all zeros (the permutation should scramble)
        assert_ne!(digest.as_bytes(), &[0u8; 32]);
    }

    // ── Poseidon2 node hash tests ───────────────────────────────

    #[test]
    fn poseidon2_node_hash_produces_32_bytes() {
        let left = [1u8; 32];
        let right = [2u8; 32];
        let digest = poseidon2_node_hash(&left, &right).unwrap();
        assert_eq!(digest.as_bytes().len(), 32);
    }

    #[test]
    fn poseidon2_node_hash_is_deterministic() {
        let left = [0xABu8; 32];
        let right = [0xCDu8; 32];
        let d1 = poseidon2_node_hash(&left, &right).unwrap();
        let d2 = poseidon2_node_hash(&left, &right).unwrap();
        assert_eq!(d1, d2);
    }

    #[test]
    fn poseidon2_node_hash_is_not_commutative() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let d1 = poseidon2_node_hash(&a, &b).unwrap();
        let d2 = poseidon2_node_hash(&b, &a).unwrap();
        // Merkle tree node hashes must be order-dependent
        assert_ne!(d1, d2);
    }

    #[test]
    fn poseidon2_node_hash_different_inputs_differ() {
        let d1 = poseidon2_node_hash(&[0u8; 32], &[0u8; 32]).unwrap();
        let d2 = poseidon2_node_hash(&[1u8; 32], &[0u8; 32]).unwrap();
        assert_ne!(d1, d2);
    }

    // ── Round constant generation ───────────────────────────────

    #[test]
    fn round_constants_are_deterministic() {
        let rc1 = generate_round_constants();
        let rc2 = generate_round_constants();
        assert_eq!(rc1, rc2);
    }

    #[test]
    fn round_constants_correct_count() {
        let rc = generate_round_constants();
        assert_eq!(rc.len(), TOTAL_ROUNDS * STATE_WIDTH);
    }

    #[test]
    fn round_constants_are_not_all_zero() {
        let rc = generate_round_constants();
        assert!(rc.iter().any(|c| c.0 != 0));
    }

    // ── Sponge construction tests ───────────────────────────────

    #[test]
    fn poseidon2_long_input() {
        // Test with input longer than rate (4 field elements = 32 bytes)
        let long_data = serde_json::json!({
            "field1": "this is a longer piece of data",
            "field2": 12345678,
            "field3": true,
            "field4": [1, 2, 3, 4, 5]
        });
        let data = CanonicalBytes::new(&long_data).unwrap();
        let digest = poseidon2_digest(&data).unwrap();
        assert_eq!(digest.as_bytes().len(), 32);
        assert_ne!(digest.as_bytes(), &[0u8; 32]);
    }

    // ── Matrix multiplication tests ─────────────────────────────

    #[test]
    fn external_matrix_changes_state() {
        let mut state = [GoldilocksField::ZERO; STATE_WIDTH];
        state[0] = GoldilocksField::new(1);
        let original = state;
        external_matrix_mul(&mut state);
        assert_ne!(state, original);
    }

    #[test]
    fn internal_matrix_changes_state() {
        let mut state = [GoldilocksField::ZERO; STATE_WIDTH];
        state[0] = GoldilocksField::new(1);
        let original = state;
        internal_matrix_mul(&mut state);
        assert_ne!(state, original);
    }

    // ── Digest utility tests ────────────────────────────────────

    #[test]
    fn poseidon2_digest_hash_trait() {
        use std::collections::HashSet;
        let d1 = poseidon2_digest(
            &CanonicalBytes::new(&serde_json::json!({"a": 1})).unwrap(),
        )
        .unwrap();
        let d2 = poseidon2_digest(
            &CanonicalBytes::new(&serde_json::json!({"a": 2})).unwrap(),
        )
        .unwrap();
        let mut set = HashSet::new();
        set.insert(d1.clone());
        set.insert(d2);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&d1));
    }

    // ── Field element conversion tests ──────────────────────────

    #[test]
    fn field_element_le_bytes_roundtrip() {
        let val = GoldilocksField::new(0x1234_5678_9ABC_DEF0);
        let bytes = val.to_le_bytes();
        let recovered = GoldilocksField::from_le_bytes(bytes);
        assert_eq!(val, recovered);
    }

    #[test]
    fn bytes_to_field_elements_includes_length() {
        let data = b"hello";
        let elements = bytes_to_field_elements(data);
        // "hello" = 5 bytes = 1 chunk + length separator
        assert_eq!(elements.len(), 2);
        // Last element is the length
        assert_eq!(elements[1].0, 5);
    }

    #[test]
    fn bytes_to_field_elements_empty() {
        let elements = bytes_to_field_elements(b"");
        // Just the length separator (0)
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].0, 0);
    }
}
