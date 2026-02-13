"""Merkle Mountain Range (MMR) utilities for MSEZ corridor state channels.

This module implements an **append-only accumulator** that supports compact inclusion proofs
for receipts without requiring disclosure of the full receipt set.

Design goals:
- Deterministic across implementations
- Simple reference implementation (not optimized)
- Inclusion-proof-friendly (MMR) for receipt commitments

Hashing:
- SHA-256
- Domain separation:
  - leaf = SHA256(0x00 || leaf_bytes)
  - node = SHA256(0x01 || left || right)

In MSEZ v0.4.3+, the recommended leaf_bytes is the 32-byte `next_root` digest of a corridor
state receipt (decoded from hex).

"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from typing import List, Tuple, Dict, Any


_SHA256_HEX_RE = None


def _sha256(b: bytes) -> bytes:
    return hashlib.sha256(b).digest()


def _is_hex_32(s: str) -> bool:
    if not isinstance(s, str):
        return False
    ss = s.strip().lower()
    if len(ss) != 64:
        return False
    try:
        bytes.fromhex(ss)
        return True
    except ValueError:
        return False


def mmr_leaf_hash(next_root_hex: str) -> str:
    """Compute the MMR leaf hash from a 32-byte digest encoded as 64 hex chars."""
    if not _is_hex_32(next_root_hex):
        raise ValueError("next_root_hex must be 64 lowercase hex chars")
    leaf_bytes = bytes.fromhex(next_root_hex.strip().lower())
    return _sha256(b"\x00" + leaf_bytes).hex()


def mmr_node_hash(left_hex: str, right_hex: str) -> str:
    """Compute a parent hash from two child hashes (each 32 bytes hex)."""
    if not _is_hex_32(left_hex) or not _is_hex_32(right_hex):
        raise ValueError("left_hex and right_hex must be 64 hex chars")
    left = bytes.fromhex(left_hex.strip().lower())
    right = bytes.fromhex(right_hex.strip().lower())
    return _sha256(b"\x01" + left + right).hex()


@dataclass(frozen=True)
class Peak:
    height: int
    hash: str


def build_peaks(leaf_hashes: List[str]) -> List[Peak]:
    """Build MMR peaks for a list of leaf hashes (left-to-right append order)."""
    peaks: List[Tuple[int, str]] = []  # (height, hash)
    for lh in leaf_hashes:
        if not _is_hex_32(lh):
            raise ValueError("leaf_hash must be 64 hex chars")
        cur_h = 0
        cur = lh.strip().lower()
        # Merge while the top peak has the same height.
        while peaks and peaks[-1][0] == cur_h:
            left_h, left = peaks.pop()
            cur = mmr_node_hash(left, cur)
            cur_h = left_h + 1
        peaks.append((cur_h, cur))
    return [Peak(height=h, hash=d) for (h, d) in peaks]


def bag_peaks(peaks: List[Peak]) -> str:
    """Compute the bagged root from peaks.

    The root is computed by folding peaks from right-to-left using the same node hash:

      bag = peaks[-1]
      for peak in reversed(peaks[:-1]):
          bag = node_hash(peak, bag)

    """
    if not peaks:
        # Conventional empty commitment: SHA256(0x00) is not great; but MMR size MUST be >= 1 in practice.
        return ""  # callers should handle
    bag = peaks[-1].hash
    for p in reversed(peaks[:-1]):
        bag = mmr_node_hash(p.hash, bag)
    return bag


def mmr_root_from_next_roots(next_roots_hex: List[str]) -> Dict[str, Any]:
    """Compute MMR root and peaks from a sequence of receipt next_root digests."""
    leaf_hashes = [mmr_leaf_hash(nr) for nr in next_roots_hex]
    peaks = build_peaks(leaf_hashes)
    root = bag_peaks(peaks)
    return {
        "size": len(leaf_hashes),
        "root": root,
        "peaks": [{"height": p.height, "hash": p.hash} for p in peaks],
    }


def _peak_plan(size: int) -> List[Tuple[int, int]]:
    """Return a list of peaks as (height, leaf_count) from left-to-right for a given leaf size."""
    if size < 0:
        raise ValueError("size must be >= 0")
    out: List[Tuple[int, int]] = []
    n = size
    while n > 0:
        # highest power of two <= n
        h = n.bit_length() - 1
        cnt = 1 << h
        out.append((h, cnt))
        n -= cnt
    return out


def _find_peak_for_leaf(size: int, leaf_index: int) -> Tuple[int, int, int]:
    """Return (peak_index, peak_start, peak_height) for leaf_index in an MMR of given size."""
    if leaf_index < 0 or leaf_index >= size:
        raise ValueError("leaf_index out of range")
    start = 0
    plan = _peak_plan(size)
    for i, (h, cnt) in enumerate(plan):
        if start <= leaf_index < start + cnt:
            return (i, start, h)
        start += cnt
    raise RuntimeError("unable to locate peak")


def merkle_path_for_power_of_two(leaf_hashes: List[str], leaf_pos: int) -> Tuple[str, List[Dict[str, str]]]:
    """Compute a Merkle root and sibling path for a power-of-two sized leaf list."""
    n = len(leaf_hashes)
    if n == 0:
        raise ValueError("leaf_hashes must be non-empty")
    if n & (n - 1) != 0:
        raise ValueError("leaf_hashes length must be power of two")
    if leaf_pos < 0 or leaf_pos >= n:
        raise ValueError("leaf_pos out of range")

    level = [h.strip().lower() for h in leaf_hashes]
    for h in level:
        if not _is_hex_32(h):
            raise ValueError("invalid leaf hash")

    pos = leaf_pos
    path: List[Dict[str, str]] = []

    while len(level) > 1:
        sibling_pos = pos ^ 1
        sibling_hash = level[sibling_pos]
        # If sibling_pos < pos, sibling is on the left.
        side = "left" if sibling_pos < pos else "right"
        path.append({"side": side, "hash": sibling_hash})

        # Build next level
        nxt: List[str] = []
        for i in range(0, len(level), 2):
            nxt.append(mmr_node_hash(level[i], level[i + 1]))
        level = nxt
        pos //= 2

    return level[0], path


def build_inclusion_proof(next_roots_hex: List[str], leaf_index: int) -> Dict[str, Any]:
    """Build an inclusion proof for `leaf_index` in the MMR built from next_roots_hex."""
    size = len(next_roots_hex)
    if size <= 0:
        raise ValueError("cannot build proof for empty MMR")
    if leaf_index < 0 or leaf_index >= size:
        raise ValueError("leaf_index out of range")

    leaf_hashes = [mmr_leaf_hash(nr) for nr in next_roots_hex]
    peaks = build_peaks(leaf_hashes)
    root = bag_peaks(peaks)

    peak_index, peak_start, peak_height = _find_peak_for_leaf(size, leaf_index)
    peak_leaf_count = 1 << peak_height
    local_pos = leaf_index - peak_start
    peak_leaves = leaf_hashes[peak_start : peak_start + peak_leaf_count]

    peak_root, path = merkle_path_for_power_of_two(peak_leaves, local_pos)

    proof = {
        "size": size,
        "root": root,
        "leaf_index": leaf_index,
        "receipt_next_root": next_roots_hex[leaf_index].strip().lower(),
        "leaf_hash": leaf_hashes[leaf_index],
        "peak_index": peak_index,
        "peak_height": peak_height,
        "path": path,
        "peaks": [{"height": p.height, "hash": p.hash} for p in peaks],
        "computed_peak_root": peak_root,
    }
    return proof


def verify_inclusion_proof(proof: Dict[str, Any]) -> bool:
    """Verify an inclusion proof object produced by build_inclusion_proof()."""
    try:
        size = int(proof.get("size"))
        leaf_index = int(proof.get("leaf_index"))
        receipt_next_root = str(proof.get("receipt_next_root") or "").strip().lower()
        leaf_hash = str(proof.get("leaf_hash") or "").strip().lower()
        peak_index = int(proof.get("peak_index"))
        peak_height = int(proof.get("peak_height"))
        root = str(proof.get("root") or "").strip().lower()
        peaks_in = proof.get("peaks")
        path = proof.get("path")
    except (TypeError, ValueError, AttributeError):
        return False

    if size <= 0 or leaf_index < 0 or leaf_index >= size:
        return False
    if not _is_hex_32(receipt_next_root) or not _is_hex_32(root):
        return False

    expected_leaf = mmr_leaf_hash(receipt_next_root)
    if expected_leaf != leaf_hash:
        return False

    if not isinstance(peaks_in, list) or not peaks_in:
        return False
    peaks: List[Peak] = []
    for p in peaks_in:
        if not isinstance(p, dict):
            return False
        h = int(p.get("height"))
        hh = str(p.get("hash") or "").strip().lower()
        if h < 0 or not _is_hex_32(hh):
            return False
        peaks.append(Peak(height=h, hash=hh))

    if peak_index < 0 or peak_index >= len(peaks):
        return False
    if peaks[peak_index].height != peak_height:
        return False

    # Verify peak selection is consistent with MMR size.
    try:
        exp_peak_index, _start, exp_height = _find_peak_for_leaf(size, leaf_index)
        if exp_peak_index != peak_index or exp_height != peak_height:
            return False
    except (ValueError, RuntimeError):
        return False

    # Compute peak root from path.
    cur = leaf_hash
    if not isinstance(path, list):
        return False
    for step in path:
        if not isinstance(step, dict):
            return False
        side = str(step.get("side") or "").strip().lower()
        h = str(step.get("hash") or "").strip().lower()
        if side not in {"left", "right"}:
            return False
        if not _is_hex_32(h):
            return False
        cur = mmr_node_hash(h, cur) if side == "left" else mmr_node_hash(cur, h)

    # Substitute computed peak root.
    peaks2 = list(peaks)
    peaks2[peak_index] = Peak(height=peak_height, hash=cur)

    computed_root = bag_peaks(peaks2)
    return computed_root == root


def peaks_from_json(peaks_json: Any) -> List[Peak]:
    """Parse a checkpoint MMR peaks list from JSON."""
    peaks: List[Peak] = []
    if not isinstance(peaks_json, list):
        return peaks
    for p in peaks_json:
        if not isinstance(p, dict):
            continue
        h = p.get("height")
        d = p.get("hash")
        if isinstance(h, int) and isinstance(d, str) and d:
            peaks.append(Peak(height=h, hash=d))
    return peaks


def peaks_to_json(peaks: List[Peak]) -> List[Dict[str, Any]]:
    return [{"height": int(p.height), "hash": str(p.hash)} for p in peaks]


def append_peaks(existing_peaks: List[Peak], new_leaf_hashes: List[str]) -> List[Peak]:
    """Incrementally append leaves to an existing MMR peak set.

    This enables verifiers to start from a checkpoint (size + peaks) and extend the
    accumulator with new receipts without replaying the entire history.

    Algorithm: treat `existing_peaks` as the current stack of (height,hash) nodes. For each
    new leaf hash, merge while the top of the stack has the same height.
    """
    stack: List[Tuple[int, str]] = [(int(p.height), str(p.hash)) for p in existing_peaks]
    for leaf in new_leaf_hashes:
        cur_h = 0
        cur = str(leaf)
        while stack and stack[-1][0] == cur_h:
            left_h, left = stack.pop()
            cur = mmr_node_hash(left, cur)
            cur_h = left_h + 1
        stack.append((cur_h, cur))
    return [Peak(height=h, hash=d) for (h, d) in stack]
