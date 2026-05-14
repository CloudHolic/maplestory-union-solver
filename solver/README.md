# maplestory-union-solver/solver

Rust solver for the MapleStory Union placement puzzle. Builds for two
targets:

- **Native** (default): standalone library and CLI binaries.
- **WebAssembly**: browser-loadable module via wasm-bindgen and
  wasm-pack, consumed by the React UI in `../frontend/`.

Additionally provides an ML training data generator (cfg-tracing
gated) that produces `.jsonl.gz` shards of branch traces.

See [`docs/algorithms/exact-cover.md`](../docs/algorithms/exact-cover.md)
for the algorithmic background.

## Layout

```
.cargo/
├── config.toml             build flags required for the wasm32 target
src/
├── lib.rs                  public API + WASM entry point
├── error.rs                SolverError enum
├── base/                   primitives (bitset, RNG, DIRS const)
├── domain/                 puzzle domain (pieces, placements)
├── io/                     JSON wire format types
├── solver/                 backtracking algorithm, pruning, cancel
├── ml/                     ML training data (cfg-tracing only)
└── bin/
    ├── benchmark.rs        solver portfolio benchmark
    └── generate.rs         ML training data generator (cfg-tracing only)
tests/
├── exact_cover_basic.rs           integration tests (algorithm)
└── exact_cover_with_tracing.rs    integration tests (tracing feature, b2-variant)
```

## Build

### Native library and tests

```sh
cargo build
cargo test
```

### Native benchmark binary

```sh
cargo build --release --bin benchmark
./target/release/benchmark <input.json> [--seed N] [--runs N] [--output out.json]
```

Run `benchmark --help` for the full flag list.

### ML training data generator binary

Requires `tracing` feature enabled.

Synthesizes random ExactCover instances and runs the solver with
branch tracing enabled, emitting `.jsonl.gz` shards for downstream
PyTorch training.

```sh
cargo build --release --bin generate --features tracing
./target/release/generate [--seed N] [--count N] [--out <path/to/output>]  [--shard-size-mb 100]
```

Run `generate --help` for the full flag list.

### WebAssembly module

Requires `wasm-pack`.

The `frontend/` project drives WASM builds via its `build:wasm` npm script,
which writes output into `frontend/wasm-pkg`:

```sh
cd ../frontend && pnpm build:wasm
```

Output is a `pkg/`-style directory containing `.wasm`, `.js`, and
`.d.ts` files ready to import from a browser via ES modules.

## Using from JavaScript / TypeScript

```typescript
import init, { solveExactCover } from '@solver/wasm';

await init();

const result = solveExactCover(
    {
        targetCells: ['0,0', '0,1', '1,0', '1,1'],
        pieces: [{ defId: 'square', index: 0 }],
        pieceDefs: [
            ['square', { id: 'square', cells: [[0,0],[0,1],[1,0],[1,1]], markIndex: 0 }],
        ],
        centerCells: ['0,0'],
    },
    { seed: 42 },
);

if (result.solution) {
    console.log(`Solved in ${result.stats.elapsedMs}ms`, result.solution);
}
```

All wire-format types are exported as TypeScript interfaces in the
generated `.d.ts`.

## License

AGPL-3.0-or-later. See `LICENSE`.
