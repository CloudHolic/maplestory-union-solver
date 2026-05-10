# maplestory-union-solver/POSET

**P**olyomino-set **S**tate **E**mbedding for **T**rees — branching policy
for the MapleStory Union ExactCover solver.

The solver uses cell-MRV backtracking with Luby restart. At each branch
point, candidate placements are tried in some order — POSET decides that
order. A small DeepSet + MLP scores each candidate's post-state, and the
solver tries them by descending score.

Trained on synthetic instance traces from the Rust solver. Exported to
ONNX and integrated into the solver via `tract` (Phase 4).

## Setup

```bash
uv sync                    # inference: numpy + torch + onnxruntime-gpu + safetensors
uv sync --group training   # + training: tqdm
uv sync --group dev        # + dev: pytest, ruff
```

Python 3.14+. Build system: `hatchling` (`src/poset/` is the only
package shipped to the wheel; `training/` and `scripts/` stay outside).

## Layout

```
src/poset/              # inference-only package (in wheel)
├── schema.py           # PostState dataclass + board/piece constants
├── transforms.py       # raw → tensor; piece-set padding shared with training
├── model.py            # DeepSet (shared piece encoder) + MLP head
└── infer.py            # POSETScorer.from_checkpoint / from_onnx

training/               # training-only (outside wheel)
├── trace_schema.py     # JSONL parsing dataclasses
├── dataset.py          # gzip JSONL → BranchCandidate stream
├── loss.py             # graded relevance label + pairwise margin loss
└── train.py            # training loop; --init-from also handles fine-tuning

scripts/                # CLI entry points (outside wheel)
├── cli.py              # subcommand dispatcher
├── export.py           # checkpoint → ONNX + numerical verification
└── infer.py            # JSON in → scores out (used to validate Rust ↔ Python parity)

tests/                  # unit tests for transforms + loss
```

## Data

Training shards (`synth-NNNN.jsonl.gz`) are produced by the Rust generator
in `solver/`, not by this package. They're hosted on the Hugging Face Hub.

Download before training:

```bash
huggingface-cli download <user>/<dataset> --repo-type dataset --local-dir ./data
```

## Train

```bash
uv run python -m scripts.cli train --data-dir ./data --epochs 3
```

Saves the best checkpoint as a Hugging Face Hub-style directory:

```
runs/best/
├── model.safetensors    # weights
└── config.json          # architecture hyperparameters
```

## Fine-tune

There is no separate `finetune` command — pass an existing checkpoint via
`--init-from` and a smaller learning rate:

```bash
uv run python -m scripts.cli train \
    --init-from runs/best \
    --lr 1e-4 \
    --data-dir ./additional-data
```

## Export to ONNX

```bash
uv run python -m scripts.cli export --ckpt runs/best --out runs/best.onnx
```

Verifies numerical agreement between PyTorch and onnxruntime on a sample
input. Raises if they disagree above tolerance.

## Inference

CLI (JSON in, scores out — same input format as the Rust solver will use
in Phase 4):

```bash
uv run python -m scripts.cli infer --ckpt runs/best --input sample.json
# or
uv run python -m scripts.cli infer --onnx runs/best.onnx --input sample.json
```

Library:

```python
from poset.infer import POSETScorer

scorer = POSETScorer.from_checkpoint("runs/best")  # or .from_onnx("runs/best.onnx")
scores = scorer.score_batch(post_states, instance_bitmaps)
```

Both backends use CUDA if available, falling back to CPU.

## License

GPL-3.0-or-later. See `COPYING`.
