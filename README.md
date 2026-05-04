# maplestory-union-solver

A browser-based solver for the MapleStory Union placement puzzle.

[한국어 README](README.ko.md)

## What is this?

**MapleStory Union** is a meta-progression system in the MMORPG *MapleStory*.
Each character a player owns is represented as a polyomino-shaped block
(1 to 5 cells, depending on level), and these blocks must be arranged on a
shared board to grant stat bonuses. The placement problem has real
constraints:

- Up to 42 character blocks must fit on the board
- Each block has a designated "mark" cell, and at least one mark must land
  on the central 4 cells of the board
- Named regions on the board (groups) have individual cell-count targets
- All placed blocks must form a single connected region

Finding a valid (let alone optimal) placement by hand is tedious. This
project provides a solver that runs entirely in the browser and returns a
placement satisfying all constraints.

## Tech stack

| Component | Technology |
|---|---|
| Frontend | React 19 + Vite + TypeScript |
| Solver core | Rust, compiled to WebAssembly |
| ML inference (in WASM) | [`tract`](https://github.com/sonos/tract) (pure Rust ONNX runtime) |
| ML training (dev-time only) | Python + LightGBM |
| Deployment | Docker + nginx, self-hosted via Cloudflare Tunnel |

All computation happens client-side. No server-side calculation is involved.

## Repository layout

```
ui/       React frontend
wasm/     Rust solver (WASM + native targets)
ml/       Python ML training pipeline (dev-time only)
models/   Trained ONNX models
docs/     Architecture and algorithm documentation
```

Each subdirectory has its own `README.md` with build instructions.

## Building

See individual subproject READMEs. A top-level build script is provided:

```bash
./scripts/build-all.sh
```

which produces a static bundle in `ui/dist/`.

## Documentation

- [Architecture](docs/architecture.md)
- [ExactCover algorithm](docs/algorithms/exact-cover.md)
- [ML feature design](docs/ml/features.md)

## License

This repository uses multiple licenses. See [`LICENSE-POLICY.md`](LICENSE-POLICY.md)
for the mapping between directories and applicable licenses.

- Solver core (`wasm/`): **AGPL-3.0-or-later**
- ML pipeline (`ml/`): **GPL-3.0-or-later**
- Frontend (`ui/`): **MIT**
- Server (`server/`): **MIT**
- Trained models (`models/`): **CC-BY-4.0**
- Documentation (`docs/`): **CC-BY-4.0**
