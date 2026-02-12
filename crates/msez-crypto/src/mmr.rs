//! # Merkle Mountain Range (MMR)
//!
//! An append-only authenticated data structure used for corridor receipt
//! chains. The MMR provides efficient proofs of inclusion for any historical
//! receipt without requiring the full chain.
//!
//! ## Algorithm
//!
//! Ported from `tools/mmr.py` (326 lines). Uses domain-separated SHA-256:
//! - Leaf: `SHA256(0x00 || leaf_bytes)` where leaf_bytes is a 32-byte digest.
//! - Node: `SHA256(0x01 || left || right)`.
//!
//! Peaks are bagged right-to-left:
//! `bag = peaks[-1]; for p in rev(peaks[:-1]): bag = node_hash(p, bag)`.
//!
//! ## Security Invariant
//!
//! All leaf hashes are computed from content digests through the domain-separated
//! hashing pipeline. The MMR operates on 32-byte hex digest strings matching
//! the Python implementation for cross-language compatibility.
//!
//! ## Implements
//!
//! Spec §16 — Receipt chain structure and inclusion proofs.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use msez_core::error::CryptoError;

// ---------------------------------------------------------------------------
// Core hashing (domain-separated SHA-256, matching tools/mmr.py)
// ---------------------------------------------------------------------------

/// Compute SHA-256 of raw bytes.
fn sha256_raw(b: &[u8]) -> [u8; 32] {
    let hash = Sha256::digest(b);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hash);
    out
}

/// Encode 32 bytes as lowercase hex.
fn bytes_to_hex(b: &[u8; 32]) -> String {
    b.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Decode a 64-char hex string to 32 bytes.
fn hex_to_32bytes(hex: &str) -> Result<[u8; 32], CryptoError> {
    let hex = hex.trim().to_lowercase();
    if hex.len() != 64 {
        return Err(CryptoError::DigestError(format!(
            "expected 64 hex chars, got {}",
            hex.len()
        )));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk)
            .map_err(|e| CryptoError::DigestError(format!("invalid hex: {e}")))?;
        out[i] = u8::from_str_radix(s, 16)
            .map_err(|e| CryptoError::DigestError(format!("invalid hex at {i}: {e}")))?;
    }
    Ok(out)
}

fn is_hex_32(s: &str) -> bool {
    let s = s.trim().to_lowercase();
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Compute the MMR leaf hash: `SHA256(0x00 || leaf_bytes)`.
///
/// `next_root_hex` is a 32-byte digest encoded as 64 hex chars
/// (matching the Python `mmr_leaf_hash` function).
pub fn mmr_leaf_hash(next_root_hex: &str) -> Result<String, CryptoError> {
    let bytes = hex_to_32bytes(next_root_hex)?;
    let mut input = Vec::with_capacity(33);
    input.push(0x00);
    input.extend_from_slice(&bytes);
    Ok(bytes_to_hex(&sha256_raw(&input)))
}

/// Compute a parent node hash: `SHA256(0x01 || left || right)`.
///
/// Both inputs are 32-byte digests encoded as 64 hex chars
/// (matching the Python `mmr_node_hash` function).
pub fn mmr_node_hash(left_hex: &str, right_hex: &str) -> Result<String, CryptoError> {
    let left = hex_to_32bytes(left_hex)?;
    let right = hex_to_32bytes(right_hex)?;
    let mut input = Vec::with_capacity(65);
    input.push(0x01);
    input.extend_from_slice(&left);
    input.extend_from_slice(&right);
    Ok(bytes_to_hex(&sha256_raw(&input)))
}

// ---------------------------------------------------------------------------
// Peak representation
// ---------------------------------------------------------------------------

/// A peak in the MMR: a complete binary tree root at a given height.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peak {
    /// Height of the binary tree (0 = single leaf).
    pub height: u32,
    /// The root hash of this peak's subtree (64 hex chars).
    pub hash: String,
}

// ---------------------------------------------------------------------------
// Core MMR operations
// ---------------------------------------------------------------------------

