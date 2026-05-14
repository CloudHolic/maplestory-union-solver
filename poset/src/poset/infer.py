# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Inference entry point for the POSET branching policy.

Loads a trained model - either as a PyTorch checkpoint (`.pt`) or as an ONNX file (`.onnx`).
"""

from pathlib import Path
from typing import Protocol

import numpy as np
import onnxruntime as ort
import torch

from poset.model import POSET
from poset.schema import PostState
from poset.transforms import pad_piece_set, post_state_to_tensors


class _ScoringBackend(Protocol):
    """Backend that scores a single post_state given pre-built tensors."""

    def __call__(
        self,
        empty_target: torch.Tensor,
        center_mark: torch.Tensor,
        pieces: torch.Tensor,
        counts: torch.Tensor,
        piece_mask: torch.Tensor,
    ) -> list[float]: ...


class POSETScorer:
    """Scores a (post_state, instance_bitmaps) pair via a loaded model."""

    def __init__(self, backend: _ScoringBackend) -> None:
        self._backend = backend


    # Constructors

    @classmethod
    def from_pretrained(
        cls,
        name_or_path: str | Path,
        revision: str | None = None,
        token: str | None = None
    ) -> POSETScorer:
        """Load from a local directory or download from the HuggingFace Hub."""

        device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        model = POSET.from_pretrained(name_or_path, revision=revision, token=token)
        model = model.to(device)
        model.eval()

        def backend(empty_target, center_mark, pieces, counts, piece_mask):
            with torch.no_grad():
                out = model(
                    empty_target.to(device),
                    center_mark.to(device),
                    pieces.to(device),
                    counts.to(device),
                    piece_mask.to(device)
                )

            return out.squeeze(-1).cpu().tolist()

        return cls(backend)


    @classmethod
    def from_onnx(cls, path: str | Path) -> POSETScorer:
        """Load an ONNX file via onnxruntime."""

        providers = ["CUDAExecutionProvider", "CPUExecutionProvider"]
        session = ort.InferenceSession(str(path), providers=providers)

        def backend(empty_target, center_mark, pieces, counts, piece_mask):
            outputs = session.run(
                None,
                {
                    "empty_target": empty_target.numpy(),
                    "center_mark": center_mark.numpy(),
                    "pieces": pieces.numpy(),
                    "counts": counts.numpy(),
                    "piece_mask": piece_mask.numpy()
                }
            )

            arr = np.squeeze(outputs[0], axis=-1)
            return [float(x) for x in np.atleast_1d(arr)]

        return cls(backend)


    # Scoring

    def score(self, post_state: PostState, instance_bitmaps: list[list[int]]) -> float:
        """Score 1 post_state. Returns the model's scalar output."""

        return self.score_batch([post_state], instance_bitmaps)[0]


    def score_batch(
        self,
        post_states: list[PostState],
        instance_bitmaps: list[list[int]]
    ) -> list[float]:
        """Score multiple post_states from the same instance."""

        if not post_states:
            return []

        per_state = [post_state_to_tensors(s, instance_bitmaps) for s in post_states]
        padded = pad_piece_set(per_state)

        return self._backend(
            padded["empty_target"],
            padded["center_mark"],
            padded["pieces"],
            padded["counts"],
            padded["piece_mask"]
        )