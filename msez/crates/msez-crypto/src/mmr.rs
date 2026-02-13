//! # Merkle Mountain Range (MMR)
//!
//! Port of `tools/mmr.py` — an append-only accumulator that supports compact
//! inclusion proofs for receipts without requiring disclosure of the full
//! receipt set.
//!
//! ## Design Goals
//!
//! - **Deterministic** across Rust and Python implementations.
//! - **Inclusion-proof-friendly** for receipt commitments.
//! - **Cross-language compatible** — produces identical roots and proofs as
//!   the Python `tools/mmr.py` for the same input sequence.
//!
//! ## Hashing (Domain Separation)
//!
//! - Leaf: `SHA256(0x00 || leaf_bytes)` where `leaf_bytes` is the 32-byte
//!   `next_root` digest of a corridor state receipt.
//! - Node: `SHA256(0x01 || left_hash || right_hash)`.
//!
//! ## Spec Reference
//!
//! Implements the receipt chain MMR per `spec/40-corridors.md` and the audit
//! document Part IV (receipt chain + MMR).

use sha2::{Digest, Sha256};

use crate::error::CryptoError;

// ---------------------------------------------------------------------------
// Internal helpers (matching tools/mmr.py)
// ---------------------------------------------------------------------------

/// SHA256 helper returning raw 32 bytes.
fn sha256_raw(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Encode bytes as lowercase hex string.
fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Decode hex string to bytes. Returns error if invalid.
fn from_hex(s: &str) -> Result<Vec<u8>, CryptoError> {
    let s = s.trim().to_lowercase();
    if s.len() % 2 != 0 {
        return Err(CryptoError::Mmr(format!(
            "hex string has odd length: {}",
            s.len()
        )));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| CryptoError::Mmr(format!("invalid hex at position {i}: {e}")))
        })
        .collect()
}

/// Validate that a string is 64 lowercase hex chars (32 bytes).
fn is_hex_32(s: &str) -> bool {
    let s = s.trim();
    if s.len() != 64 {
        return false;
    }
    s.chars().all(|c| c.is_ascii_hexdigit())
}

// ---------------------------------------------------------------------------
// Public hashing functions (matching tools/mmr.py API)
// ---------------------------------------------------------------------------

/// Compute the MMR leaf hash from a 32-byte digest encoded as 64 hex chars.
///
/// Domain separation: `SHA256(0x00 || leaf_bytes)`.
///
/// All leaf data entering the MMR must have been produced through the
/// `CanonicalBytes` -> `ContentDigest` pipeline. The `next_root_hex` parameter
/// is the hex encoding of such a `ContentDigest`.
///
/// Matches `tools/mmr.py:mmr_leaf_hash()`.
pub fn mmr_leaf_hash(next_root_hex: &str) -> Result<String, CryptoError> {
    if !is_hex_32(next_root_hex) {
        return Err(CryptoError::Mmr(
            "next_root_hex must be 64 lowercase hex chars".into(),
        ));
    }
    let leaf_bytes = from_hex(next_root_hex.trim())?;
    let mut input = Vec::with_capacity(1 + 32);
    input.push(0x00);
    input.extend_from_slice(&leaf_bytes);
    Ok(to_hex(&sha256_raw(&input)))
}

/// Compute a parent hash from two child hashes (each 64 hex chars).
///
/// Domain separation: `SHA256(0x01 || left || right)`.
///
/// Matches `tools/mmr.py:mmr_node_hash()`.
pub fn mmr_node_hash(left_hex: &str, right_hex: &str) -> Result<String, CryptoError> {
    if !is_hex_32(left_hex) || !is_hex_32(right_hex) {
        return Err(CryptoError::Mmr(
            "left_hex and right_hex must be 64 hex chars".into(),
        ));
    }
    let left = from_hex(left_hex.trim())?;
    let right = from_hex(right_hex.trim())?;
    let mut input = Vec::with_capacity(1 + 32 + 32);
    input.push(0x01);
    input.extend_from_slice(&left);
    input.extend_from_slice(&right);
    Ok(to_hex(&sha256_raw(&input)))
}

// ---------------------------------------------------------------------------
// Peak
// ---------------------------------------------------------------------------

/// A peak in the MMR — a perfect binary tree at a given height.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peak {
    /// Height of the peak (0 = single leaf, 1 = two leaves merged, etc.).
    pub height: usize,
    /// SHA256 hash as 64 lowercase hex chars.
    pub hash: String,
}

// ---------------------------------------------------------------------------
// Core MMR algorithms (matching tools/mmr.py)
// ---------------------------------------------------------------------------

/// Build MMR peaks for a list of leaf hashes (left-to-right append order).
///
/// Matches `tools/mmr.py:build_peaks()`.
pub fn build_peaks(leaf_hashes: &[String]) -> Result<Vec<Peak>, CryptoError> {
    let mut peaks: Vec<(usize, String)> = Vec::new();

    for lh in leaf_hashes {
        if !is_hex_32(lh) {
            return Err(CryptoError::Mmr("leaf_hash must be 64 hex chars".into()));
        }
        let mut cur_h: usize = 0;
        let mut cur = lh.trim().to_lowercase();

        // Merge while the top peak has the same height.
        while let Some(top) = peaks.last() {
            if top.0 != cur_h {
                break;
            }
            let (_, left) = peaks
                .pop()
                .ok_or_else(|| CryptoError::Mmr("peaks unexpectedly empty during merge".into()))?;
            cur = mmr_node_hash(&left, &cur)?;
            cur_h += 1;
        }
        peaks.push((cur_h, cur));
    }

    Ok(peaks
        .into_iter()
        .map(|(h, hash)| Peak { height: h, hash })
        .collect())
}

