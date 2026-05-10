# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Wire-format dataclasses for the synthetic branch trace JSONL schema."""

from dataclasses import dataclass

from poset.schema import PostState


@dataclass(slots=True, frozen=True)
class InstanceHeader:
    """Once per instance. Cached and joined to every branch row in that instance.

    Fields:
        - instance_id: matches `branch.instance_id` for join.
        - canonical_bitmaps: each bitmap is 36 ints (0/1), row-major over the 6x6 canonical form.
    """

    instance_id: str
    canonical_bitmaps: list[list[int]]


@dataclass(slots=True, frozen=True)
class Candidate:
    """A single placement candidate at a branch point.

    Fields:
        - placement_idx: solver's placement index.
        - post_state: board state after virtually applying this candidate.
        - tried: True iff the solver descended into this candidate.
        - succeeded: meaningful only when tried=True.
        - subtree_nodes: meaningful only when tried=True. Used by the graded-relevance label:
            succeeded=True  -> 3.0
            succeeded=False -> 2 / (1 + log(subtree_nodes))
            tried=False     -> excluded from training.
    """

    placement_idx: int
    post_state: PostState
    tried: bool
    succeeded: bool
    subtree_nodes: int


@dataclass(slots=True, frozen=True)
class BranchRow:
    """One branch point with its candidate set."""

    instance_id: str
    branch_id: int
    candidates: list[Candidate]
