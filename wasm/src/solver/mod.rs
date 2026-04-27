// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Solver-specific logic.
//!
//! Search state, pruning checks, and the backtracking algorithm.
//! Crate-internal; the public API surface is the `solve_*` entry-point
//! functions exposed at the crate root.

pub(crate) mod pruning;
pub(crate) mod state;

pub(crate) use state::SearchState;