/// Build MMR peaks from a list of leaf hashes (left-to-right append order).
///
/// Port of Python `build_peaks()`. Each leaf hash must be a 64-char hex string.
pub fn build_peaks(leaf_hashes: &[String]) -> Result<Vec<Peak>, CryptoError> {
    let mut peaks: Vec<(u32, String)> = Vec::new();

    for lh in leaf_hashes {
        if !is_hex_32(lh) {
            return Err(CryptoError::DigestError(
                "leaf_hash must be 64 hex chars".to_string(),
            ));
        }
        let mut cur_h: u32 = 0;
        let mut cur = lh.trim().to_lowercase();

        // Merge while the top peak has the same height.
        while let Some(top) = peaks.last() {
            if top.0 != cur_h {
                break;
            }
            let (_, left) = peaks.pop().unwrap();
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
/// The root is computed by folding peaks from right-to-left:
///   `bag = peaks[-1]; for peak in reversed(peaks[:-1]): bag = node_hash(peak, bag)`
///
/// Returns empty string for empty peaks (callers should handle).
pub fn bag_peaks(peaks: &[Peak]) -> Result<String, CryptoError> {
    if peaks.is_empty() {
        return Ok(String::new());
    }
    let mut bag = peaks.last().unwrap().hash.clone();
    for p in peaks[..peaks.len() - 1].iter().rev() {
        bag = mmr_node_hash(&p.hash, &bag)?;
    }
    Ok(bag)
}

/// Compute MMR root and peaks from a sequence of receipt `next_root` digests.
///
/// Port of Python `mmr_root_from_next_roots()`.
///
/// Returns `(root_hex, peaks)`.
pub fn mmr_root_from_next_roots(
    next_roots_hex: &[String],
) -> Result<(String, Vec<Peak>), CryptoError> {
    let leaf_hashes: Result<Vec<String>, _> =
        next_roots_hex.iter().map(|nr| mmr_leaf_hash(nr)).collect();
    let leaf_hashes = leaf_hashes?;
    let peaks = build_peaks(&leaf_hashes)?;
    let root = bag_peaks(&peaks)?;
    Ok((root, peaks))
}

// ---------------------------------------------------------------------------
// Inclusion proof generation
// ---------------------------------------------------------------------------

/// Return a list of peaks as `(height, leaf_count)` from left-to-right
/// for a given leaf count.
fn peak_plan(size: usize) -> Vec<(u32, usize)> {
    let mut out = Vec::new();
    let mut n = size;
    while n > 0 {
        let h = usize::BITS - n.leading_zeros() - 1;
        let cnt = 1usize << h;
        out.push((h, cnt));
        n -= cnt;
    }
    out
}

/// Return `(peak_index, peak_start, peak_height)` for `leaf_index` in an
/// MMR of the given size.
fn find_peak_for_leaf(
    size: usize,
    leaf_index: usize,
) -> Result<(usize, usize, u32), CryptoError> {
    if leaf_index >= size {
        return Err(CryptoError::DigestError(
            "leaf_index out of range".to_string(),
        ));
    }
    let plan = peak_plan(size);
    let mut start = 0usize;
    for (i, (h, cnt)) in plan.iter().enumerate() {
        if start <= leaf_index && leaf_index < start + cnt {
            return Ok((i, start, *h));
        }
        start += cnt;
    }
    Err(CryptoError::DigestError(
        "unable to locate peak".to_string(),
    ))
}

/// A path step in a Merkle inclusion proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathStep {
    /// Which side the sibling is on: `"left"` or `"right"`.
    pub side: String,
    /// The sibling hash (64 hex chars).
    pub hash: String,
}

/// Compute a Merkle root and sibling path for a power-of-two-sized leaf list.
///
/// Port of Python `merkle_path_for_power_of_two()`.
fn merkle_path_for_power_of_two(
    leaf_hashes: &[String],
    leaf_pos: usize,
) -> Result<(String, Vec<PathStep>), CryptoError> {
    let n = leaf_hashes.len();
    if n == 0 {
        return Err(CryptoError::DigestError(
            "leaf_hashes must be non-empty".to_string(),
        ));
    }
    if n & (n - 1) != 0 {
        return Err(CryptoError::DigestError(
            "leaf_hashes length must be power of two".to_string(),
        ));
    }
    if leaf_pos >= n {
        return Err(CryptoError::DigestError(
            "leaf_pos out of range".to_string(),
        ));
    }

    let mut level: Vec<String> = leaf_hashes
        .iter()
        .map(|h| h.trim().to_lowercase())
        .collect();
    for h in &level {
        if !is_hex_32(h) {
            return Err(CryptoError::DigestError("invalid leaf hash".to_string()));
        }
    }

    let mut pos = leaf_pos;
    let mut path: Vec<PathStep> = Vec::new();

    while level.len() > 1 {
        let sibling_pos = pos ^ 1;
        let sibling_hash = level[sibling_pos].clone();
        let side = if sibling_pos < pos { "left" } else { "right" };
        path.push(PathStep {
            side: side.to_string(),
            hash: sibling_hash,
        });

        // Build next level
        let mut next: Vec<String> = Vec::new();
        for i in (0..level.len()).step_by(2) {
            next.push(mmr_node_hash(&level[i], &level[i + 1])?);
        }
        level = next;
        pos /= 2;
    }

    Ok((level[0].clone(), path))
}

/// An inclusion proof for a leaf in the MMR.
#[derive(Debug, Clone)]
pub struct InclusionProof {
    /// Total number of leaves in the MMR.
    pub size: usize,
    /// The MMR root hash (64 hex chars).
    pub root: String,
    /// Index of the leaf being proven.
    pub leaf_index: usize,
    /// The receipt's `next_root` digest (64 hex chars).
    pub receipt_next_root: String,
    /// The leaf hash (64 hex chars).
    pub leaf_hash: String,
    /// Index of the peak containing this leaf.
    pub peak_index: usize,
    /// Height of the peak.
    pub peak_height: u32,
    /// The Merkle path from the leaf to the peak root.
    pub path: Vec<PathStep>,
    /// All peaks in the MMR.
    pub peaks: Vec<Peak>,
    /// The computed peak root from the path.
    pub computed_peak_root: String,
}

/// Build an inclusion proof for `leaf_index` in the MMR built from
/// `next_roots_hex`.
///
/// Port of Python `build_inclusion_proof()`.
pub fn build_inclusion_proof(
    next_roots_hex: &[String],
    leaf_index: usize,
) -> Result<InclusionProof, CryptoError> {
    let size = next_roots_hex.len();
    if size == 0 {
        return Err(CryptoError::DigestError(
            "cannot build proof for empty MMR".to_string(),
        ));
    }
    if leaf_index >= size {
        return Err(CryptoError::DigestError(
            "leaf_index out of range".to_string(),
        ));
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

    Ok(InclusionProof {
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

// ---------------------------------------------------------------------------
// Inclusion proof verification
// ---------------------------------------------------------------------------

/// Verify an inclusion proof.
///
/// Port of Python `verify_inclusion_proof()`. Returns `true` if the proof
/// is valid, `false` otherwise. Does not return errors for malformed proofs
/// — returns `false` instead, matching the Python behavior.
pub fn verify_inclusion_proof(proof: &InclusionProof) -> bool {
    let size = proof.size;
    let leaf_index = proof.leaf_index;

    if size == 0 || leaf_index >= size {
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
    if proof.peak_index >= proof.peaks.len() {
        return false;
    }
    if proof.peaks[proof.peak_index].height != proof.peak_height {
        return false;
    }

    // Verify peak selection is consistent with MMR size.
    match find_peak_for_leaf(size, leaf_index) {
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
        cur = match step.side.as_str() {
            "left" => match mmr_node_hash(&step.hash, &cur) {
                Ok(h) => h,
                Err(_) => return false,
            },
            "right" => match mmr_node_hash(&cur, &step.hash) {
                Ok(h) => h,
                Err(_) => return false,
            },
            _ => return false,
        };
    }

    // Substitute computed peak root and recompute the MMR root.
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
// MerkleMountainRange — stateful wrapper
// ---------------------------------------------------------------------------

/// A Merkle Mountain Range for append-only commitment chains.
///
/// Maintains the peak set incrementally, supporting efficient append
/// and root computation without replaying the full history.
///
/// Implements Spec §16 — Receipt chain structure.
#[derive(Debug, Clone)]
pub struct MerkleMountainRange {
    /// The peaks of the MMR, stored as `(height, hash_hex)`.
    peak_stack: Vec<(u32, String)>,
    /// Total number of leaves appended.
    leaf_count: u64,
    /// All leaf hashes (for inclusion proof generation).
    leaf_hashes: Vec<String>,
    /// All next_root digests (for inclusion proof generation).
    next_roots: Vec<String>,
}

impl MerkleMountainRange {
    /// Create an empty MMR.
    pub fn new() -> Self {
        Self {
            peak_stack: Vec::new(),
            leaf_count: 0,
            leaf_hashes: Vec::new(),
            next_roots: Vec::new(),
        }
    }

    /// Returns the number of leaves in the MMR.
    pub fn leaf_count(&self) -> u64 {
        self.leaf_count
    }

    /// Returns the current peaks of the MMR.
    pub fn peaks(&self) -> Vec<Peak> {
        self.peak_stack
            .iter()
            .map(|(h, hash)| Peak {
                height: *h,
                hash: hash.clone(),
            })
            .collect()
    }

    /// Compute the current root hash by bagging peaks.
    ///
    /// Returns empty string if the MMR is empty.
    pub fn root(&self) -> Result<String, CryptoError> {
        bag_peaks(&self.peaks())
    }

    /// Append a receipt `next_root` digest (64 hex chars) to the MMR.
    ///
    /// Computes the leaf hash with domain separation and merges peaks
    /// as needed. This is the incremental equivalent of rebuilding
    /// via `build_peaks()`.
    pub fn append(&mut self, next_root_hex: &str) -> Result<String, CryptoError> {
        let leaf_hash = mmr_leaf_hash(next_root_hex)?;
        self.next_roots.push(next_root_hex.trim().to_lowercase());
        self.leaf_hashes.push(leaf_hash.clone());

        let mut cur_h: u32 = 0;
        let mut cur = leaf_hash.clone();

        while let Some(top) = self.peak_stack.last() {
            if top.0 != cur_h {
                break;
            }
            let (_, left) = self.peak_stack.pop().unwrap();
            cur = mmr_node_hash(&left, &cur)?;
            cur_h += 1;
        }
        self.peak_stack.push((cur_h, cur));
        self.leaf_count += 1;

        Ok(leaf_hash)
    }

    /// Build an inclusion proof for a leaf at the given index.
    pub fn build_proof(&self, leaf_index: usize) -> Result<InclusionProof, CryptoError> {
        build_inclusion_proof(&self.next_roots, leaf_index)
    }

    /// Verify an inclusion proof against this MMR's current root.
    pub fn verify_proof(&self, proof: &InclusionProof) -> Result<bool, CryptoError> {
        let root = self.root()?;
        Ok(verify_inclusion_proof(proof) && proof.root == root)
    }
}

impl Default for MerkleMountainRange {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Append peaks from checkpoint (matching Python append_peaks)
// ---------------------------------------------------------------------------

/// Incrementally append leaves to an existing MMR peak set.
///
/// Enables verifiers to start from a checkpoint (peaks) and extend the
/// accumulator with new receipts without replaying the entire history.
///
/// Port of Python `append_peaks()`.
pub fn append_peaks(
    existing_peaks: &[Peak],
    new_leaf_hashes: &[String],
) -> Result<Vec<Peak>, CryptoError> {
    let mut stack: Vec<(u32, String)> = existing_peaks
        .iter()
        .map(|p| (p.height, p.hash.clone()))
        .collect();

    for leaf in new_leaf_hashes {
        let mut cur_h: u32 = 0;
        let mut cur = leaf.clone();
        while let Some(top) = stack.last() {
            if top.0 != cur_h {
                break;
            }
            let (_, left) = stack.pop().unwrap();
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

/// Serialize peaks to a JSON-compatible map list.
pub fn peaks_to_json(peaks: &[Peak]) -> Vec<BTreeMap<String, serde_json::Value>> {
    peaks
        .iter()
        .map(|p| {
            let mut m = BTreeMap::new();
            m.insert(
                "height".to_string(),
                serde_json::Value::Number(serde_json::Number::from(p.height)),
            );
            m.insert(
                "hash".to_string(),
                serde_json::Value::String(p.hash.clone()),
            );
            m
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper matching Python's `_h(i)` = `sha256(f"receipt-{i}".encode()).hexdigest()`.
    fn h(i: usize) -> String {
        let input = format!("receipt-{i}");
        let hash = Sha256::digest(input.as_bytes());
        hash.iter().map(|b| format!("{b:02x}")).collect()
    }

    // -----------------------------------------------------------------------
    // Basic hash function tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_leaf_hash_known_vector() {
        // Python: mmr_leaf_hash("0"*64) = "7f9c9e31..."
        let zero_hex = "0".repeat(64);
        let result = mmr_leaf_hash(&zero_hex).unwrap();
        assert_eq!(
            result,
            "7f9c9e31ac8256ca2f258583df262dbc7d6f68f2a03043d5c99a4ae5a7396ce9"
        );
    }

    #[test]
    fn test_leaf_hash_receipt_1() {
        // Python fixture: _h(1) and its leaf hash
        let nr = h(1);
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
    fn test_node_hash() {
        let lh0 = mmr_leaf_hash(&h(1)).unwrap();
        let lh1 = mmr_leaf_hash(&h(2)).unwrap();
        let node = mmr_node_hash(&lh0, &lh1).unwrap();
        // This should equal the single peak for a 2-leaf MMR.
        assert_eq!(
            node,
            "a34e6d5dac1ff07f820cc5156b41a50ce78c6a6868d81d5c5824cad0cab7250b"
        );
    }

    #[test]
    fn test_invalid_hex_rejected() {
        assert!(mmr_leaf_hash("not-hex").is_err());
        assert!(mmr_leaf_hash("aabb").is_err());
        assert!(mmr_node_hash("aabb", &"00".repeat(32)).is_err());
    }

    // -----------------------------------------------------------------------
    // MMR root computation (cross-language fixtures from Python)
    // -----------------------------------------------------------------------

    #[test]
    fn test_single_leaf_root() {
        let nr = vec![h(1)];
        let (root, peaks) = mmr_root_from_next_roots(&nr).unwrap();
        assert_eq!(peaks.len(), 1);
        assert_eq!(peaks[0].height, 0);
        assert_eq!(
            root,
            "29534994a3ad2af6dd418f46d4093897971cd14bea312167ad82c4b31dbbfcec"
        );
    }

    #[test]
    fn test_two_leaf_root() {
        let nr = vec![h(1), h(2)];
        let (root, peaks) = mmr_root_from_next_roots(&nr).unwrap();
        assert_eq!(peaks.len(), 1);
        assert_eq!(peaks[0].height, 1);
        assert_eq!(
            root,
            "a34e6d5dac1ff07f820cc5156b41a50ce78c6a6868d81d5c5824cad0cab7250b"
        );
    }

    #[test]
    fn test_three_leaf_root() {
        let nr = vec![h(1), h(2), h(3)];
        let (root, peaks) = mmr_root_from_next_roots(&nr).unwrap();
        assert_eq!(peaks.len(), 2);
        assert_eq!(peaks[0].height, 1);
        assert_eq!(peaks[1].height, 0);
        assert_eq!(
            root,
            "59140b9d370d775da9f6e8e0fb3baa63489f19f4e62d8402495b9a576ce9fa51"
        );
    }

    #[test]
    fn test_17_leaf_root_cross_language() {
        // Matches Python test_mmr.py fixture: 17 leaves
        let nr: Vec<String> = (1..=17).map(h).collect();
        let (root, peaks) = mmr_root_from_next_roots(&nr).unwrap();
        assert_eq!(peaks.len(), 2);
        assert_eq!(peaks[0].height, 4);
        assert_eq!(peaks[1].height, 0);
        assert_eq!(
            root,
            "338734a36416c5153c1a46812c5b3c2bfc3a4fa688beae56eb6ae8447143db2e"
        );
    }

    // -----------------------------------------------------------------------
    // Inclusion proof (ported from tests/test_mmr.py)
    // -----------------------------------------------------------------------

    #[test]
    fn test_mmr_inclusion_proofs_roundtrip() {
        // Port of Python test_mmr_inclusion_proofs_roundtrip
        let next_roots: Vec<String> = (1..=17).map(h).collect();
        let (root, _) = mmr_root_from_next_roots(&next_roots).unwrap();
        assert_eq!(next_roots.len(), 17);
        assert_eq!(root.len(), 64);

        for idx in [0, 1, 2, 7, 8, 16] {
            let proof = build_inclusion_proof(&next_roots, idx).unwrap();
            assert_eq!(proof.root, root);
            assert_eq!(proof.size, 17);
            assert!(
                verify_inclusion_proof(&proof),
                "inclusion proof failed for index {idx}"
            );
        }
    }

    #[test]
    fn test_mmr_inclusion_proof_tamper_fails() {
        // Port of Python test_mmr_inclusion_proof_tamper_fails
        let next_roots: Vec<String> = (1..=9).map(h).collect();
        let proof = build_inclusion_proof(&next_roots, 3).unwrap();
        assert!(verify_inclusion_proof(&proof));

        // Tamper with one sibling hash
        let mut tampered = proof.clone();
        tampered.path[0].hash = "00".repeat(32);
        assert!(!verify_inclusion_proof(&tampered));
    }

    #[test]
    fn test_9_leaf_proof_index_3_cross_language() {
        // Cross-language fixture: all values verified against Python output
        let next_roots: Vec<String> = (1..=9).map(h).collect();
        let proof = build_inclusion_proof(&next_roots, 3).unwrap();

        assert_eq!(
            proof.root,
            "2cbf59819f3070f62601d1e0c47af000896f1c7bcb3f1e421c9bcdbf6c20ef09"
        );
        assert_eq!(
            proof.receipt_next_root,
            "9bf922d8ee39a15df6c2b0081aa0f508601a13287fc31a902aa34eccbf835def"
        );
        assert_eq!(
            proof.leaf_hash,
            "975f3a064d539cced75a2da9659911b5d5429496bfede6d7c83fa0cca6bb1579"
        );
        assert_eq!(proof.peak_index, 0);
        assert_eq!(proof.peak_height, 3);
        assert_eq!(proof.peaks.len(), 2);
        assert_eq!(proof.peaks[0].height, 3);
        assert_eq!(proof.peaks[1].height, 0);
        assert_eq!(
            proof.computed_peak_root,
            "cc7b5fd2bc078fd0dc219b51d725405b4ba7cc936aeb83b4fb56d8a0ade8a9fb"
        );

        assert!(verify_inclusion_proof(&proof));
    }

    // -----------------------------------------------------------------------
    // MerkleMountainRange stateful wrapper
    // -----------------------------------------------------------------------

    #[test]
    fn test_mmr_incremental_matches_batch() {
        let next_roots: Vec<String> = (1..=17).map(h).collect();

        // Batch computation
        let (batch_root, batch_peaks) = mmr_root_from_next_roots(&next_roots).unwrap();

        // Incremental computation
        let mut mmr = MerkleMountainRange::new();
        for nr in &next_roots {
            mmr.append(nr).unwrap();
        }

        assert_eq!(mmr.leaf_count(), 17);
        assert_eq!(mmr.root().unwrap(), batch_root);
        let inc_peaks = mmr.peaks();
        assert_eq!(inc_peaks.len(), batch_peaks.len());
        for (a, b) in inc_peaks.iter().zip(batch_peaks.iter()) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn test_mmr_build_proof_from_state() {
        let mut mmr = MerkleMountainRange::new();
        for i in 1..=9 {
            mmr.append(&h(i)).unwrap();
        }

        let proof = mmr.build_proof(3).unwrap();
        assert!(verify_inclusion_proof(&proof));
        assert!(mmr.verify_proof(&proof).unwrap());
    }

    #[test]
    fn test_mmr_empty() {
        let mmr = MerkleMountainRange::new();
        assert_eq!(mmr.leaf_count(), 0);
        assert!(mmr.peaks().is_empty());
        assert_eq!(mmr.root().unwrap(), "");
    }

    // -----------------------------------------------------------------------
    // append_peaks (checkpoint extension)
    // -----------------------------------------------------------------------

    #[test]
    fn test_append_peaks_matches_full_build() {
        // Build peaks from first 8 leaves
        let leaf_hashes_8: Vec<String> =
            (1..=8).map(|i| mmr_leaf_hash(&h(i)).unwrap()).collect();
        let peaks_8 = build_peaks(&leaf_hashes_8).unwrap();

        // Append leaves 9..=17
        let new_leaf_hashes: Vec<String> =
            (9..=17).map(|i| mmr_leaf_hash(&h(i)).unwrap()).collect();
        let extended = append_peaks(&peaks_8, &new_leaf_hashes).unwrap();

        // Compare with full build
        let all_leaf_hashes: Vec<String> =
            (1..=17).map(|i| mmr_leaf_hash(&h(i)).unwrap()).collect();
        let full_peaks = build_peaks(&all_leaf_hashes).unwrap();

        assert_eq!(extended.len(), full_peaks.len());
        for (a, b) in extended.iter().zip(full_peaks.iter()) {
            assert_eq!(a, b);
        }
    }

    // -----------------------------------------------------------------------
    // Peak plan tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_peak_plan() {
        assert_eq!(peak_plan(1), vec![(0, 1)]);
        assert_eq!(peak_plan(2), vec![(1, 2)]);
        assert_eq!(peak_plan(3), vec![(1, 2), (0, 1)]);
        assert_eq!(peak_plan(17), vec![(4, 16), (0, 1)]);
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_proof_out_of_range() {
        let next_roots: Vec<String> = (1..=5).map(h).collect();
        assert!(build_inclusion_proof(&next_roots, 5).is_err());
        assert!(build_inclusion_proof(&next_roots, 100).is_err());
    }

    #[test]
    fn test_proof_empty_mmr() {
        let next_roots: Vec<String> = vec![];
        assert!(build_inclusion_proof(&next_roots, 0).is_err());
    }

    #[test]
    fn test_single_leaf_proof() {
        let next_roots = vec![h(1)];
        let proof = build_inclusion_proof(&next_roots, 0).unwrap();
        assert!(verify_inclusion_proof(&proof));
        assert!(proof.path.is_empty());
    }

    #[test]
    fn test_all_indices_for_various_sizes() {
        for size in [1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33] {
            let next_roots: Vec<String> = (1..=size).map(h).collect();
            let (root, _) = mmr_root_from_next_roots(&next_roots).unwrap();
            for idx in 0..size {
                let proof = build_inclusion_proof(&next_roots, idx).unwrap();
                assert_eq!(proof.root, root, "root mismatch at size={size}, idx={idx}");
                assert!(
                    verify_inclusion_proof(&proof),
                    "proof failed at size={size}, idx={idx}"
                );
            }
        }
    }
}
