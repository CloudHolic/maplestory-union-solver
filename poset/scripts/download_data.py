# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Download the POSET training dataset from the HuggingFace Hub.

Equivalent to:
    hf download CloudHolic/poset-traces --repo-type dataset --local-dir ./data
"""

import argparse
from pathlib import Path

from huggingface_hub import snapshot_download

DEFAULT_REPO_ID = "CloudHolic/poset-traces"
DEFAULT_LOCAL_DIR = "./data"


def run_download(args: argparse.Namespace) -> None:
    local_dir = Path(args.local_dir)
    local_dir.mkdir(parents=True, exist_ok=True)

    path = snapshot_download(
        repo_id=args.repo_id,
        repo_type="dataset",
        local_dir=str(local_dir),
        revision=args.revision
    )
    print(f"download {args.repo_id} to {path}")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="download_data",
        description="Download the POSET training dataset from the HuggingFace Hub."
    )
    parser.add_argument("--repo-id", type=str, default=DEFAULT_REPO_ID)
    parser.add_argument("--local-dir", type=str, default=DEFAULT_LOCAL_DIR)
    parser.add_argument("--revision", type=str, default=None,
                        help="Optional git revision (branch / tag / commit hash).")

    return parser


def main() -> None:
    args = build_parser().parse_args()
    run_download(args)


if __name__ == "__main__":
    main()