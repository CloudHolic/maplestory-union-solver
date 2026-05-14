# Architecture

## Execution model

The user-facing solve runs entirely in the browser. The server only
proxies auxiliary read-only data (NEXON character lookups) and serves
static assets.

```
Browser
├── Frontend (React + Vite)                  frontend/
│   ├── UI + state management
│   └── Web Workers (portfolio race)
│       └── WASM solver module               frontend/wasm-pkg/ ← built from solver/
│           ├── Cell-MRV backtracking + Luby restart
│           └── (Phase 4) POSET inference via tract

Backend (Go thin proxy)                      backend/
└── NEXON Open API proxy + SQLite cache for character data

(Dev-time, not in deployed bundle)
├── Rust solver source                       solver/
├── POSET training (PyTorch)                 poset/
└── JSONL → Parquet ETL                      etl/
```

The user submits a board state + piece configuration. The frontend spawns
N Web Workers (N ≈ `hardwareConcurrency - 1`), each running an independent
WASM solver instance with a different random seed. The first worker to
find a valid placement wins; the others are terminated.

The Go backend (`backend/`) is a read-only proxy: it resolves MapleStory
nicknames to character data via the NEXON Open API and caches results in
SQLite so the frontend can populate the piece configuration. It does *not*
participate in solving.

## Why client-side

- **Cost**: a hard instance can take seconds to minutes of CPU time.
  Running per-user solves on a shared server does not scale economically
  for a small project.
- **Privacy**: user board state and solve attempts never leave the device.
- **Deployment simplicity**: a static bundle is served by any web server
  or CDN.
- **Offline capable**: with PWA caching, the solver itself runs offline
  (character lookup obviously requires the backend).

The trade-off is that the user's CPU must be capable. The target audience
plays the game on desktop, which is a reasonable assumption.

## Module boundaries

### `solver/` — Single source of truth for solver code

Rust workspace builds for two targets from one codebase:

- **WebAssembly** via `wasm-pack build` for browser deployment (artifact
  lives at `frontend/wasm-pkg/`).
- **Native** binaries (`benchmark`, `generate`) for local benchmarking
  and ML training data generation.

Algorithm modules are pure Rust with no WebAssembly-specific
dependencies. Only the top-level wasm-bindgen entry contains glue. This
isolation keeps the algorithm testable outside the browser and allows the
same code to be repurposed (e.g., native CLI tools or potential
server-side deployment later).

License: AGPL-3.0-or-later.

### `frontend/` — React presentation layer

React + Vite. Imports the compiled WASM module from `frontend/wasm-pkg/`
and invokes it from Web Workers. Contains no algorithm logic of its own —
only UI, state management, and worker orchestration.

License: MIT.

### `backend/` — Go thin proxy

Read-only HTTP service backed by Echo + SQLite (WAL). Caches NEXON Open
API nickname → character data lookups. Per-IP rate limited. Pure-Go
SQLite driver (`modernc.org/sqlite`), so the binary runs on `scratch`
Docker base. Telemetry (recording opt-in solve runs) is planned but not
yet implemented.

License: MIT.

### `poset/` — POSET training & inference (dev-time)

Python package implementing POSET (DeepSet shared piece encoder + MLP
head for branching policy). Produces Hugging Face-style checkpoints
(`model.safetensors` + `config.json`) and ONNX exports. Used
**development-time only**; only its ONNX output ships with the release
(consumed by `solver/` via `tract` in Phase 4).

License: GPL-3.0-or-later.

### `etl/` — JSONL → Parquet → HF Hub (dev-time)

Python scripts converting the Rust solver's trace output (JSONL gz) to
Parquet shards (`instances.parquet` + `branches.parquet`) and pushing to
Hugging Face Hub. Decoupled from `poset/` so the training package stays
focused on inference + training and stays free of ETL dependencies.

License: MIT.

## The role of machine learning

ML does not replace the solver algorithm. It guides one specific decision
inside the algorithm: **the order in which candidate placements are tried
at each branch point**.

A trained POSET model (DeepSet shared piece encoder + MLP head) scores
each candidate's post-state. The solver tries them in descending score
order. The correctness of the solver is unaffected by the model's
accuracy: every candidate is eventually tried if its subtree fails. Only
the expected time-to-solution improves.

Inference runs inside the WASM module via the `tract` crate, avoiding
per-call JavaScript boundary crossings that would accumulate prohibitively
across millions of invocations.

For full POSET model design, training signal, and pipeline, see
[`docs/poset/features.md`](poset/features.md).

## Deployment (Planned)

The production build will be a static bundle (frontend + WASM) served by
nginx inside a Docker container, with the Go backend reverse-proxied
alongside. Exposed via Cloudflare Tunnel.

Not yet implemented.