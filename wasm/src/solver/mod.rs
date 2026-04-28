// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Solver-specific logic.

mod cancel;
mod exact_cover;
mod pruning;
mod state;

pub use cancel::CancelFlag;
pub use exact_cover::{SolveOptions, solve_exact_cover};
pub(crate) use pruning::{IslandWorkspace, island_check, neighbor_check, parity_check};
pub(crate) use state::{SearchState, PlacementUndo};