/// Compute the bagged root from peaks.
///
/// The root is computed by folding peaks from right-to-left using node hash:
/// ```text
/// bag = peaks[-1]
/// for peak in reversed(peaks[:-1]):
///     bag = node_hash(peak, bag)
/// ```
///
/// Matches `tools/mmr.py:bag_peaks()`.
pub fn bag_peaks(peaks: &[Peak]) -> Result<String, CryptoError> {
    if peaks.is_empty() {
        return Ok(String::new());
    }
    // Safe: is_empty() check above guarantees last() returns Some.
    let mut bag = peaks
        .last()
        .ok_or_else(|| CryptoError::Mmr("peaks unexpectedly empty after is_empty check".into()))?
        .hash
        .clone();
    for p in peaks[..peaks.len() - 1].iter().rev() {
        bag = mmr_node_hash(&p.hash, &bag)?;
    }
    Ok(bag)
}

/// Result of computing an MMR root from a sequence of receipt next_root digests.
#[derive(Debug, Clone)]
pub struct MmrRootInfo {
    /// Number of leaves in the MMR.
    pub size: usize,
    /// Bagged root hash (64 hex chars).
    pub root: String,
    /// Peak list.
    pub peaks: Vec<Peak>,
}

/// Compute MMR root and peaks from a sequence of receipt `next_root` digests.
///
/// Matches `tools/mmr.py:mmr_root_from_next_roots()`.
pub fn mmr_root_from_next_roots(next_roots_hex: &[String]) -> Result<MmrRootInfo, CryptoError> {
    let leaf_hashes: Result<Vec<String>, _> =
        next_roots_hex.iter().map(|nr| mmr_leaf_hash(nr)).collect();
    let leaf_hashes = leaf_hashes?;
    let peaks = build_peaks(&leaf_hashes)?;
    let root = bag_peaks(&peaks)?;
    Ok(MmrRootInfo {
        size: leaf_hashes.len(),
        root,
        peaks,
    })
}

// ---------------------------------------------------------------------------
// Peak plan and leaf location (for inclusion proofs)
// ---------------------------------------------------------------------------

/// Return a list of peaks as (height, leaf_count) from left-to-right for a
/// given leaf count.
///
/// Matches `tools/mmr.py:_peak_plan()`.
fn peak_plan(size: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut n = size;
    while n > 0 {
        let h = (usize::BITS - 1 - n.leading_zeros()) as usize;
        let cnt = 1usize << h;
        out.push((h, cnt));
        n -= cnt;
    }
    out
}

/// Return (peak_index, peak_start, peak_height) for a leaf at `leaf_index`
/// in an MMR of given size.
///
/// Matches `tools/mmr.py:_find_peak_for_leaf()`.
fn find_peak_for_leaf(
    size: usize,
    leaf_index: usize,
) -> Result<(usize, usize, usize), CryptoError> {
    if leaf_index >= size {
        return Err(CryptoError::Mmr("leaf_index out of range".into()));
    }
    let plan = peak_plan(size);
    let mut start = 0usize;
    for (i, (h, cnt)) in plan.iter().enumerate() {
        if leaf_index >= start && leaf_index < start + cnt {
            return Ok((i, start, *h));
        }
        start += cnt;
    }
    Err(CryptoError::Mmr("unable to locate peak".into()))
}

// ---------------------------------------------------------------------------
// Merkle path for power-of-two subtree
// ---------------------------------------------------------------------------

/// A single step in a Merkle inclusion proof path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathStep {
    /// Which side the sibling is on.
    pub side: Side,
    /// Sibling hash (64 hex chars).
    pub hash: String,
}

/// Side indicator for a Merkle proof path step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Sibling is to the left of the current node.
    Left,
    /// Sibling is to the right of the current node.
    Right,
}

