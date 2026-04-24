# License Policy

This repository contains code, models, and documentation under different
licenses. The mapping below defines which license applies to which part.

## Applicable licenses

| Path | License | File |
|---|---|---|
| `wasm/**`, `ml/**`, `scripts/**`, `deploy/**` | LGPL-3.0-or-later | [`LICENSE`](LICENSE) |
| `ui/**` | MIT | [`ui/LICENSE`](ui/LICENSE) |
| `models/**` | CC-BY-4.0 | [`models/LICENSE`](models/LICENSE) |
| `docs/**` | CC-BY-4.0 | [`docs/LICENSE`](docs/LICENSE) |
| Everything else (root-level files, etc.) | LGPL-3.0-or-later | [`LICENSE`](LICENSE) |

A `LICENSE` file within a subdirectory overrides the root license for that
subtree.

## SPDX identifiers

Source files should carry an SPDX identifier at the top. Examples:

```rust
// SPDX-License-Identifier: LGPL-3.0-or-later
```

```typescript
// SPDX-License-Identifier: MIT
```

## Why multiple licenses?

- The solver core (`wasm/`) and ML training pipeline (`ml/`) are the
  substantive intellectual contribution of this project and use copyleft to
  ensure modifications remain open.
- The frontend (`ui/`) is conventional React/TypeScript code with low
  reusability concerns, and uses MIT for minimal friction.
- Trained models (`models/`) are data artifacts rather than code. CC-BY-4.0
  is the common choice for ML model weights, requiring only attribution.
- Documentation (`docs/`) uses CC-BY-4.0 to encourage reuse in external
  writing.

## Third-party code

This project may depend on third-party libraries via `Cargo.toml`,
`package.json`, and `pyproject.toml`. Those dependencies retain their
original licenses; this policy applies only to code authored within this
repository.
