# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Push a POSET checkpoint to the HuggingFace Hub.

Equivalent to:
    hf upload CloudHolic/poset ./runs/best
"""

import argparse
from pathlib import Path

from huggingface_hub import HfApi

from poset.checkpoint import load_checkpoint

DEFAULT_REPO_ID = "CloudHolic/poset"


def run_push(args: argparse.Namespace) -> None:
    weights_dir = Path(args.weights)
    if not weights_dir.is_dir():
        raise ValueError(f"--weights must be a local directory, got {args.weigths!r}")

    # Validate before upload
    load_checkpoint(weights_dir)

    api = HfApi(token=args.token)
    api.create_repo(repo_id=args.repo_id, repo_type="model", exist_ok=True)
    commit = api.upload_folder(
        folder_path=str(weights_dir),
        repo_id=args.repo_id,
        repo_type="model",
        commit_message=args.message
    )

    print(f"pushed {weights_dir} to {args.repo_id}")
    print(f"commit: {commit.commit_url}")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="push_model",
        description="Push a POSET checkpoint to the HuggingFace Hub.",
    )
    parser.add_argument("--weights", type=str, required=True,
                        help="Local checkpoint directory to upload.")
    parser.add_argument("--repo-id", type=str, default=DEFAULT_REPO_ID)
    parser.add_argument("--message", type=str, default="Update POSET checkpoint",
                        help="Commit message.")
    parser.add_argument("--token", type=str, default=None,
                        help="HF write token. If omitted, picked up from environment.")

    return parser


def main() -> None:
    args = build_parser().parse_args()
    run_push(args)


if __name__ == "__main__":
    main()