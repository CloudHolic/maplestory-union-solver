# ML-guided branching: POSET

## Role of the model

At each branching point in the solver's search tree, multiple candidate
placements are available. The baseline order in which they are tried is
heuristic-driven (cell-MRV + Luby restart). A trained model replaces this
with a learned ordering: score each candidate's post-state, try them in
descending score.

The correctness of the solver is preserved: every candidate is eventually
tried if the subtree fails. Only the expected time-to-solution is affected.

## Model choice — POSET

**POSET** (Polyomino-set State Embedding for Trees) — a small DeepSet
shared piece encoder + MLP head. Trained in PyTorch, exported to ONNX,
executed via [`tract`](https://github.com/sonos/tract) inside the WASM
solver (Phase 4).

### Why DeepSet + MLP

- **The piece-set is naturally a set**: a state's piece-pool has no
  meaningful order. DeepSet respects permutation invariance without
  explicit feature engineering.
- **Hand-crafted features were brittle**: every new heuristic insight
  required re-engineering. Learned embeddings are end-to-end optimized
  for the ranking objective.
- **Inference latency is acceptable**: a small DeepSet (32-64 piece
  encoder dim, 2-3 MLP layers) runs in microseconds per call via
  `tract`, within the per-node work budget.

## Input — post-state representation

Each candidate produces a *post-state*: the board state after applying
that candidate. POSET scores each post-state independently and the solver
tries candidates by descending score.

The post-state is represented as:

- `empty_bitmap` — 28-byte bit-packed bitmap. Bit `i` set means board cell
  `i` is still empty. Board has 215 cells; 28 bytes = 224 bits with safety
  margin.
- `center_mark` — bool. Whether any applied placement has marked the
  center 4-cell region (constraint: the final solution must mark the
  center).
- `pieces` — a *set* of remaining piece types, each described by:
  - `canonical_bitmap` — 36-byte byte-per-cell 6×6 canonical bitmap of
    the piece shape (lex-min normalized variant)
  - `count` — how many instances of this piece type remain in the pool

Piece types with `count = 0` are filtered out before model input (no
contribution to the embedding).

Instance-level fixed data (full piece pool, board target cells, etc.) is
shared across all branches of one solve. Only the post-state varies
candidate-to-candidate.

## Training signal — graded relevance

Each branch row in the JSONL trace (one row per branching decision)
contains candidates with `tried`, `succeeded`, `subtree_nodes` fields.
Labels are graded:

- `tried = false` (neighbor-check rejected) → excluded from training
- `succeeded = true` → label `3.0` (max — this candidate led to solution)
- `succeeded = false` → label `2.0 / (1 + log(subtree_nodes))` (decays
  with how much subtree work was wasted before failing)

The graded scale rewards both *correct* choices (success) and *fast
failures* (small subtree). Losing the same way faster is still better
than losing the same way slower.

The loss is **pairwise margin ranking**: within one branch's candidate
list, the model should rank higher-label candidates above lower-label
ones, with a margin proportional to the label gap.

## Training data

Source: synthetic instances from the Rust generator
(`solver/src/bin/generate`).

Generator distribution (Stage 4 lock, 2026-05-13):
- Piece count `U[35, 43]` per instance
- size-5 vs size-4 ratio `Beta(7, 2)` → ~3.5:1 mean (matches real player
  observations)
- Same-size shape draws: symmetric Dirichlet-multinomial, with size-5 α
  sampled `log-uniform[0.5, 1.5]` per instance (models varying player
  shape concentration)


Real PoC instances validated anecdotal distribution fit: 3 cases at
2.8s, 5.7s, 9.3s wall-clock on c7g.4xlarge 15-worker — landing inside
the synthetic luby-cluster and middle-tail regions.

## Pipeline

```
Rust solver → JSONL gz trace shards → Python ETL (parquet) → HF Hub
                                                                  ↓
                                                           PyTorch training
                                                                  ↓
                                                           model.safetensors
                                                                  ↓
                                                           ONNX export
                                                                  ↓
                                                           tract / WASM
```

- **Trace shards**: per-instance JSONL gz from `solver/src/ml/tracer.rs`.
  Schema is b2-variant: per-instance header + per-branch rows with
  `pre_state` + outcome-only candidates. Post-state reconstruction at the
  reader side.
- **ETL**: `etl/jsonl_to_parquet.py` converts JSONL gz shards to
  `instances.parquet` + `branches.parquet` with FK = instance_id.
- **Training**: PyTorch (`poset/training/`), saved as Hugging Face-style
  checkpoint directory (`model.safetensors` + `config.json`).
- **Export**: ONNX with numerical agreement verification against PyTorch
  output.
- **Deployment**: `tract` runtime inside the WASM solver (Phase 4).

## Evaluation

Primary metric: **median and p95 wall-clock** on a held-out benchmark
suite, comparing the model-guided solver to the heuristic baseline.

Secondary metrics:
- **Pairwise ranking AUC** on held-out branching decisions
- **Node count reduction** at fixed success rate
- **Inference latency** (wall-clock microseconds per call, measured
  inside WASM)

The last is a constraint: if inference latency exceeds the per-node work
budget, the model is too expensive regardless of accuracy.

## Self-improvement loop

As POSET becomes more capable, the solver solves harder instances that
previously timed out. These additional traces feed back into the training
set, improving the model further. The loop is bounded by:

- Synthetic generator's hardness ceiling: a portion of synthetic
  instances are fundamentally over-constrained — no random seed solves
  them within reasonable time. Empirical distribution of solver runtime
  on synthetic batches splits into three phases: trivial (cascade-only,
  sub-second), luby-cluster (random-restart hits a solution within a
  few seconds), and hopeless.
- Compute willingness for new data generation rounds