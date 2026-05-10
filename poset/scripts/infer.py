# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""POSET inference CLI."""

import argparse
import json
import sys
from pathlib import Path

from poset.infer import POSETScorer
from poset.schema import PostState


def run_infer(args: argparse.Namespace) -> None:
    if (args.weights is None) == (args.onnx is None):
        raise ValueError("exactly one of --weights or --onnx is required")

    with Path(args.input).open() as f:
        payload = json.load(f)

    bitmaps = payload["canonical_bitmaps"]
    post_states = [
        PostState(
            empty_target_indices=s["empty_target_indices"],
            center_mark=s["center_mark"],
            counts=s["counts"]
        )
        for s in payload["post_states"]
    ]

    if args.weights is not None:
        scorer = POSETScorer.from_checkpoint(args.weights)
    else:
        scorer = POSETScorer.from_onnx(args.onnx)

    scores = scorer.score_batch(post_states, bitmaps)

    out_text = json.dumps(scores, indent=2)
    if args.out is None:
        sys.stdout.write(out_text + "\n")
    else:
        Path(args.out).parent.mkdir(parents=True, exist_ok=True)
        Path(args.out).write_text(out_text + "\n")
        print(f"wrote {len(scores)} scores -> {args.out}", file=sys.stderr)


def build_parser(*, add_help: bool = True) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="infer",
        description="Run POSET inference on a JSON input.",
        add_help=add_help
    )

    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--weights", type=str, default=None,
        help="Checkpoint directory.")
    source.add_argument("--onnx", type=str, default=None,
        help="ONNX file (.onnx).")

    parser.add_argument("--input", type=str, required=True,
        help="JSON input file (canonical_bitmaps + post_states).")
    parser.add_argument("--out", type=str, default=None,
        help="JSON output file. If omitted, scores print to stdout.")

    return parser


def main() -> None:
    args = build_parser().parse_args()
    run_infer(args)


if __name__ == "__main__":
    main()