impl Side {
    /// String representation matching the Python implementation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Compute a Merkle root and sibling path for a power-of-two sized leaf list.
///
/// Matches `tools/mmr.py:merkle_path_for_power_of_two()`.
fn merkle_path_for_power_of_two(
    leaf_hashes: &[String],
    leaf_pos: usize,
) -> Result<(String, Vec<PathStep>), CryptoError> {
    let n = leaf_hashes.len();
    if n == 0 {
        return Err(CryptoError::Mmr("leaf_hashes must be non-empty".into()));
    }
    if n & (n - 1) != 0 {
        return Err(CryptoError::Mmr(
            "leaf_hashes length must be power of two".into(),
        ));
    }
    if leaf_pos >= n {
        return Err(CryptoError::Mmr("leaf_pos out of range".into()));
    }

    let mut level: Vec<String> = leaf_hashes
        .iter()
        .map(|h| h.trim().to_lowercase())
        .collect();

    for h in &level {
        if !is_hex_32(h) {
            return Err(CryptoError::Mmr("invalid leaf hash".into()));
        }
    }

    let mut pos = leaf_pos;
    let mut path = Vec::new();

    while level.len() > 1 {
        let sibling_pos = pos ^ 1;
        let sibling_hash = level[sibling_pos].clone();
        let side = if sibling_pos < pos {
            Side::Left
        } else {
            Side::Right
        };
        path.push(PathStep {
            side,
            hash: sibling_hash,
        });

        // Build next level.
        let mut next_level = Vec::with_capacity(level.len() / 2);
        let mut i = 0;
        while i < level.len() {
            next_level.push(mmr_node_hash(&level[i], &level[i + 1])?);
            i += 2;
        }
        level = next_level;
        pos /= 2;
    }

    Ok((level[0].clone(), path))
}

// ---------------------------------------------------------------------------
// Inclusion proof
// ---------------------------------------------------------------------------

/// An inclusion proof for a leaf in the MMR.
///
/// Contains all information needed to independently verify that a receipt
/// with a given `next_root` is included in the MMR at the claimed root.
#[derive(Debug, Clone)]
pub struct MmrInclusionProof {
    /// Total number of leaves in the MMR.
    pub size: usize,
    /// Bagged root hash (64 hex chars).
    pub root: String,
    /// Index of the leaf being proved.
    pub leaf_index: usize,
    /// The receipt's `next_root` digest (64 hex chars).
    pub receipt_next_root: String,
    /// The leaf hash: `SHA256(0x00 || next_root_bytes)`.
    pub leaf_hash: String,
    /// Index of the peak containing this leaf.
    pub peak_index: usize,
    /// Height of the peak containing this leaf.
    pub peak_height: usize,
    /// Sibling path from leaf to peak root.
    pub path: Vec<PathStep>,
    /// All peaks in the MMR.
    pub peaks: Vec<Peak>,
    /// Root of the peak subtree computed from the path.
    pub computed_peak_root: String,
}

/// Build an inclusion proof for `leaf_index` in the MMR built from
/// `next_roots_hex`.
///
/// Matches `tools/mmr.py:build_inclusion_proof()`.
pub fn build_inclusion_proof(
    next_roots_hex: &[String],
    leaf_index: usize,
) -> Result<MmrInclusionProof, CryptoError> {
    let size = next_roots_hex.len();
    if size == 0 {
        return Err(CryptoError::Mmr("cannot build proof for empty MMR".into()));
    }
    if leaf_index >= size {
        return Err(CryptoError::Mmr("leaf_index out of range".into()));
    }

    let leaf_hashes: Result<Vec<String>, _> =
        next_roots_hex.iter().map(|nr| mmr_leaf_hash(nr)).collect();
    let leaf_hashes = leaf_hashes?;
    let peaks = build_peaks(&leaf_hashes)?;
    let root = bag_peaks(&peaks)?;

    let (peak_index, peak_start, peak_height) = find_peak_for_leaf(size, leaf_index)?;
    let peak_leaf_count = 1usize << peak_height;
    let local_pos = leaf_index - peak_start;
    let peak_leaves = &leaf_hashes[peak_start..peak_start + peak_leaf_count];

    let (peak_root, path) = merkle_path_for_power_of_two(peak_leaves, local_pos)?;

    Ok(MmrInclusionProof {
        size,
        root,
        leaf_index,
        receipt_next_root: next_roots_hex[leaf_index].trim().to_lowercase(),
        leaf_hash: leaf_hashes[leaf_index].clone(),
        peak_index,
        peak_height,
        path,
        peaks,
        computed_peak_root: peak_root,
    })
}

/// Verify an inclusion proof.
///
/// Returns `true` if the proof is valid, `false` otherwise.
///
/// Matches `tools/mmr.py:verify_inclusion_proof()`.
pub fn verify_inclusion_proof(proof: &MmrInclusionProof) -> bool {
    if proof.size == 0 || proof.leaf_index >= proof.size {
        return false;
    }
    if !is_hex_32(&proof.receipt_next_root) || !is_hex_32(&proof.root) {
        return false;
    }

    // Verify leaf hash.
    let expected_leaf = match mmr_leaf_hash(&proof.receipt_next_root) {
        Ok(h) => h,
        Err(_) => return false,
    };
    if expected_leaf != proof.leaf_hash {
        return false;
    }

    // Validate peaks.
    if proof.peaks.is_empty() {
        return false;
    }
    for p in &proof.peaks {
        if !is_hex_32(&p.hash) {
            return false;
        }
    }

    if proof.peak_index >= proof.peaks.len() {
        return false;
    }
    if proof.peaks[proof.peak_index].height != proof.peak_height {
        return false;
    }

    // Verify peak selection is consistent with MMR size.
    match find_peak_for_leaf(proof.size, proof.leaf_index) {
        Ok((exp_pi, _, exp_h)) => {
            if exp_pi != proof.peak_index || exp_h != proof.peak_height {
                return false;
            }
        }
        Err(_) => return false,
    }

    // Compute peak root from path.
    let mut cur = proof.leaf_hash.clone();
    for step in &proof.path {
        if !is_hex_32(&step.hash) {
            return false;
        }
        cur = match step.side {
            Side::Left => match mmr_node_hash(&step.hash, &cur) {
                Ok(h) => h,
                Err(_) => return false,
            },
            Side::Right => match mmr_node_hash(&cur, &step.hash) {
                Ok(h) => h,
                Err(_) => return false,
            },
        };
    }

    // Substitute computed peak root and recompute bagged root.
    let mut peaks2 = proof.peaks.clone();
    peaks2[proof.peak_index] = Peak {
        height: proof.peak_height,
        hash: cur,
    };

    match bag_peaks(&peaks2) {
        Ok(computed_root) => computed_root == proof.root,
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Incremental append (matching tools/mmr.py:append_peaks)
// ---------------------------------------------------------------------------

/// Incrementally append leaves to an existing MMR peak set.
///
/// This enables verifiers to start from a checkpoint (peaks) and extend the
/// accumulator with new receipts without replaying the entire history.
///
/// Matches `tools/mmr.py:append_peaks()`.
pub fn append_peaks(
    existing_peaks: &[Peak],
    new_leaf_hashes: &[String],
) -> Result<Vec<Peak>, CryptoError> {
    let mut stack: Vec<(usize, String)> = existing_peaks
        .iter()
        .map(|p| (p.height, p.hash.clone()))
        .collect();

    for leaf in new_leaf_hashes {
        let mut cur_h: usize = 0;
        let mut cur = leaf.clone();
        while let Some(top) = stack.last() {
            if top.0 != cur_h {
                break;
            }
            let (_, left) = stack
                .pop()
                .ok_or_else(|| CryptoError::Mmr("stack unexpectedly empty during merge".into()))?;
            cur = mmr_node_hash(&left, &cur)?;
            cur_h += 1;
        }
        stack.push((cur_h, cur));
    }

    Ok(stack
        .into_iter()
        .map(|(h, hash)| Peak { height: h, hash })
        .collect())
}

// ---------------------------------------------------------------------------
// MerkleMountainRange (stateful wrapper)
// ---------------------------------------------------------------------------

/// A stateful Merkle Mountain Range for append-only receipt chain commitment.
///
/// Wraps the functional API above into a struct that maintains internal state
/// across sequential append operations.
#[derive(Debug, Clone)]
pub struct MerkleMountainRange {
    /// All leaf hashes (in append order).
    leaf_hashes: Vec<String>,
    /// Current peak set.
    peaks: Vec<Peak>,
}

impl MerkleMountainRange {
    /// Create an empty MMR.
    pub fn new() -> Self {
        Self {
            leaf_hashes: Vec::new(),
            peaks: Vec::new(),
        }
    }

    /// Append a receipt `next_root` digest (64 hex chars) to the MMR.
    ///
    /// The leaf hash is computed using domain separation:
    /// `SHA256(0x00 || next_root_bytes)`.
    ///
    /// All leaf data entering the MMR must have been produced through the
    /// `CanonicalBytes` -> `ContentDigest` pipeline. The `next_root_hex` is
    /// the hex encoding of such a `ContentDigest`.
    pub fn append(&mut self, next_root_hex: &str) -> Result<(), CryptoError> {
        let leaf_hash = mmr_leaf_hash(next_root_hex)?;
        self.peaks = append_peaks(&self.peaks, std::slice::from_ref(&leaf_hash))?;
        self.leaf_hashes.push(leaf_hash);
        Ok(())
    }

    /// Return the current number of leaves.
    pub fn size(&self) -> usize {
        self.leaf_hashes.len()
    }

    /// Return the current bagged root hash (64 hex chars).
    ///
    /// Returns empty string if the MMR has no leaves.
    pub fn root(&self) -> Result<String, CryptoError> {
        bag_peaks(&self.peaks)
    }

    /// Return a snapshot of the current peaks.
    pub fn peaks(&self) -> &[Peak] {
        &self.peaks
    }
}

impl Default for MerkleMountainRange {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper matching Python `_h(i)`: SHA256("receipt-{i}") as hex.
    fn receipt_hash(i: usize) -> String {
        let data = format!("receipt-{i}");
        to_hex(&sha256_raw(data.as_bytes()))
    }

    // -- Leaf hash fixtures (from Python) ---------------------------------

    #[test]
    fn leaf_hash_receipt_1() {
        let nr = receipt_hash(1);
        assert_eq!(
            nr,
            "fea5396a7f4325c408b1b65b33a4d77ba5486ceba941804d8889a8546cfbab96"
        );
        let lh = mmr_leaf_hash(&nr).unwrap();
        assert_eq!(
            lh,
            "29534994a3ad2af6dd418f46d4093897971cd14bea312167ad82c4b31dbbfcec"
        );
    }

    #[test]
    fn leaf_hash_receipt_2() {
        let nr = receipt_hash(2);
        assert_eq!(
            nr,
            "774ad7ab1a3d41b114b5f4a34e2d8fc19c2ee8d83dfc133f1d80068ed205597f"
        );
        let lh = mmr_leaf_hash(&nr).unwrap();
        assert_eq!(
            lh,
            "70cd9dc55d5eb8e95d7678e9e63850bf6eab82e4c8320bd921189cf33650a6dd"
        );
    }

    #[test]
    fn leaf_hash_receipt_3() {
        let nr = receipt_hash(3);
        let lh = mmr_leaf_hash(&nr).unwrap();
        assert_eq!(
            lh,
            "92f8fcff2113db0203a7c8a5fab996979cc75acb286f05888b80913a4046426c"
        );
    }

    #[test]
    fn leaf_hash_receipt_4() {
        let nr = receipt_hash(4);
        let lh = mmr_leaf_hash(&nr).unwrap();
        assert_eq!(
            lh,
            "975f3a064d539cced75a2da9659911b5d5429496bfede6d7c83fa0cca6bb1579"
        );
    }

    // -- Node hash fixtures -----------------------------------------------

    #[test]
    fn node_hash_matches_python() {
        let lh1 = "29534994a3ad2af6dd418f46d4093897971cd14bea312167ad82c4b31dbbfcec";
        let lh2 = "70cd9dc55d5eb8e95d7678e9e63850bf6eab82e4c8320bd921189cf33650a6dd";
        let nh = mmr_node_hash(lh1, lh2).unwrap();
        assert_eq!(
            nh,
            "a34e6d5dac1ff07f820cc5156b41a50ce78c6a6868d81d5c5824cad0cab7250b"
        );
    }

    // -- Single and two receipt MMR ---------------------------------------

    #[test]
    fn single_receipt_mmr() {
        let next_roots: Vec<String> = vec![receipt_hash(1)];
        let info = mmr_root_from_next_roots(&next_roots).unwrap();
        assert_eq!(info.size, 1);
        assert_eq!(info.peaks.len(), 1);
        assert_eq!(info.peaks[0].height, 0);
        assert_eq!(
            info.root,
            "29534994a3ad2af6dd418f46d4093897971cd14bea312167ad82c4b31dbbfcec"
        );
    }

    #[test]
    fn two_receipt_mmr() {
        let next_roots: Vec<String> = vec![receipt_hash(1), receipt_hash(2)];
        let info = mmr_root_from_next_roots(&next_roots).unwrap();
        assert_eq!(info.size, 2);
        assert_eq!(info.peaks.len(), 1);
        assert_eq!(
            info.root,
            "a34e6d5dac1ff07f820cc5156b41a50ce78c6a6868d81d5c5824cad0cab7250b"
        );
    }

    // -- 17 receipt MMR (port of test_mmr_inclusion_proofs_roundtrip) -----

    #[test]
    fn mmr_17_receipts_root_matches_python() {
        let next_roots: Vec<String> = (1..=17).map(receipt_hash).collect();
        let info = mmr_root_from_next_roots(&next_roots).unwrap();
        assert_eq!(info.size, 17);
        assert_eq!(
            info.root,
            "338734a36416c5153c1a46812c5b3c2bfc3a4fa688beae56eb6ae8447143db2e"
        );
        assert_eq!(info.peaks.len(), 2);
        assert_eq!(info.peaks[0].height, 4);
        assert_eq!(
            info.peaks[0].hash,
            "a4064c64f52cf7cc433bf372e64076ebd2b78a49e1e223748802831e7a7f32ad"
        );
        assert_eq!(info.peaks[1].height, 0);
        assert_eq!(
            info.peaks[1].hash,
            "2ba384ee5925e4ff8953fd5a59583c8fbff6fdf2f06ee056b6fae639ff1b66eb"
        );
    }

    #[test]
    fn mmr_17_inclusion_proofs_roundtrip() {
        let next_roots: Vec<String> = (1..=17).map(receipt_hash).collect();
        let info = mmr_root_from_next_roots(&next_roots).unwrap();
        assert_eq!(info.size, 17);
        assert_eq!(info.root.len(), 64);

        for idx in [0, 1, 2, 7, 8, 16] {
            let proof = build_inclusion_proof(&next_roots, idx).unwrap();
            assert_eq!(proof.root, info.root, "Root mismatch at idx={idx}");
            assert_eq!(proof.size, info.size, "Size mismatch at idx={idx}");
            assert!(
                verify_inclusion_proof(&proof),
                "Verification failed at idx={idx}"
            );
        }
    }

    #[test]
    fn mmr_17_proof_idx_0_details() {
        let next_roots: Vec<String> = (1..=17).map(receipt_hash).collect();
        let proof = build_inclusion_proof(&next_roots, 0).unwrap();
        assert_eq!(
            proof.leaf_hash,
            "29534994a3ad2af6dd418f46d4093897971cd14bea312167ad82c4b31dbbfcec"
        );
        assert_eq!(proof.peak_index, 0);
        assert_eq!(proof.peak_height, 4);
        assert_eq!(
            proof.computed_peak_root,
            "a4064c64f52cf7cc433bf372e64076ebd2b78a49e1e223748802831e7a7f32ad"
        );
        assert_eq!(proof.path.len(), 4);
    }

    #[test]
    fn mmr_17_proof_idx_16_is_singleton_peak() {
        let next_roots: Vec<String> = (1..=17).map(receipt_hash).collect();
        let proof = build_inclusion_proof(&next_roots, 16).unwrap();
        assert_eq!(
            proof.leaf_hash,
            "2ba384ee5925e4ff8953fd5a59583c8fbff6fdf2f06ee056b6fae639ff1b66eb"
        );
        assert_eq!(proof.peak_index, 1);
        assert_eq!(proof.peak_height, 0);
        assert_eq!(proof.path.len(), 0);
        assert!(verify_inclusion_proof(&proof));
    }

    // -- 9 receipt tamper test (port of test_mmr_inclusion_proof_tamper_fails) --

    #[test]
    fn mmr_9_receipts_root_matches_python() {
        let next_roots: Vec<String> = (1..=9).map(receipt_hash).collect();
        let info = mmr_root_from_next_roots(&next_roots).unwrap();
        assert_eq!(
            info.root,
            "2cbf59819f3070f62601d1e0c47af000896f1c7bcb3f1e421c9bcdbf6c20ef09"
        );
        assert_eq!(info.peaks.len(), 2);
    }

    #[test]
    fn mmr_inclusion_proof_tamper_fails() {
        let next_roots: Vec<String> = (1..=9).map(receipt_hash).collect();
        let proof = build_inclusion_proof(&next_roots, 3).unwrap();
        assert!(verify_inclusion_proof(&proof));

        let mut tampered = proof.clone();
        if !tampered.path.is_empty() {
            tampered.path[0].hash = "00".repeat(32);
        }
        assert!(!verify_inclusion_proof(&tampered));
    }

    // -- Incremental append matches full rebuild --------------------------

    #[test]
    fn append_peaks_matches_full_build() {
        let next_roots: Vec<String> = (1..=10).map(receipt_hash).collect();

        let all_leaf_hashes: Vec<String> = next_roots
            .iter()
            .map(|nr| mmr_leaf_hash(nr).unwrap())
            .collect();
        let full_peaks = build_peaks(&all_leaf_hashes).unwrap();

        let first5: Vec<String> = all_leaf_hashes[..5].to_vec();
        let rest5: Vec<String> = all_leaf_hashes[5..].to_vec();
        let peaks5 = build_peaks(&first5).unwrap();
        let inc_peaks = append_peaks(&peaks5, &rest5).unwrap();

        assert_eq!(full_peaks, inc_peaks);
    }

    // -- Stateful MMR matches functional API ------------------------------

    #[test]
    fn stateful_mmr_matches_functional() {
        let next_roots: Vec<String> = (1..=17).map(receipt_hash).collect();

        let info = mmr_root_from_next_roots(&next_roots).unwrap();

        let mut mmr = MerkleMountainRange::new();
        for nr in &next_roots {
            mmr.append(nr).unwrap();
        }

        assert_eq!(mmr.size(), 17);
        assert_eq!(mmr.root().unwrap(), info.root);
        assert_eq!(mmr.peaks().len(), info.peaks.len());
        for (a, b) in mmr.peaks().iter().zip(info.peaks.iter()) {
            assert_eq!(a, b);
        }
    }

    // -- Edge cases -------------------------------------------------------

    #[test]
    fn leaf_hash_rejects_invalid_hex() {
        assert!(mmr_leaf_hash("not_hex").is_err());
        assert!(mmr_leaf_hash("abcd").is_err());
        assert!(mmr_leaf_hash(&"zz".repeat(32)).is_err());
    }

    #[test]
    fn node_hash_rejects_invalid_hex() {
        let valid = "00".repeat(32);
        assert!(mmr_node_hash("bad", &valid).is_err());
        assert!(mmr_node_hash(&valid, "bad").is_err());
    }

    #[test]
    fn build_proof_rejects_out_of_range() {
        let next_roots: Vec<String> = (1..=5).map(receipt_hash).collect();
        assert!(build_inclusion_proof(&next_roots, 5).is_err());
        assert!(build_inclusion_proof(&[], 0).is_err());
    }

    #[test]
    fn empty_mmr_has_empty_root() {
        let mmr = MerkleMountainRange::new();
        assert_eq!(mmr.size(), 0);
        assert_eq!(mmr.root().unwrap(), "");
    }

    #[test]
    fn peak_plan_correctness() {
        assert_eq!(peak_plan(17), vec![(4, 16), (0, 1)]);
        assert_eq!(peak_plan(9), vec![(3, 8), (0, 1)]);
        assert_eq!(peak_plan(7), vec![(2, 4), (1, 2), (0, 1)]);
        assert_eq!(peak_plan(8), vec![(3, 8)]);
        assert!(peak_plan(0).is_empty());
    }

    // -- Cross-language compatibility test ---------------------------------

    #[test]
    fn cross_language_17_receipt_fixture() {
        let expected_next_roots = [
            "fea5396a7f4325c408b1b65b33a4d77ba5486ceba941804d8889a8546cfbab96",
            "774ad7ab1a3d41b114b5f4a34e2d8fc19c2ee8d83dfc133f1d80068ed205597f",
            "de84f16f82e8cf8c184f7883460865481ba6f1fa5b48c3ae4e75b9e9786a6b03",
            "9bf922d8ee39a15df6c2b0081aa0f508601a13287fc31a902aa34eccbf835def",
            "9f11d91831441bd7245531a84f92a4e1a7a2e2ab7b67786351f077a4fd10efea",
            "1f88814ff1d5f5300396ba391fd12fc9e46cea4f68d1e70b3b219ab3dbcd480c",
            "b9e8be61a195415d90f6453066a979ea13b30499b622f707ee5dbbe1ed58294a",
            "eb75bd2e1c056b3ffc82bab6e9d09051e2547f2046baa7b62056e0c0a07fa1dd",
            "e6fdb34dc858fdc85c667742163c42a257b8eaefab62c6c7138d2da4aa4344d2",
            "a40b8ea6c3bfbe2a58cd15f870bd261297d1604b78bbb299b4b62afdb3fb2897",
            "651e9f306453649cf1b3be1c468868c31f953a6268144f93a2f83e41ac48b77f",
            "e90d497520fcfaa2c133d711f989d877eb7cfd8b57c10f8f91520fed45fcb849",
            "63e81058f8e39f0b6551e2876b87c8cac05df6a6b3888f3fa18858c11b48a349",
            "b27e6eebf85812aa8d6bb982bd60fe21acdee1dd1252e3a02bff48e7d11be237",
            "83a00529fc37dea7e5fc90663041498d12447ba74524368dc2d8026406024528",
            "4ce306d785e156ecce175d1c1693c2c72599eed1ce7e90dfcca7214e65ef2446",
            "3d9b849244959f3c63eab896d824a077a145676a6205fb17b3d23df3a3f560b6",
        ];

        let rust_next_roots: Vec<String> = (1..=17).map(receipt_hash).collect();
        for (i, (expected, actual)) in expected_next_roots
            .iter()
            .zip(rust_next_roots.iter())
            .enumerate()
        {
            assert_eq!(expected, actual, "next_root mismatch at receipt-{}", i + 1);
        }

        let info = mmr_root_from_next_roots(&rust_next_roots).unwrap();
        assert_eq!(
            info.root, "338734a36416c5153c1a46812c5b3c2bfc3a4fa688beae56eb6ae8447143db2e",
            "17-receipt MMR root does not match Python fixture"
        );
    }

    #[test]
    fn cross_language_9_receipt_fixture() {
        let next_roots: Vec<String> = (1..=9).map(receipt_hash).collect();
        let info = mmr_root_from_next_roots(&next_roots).unwrap();
        assert_eq!(
            info.root, "2cbf59819f3070f62601d1e0c47af000896f1c7bcb3f1e421c9bcdbf6c20ef09",
            "9-receipt MMR root does not match Python fixture"
        );
    }

    // ── Coverage expansion tests ─────────────────────────────────────

    #[test]
    fn from_hex_rejects_odd_length() {
        let result = from_hex("abc");
        assert!(result.is_err());
    }

    #[test]
    fn from_hex_rejects_invalid_chars() {
        let result = from_hex("zzzz");
        assert!(result.is_err());
    }

    #[test]
    fn from_hex_valid() {
        let result = from_hex("deadbeef").unwrap();
        assert_eq!(result, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn is_hex_32_rejects_short() {
        assert!(!is_hex_32("abcd"));
    }

    #[test]
    fn is_hex_32_rejects_non_hex() {
        assert!(!is_hex_32(&"zz".repeat(32)));
    }

    #[test]
    fn is_hex_32_accepts_valid() {
        assert!(is_hex_32(&"ab".repeat(32)));
    }

    #[test]
    fn side_display() {
        assert_eq!(format!("{}", Side::Left), "left");
        assert_eq!(format!("{}", Side::Right), "right");
    }

    #[test]
    fn side_as_str() {
        assert_eq!(Side::Left.as_str(), "left");
        assert_eq!(Side::Right.as_str(), "right");
    }

    #[test]
    fn mmr_default_trait() {
        let mmr = MerkleMountainRange::default();
        assert_eq!(mmr.size(), 0);
        assert_eq!(mmr.root().unwrap(), "");
        assert!(mmr.peaks().is_empty());
    }

    #[test]
    fn verify_proof_rejects_size_zero() {
        let proof = MmrInclusionProof {
            size: 0,
            root: "00".repeat(32),
            leaf_index: 0,
            receipt_next_root: "00".repeat(32),
            leaf_hash: "00".repeat(32),
            peak_index: 0,
            peak_height: 0,
            path: vec![],
            peaks: vec![],
            computed_peak_root: "00".repeat(32),
        };
        assert!(!verify_inclusion_proof(&proof));
    }

    #[test]
    fn verify_proof_rejects_leaf_index_out_of_range() {
        let proof = MmrInclusionProof {
            size: 1,
            root: "00".repeat(32),
            leaf_index: 5,
            receipt_next_root: "00".repeat(32),
            leaf_hash: "00".repeat(32),
            peak_index: 0,
            peak_height: 0,
            path: vec![],
            peaks: vec![],
            computed_peak_root: "00".repeat(32),
        };
        assert!(!verify_inclusion_proof(&proof));
    }

    #[test]
    fn verify_proof_rejects_invalid_root_hex() {
        let nr = receipt_hash(1);
        let lh = mmr_leaf_hash(&nr).unwrap();
        let proof = MmrInclusionProof {
            size: 1,
            root: "not_valid_hex".to_string(),
            leaf_index: 0,
            receipt_next_root: nr,
            leaf_hash: lh,
            peak_index: 0,
            peak_height: 0,
            path: vec![],
            peaks: vec![Peak {
                height: 0,
                hash: "00".repeat(32),
            }],
            computed_peak_root: "00".repeat(32),
        };
        assert!(!verify_inclusion_proof(&proof));
    }

    #[test]
    fn verify_proof_rejects_empty_peaks() {
        let nr = receipt_hash(1);
        let lh = mmr_leaf_hash(&nr).unwrap();
        let proof = MmrInclusionProof {
            size: 1,
            root: "00".repeat(32),
            leaf_index: 0,
            receipt_next_root: nr,
            leaf_hash: lh,
            peak_index: 0,
            peak_height: 0,
            path: vec![],
            peaks: vec![],
            computed_peak_root: "00".repeat(32),
        };
        assert!(!verify_inclusion_proof(&proof));
    }

    #[test]
    fn verify_proof_rejects_peak_index_out_of_bounds() {
        let nr = receipt_hash(1);
        let lh = mmr_leaf_hash(&nr).unwrap();
        let proof = MmrInclusionProof {
            size: 1,
            root: lh.clone(),
            leaf_index: 0,
            receipt_next_root: nr,
            leaf_hash: lh.clone(),
            peak_index: 5,
            peak_height: 0,
            path: vec![],
            peaks: vec![Peak {
                height: 0,
                hash: lh,
            }],
            computed_peak_root: "00".repeat(32),
        };
        assert!(!verify_inclusion_proof(&proof));
    }

    #[test]
    fn verify_proof_rejects_wrong_peak_height() {
        let nr = receipt_hash(1);
        let lh = mmr_leaf_hash(&nr).unwrap();
        let proof = MmrInclusionProof {
            size: 1,
            root: lh.clone(),
            leaf_index: 0,
            receipt_next_root: nr,
            leaf_hash: lh.clone(),
            peak_index: 0,
            peak_height: 3, // wrong: should be 0 for single leaf
            path: vec![],
            peaks: vec![Peak {
                height: 0,
                hash: lh,
            }],
            computed_peak_root: "00".repeat(32),
        };
        assert!(!verify_inclusion_proof(&proof));
    }

    #[test]
    fn verify_proof_rejects_invalid_peak_hash() {
        let nr = receipt_hash(1);
        let lh = mmr_leaf_hash(&nr).unwrap();
        let proof = MmrInclusionProof {
            size: 1,
            root: lh.clone(),
            leaf_index: 0,
            receipt_next_root: nr,
            leaf_hash: lh.clone(),
            peak_index: 0,
            peak_height: 0,
            path: vec![],
            peaks: vec![Peak {
                height: 0,
                hash: "not_hex".to_string(),
            }],
            computed_peak_root: "00".repeat(32),
        };
        assert!(!verify_inclusion_proof(&proof));
    }

    #[test]
    fn verify_proof_rejects_wrong_leaf_hash() {
        let nr = receipt_hash(1);
        let proof = MmrInclusionProof {
            size: 1,
            root: "00".repeat(32),
            leaf_index: 0,
            receipt_next_root: nr,
            leaf_hash: "00".repeat(32), // wrong leaf hash
            peak_index: 0,
            peak_height: 0,
            path: vec![],
            peaks: vec![Peak {
                height: 0,
                hash: "00".repeat(32),
            }],
            computed_peak_root: "00".repeat(32),
        };
        assert!(!verify_inclusion_proof(&proof));
    }

    #[test]
    fn verify_proof_rejects_invalid_path_step_hash() {
        let next_roots: Vec<String> = (1..=2).map(receipt_hash).collect();
        let mut proof = build_inclusion_proof(&next_roots, 0).unwrap();
        assert!(verify_inclusion_proof(&proof));
        // Corrupt a path step with invalid hex
        if !proof.path.is_empty() {
            proof.path[0].hash = "invalid_hex".to_string();
        }
        assert!(!verify_inclusion_proof(&proof));
    }

    #[test]
    fn build_peaks_rejects_invalid_leaf_hash() {
        let result = build_peaks(&["not_valid_hex".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn merkle_path_pow2_empty() {
        let result = merkle_path_for_power_of_two(&[], 0);
        assert!(result.is_err());
    }

    #[test]
    fn merkle_path_pow2_non_power_of_two() {
        let hashes: Vec<String> = (1..=3)
            .map(|i| mmr_leaf_hash(&receipt_hash(i)).unwrap())
            .collect();
        let result = merkle_path_for_power_of_two(&hashes, 0);
        assert!(result.is_err());
    }

    #[test]
    fn merkle_path_pow2_out_of_range() {
        let hashes: Vec<String> = (1..=2)
            .map(|i| mmr_leaf_hash(&receipt_hash(i)).unwrap())
            .collect();
        let result = merkle_path_for_power_of_two(&hashes, 5);
        assert!(result.is_err());
    }

    #[test]
    fn three_receipt_mmr() {
        let next_roots: Vec<String> = (1..=3).map(receipt_hash).collect();
        let info = mmr_root_from_next_roots(&next_roots).unwrap();
        assert_eq!(info.size, 3);
        assert_eq!(info.peaks.len(), 2);
        assert_eq!(info.peaks[0].height, 1);
        assert_eq!(info.peaks[1].height, 0);
        assert_eq!(info.root.len(), 64);
    }

    #[test]
    fn five_receipt_mmr() {
        let next_roots: Vec<String> = (1..=5).map(receipt_hash).collect();
        let info = mmr_root_from_next_roots(&next_roots).unwrap();
        assert_eq!(info.size, 5);
        assert_eq!(info.peaks.len(), 2);
        assert_eq!(info.peaks[0].height, 2);
        assert_eq!(info.peaks[1].height, 0);
    }

    #[test]
    fn mmr_root_info_debug() {
        let next_roots: Vec<String> = (1..=2).map(receipt_hash).collect();
        let info = mmr_root_from_next_roots(&next_roots).unwrap();
        let debug_str = format!("{info:?}");
        assert!(debug_str.contains("MmrRootInfo"));
    }

    #[test]
    fn stateful_mmr_clone() {
        let mut mmr = MerkleMountainRange::new();
        mmr.append(&receipt_hash(1)).unwrap();
        let cloned = mmr.clone();
        assert_eq!(cloned.size(), mmr.size());
        assert_eq!(cloned.root().unwrap(), mmr.root().unwrap());
    }

    #[test]
    fn stateful_mmr_append_rejects_invalid() {
        let mut mmr = MerkleMountainRange::new();
        assert!(mmr.append("not_hex").is_err());
        assert_eq!(mmr.size(), 0);
    }

    #[test]
    fn inclusion_proof_all_indices_for_4_receipts() {
        let next_roots: Vec<String> = (1..=4).map(receipt_hash).collect();
        for idx in 0..4 {
            let proof = build_inclusion_proof(&next_roots, idx).unwrap();
            assert!(
                verify_inclusion_proof(&proof),
                "Verification failed at idx={idx} for 4-receipt MMR"
            );
        }
    }

    #[test]
    fn find_peak_for_leaf_out_of_range() {
        let result = find_peak_for_leaf(5, 5);
        assert!(result.is_err());
    }

    #[test]
    fn peak_plan_power_of_two() {
        assert_eq!(peak_plan(16), vec![(4, 16)]);
        assert_eq!(peak_plan(4), vec![(2, 4)]);
        assert_eq!(peak_plan(1), vec![(0, 1)]);
    }

    #[test]
    fn mmr_leaf_hash_accepts_uppercase_hex() {
        // is_hex_32 checks c.is_ascii_hexdigit() which accepts A-F
        let nr = "FEA5396A7F4325C408B1B65B33A4D77BA5486CEBA941804D8889A8546CFBAB96";
        // from_hex lowercases, so this should work
        let result = mmr_leaf_hash(nr);
        assert!(result.is_ok());
    }
}
