"""Add `-of-MMMMM` suffix to sharded parquet filenames.

Renames `<name>-NNNN.parquet` to `<name>-NNNNN-of-MMMMM.parquet` for the HF Hub convention.
"""

import argparse
import re
from pathlib import Path


PATTERN = re.compile(r"^(.+)-(\d{4})\.parquet$")


def rename_split(split_dir: Path) -> int:
    files = sorted(split_dir.glob("*.parquet"))
    matches = [(f, PATTERN.match(f.name)) for f in files]
    bad = [f for f, m in matches if m is None]
    if bad:
        raise ValueError(f"unexpected filename(s) in {split_dir}: {bad}")

    total = len(files)
    for f, m in matches:
        if m is None:
            continue

        name, idx = m.group(1), int(m.group(2))
        new_name = f"{name}-{idx:05d}-of-{total:05d}.parquet"
        f.rename(f.parent / new_name)

    return total

def main() -> None:
    ap = argparse.ArgumentParser(description="Add total-count suffix to sharded parquet names.")
    ap.add_argument("--dir", type=Path, required=True,
                    help="Output directory from jsonl_to_parquet.py.")
    args = ap.parse_args()

    for split_name in ("instances", "branches"):
        split_dir = args.dir / split_name
        if not split_dir.is_dir():
            print(f"skip: {split_dir} not found")
            continue
        count = rename_split(split_dir)
        print(f"renamed {count} file(s) in {split_dir}")


if __name__ == "__main__":
    main()
