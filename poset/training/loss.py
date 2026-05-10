# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Label and loss for the POSET branching policy."""

import math

import torch
from torch import Tensor

from training.schema import Candidate

SUCCESS_LABEL = 3.0


def compute_label(candidate: Candidate) -> float:
    """Graded relevance label for 1 tried candidate."""
    if not candidate.tried:
        raise ValueError(
            "compute_label called on tried=False candidate; "
            "dataset should have filtered it out"
        )

    if candidate.succeeded:
        return SUCCESS_LABEL

    # Failed: 2 / (1 + log(subtree_nodes)).
    return 2.0 / (1.0 + math.log(max(candidate.subtree_nodes, 1)))


def pairwise_margin_loss(
    scores: Tensor,         # [M], 1 per candidate
    labels: Tensor,         # [M], same flat layout
    group_ids: Tensor,      # [M], integer branch id per candidate
    *,
    margin: float = 1.0
) -> Tensor:
    """Pairwise margin ranking loss, grouped by branch."""

    if scores.shape != labels.shape or scores.shape != group_ids.shape:
        raise ValueError(
            f"shape mismatch: scores={tuple(scores.shape)}, "
            f"labels={tuple(labels.shape)}, group_ids={tuple(group_ids.shape)}"
        )

    # Pair grid: (i, j) with i, j in same group, labels[i] > labels[j].
    same_group = group_ids.unsqueeze(0) == group_ids.unsqueeze(1)   # [M, M]
    label_gt = labels.unsqueeze(0) > labels.unsqueeze(1)            # [M, M]
    eligible = same_group & label_gt                                # [M, M]

    if not eligible.any():
        return scores.sum() * 0.0   # 0 with grad

    # Score differences for eligible pairs.
    score_diff = scores.unsqueeze(0) - scores.unsqueeze(1)          # [M, M]
    pair_losses = torch.clamp(margin - score_diff, min=0.0)         # [M, M]
    return pair_losses[eligible].mean()