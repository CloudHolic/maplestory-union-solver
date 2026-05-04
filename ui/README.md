# maplestory-union-solver/ui

React frontend for the MapleStory Union placement solver.

## Stack

| | |
|---|---|
| Framework | React 19 + Vite + TypeScript |
| UI components | HeroUI v3 + Tailwind v4 |
| State | Zustand (UI state) + Effect-TS (services) |
| Formatting | @stylistic/eslint-plugin |

## Prerequisites

- Node.js 22+
- pnpm

## Quick start

```sh
cd ui
pnpm install
pnpm build:wasm	# build WASM solver	
pnpm dev		# dev server at localhost:5173
```

The dev server proxies `/api` to `localhost:8888` (Go server).
Start the server separately — see `../server/README.md`.

## WASM solver

The solver runs in Web Workers as a compiled WebAssembly module.
The pre-built artifact is checked in at `wasm-pkg/`. To rebuild
from Rust source:

```sh
pnpm build:wasm   # runs wasm-pack, outputs to wasm-pkg/
```

Requires `wasm-pack` and the Rust toolchain — see
`../wasm/README.md`.

## Build

```sh
pnpm build        # type-check + Vite build → dist/
pnpm preview      # preview the production build locally
```

## Lint

```sh
pnpm lint         # eslint (includes @stylistic formatting check)
```

## Tests

```sh
pnpm test         # vitest (unit tests for domain/, utils/)
```

## Project layout

```
src/
├── components/
│   ├── board/        SVG board grid, overlays, controls
│   ├── characters/   Nickname search, preset tabs, shape grid
│   └── solver/       Elapsed timer
├── domain/           Pure business logic (pieces, validation, layout)
├── hooks/            Favicon, notifications, solver outcome
├── services/         Effect-TS services (API client, solver, selection)
├── solver/           Worker pool (SolverPortfolio, SolverWorker, worker entry)
├── state/            Zustand stores (board, character, solver, recent searches)
├── types/            Shared TypeScript types
└── utils/            Coords, board outline, favicon SVG
```

## License

MIT. See the repository root `LICENSE-POLICY.md`.