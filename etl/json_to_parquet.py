"""Convert solver JSONL gz trace shards to parquet shards.

Reads every `*.jsonl.gz` shard from an input directory, parses each line
as either an instance-header row or a branch row, and writes two parquet files
in the output directory:

- `instances.parquet` - one row per ML training instance
- `branches.parquet` - one row per branch (FK = instance_id)
"""

import argparse
import gzip
import json
import sys
from pathlib import Path

import pyarrow as pa
import pyarrow.parquet as pq


# Schemas

PLACEMENT_STRUCT = pa.struct([
    ("cells", pa.list_(pa.uint16())),
    ("piece_def_idx", pa.uint16()),
    ("mark_on_center", pa.bool_())
])

INSTANCES_SCHEMA = pa.schema([
    ("instance_id", pa.string()),
    # 6x6 byte-per-cell canonical bitmap per piece def, 36 bytes each.
    ("canonical_bitmaps", pa.list_(pa.binary(36))),
    ("cell_to_grid_idx", pa.list_(pa.uint16())),
    ("placements", pa.list_(PLACEMENT_STRUCT))
])

PRE_STATE_STRUCT = pa.struct([
    # 28-byte bit-packed empty-cell bitmap; bit `i` set means cell `i` empty.
    ("empty_bitmap", pa.binary(28)),
    ("center_mark", pa.bool_()),
    ("counts", pa.list_(pa.uint8()))
])

CANDIDATE_STRUCT = pa.struct([
    ("placement_idx", pa.uint32()),
    ("tried", pa.bool_()),
    ("succeeded", pa.bool_()),
    ("subtree_nodes", pa.uint64())
])

BRANCHES_SCHEMA = pa.schema([
    ("instance_id", pa.string()),
    ("branch_id", pa.uint32()),
    ("pre_state", PRE_STATE_STRUCT),
    ("candidates", pa.list_(CANDIDATE_STRUCT))
])


# Transforms - JSON row dict -> pyarrow-friendly dict matching schemas.

def transform_instance(row: dict) -> dict:
    """Instance header row -> dict matching INSTANCES_SCHEMA."""

    return {
        "instance_id": row["instance_id"],
        "canonical_bitmaps": [bytes(bm) for bm in row["canonical_bitmaps"]],
        "cell_to_grid_idx": row["cell_to_grid_idx"],
        "placements": row["placements"]
    }


def transform_branch(row: dict) -> dict:
    """Branch row -> dict matching BRANCHES_SCHEMA."""

    pre = row["pre_state"]
    return {
        "instance_id": row["instance_id"],
        "branch_id": row["branch_id"],
        "pre_state": {
            "empty_bitmap": bytes(pre["empty_bitmap"]),
            "center_mark": pre["center_mark"],
            "counts": pre["counts"]
        },
        "candidates": pre["candidates"]
    }


# Pipeline

def iter_jsonl_gz(path: Path):
    """Yield parsed JSON objects line by line from a gzipped jsonl file."""
    with gzip.open(path, "rt", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue

            yield json.loads(line)


def convert(in_dir: Path, out_dir: Path, batch_size: int) -> tuple[int, int]:
    """Convert all jsonl.gz shards in `in_dir` to 2 parquet files in `out_dir`.

    Returns `(instance_count, branch_count)`.
    """

    out_dir.mkdir(parents=True, exist_ok=True)
    instances_path = out_dir / "instances.parquet"
    branches_path = out_dir / "branches.parquet"

    inst_writer = pq.ParquetWriter(instances_path, INSTANCES_SCHEMA, compression="snappy")
    br_writer = pq.ParquetWriter(branches_path, BRANCHES_SCHEMA, compression="snappy")

    inst_batch: list[dict] = []
    br_batch: list[dict] = []
    instance_count = 0
    branch_count = 0