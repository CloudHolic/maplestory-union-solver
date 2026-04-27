// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! External interface types for JSON I/O.

pub mod input;
pub mod output;
mod common;

pub use input::{ExactCoverInput, GroupConstraintJson, GroupCountInput};
pub use output::{
    Solution, SolutionPlacement,
    ExactCoverResult, ExactCoverStats,
    GroupCountResult, GroupCountStats,
};
pub(crate) use common::{PieceDefJson, PieceInstanceJson, SolverInput, SolverStats, parse_cell_key};