// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Cloudholic

//! External interface types for JSON I/O.

pub mod common;
pub mod input;
pub mod output;

pub use common::{PieceDefJson, PieceInstanceJson, SolverInput, SolverStats};
pub use input::{GroupConstraintJson, ExactCoverInput, GroupCountInput};
pub use output::{
    Solution, SolutionPlacement, ExactCoverResult, ExactCoverStats, 
    GroupCountResult, GroupCountStats
};