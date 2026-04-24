# ML-guided branching: feature design

## Role of the model

At each branching point in the solver's search tree, there are multiple
candidate placements. The current order in which they are tried is
effectively random (after seed-dependent shuffling). A trained model
replaces this with an informed ordering: score each candidate, try them in
descending score.

The correctness of the solver is preserved: every candidate is eventually
tried if the subtree fails. Only the expected time-to-solution is affected.

## Model choice

**Gradient-boosted decision trees (LightGBM)**, exported to ONNX and
executed via [`tract`](https://github.com/sonos/tract) inside the WASM
solver.

Rationale:

- **Inference latency**: branching decisions happen millions of times per
  solve. Per-decision budget is in microseconds. GBDT with a few hundred
  shallow trees satisfies this; larger neural networks do not.
- **Feature regime**: the problem has tens of interpretable features per
  decision. Tree ensembles handle tabular data with heterogeneous scales
  well, without feature engineering overhead like normalization.
- **ONNX support**: LightGBM exports to ONNX cleanly, and `tract` supports
  the relevant operators.

## Feature groups

Features are grouped by what they describe. Each branching decision
produces one feature vector per candidate, which is scored independently.

### Node-level features (shared across candidates)

Properties of the current search state:

- Depth in the search tree
- Fraction of cells covered
- Number of each piece size remaining (1 to 5)
- Parity imbalance: `|alive_black_cells - alive_white_cells|`
- Number of recent prune events by type (parity / island / neighbor)

### Branch-cell features (shared across candidates)

Properties of the cell being branched on:

- Number of alive neighbors (degree in the uncovered subgraph)
- Whether the cell is a "corner" (≤2 alive neighbors)
- Cell parity color
- MRV count (total valid placements at this cell)

### Candidate-placement features (vary across candidates)

Properties specific to each candidate placement:

- Piece size
- Number of "corner" cells in the placement
- Number of dead cells predicted to result (one-step lookahead)
- Frontier expansion contribution (new adjacent cells exposed)
- Whether this placement lands a mark on the center region
- Parity contribution: black cells in this placement
- Distance from the center region

## Training signal

The training dataset comes from the solver itself in a logging mode. Each
run produces a JSONL file where each line is a branching decision:

```json
{
  "state_features": {...},
  "branch_cell_features": {...},
  "candidates": [
    {"placement_id": 42, "candidate_features": {...}, "chosen": true,  "led_to_solution": true},
    {"placement_id": 17, "candidate_features": {...}, "chosen": false, "led_to_solution": false},
    ...
  ]
}
```

For a successful solve, the `chosen` path is the solution path. For
failed branches, `led_to_solution` is false for all candidates.

The learning task is a **pairwise ranking** problem: given two candidates
at the same decision point, predict which is more likely to lead to a
solution. LightGBM's `rank_xendcg` objective is suited for this.

## Training data sources

The primary source is instrumented solver runs on a benchmark suite of
hard instances. Longer-term, opt-in telemetry from real user solves could
supplement this, though the current deployment does not include
telemetry.

## Self-improvement loop

A secondary consideration: as the ML-guided solver becomes more capable,
it can solve harder instances that previously timed out. These instances
produce additional training data that in turn improves the model.

This loop is bounded by the size of the benchmark suite and the
willingness to invest compute in data generation.

## Evaluation

Primary metric: **median and p95 runtime** on the benchmark suite,
comparing the model-guided solver to the baseline.

Secondary metrics:

- Pairwise ranking AUC on held-out branching decisions
- Node count reduction at fixed success rate
- Inference latency (wall clock microseconds per call, measured inside
  WASM)

The last metric is a constraint: if inference latency exceeds the per-node
work budget, the model is too expensive regardless of accuracy.
