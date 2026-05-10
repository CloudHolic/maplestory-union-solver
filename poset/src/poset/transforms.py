# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Tensor conversion for POSET inputs."""

import torch
import torch.nn.functional as f
from torch import Tensor

from poset.schema import BOARD_SIZE, CANONICAL_SIZE, PostState


def post_state_to_tensors(
    state: PostState,
    instance_bitmaps: list[list[int]]
) -> dict[str, Tensor]:
    """Convert a PostState + the instance's canonical bitmaps to model input tensors.

    Args:
        - state: post-candidate board state.
        - instance_bitmaps: the instance header's `canonical_bitmaps`.

    Returns dict with:
        - empty_target: float32 Tensor [BOARD_SIZE], 1 where empty target.
        - center_mark:  float32 Tensor [1].
        - pieces:       float32 Tensor [N_active x CANONICAL_SIZE].
        - counts:       float32 Tensor [N_active x CANONICAL_SIZE].

    N_active is the number of piece types with count > 0. Guaranteed >= 1 at any branch point.
    """

    if len(instance_bitmaps) != len(state.counts):
        raise ValueError(
            f"instance_bitmaps has {len(instance_bitmaps)} entries but "
            f"state.counts has {len(state.counts)}; they must align"
        )

    # Empty-target bitmap: dense BOARD_SIZE with 1 at each indexed cell.
    empty_target = torch.zeros(BOARD_SIZE, dtype=torch.float32)
    if state.empty_target_indices:
        idx = torch.tensor(state.empty_target_indices, dtype=torch.long)
        empty_target[idx] = 1.0

    # Center mark: scalar wrapped to [1].
    center_mark = torch.tensor([state.center_mark], dtype=torch.float32)

    # Count > 0 filter - DeepSet sees only pieces actually available.
    active_pairs = [
        (bitmap, count)
        for bitmap, count in zip(instance_bitmaps, state.counts, strict=True)
        if count > 0
    ]
    if not active_pairs:
        raise ValueError("No pieces with count > 0.")

    pieces = torch.tensor([bitmap for bitmap, _ in active_pairs], dtype=torch.float32)
    if pieces.shape[1] != CANONICAL_SIZE:
        raise ValueError(
            f"piece bitmap length {pieces.shape[1]} != CANONICAL_SIZE {CANONICAL_SIZE}"
        )

    counts = torch.tensor([count for _, count in active_pairs], dtype=torch.float32)

    return {
        "empty_target": empty_target,
        "center_mark": center_mark,
        "pieces": pieces,
        "counts": counts
    }


def pad_piece_set(per_state_tensors: list[dict[str, Tensor]]) -> dict[str, Tensor]:
    """Pad varying-N piece sets across multiple states to a common max_n."""

    if not per_state_tensors:
        raise ValueError("per_state_tensors is empty")

    max_n = max(t["pieces"].shape[0] for t in per_state_tensors)

    padded_pieces, padded_counts, masks = [], [], []
    for t in per_state_tensors:
        n = t["pieces"].shape[0]
        pad = max_n - n
        padded_pieces.append(f.pad(t["pieces"], (0, 0, 0, pad)))
        padded_counts.append(f.pad(t["counts"], (0, pad)))

        mask = torch.zeros(max_n, dtype=torch.float32)
        mask[:n] = 1.0
        masks.append(mask)

    return {
        "empty_target": torch.stack([t["empty_target"] for t in per_state_tensors]),
        "center_mark": torch.stack([t["center_mark"] for t in per_state_tensors]),
        "pieces": torch.stack(padded_pieces),
        "counts": torch.stack(padded_counts),
        "piece_mask": torch.stack(masks)
    }