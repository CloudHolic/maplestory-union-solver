# License Policy

This repository contains code, models, and documentation under
different licenses. The mapping below defines which license applies
to which part.

## Applicable licenses

| Path | License | File |
|------|---------|------|
| `wasm/**` | AGPL-3.0-or-later | [`wasm/LICENSE`](wasm/LICENSE) |
| `ml/**` | GPL-3.0-or-later | [`ml/COPYING`](ml/COPYING) |
| `server/**` | MIT | [`server/LICENSE`](server/LICENSE) |
| `ui/**` | MIT | [`ui/LICENSE`](ui/LICENSE) |
| `models/**` | CC-BY-4.0 | [`models/LICENSE`](models/LICENSE) |
| `docs/**` | CC-BY-4.0 | [`docs/LICENSE`](docs/LICENSE) |
| `scripts/**`, `deploy/**` | MIT | (covered by [`ui/LICENSE`](ui/LICENSE)) |

Each top-level directory has its own license, listed above. There is
no repository-wide default — root-level files (README.md, LICENSE-POLICY.md, 
.gitignore) are organizational metadata and not subject to a code license.

## SPDX identifiers

Source files should carry an SPDX identifier at the top:

```rust
// SPDX-License-Identifier: AGPL-3.0-or-later
```

```go
// SPDX-License-Identifier: AGPL-3.0-or-later
```

```python
# SPDX-License-Identifier: GPL-3.0-or-later
```

```typescript
// SPDX-License-Identifier: MIT
```

## Why these licenses?

The intent is to allow noncommercial reuse (personal sites, research,
forks-for-improvement) while preventing the solver from being
embedded into commercial paid services without contribution back.

- **`wasm/` (AGPL-3.0-or-later)**: The solver is the substantive
  intellectual contribution. AGPL specifically closes the "SaaS
  loophole" of plain GPL — anyone running a network service that
  exposes solver functionality must publish their full server-side
  source. This achieves the noncommercial-friendly / commercial-
  unfriendly balance without abandoning OSI-approved open source.

- **`ml/` (GPL-3.0-or-later)**: The training pipeline is a
  command-line tool, not a network service. AGPL's §13 (network
  interaction clause) has no practical effect here. Plain GPL
  achieves the same fork-back-to-community goal without the unused
  SaaS clause.

- **`server/` (MIT)**: The server is a thin proxy/cache layer
  (NEXON API forwarding, character cache, run logs). It does not
  embed the solver core and is not the substantive intellectual
  contribution. MIT minimizes friction for any reuse.

- **`ui/` (MIT)**: Frontend code is conventional and has lower reuse
  concern; MIT minimizes friction.

- **`models/` (CC-BY-4.0)**: Trained model weights are data
  artifacts rather than code. CC-BY is the common choice for ML
  weights, requiring only attribution.

- **`docs/` (CC-BY-4.0)**: Documentation reuse is encouraged with
  attribution.

## License compatibility note

AGPL-3.0 and GPL-3.0 are compatible (AGPL is GPL plus §13). Code can
be combined into an AGPL-licensed work. The reverse direction
(combining AGPL code into a GPL-only work) is not permitted.

`wasm/` and `ml/` do not share source code in either direction
(Rust vs Python; communication is through ONNX model files and JSONL
data files), so license divergence between them creates no practical
issue.

## Third-party code

This project depends on third-party libraries via `Cargo.toml`,
`package.json`, `go.mod`, and `pyproject.toml`. Those dependencies
retain their original licenses; this policy applies only to code
authored within this repository.
