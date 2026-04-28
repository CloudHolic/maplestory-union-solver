# wasm

Rust solver for the MapleStory Union placement puzzle. Builds for two
targets:

- **Native** (default): standalone library and benchmark binary.
- **WebAssembly**: browser-loadable module via wasm-bindgen and wasm-pack.

See [`docs/algorithms/exact-cover.md`](../docs/algorithms/exact-cover.md)
for the algorithmic background.

## Layout

```
.cargo/
├── config.toml             build flags required for the wasm32 target
src/
├── lib.rs                  public API + WASM entry point
├── error.rs                SolverError enum
├── base/                   primitives (bitset, RNG)
├── domain/                 puzzle domain (pieces, placements)
├── io/                     JSON wire format types
├── solver/                 backtracking algorithm and pruning
└── bin/benchmark.rs        native CLI runner
tests/
└── exact_cover_basic.rs    integration tests (cargo test)
wasm-test/
└── index.html              browser-side smoke test for the WASM build
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

### WebAssembly module

Requires [wasm-pack](https://rustwasm.github.io/wasm-pack/).

```sh
wasm-pack build --target web --release
```

Produces a `pkg/` directory containing `.wasm`, `.js`, and `.d.ts`
files ready to import from a browser via ES modules.

### Browser smoke test

After `wasm-pack build`, serve the crate root over HTTP and open the
test page. Any static-file server works; Vite is convenient because it
sets the `application/wasm` MIME type automatically:

    npx vite . --port 8000
    # then open http://localhost:8000/wasm-test/index.html

The page loads the WASM module, runs the solver on a small input, and
prints the result. Useful for verifying that a fresh `wasm-pack build`
produces a working module.

## Using from JavaScript / TypeScript

```typescript
import init, { solveExactCover } from './pkg/maplestory_union_solver.js';

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

All wire-format types are exported as TypeScript interfaces in
`pkg/maplestory_union_solver.d.ts`.

## License

LGPL-3.0-or-later. See the repository root `LICENSE` file.