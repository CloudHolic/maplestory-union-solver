"""Convert solver JSONL gz trace shards to parquet shards.

Reads every `*.jsonl.gz` shard from an input directory, parses each line
as either an instance-header row or a branch row, and writes two parquet files
in the output directory:

- `instances/instances-NNNN.parquet`
- `branches/branches-NNNN.parquet`
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
    ("canonical_bitmaps", pa.list_(pa.binary())),
    ("cell_to_grid_idx", pa.list_(pa.uint16())),
    ("placements", pa.list_(PLACEMENT_STRUCT))
])

PRE_STATE_STRUCT = pa.struct([
    # 28-byte bit-packed empty-cell bitmap; bit `i` set means cell `i` empty.
    ("empty_bitmap", pa.binary()),
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

DEFAULT_SHARD_SIZE_MB = 500
DEFAULT_BATCH_SIZE = 1024


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
        "candidates": row["candidates"]
    }


# Sharded writer

class ShardedWriter:
    """Write a stream of rows into size-capped parquet shards."""

    def __init__(self, out_dir: Path, name: str, schema: pa.Schema, target_mb: int):
        self.out_dir = out_dir
        self.name = name
        self.schema = schema
        self.target_bytes = target_mb * 1024 * 1024
        self.shard_idx = 0
        self.current_bytes = 0
        self.writer: pq.ParquetWriter | None = None

    def write_table(self, table: pa.Table) -> None:
        if self.writer is None:
            self._open_new_shard()

        if self.writer is not None:
            self.writer.write_table(table)
            self.current_bytes += table.nbytes
            if self.current_bytes >= self.target_bytes:
                self.close()
                self.shard_idx += 1

    def close(self) -> None:
        if self.writer is not None:
            self.writer.close()
            self.writer = None

    def _open_new_shard(self) -> None:
        path = self.out_dir / f"{self.name}-{self.shard_idx:04d}.parquet"
        self.writer = pq.ParquetWriter(path, self.schema, compression="snappy")
        self.current_bytes = 0
        print(f"  opened {path.name}", file=sys.stderr)


# Pipeline

def iter_jsonl_gz(path: Path):
    """Yield parsed JSON objects line by line from a gzipped jsonl file."""
    with gzip.open(path, "rt", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue

            yield json.loads(line)


def convert(in_dir: Path, out_dir: Path, batch_size: int, shard_mb: int) -> tuple[int, int]:
    """Convert all jsonl.gz shards in `in_dir` to sharded parquet.

    Returns `(instance_count, branch_count)`.
    """

    inst_dir = out_dir / "instances"
    br_dir = out_dir / "branches"
    inst_dir.mkdir(parents=True, exist_ok=True)
    br_dir.mkdir(parents=True, exist_ok=True)

    inst_writer = ShardedWriter(inst_dir, "instances", INSTANCES_SCHEMA, shard_mb)
    br_writer = ShardedWriter(br_dir, "branches", BRANCHES_SCHEMA, shard_mb)

    inst_batch: list[dict] = []
    br_batch: list[dict] = []
    instance_count = 0
    branch_count = 0

    def flush_instances() -> None:
        nonlocal inst_batch
        if not inst_batch:
            return

        table = pa.Table.from_pylist(inst_batch, schema=INSTANCES_SCHEMA)
        inst_writer.write_table(table)
        inst_batch = []

    def flush_branches() -> None:
        nonlocal br_batch
        if not br_batch:
            return

        table = pa.Table.from_pylist(br_batch, schema=BRANCHES_SCHEMA)
        br_writer.write_table(table)
        br_batch = []

    shards = sorted(in_dir.rglob("*.jsonl.gz"))
    if not shards:
        print(f"no .jsonl.gz files found in {in_dir}", file=sys.stderr)
        sys.exit(1)

    for shard in shards:
        print(f"  reading {shard.name}", file=sys.stderr)
        for row in iter_jsonl_gz(shard):
            kind = row.get("_kind")
            if kind == "instance":
                inst_batch.append(transform_instance(row))
                instance_count += 1
                if len(inst_batch) >= batch_size:
                    flush_instances()
            elif kind == "branch":
                br_batch.append(transform_branch(row))
                branch_count += 1
                if len(br_batch) >= batch_size:
                    flush_branches()
            else:
                print(f"  warning: unknown _kind {kind!r}, skipping", file=sys.stderr)

    flush_instances()
    flush_branches()
    inst_writer.close()
    br_writer.close()

    return instance_count, branch_count


def main() -> None:
    ap = argparse.ArgumentParser(description="Convert JSONL gz trace shards to parquet")
    ap.add_argument("--in", dest="in_dir", type=Path, required=True,
                    help="Input directory containing *.jsonl.gz files.")
    ap.add_argument("--out", dest="out_dir", type=Path, required=True,
                    help="Output directory for parquet files")
    ap.add_argument("--batch-size", type=int, default=DEFAULT_BATCH_SIZE,
                    help=f"Rows per parquet write_table call (default: {DEFAULT_BATCH_SIZE})")
    ap.add_argument("--shard-size-mb", type=int, default=DEFAULT_SHARD_SIZE_MB,
                    help=f"Target uncompressed shard size in MB (default: {DEFAULT_SHARD_SIZE_MB})")
    args = ap.parse_args()

    instances, branches = convert(args.in_dir, args.out_dir, args.batch_size, args.shard_size_mb)
    print(f"done. instances={instances}, branches={branches}", file=sys.stderr)


if __name__ == "__main__":
    main()
