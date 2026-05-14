# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Wire-format dataclasses for the synthetic branch trace JSONL schema."""

from dataclasses import dataclass

from poset.schema import PreState


@dataclass(slots=True, frozen=True)
class Placement:
    """Compact placement description carried in the instance header.

    Fields:
        - cells: board-cells indices that this placement covers.
        - piece_def_idx: index into the header's `canonical_bitmap` and
          the pre_state's `count` arrays, identifying which piece definition
          this placement instantiates.
        - mark_on_center: whether this placement's marked cell falls in the center 4-cell region.
    """
    cells: list[int]
    piece_def_idx: int
    mark_on_center: bool


@dataclass(slots=True, frozen=True)
class InstanceHeader:
    """Once per instance. Cached and joined to every branch row in that instance.

    Fields:
        - instance_id: matches `branch.instance_id` for join.
        - canonical_bitmaps: each bitmap is 36 ints (0/1), row-major over the 6x6 canonical form.
        - cell_to_grid_idx: maps board-cell index to grid row-major index.
        - placements: The instance's flat palcement table, with one entry per valid placement.
    """

    instance_id: str
    canonical_bitmaps: list[list[int]]
    cell_to_grid_idx: list[int]
    placements: list[Placement]


@dataclass(slots=True, frozen=True)
class Candidate:
    """A single placement candidate at a branch point.

    Fields:
        - placement_idx: solver's placement index.
        - tried: True iff the solver descended into this candidate.
        - succeeded: meaningful only when tried=True.
        - subtree_nodes: meaningful only when tried=True. Used by the graded-relevance label:
            succeeded=True  -> 3.0
            succeeded=False -> 2 / (1 + log(subtree_nodes))
            tried=False     -> excluded from training.
    """

    placement_idx: int
    tried: bool
    succeeded: bool
    subtree_nodes: int


@dataclass(slots=True, frozen=True)
class BranchRow:
    """One branch point with its pre-branch state and candidate set.

    Fields:
        - instance_id: Foreign key to the cached InstanceHeader.
        - branch_id: Sequential branch identifier within the instance,
          assigned in solver-visit order.
        - pre_state: Board state at branch entry.
        - candidates: All candidates the solver considered at this branch, in attempt order.
    """

    instance_id: str
    branch_id: int
    pre_state: PreState
    candidates: list[Candidate]

# Parquet row -> dataclass converters

def instance_from_arrow(row: dict) -> InstanceHeader:
    """Convert a pyarrow row dict to InstanceHeader."""

    return InstanceHeader(
        instance_id=row["instance_id"],
        canonical_bitmaps=[list(bm) for bm in row["canonical_bitmaps"]],
        cell_to_grid_idx=row["cell_to_grid_idx"],
        placements=[
            Placement(
                cells=list(p["cells"]),
                piece_def_idx=p["piece_def_idx"],
                mark_on_center=p["mark_on_center"],
            )
            for p in row["placements"]
        ]
    )


def branch_from_arrow(row: dict) -> BranchRow:
    """Convert a pyarrow row dict to BranchRow."""

    pre = row["pre_state"]
    return BranchRow(
        instance_id=row["instance_id"],
        branch_id=row["branch_id"],
        pre_state=PreState(
            empty_bitmap=bytes(pre["empty_bitmap"]),
            center_mark=pre["center_mark"],
            counts=list(pre["counts"])
        ),
        candidates=[
            Candidate(
                placement_idx=c["placement_idx"],
                tried=c["tried"],
                succeeded=c["succeeded"],
                subtree_nodes=c["subtree_nodes"]
            )
            for c in row["candidates"]
        ]
    )