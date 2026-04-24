# Architecture

## Execution model

All computation runs in the user's browser. The server only serves static
files.

```
Browser
├── React UI                       (user-facing, state management)
├── Web Workers (portfolio)        (parallel solver instances)
│   └── WASM solver module         (Rust, compiled to WebAssembly)
│       ├── Backtracking algorithm
│       └── ONNX inference engine  (tract, embedded model)
```

The user submits a board state and piece configuration; the UI spawns N Web
Workers (N ≈ `hardwareConcurrency - 1`), each running an independent instance
of the WASM solver with a different random seed. The first worker to find a
valid placement wins; the others are terminated.

## Why client-side

- **Cost**: solving a hard instance takes 30 seconds to several minutes of
  CPU time. Running this on a shared server does not scale economically for
  a small project.
- **Privacy**: user input never leaves the device.
- **Deployment simplicity**: a static bundle is served by any web server or
  CDN.
- **Offline capable**: with PWA caching, the tool can run offline.

The tradeoff is that the user's CPU must be capable. The target audience
uses desktop machines while playing the game, which is a reasonable
assumption.

## Language boundaries

### `wasm/` — Solver algorithm, single source of truth

A single Rust crate builds for two targets:

- **WebAssembly** via `wasm-pack build --target web` for browser use.
- **Native** via `cargo build --bin benchmark` for local benchmarking and
  training data generation.

The algorithm modules are pure Rust with no WebAssembly-specific
dependencies. Only the top-level `lib.rs` contains `wasm-bindgen` glue. This
isolation keeps the algorithm testable outside the browser and allows the
same code to be repurposed (e.g., for a potential server-side deployment
later).

### `ml/` — Training pipeline, development-time only

Python scripts that consume JSONL search logs produced by the solver and
produce ONNX models. This pipeline is **not part of the deployed
application**. Only its output (ONNX files under `models/`) ships with the
release.

### `ui/` — Presentation layer

React application. Imports the compiled WASM module and invokes it from Web
Workers. Contains no algorithm logic of its own.

## The role of machine learning

ML does not replace the algorithm. It guides one specific decision inside
the algorithm: **the order in which candidate placements are tried at each
branch point**.

A trained GBDT model scores each candidate, and the solver tries them in
descending score order. The correctness of the solver is unaffected by the
model's accuracy: every candidate is eventually tried if needed. Only the
time to reach a valid placement is affected.

This design choice is driven by inference latency constraints. Branching
decisions occur millions of times per solve. A model must return a score in
microseconds, which rules out neural networks of meaningful size. GBDT
models satisfy this constraint and export cleanly to ONNX.

Inference runs inside the WASM module via the `tract` crate, avoiding the
per-call JavaScript boundary crossing that would accumulate prohibitively
across millions of invocations.

## Data contracts

The three languages share data formats defined as JSON Schema in
[`docs/schemas/`](schemas/). These are the canonical definitions; language-
specific types are generated or hand-written to match.

| Schema | Producer | Consumer |
|---|---|---|
| `solver-input.json` | `ui/` | `wasm/` |
| `solution.json` | `wasm/` | `ui/` |
| `solve-log.json` | `wasm/` (logging mode) | `ml/` |
| `features.json` | `ml/` (training) and `wasm/` (inference) | (shared) |

Changes to these schemas require coordinated updates across all three
languages.

## Deployment

The production build is a static bundle served by nginx inside a Docker
container, exposed via Cloudflare Tunnel. See the deployment notes in the
repository for specifics.
