# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Label and loss for the POSET branching policy."""

import math

from torch import Tensor
from torch.nn.functional import huber_loss

from training.schema import Candidate

_FAIL_LABEL_NUM = 2.0
_SUCCESS_LABEL = 3.0


def compute_label(candidate: Candidate) -> float:
    """Graded relevance label for 1 tried candidate."""

    if not candidate.tried:
        raise ValueError("compute_label called on untried candidate")
    if candidate.succeeded:
        return _SUCCESS_LABEL

    return _FAIL_LABEL_NUM / (1.0 + math.log1p(candidate.subtree_nodes))


def regression_loss(scores: Tensor, labels: Tensor) -> Tensor:
    """Huber loss between predicted scores and graded relevance labels."""

    return huber_loss(scores, labels, reduction="mean", delta=1.0)