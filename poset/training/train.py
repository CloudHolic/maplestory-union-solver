# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""POSET training loop."""

import argparse
import math
from dataclasses import dataclass
from pathlib import Path

import torch
from torch import Tensor
from torch.optim import AdamW, Optimizer
from torch.utils.data import DataLoader
from tqdm import tqdm

from poset.model import POSET
from poset.transforms import pad_piece_set, post_state_to_tensors
from training.dataset import BranchTraceDataset, TrainingItem
from training.loss import compute_label, regression_loss

# Collation

@dataclass(slots=True, frozen=True)
class TrainBatch:
    """One mini-batch ready for the model."""

    empty_target: Tensor    # [M, 440]
    center_mark: Tensor     # [M, 1]
    pieces: Tensor          # [M, N_max, 36]
    counts: Tensor          # [M, N_max]
    piece_mask: Tensor      # [M, N_max]
    labels: Tensor          # [M]

    def to(self, device: torch.device) -> TrainBatch:
        """Move all tensors to a device. Returns a new TrainBatch."""

        return TrainBatch(
            empty_target=self.empty_target.to(device),
            center_mark=self.center_mark.to(device),
            pieces=self.pieces.to(device),
            counts=self.counts.to(device),
            piece_mask=self.piece_mask.to(device),
            labels=self.labels.to(device)
        )


def _collate(items: list[TrainingItem]) -> TrainBatch:
    """Flatten a list of TrainingItems into a TrainBatch."""

    # Per-item tensor dicts (un-padded along piece dim).
    per_item_tensors = [
        post_state_to_tensors(it.post_state, it.header.canonical_bitmaps)
        for it in items
    ]

    # Pad piece sets to common max_n with mask.
    padded = pad_piece_set(per_item_tensors)

    # Labels.
    labels = torch.tensor([compute_label(it.candidate) for it in items], dtype=torch.float32)

    return TrainBatch(
        empty_target=padded["empty_target"],
        center_mark=padded["center_mark"],
        pieces=padded["pieces"],
        counts=padded["counts"],
        piece_mask=padded["piece_mask"],
        labels=labels
    )


# Training step

def _forward_scores(model: POSET, batch: TrainBatch) -> Tensor:
    """Run the model on a batch and return scores. Shape: [M]."""

    return model(
        batch.empty_target,
        batch.center_mark,
        batch.pieces,
        batch.counts,
        batch.piece_mask
    ).squeeze(-1)


def _train_step(
    model: POSET,
    batch: TrainBatch,
    optimizer: Optimizer,
    device: torch.device
) -> float:
    model.train()
    batch = batch.to(device)
    scores = _forward_scores(model, batch)
    loss = regression_loss(scores, batch.labels)

    optimizer.zero_grad()
    loss.backward()
    optimizer.step()

    return loss.item()


def _eval_loss(model: POSET, loader: DataLoader, device: torch.device) -> float:
    """Compute mean loss over a validation loader."""

    model.eval()
    running, n_batches = 0.0, 0
    pbar = tqdm(loader, desc="  val", leavel=False)

    with torch.no_grad():
        for batch in pbar:
            batch = batch.to(device)
            scores = _forward_scores(model, batch)
            step_loss = regression_loss(scores, batch.labels).item()
            running += step_loss
            n_batches += 1
            pbar.set_postfix(avg=f"{running / n_batches:.4f}")

    return running / max(n_batches, 1)


# Entry point

def run_train(args: argparse.Namespace) -> None:
    data_dir = Path(args.data_dir)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"device: {device}")

    if args.init_from is not None:
        model = POSET.from_pretrained(args.init_from).to(device)
        print(f"initialized weights from {args.init_from}")
    else:
        model = POSET().to(device)

    optimizer = AdamW(model.parameters(), lr=args.lr)

    train_set = BranchTraceDataset(data_dir, split="train", val_ratio=args.val_ratio)
    train_loader = DataLoader(
        train_set,
        batch_size=args.batch_size,
        collate_fn=_collate,
        num_workers=0
    )

    val_loader: DataLoader | None = None
    if args.val_ratio > 0.0:
        val_set = BranchTraceDataset(data_dir, split="val", val_ratio=args.val_ratio)
        val_loader = DataLoader(
            val_set,
            batch_size=args.batch_size,
            collate_fn=_collate,
            num_workers=0
        )

    best_loss = math.inf
    for epoch in range(args.epochs):
        # Train
        running, n_batches = 0.0, 0
        pbar = tqdm(train_loader, desc=f"epoch {epoch + 1}/{args.epochs} [train]")
        for batch in pbar:
            step_loss = _train_step(model, batch, optimizer, device)
            running += step_loss
            n_batches += 1
            pbar.set_postfix(loss=f"{step_loss:.4f}", avg=f"{running / n_batches:.4f}")
        train_loss = running / max(n_batches, 1)

        # Validate
        val_loss: float | None = None
        if val_loader is not None:
            val_loss = _eval_loss(model, val_loader, device)

        # Log
        if val_loss is not None:
            print(f"epoch {epoch + 1}: train_loss={train_loss:.4f}  val_loss={val_loss:.4f}")
        else:
            print(f"epoch {epoch + 1}: train_loss={train_loss:.4f}  (no val set)")

        current = val_loss if val_loss is not None else train_loss
        if current < best_loss:
            best_loss = current
            weights_dir = out_dir / "best"
            model.save_pretrained(weights_dir)
            print(f"  saved best checkpoint → {weights_dir}/")


# Main

def build_parser(*, add_help: bool = True) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="train", description="Train POSET.", add_help=add_help)
    parser.add_argument("--data-dir", type=str, default="./data",
                        help="Directory containing instances.parquet + branches.parquet.")
    parser.add_argument("--out-dir", type=str, default="./runs")
    parser.add_argument("--epochs", type=int, default=3)
    parser.add_argument("--batch-size", type=int, default=64)
    parser.add_argument("--lr", type=float, default=1e-3)
    parser.add_argument("--init-from", type=str, default=None,
                        help="Path to a checkpoint to initialize from (for fine-tuning).")
    parser.add_argument("--val-ratio", type=float, default=0.1)

    return parser


def main() -> None:
    args = build_parser().parse_args()
    run_train(args)


if __name__ == "__main__":
    main()
