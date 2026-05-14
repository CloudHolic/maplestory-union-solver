# maplestory-union-solver/POSET

**P**olyomino-set **S**tate **E**mbedding for **T**rees — branching policy
for the MapleStory Union ExactCover solver.

The solver uses cell-MRV backtracking with Luby restart. At each branch
point, candidate placements are tried in some order — POSET decides that
order. A small DeepSet + MLP scores each candidate's post-state, and the
solver tries them by descending score.

Trained on synthetic instance traces from the Rust solver. Exported to
ONNX and integrated into the solver via `tract`.

## Setup

​```bash
uv sync                    # inference: huggingface_hub + numpy + onnxruntime-gpu + safetensors + torch
uv sync --group training   # + training: pyarrow, tqdm
uv sync --group dev        # + dev: pytest, ruff
​```

## Layout

​```
src/poset/              # inference-only package (in wheel)
├── schema.py           # PostState + PreState dataclass, board / piece constants
├── transforms.py       # raw → tensor; piece-set padding shared with training
├── model.py            # POSET = DeepSet + MLP, integrates PyTorchModelHubMixin
└── infer.py            # POSETScorer (ONNX backend + torch backend)

training/               # training-only (outside wheel)
├── schema.py           # parquet row dataclasses + arrow converters
├── dataset.py          # parquet streaming → (branch, candidate) items with hash-based train/val split
├── loss.py             # graded relevance label + huber regression loss
└── train.py            # training loop with train/val split, --init-from for fine-tune

scripts/                # CLI entry points (outside wheel)
├── cli.py              # subcommand dispatcher (train / infer / export)
├── export.py           # checkpoint → ONNX + numerical verification
└── infer.py            # JSON in → scores out (used to validate Rust ↔ Python parity)

tests/                  # unit tests for transforms + loss
​```

## Hugging Face Hub

- Model: [`CloudHolic/poset`](https://huggingface.co/CloudHolic/poset)
- Dataset: [`CloudHolic/poset-traces`](https://huggingface.co/datasets/CloudHolic/poset-traces)

## Data

Download before training:

​```bash
hf download CloudHolic/poset-traces --repo-type dataset --local-dir ./data
​```

## Train

​```bash
uv run python -m scripts.cli train --data-dir ./data --epochs 3
​```

Validation split is enabled by default (`--val-ratio 0.1`). Set to `0`
to disable.

Saves the best checkpoint as a Hugging Face Hub-compatible directory:

​```
runs/best/
├── model.safetensors    # weights
├── config.json          # mixin-generated metadata
└── README.md            # mixin-generated model card
​```

## Fine-tune

Pass an existing checkpoint via `--init-from` and a smaller learning rate:

​```bash
uv run python -m scripts.cli train \
    --init-from runs/best \
    --lr 1e-4 \
    --data-dir ./additional-data
​```

`--init-from` accepts a local directory or an HF Hub `repo_id`.

## Export to ONNX

​```bash
uv run python -m scripts.cli export --weights runs/best --out runs/best.onnx
​```

Verifies numerical agreement between PyTorch and onnxruntime on a sample
input. Raises if they disagree above tolerance.

## Inference

CLI (JSON in, scores out — same input format as the Rust solver will use
in Phase 4):

​```bash
uv run python -m scripts.cli infer --weights runs/best --input sample.json
# or
uv run python -m scripts.cli infer --onnx runs/best.onnx --input sample.json
​```

`--weights` accepts a local directory or an HF Hub `repo_id`.

Library:

​```python
from poset.infer import POSETScorer

scorer = POSETScorer.from_pretrained("CloudHolic/poset")
scorer = POSETScorer.from_onnx("weights/poset.onnx")

scores = scorer.score_batch(post_states, instance_bitmaps)
​```

## License

GPL-3.0-or-later. See `COPYING`.