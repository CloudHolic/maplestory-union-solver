# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""PyTorch Dataset over the synthetic branch trace JSONL shards.

Reads `synth-*.jsonl.gz` shards produces by the Rust generator.
Each shard interleaves two row kinds:

    {"_kind":"instance", "instance_id":..., "canonical_bitmaps":[...]}
    {"_kind":"branch", "instance_id":..., "branch_id":..., "candidates":[...]}

The dataset yields one item per (branch, candidate-with-tried=true) pair.
Untried candidates are dropped here - they have valid post_state but no label.
"""

import gzip
import json
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path

from torch.utils.data import IterableDataset

from poset.schema import PostState, PreState
from training.schema import BranchRow, Candidate, InstanceHeader, Placement

# Public types

@dataclass(slots=True, frozen=True)
class TrainingItem:
    """One (branch, tried candidate) pair, ready for transforms.

    Holds raw schema dataclasses.
    """

    header: InstanceHeader
    branch: BranchRow
    candidate: Candidate
    post_state: PostState


# Dataset

class BranchTraceDataset(IterableDataset[TrainingItem]):
    """Streams (branch, candidate) items across one or more shards."""

    def __init__(self, shard_paths: list[Path]) -> None:
        if not shard_paths:
            raise ValueError("shard_paths is empty")
        self.shard_paths = shard_paths

    def __iter__(self) -> Iterator[TrainingItem]:
        for shard_path in self.shard_paths:
            yield from _iter_shard(shard_path)


# Shard iteration

def _iter_shard(shard_path: Path) -> Iterator[TrainingItem]:
    """Yield TrainingItem from a single shard, caching headers."""

    current_header: InstanceHeader | None = None

    with gzip.open(shard_path, "rt", encoding="utf-8") as f:
        for line_num, line in enumerate(f, start=1):
            line = line.strip()
            if not line:
                continue

            row = json.loads(line)
            kind = row.get("_kind")

            if kind == "instance":
                current_header = _parse_instance(row)
            elif kind == "branch":
                if current_header is None:
                    raise ValueError(
                        f"{shard_path}:{line_num}: branch row before any instance header"
                    )

                branch = _parse_branch(row)
                if branch.instance_id != current_header.instance_id:
                    raise ValueError(
                        f"{shard_path}:{line_num}: branch instance_id "
                        f"{branch.instance_id!r} does not match cached header "
                        f"{current_header.instance_id!r}"
                    )

                for candidate in branch.candidates:
                    if candidate.tried:
                        post_state = _reconstruct_post_state(
                            branch.pre_state,
                            current_header.placements[candidate.placement_idx],
                            current_header.cell_to_grid_idx
                        )
                        yield TrainingItem(current_header, branch, candidate, post_state)
            else:
                raise ValueError(f"{shard_path}:{line_num}: unknown_kind {kind!r}")


# Dataclass parsers

def _reconstruct_post_state(pre: PreState, placement: Placement, cell_to_grid_idx: list[int]) -> PostState:
    """Apply placement to pre_state to produce post_state."""

    post_bits = bytearray(pre.empty_bitmap)
    for cell_idx in placement.cells:
        byte = cell_idx // 8
        bit = cell_idx % 8
        post_bits[byte] &= ~(1 << bit) & 0xFF

    empty_target_indices = []
    for ci in range(len(cell_to_grid_idx)):
        if (post_bits[ci // 8] >> (ci % 8)) & 1:
            empty_target_indices.append(cell_to_grid_idx[ci])

    post_counts = list(pre.counts)
    post_counts[placement.piece_def_idx] -= 1

    post_center_mark = 1 if (pre.center_mark or placement.mark_on_center) else 0

    return PostState(
        empty_target_indices=empty_target_indices,
        center_mark=post_center_mark,
        counts=post_counts
    )


def _parse_instance(row: dict) -> InstanceHeader:
    placements = [
        Placement(cells=p["cells"], piece_def_idx=p["piece_def_idx"], mark_on_center=p["mark_on_center"])
        for p in row["placements"]
    ]

    return InstanceHeader(
        instance_id=row["instance_id"],
        canonical_bitmaps=row["canonical_bitmaps"],
        cell_to_grid_idx=row["cell_to_grid_idx"],
        placements=placements
    )


def _parse_branch(row: dict) -> BranchRow:
    pre = row["pre_state"]
    pre_state = PreState(
        empty_bitmap=bytes(pre["empty_bitmap"]),
        center_mark=pre["center_mark"],
        counts=pre["counts"]
    )
    candidates = [_parse_candidate(c) for c in row["candidates"]]

    return BranchRow(
        instance_id=row["instance_id"],
        branch_id=row["branch_id"],
        pre_state=pre_state,
        candidates=candidates
    )


def _parse_candidate(row: dict) -> Candidate:
    post = row["post_state"]
    return Candidate(
        placement_idx=row["placement_idx"],
        tried=post["tried"],
        succeeded=post["succeeded"],
        subtree_nodes=post["subtree_nodes"]
    )
