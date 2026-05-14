# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""POSET model: DeepSet over piece set + MLP over combined state."""

import torch
import torch.nn as nn
from torch import Tensor

from poset.schema import BOARD_SIZE, CANONICAL_SIZE


_PIECE_HIDDEN = 64
_PIECE_OUT = 64
_MLP_HIDDEN = 128


class POSET(nn.Module):
    """DeepSet over piece set + MLP over combined state."""

    def __init__(self) -> None:
        super().__init__()

        self.piece_encoder = nn.Sequential(
            nn.Linear(CANONICAL_SIZE, _PIECE_HIDDEN),
            nn.ReLU(),
            nn.Linear(_PIECE_HIDDEN, _PIECE_OUT),
            nn.ReLU()
        )

        mlp_in = BOARD_SIZE + 1 + _PIECE_OUT
        self.main_mlp = nn.Sequential(
            nn.Linear(mlp_in, _MLP_HIDDEN),
            nn.ReLU(),
            nn.Linear(_MLP_HIDDEN, _MLP_HIDDEN // 2),
            nn.ReLU(),
            nn.Linear(_MLP_HIDDEN // 2, 1),
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
