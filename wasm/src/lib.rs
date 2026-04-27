// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! MapleStory Union placement solver.
//! See `docs/algorithms/exact-cover.md`, `docs/algorithms/group-count.md`
//! for the algorithmic background.

pub mod domain;
pub mod error;
pub mod io;

mod base;
mod solver;

pub use domain::{Coord, PieceDef};
pub use error::{Result, SolverError};
pub use io::{
    SolverInput, SolverStats,
    ExactCoverInput, ExactCoverResult, ExactCoverStats,
    GroupConstraintJson, GroupCountInput, GroupCountResult, GroupCountStats,
    PieceInstanceJson, Solution, SolutionPlacement
};
pub use solver::{SolveOptions, solve_exact_cover};