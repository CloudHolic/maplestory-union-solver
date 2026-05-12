# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""POSET training loop."""

import argparse
import json
import math
from dataclasses import dataclass
from pathlib import Path

import torch
from safetensors.torch import load_file, save_file
from torch import Tensor
from torch.optim import AdamW, Optimizer
from torch.utils.data import DataLoader
from tqdm import tqdm

from poset.model import POSET
from poset.transforms import pad_piece_set, post_state_to_tensors
from training.dataset import BranchTraceDataset, TrainingItem
from training.loss import compute_label, pairwise_margin_loss

# Collation

@dataclass(slots=True, frozen=True)
class TrainBatch:
    """One mini-batch ready for the model."""

    empty_target: Tensor    # [M, 440]
    center_mark: Tensor     # [M, 1]
    pieces: Tensor          # [M, N_max, 36]
    counts: Tensor          # [M, N_max]
    piece_mark: Tensor      # [M, N_max]
    labels: Tensor          # [M]
    group_ids: Tensor       # [M]


def _collate(items: list[TrainingItem]) -> TrainBatch:
    """Flatten a list of TrainingItems into a TrainBatch."""

    # Per-item tensor dicts (un-padded along piece dim).
    per_item_tensors = [
        post_state_to_tensors(it.post_state, it.header.canonical_bitmaps)
        for it in items
    ]

    # Pad piece sets to common max_n with mask.
    padded = pad_piece_set(per_item_tensors)

    # Group ids: 1 integer per (instance_id, branch_id) seen in this batch.
    branch_keys = [(it.header.instance_id, it.branch.branch_id) for it in items]
    unique_keys = {k: i for i, k in enumerate(dict.fromkeys(branch_keys))}
    group_ids = torch.tensor([unique_keys[k] for k in branch_keys], dtype=torch.long)

    # Labels.
    labels = torch.tensor([compute_label(it.candidate) for it in items], dtype=torch.float32)

    return TrainBatch(
        empty_target=padded["empty_target"],
        center_mark=padded["center_mark"],
        pieces=padded["pieces"],
        counts=padded["counts"],
        piece_mark=padded["piece_mark"],
        labels=labels,
        group_ids=group_ids
    )


# Training step

def _train_step(
    model: POSET,
    batch: TrainBatch,
    optimizer: Optimizer,
    device: torch.device,
    margin: float
) -> float:
    model.train()
    batch_on_device = TrainBatch(
        empty_target=batch.empty_target.to(device),
        center_mark=batch.center_mark.to(device),
        pieces=batch.pieces.to(device),
        counts=batch.counts.to(device),
        piece_mark=batch.piece_mark.to(device),
        labels=batch.labels.to(device),
        group_ids=batch.group_ids.to(device),
    )

    scores = model(
        batch_on_device.empty_target,
        batch_on_device.center_mark,
        batch_on_device.pieces,
        batch_on_device.counts,
        batch_on_device.piece_mark
    ).squeeze(-1)

    loss = pairwise_margin_loss(
        scores,
        batch_on_device.labels,
        batch_on_device.group_ids,
        margin=margin
    )

    optimizer.zero_grad()
    loss.backward()
    optimizer.step()

    return loss.item()


# Entry point

def run_train(args: argparse.Namespace) -> None:
    data_dir = Path(args.data_dir)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    shard_paths = sorted(data_dir.glob("synth-*.jsonl.gz"))
    if not shard_paths:
        raise FileNotFoundError(
            f"no shards matching synth-*.jsonl.gz in {data_dir}"
        )
    print(f"found {len(shard_paths)} shard(s) under {data_dir}")

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"device: {device}")

    model = POSET(
        piece_hidden=args.piece_hidden,
        piece_out=args.piece_out,
        mlp_hidden=args.mlp_hidden,
    ).to(device)

    if args.init_from is not None:
        init_dir = Path(args.init_from)
        loaded_config = json.loads((init_dir / "config.json").read_text())
        for k in ("piece_hidden", "piece_out", "mlp_hidden"):
            if loaded_config[k] != getattr(args, k):
                raise ValueError(
                    f"init-from hparam {k}={loaded_config[k]} mismatches "
                    f"args.{k}={getattr(args, k)}"
                )

        state = load_file(str(init_dir / "model.safetensors"))
        model.load_state_dict(state)
        print(f"initialized weights from {init_dir}/")

    optimizer = AdamW(model.parameters(), lr=args.lr)

    dataset = BranchTraceDataset(shard_paths)
    loader = DataLoader(
        dataset,
        batch_size=args.batch_size,
        collate_fn=_collate,
        num_workers=0,

    )

    best_loss = math.inf
    for epoch in range(args.epochs):
        running, n_batches = 0.0, 0
        for batch in tqdm(loader, desc=f"epoch {epoch + 1}/{args.epochs}"):
            running += _train_step(model, batch, optimizer, device, args.margin)
            n_batches += 1

        mean_loss = running / max(n_batches, 1)
        print(f"epoch {epoch + 1}: mean loss = {mean_loss:.4f}")

        if mean_loss < best_loss:
            best_loss = mean_loss
            weights_dir = out_dir / "best"
            weights_dir.mkdir(exist_ok=True)
            save_file(model.state_dict(), str(weights_dir / "model.safetensors"))
            (weights_dir / "config.json").write_text(
                json.dumps(
                    {
                        "piece_hidden": args.piece_hidden,
                        "piece_out": args.piece_out,
                        "mlp_hidden": args.mlp_hidden
                    },
                    indent=2
                )
            )
            print(f"  saved best checkpoint → {weights_dir}/")


# Main

def build_parser(*, add_help: bool = True) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="train", description="Train POSET.", add_help=add_help)
    parser.add_argument("--data-dir", type=str, default="./data")
    parser.add_argument("--out-dir", type=str, default="./runs")
    parser.add_argument("--epochs", type=int, default=3)
    parser.add_argument("--batch-size", type=int, default=64)
    parser.add_argument("--lr", type=float, default=1e-3)
    parser.add_argument("--margin", type=float, default=1.0)
    parser.add_argument("--init-from", type=str, default=None,
        help="Path to a checkpoint to initialize from (for fine-tuning).")
    parser.add_argument("--piece_hidden", type=int, default=64)
    parser.add_argument("--piece_out", type=int, default=64)
    parser.add_argument("--mlp_hidden", type=int, default=128)

    return parser


def main() -> None:
    args = build_parser().parse_args()
    run_train(args)


if __name__ == "__main__":
    main()
