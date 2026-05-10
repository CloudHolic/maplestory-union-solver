# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""POSET model: DeepSet over piece set + MLP over combined state."""

import torch
import torch.nn as nn
from torch import Tensor

from poset.schema import BOARD_SIZE, CANONICAL_SIZE


class POSET(nn.Module):
    """DeepSet over piece set + MLP over combined state.

    Args:
        - piece_hidden: hidden dim of the piece encoder.
        - piece_out:    output dim of the piece encoder (= pool dim).
        - mlp_hidden:   hidden dim of the MLP.
    """

    def __init__(
        self,
        piece_hidden: int = 64,
        piece_out: int =64,
        mlp_hidden: int = 128
    ) -> None:
        super().__init__()

        self.piece_encoder = nn.Sequential(
            nn.Linear(CANONICAL_SIZE, piece_hidden),
            nn.ReLU(),
            nn.Linear(piece_hidden, piece_out),
            nn.ReLU()
        )

        mlp_in = BOARD_SIZE + 1 + piece_out
        self.main_mlp = nn.Sequential(
            nn.Linear(mlp_in, mlp_hidden),
            nn.ReLU(),
            nn.Linear(mlp_hidden, mlp_hidden // 2),
            nn.ReLU(),
            nn.Linear(mlp_hidden // 2, 1),
        )

    def forward(
        self,
        empty_target: Tensor,   # [B, BOARD_SIZE]
        center_mark: Tensor,    # [B, 1]
        pieces: Tensor,         # [B, N, CANONICAL_SIZE]
        counts: Tensor,         # [B, N]
        piece_mask: Tensor      # [B, N], 1=real piece, 0=padding
    ) -> Tensor:                # [B, 1]
        # Per-piece embedding (shared weights across N).
        embeddings = self.piece_encoder(pieces)         # [B, N, piece_out]

        weights = (counts * piece_mask).unsqueeze(-1)   # [B, N, 1]
        pool = (embeddings * weights).sum(dim=1)        # [B, piece_out]

        combined = torch.cat([empty_target, center_mark, pool], dim=1)
        return self.main_mlp(combined)                  # [B, 1]
