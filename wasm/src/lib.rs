// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Cloudholic

//! MapleStory Union placement solver.
//! See 'docs/algorithms/exact-cover.md' for the algorithmic background.

pub mod base;
pub mod domain;
pub mod error;
// pub mod io;
// pub mod solver;


// Public API re-exports.
pub use error::{Result, SolverError};

