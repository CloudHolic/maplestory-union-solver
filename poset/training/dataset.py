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

from poset.schema import PostState
from training.schema import BranchRow, Candidate, InstanceHeader

# Public types

@dataclass(slots=True, frozen=True)
class TrainingItem:
    """One (branch, tried candidate) pair, ready for transforms.

    Holds raw schema dataclasses.
    """

    header: InstanceHeader
    branch: BranchRow
    candidate: Candidate


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
                        yield TrainingItem(current_header, branch, candidate)
            else:
                raise ValueError(f"{shard_path}:{line_num}: unknown_kind {kind!r}")


# Dataclass parsers

def _parse_instance(row: dict) -> InstanceHeader:
    return InstanceHeader(
        instance_id=row["instance_id"],
        canonical_bitmaps=row["canonical_bitmaps"]
    )


def _parse_branch(row: dict) -> BranchRow:
    candidates = [_parse_candidate(c) for c in row["candidates"]]
    return BranchRow(
        instance_id=row["instance_id"],
        branch_id=row["branch_id"],
        candidates=candidates
    )


def _parse_candidate(row: dict) -> Candidate:
    post = row["post_state"]
    return Candidate(
        placement_idx=row["placement_idx"],
        post_state=PostState(
            empty_target_indices=post["empty_target_indices"],
            center_mark=post["center_mark"],
            counts=post["counts"]
        ),
        tried=post["tried"],
        succeeded=post["succeeded"],
        subtree_nodes=post["subtree_nodes"]
    )
