// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! MapleStory Union placement solver.
//! See `docs/algorithms/exact-cover.md`, `docs/algorithms/group-count.md`
//! for the algorithmic background.

mod base;
mod solver;
pub mod domain;
pub mod error;
pub mod io;

pub use domain::{Coord, PieceDef};
pub use error::{Result, SolverError};
pub use io::{
    ExactCoverInput, ExactCoverResult, ExactCoverStats, GroupConstraintJson, GroupCountInput,
    GroupCountResult, GroupCountStats, Solution, SolutionPlacement,
};
