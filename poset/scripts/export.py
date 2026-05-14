# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Export a POSET checkpoint to ONNX."""

import argparse
from pathlib import Path

import numpy as np
import onnxruntime as ort
import torch

from poset.model import POSET
from poset.schema import BOARD_SIZE, CANONICAL_SIZE

# Sample input dimensions used for both export tracing and verification.
_SAMPLE_BATCH = 2
_SAMPLE_N = 5


def run_export(args: argparse.Namespace) -> None:
    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    # Load model on CPU - export traces don't need GPU.
    model = POSET.from_pretrained(args.weights, map_location="cpu")
    model.eval()
    print(f"loaded checkpoint: {args.weights}")

    sample = _make_sample_input(_SAMPLE_BATCH, _SAMPLE_N)

    torch.onnx.export(
        model,
        sample,
        str(out_path),
        input_names=["empty_target", "center_mark","pieces", "counts", "piece_mask"],
        output_names=["score"],
        dynamic_axes={
            "empty_target": {0: "batch"},
            "center_mark": {0: "batch"},
            "pieces": {0: "batch", 1: "N"},
            "counts": {0: "batch", 1: "N"},
            "piece_mask": {0: "batch", 1: "N"},
            "score": {0: "batch"}
        },
        opset_version=args.opset
    )
    print("exported ONNX:", out_path)

    _verify(model, out_path, sample, rtol=args.rtol, atol=args.atol)


def _make_sample_input(batch: int, n: int) -> tuple[torch.Tensor, ...]:
    """Build a sample input tuple matching POSET.forward's positional order."""

    g = torch.Generator().manual_seed(0)

    # empty_target, center_mark, pieces, counts, piece_mask
    return (
        torch.rand(batch, BOARD_SIZE, generator=g),
        torch.randint(0, 2, (batch, 1), generator=g).float(),
        torch.randint(0, 2, (batch, n, CANONICAL_SIZE), generator=g).float(),
        torch.randint(0, 5, (batch, n), generator=g).float(),
        torch.ones(batch, n)
    )


def _verify(
    model: POSET,
    onnx_path: Path,
    sample: tuple[torch.Tensor, ...],
    *,
    rtol: float,
    atol: float
) -> None:
    """Run both backends on the sample and assert numerical agreement."""

    # PyTorch reference.
    model.eval()
    with torch.no_grad():
        torch_out = model(*sample).cpu().numpy()

    # ONNX runtime - CPU provider for determinism in verification.
    session = ort.InferenceSession(str(onnx_path), providers=["CPUExecutionProvider"])
    onnx_out = np.asarray(session.run(
        None,
        {
            "empty_target": sample[0].numpy(),
            "center_mark": sample[1].numpy(),
            "pieces": sample[2].numpy(),
            "counts": sample[3].numpy(),
            "piece_mask": sample[4].numpy()
        }
    )[0])

    max_abs_diff = float(np.abs(torch_out - onnx_out).max())
    max_rel_diff = float((np.abs(torch_out - onnx_out) / (np.abs(torch_out) + 1e-9)).max())
    print(f"verification: max_abs_diff={max_abs_diff:.2e}, max_rel_diff={max_rel_diff:.2e}")

    if not np.allclose(torch_out, onnx_out, rtol=rtol, atol=atol):
        raise RuntimeError(
            f"ONNX export verification failed: PyTorch and onnxruntime disagree "
            f"by max_abs={max_abs_diff:.2e} (atol={atol}), "
            f"max_rel={max_rel_diff:.2e} (rtol={rtol}). "
            f"ONNX file at {onnx_path} is suspect."
        )

    print("verification: PASS")


def build_parser(*, add_help: bool = True) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="export",
        description="Export a POSET checkpoint to ONNX.",
        add_help=add_help
    )

    parser.add_argument("--weights", type=str, required=True,
        help="Local checkpoint directory or HF Hub repo_id.")
    parser.add_argument("--out", type=str, required=True,
        help="Path to the output ONNX file (.onnx).")
    parser.add_argument("--opset", type=int, default=18,
        help="ONNX opset version. tract supports up to 18 reliably.")
    parser.add_argument("--rtol", type=float, default=1e-4,
        help="Relative tolerance for verification.")
    parser.add_argument("--atol", type=float, default=1e-5,
        help="Absolute tolerance for verification.")

    return parser


def main() -> None:
    args = build_parser().parse_args()
    run_export(args)


if __name__ == "__main__":
    main()
