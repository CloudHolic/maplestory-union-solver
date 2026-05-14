# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""PyTorch Dataset over the parquet branch trace shards.

Reads `instances.parquet` + `branches.parquet` from a directory.
`instances` is loaded fully in memory; `branches` is streamed row-group at a time.

Each yielded item in one (branch, tried-candidate) pair. Untried candidates are dropped.
"""

import hashlib
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path
from typing import Literal

import pyarrow.parquet as pq
from torch.utils.data import IterableDataset

from poset.schema import PostState, PreState
from training.schema import (
    BranchRow,
    Candidate,
    InstanceHeader,
    Placement,
    branch_from_arrow,
    instance_from_arrow
)


Split = Literal["train", "val", "all"]


@dataclass(slots=True, frozen=True)
class TrainingItem:
    """One (branch, tried candidate) pair, ready for transforms."""

    header: InstanceHeader
    branch: BranchRow
    candidate: Candidate
    post_state: PostState


# Dataset

class BranchTraceDataset(IterableDataset[TrainingItem]):
    """Streams (branch, candidate) items from parquet shards."""

    def __init__(
        self,
        data_dir: Path,
        split: Split = "all",
        val_ratio: float = 0.1
    ) -> None:
        if not (0.0 <= val_ratio < 1.0):
            raise ValueError(f"val_ratio must be in [0, 1), got {val_ratio}")

        instances_path = data_dir / "instances.parquet"
        branches_path = data_dir / "branches.parquet"
        if not instances_path.is_file():
            raise FileNotFoundError(f"missing: {instances_path}")
        if not branches_path.is_file():
            raise FileNotFoundError(f"missing: {branches_path}")

        self._branches_path = branches_path
        self._split = split
        self._val_ratio = val_ratio

        # Headers fully in memory
        inst_table = pq.read_table(instances_path)
        self._headers: dict[str, InstanceHeader] = {
            row["instance_id"]: instance_from_arrow(row)
            for row in inst_table.to_pylist()
        }

    def __iter__(self) -> Iterator[TrainingItem]:
        pf = pq.ParquetFile(self._branches_path)
        for batch in pf.iter_batches():
            for row in batch.to_pylist():
                instance_id = row["instance_id"]
                if not self._include(instance_id):
                    continue

                header = self._headers.get(instance_id)
                if header is None:
                    raise ValueError(f"branch references unknown instance_id {instance_id!r}")

                branch = branch_from_arrow(row)
                for candidate in branch.candidates:
                    if not candidate.tried:
                        continue

                    post_state = _reconstruct_post_state(
                        branch.pre_state,
                        header.placements[candidate.placement_idx],
                        header.cell_to_grid_idx
                    )

                    yield TrainingItem(header, branch, candidate, post_state)

    def _include(self, instance_id: str) -> bool:
        """Decide whether this instance belongs to the current split."""

        if self._split == "all" or self._val_ratio == 0.0:
            return self._split != "val"

        # First 8 bytes of SHA-256 -> integer mod 10000 -> fraction.
        h = hashlib.sha256(instance_id.encode("utf8")).digest()[:8]
        bucket = int.from_bytes(h, "big") % 10000
        is_val = bucket < int(self._val_ratio * 10000)
        return (self._split == "val") == is_val


# Post-state reconstruction

def _reconstruct_post_state(
    pre: PreState,
    placement: Placement,
    cell_to_grid_idx: list[int]
) -> PostState:
    """Apply placement to pre_state to produce post_state."""

    post_bits = bytearray(pre.empty_bitmap)
    for cell_idx in placement.cells:
        byte = cell_idx // 8
        bit = cell_idx % 8
        post_bits[byte] &= ~(1 << bit) & 0xFF

    empty_target_indices = [
        cell_to_grid_idx[ci]
        for ci in range(len(cell_to_grid_idx))
        if (post_bits[ci // 8] >> (ci % 8)) & 1
    ]

    post_counts = list(pre.counts)
    post_counts[placement.piece_def_idx] -= 1
    post_center_mark = 1 if (pre.center_mark or placement.mark_on_center) else 0

    return PostState(
        empty_target_indices=empty_target_indices,
        center_mark=post_center_mark,
        counts=post_counts
    )
