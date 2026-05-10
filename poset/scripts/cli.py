# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""POSET command-line interface."""

import argparse

from scripts.export import build_parser as build_export_parser
from scripts.export import run_export
from scripts.infer import build_parser as build_infer_parser
from scripts.infer import run_infer
from training.train import build_parser as build_train_parser
from training.train import run_train


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="poset",
        description="POSET: branching policy for the MapleStory Union solver."
    )
    sub = parser.add_subparsers(dest="command", required=True)

    # train
    train_parent = build_train_parser(add_help=False)
    p_train = sub.add_parser("train", parents=[train_parent],
        help="Train POSET from scratch (or from --init-from for fine-tuning).",
        description=train_parent.description
    )
    p_train.set_defaults(func=run_train)

    # infer
    infer_parent = build_infer_parser(add_help=False)
    p_infer = sub.add_parser("infer", parents=[infer_parent],
        help="Run inference on a JSON input file.",
        description=infer_parent.description
    )
    p_infer.set_defaults(func=run_infer)

    # export
    export_parent = build_export_parser(add_help=False)
    p_export = sub.add_parser("export", parents=[export_parent],
        help="Export a POSET checkpoint to ONNX.",
        description=export_parent.description
    )
    p_export.set_defaults(func=run_export)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
