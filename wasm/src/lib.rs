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
    PieceInstanceJson, PieceDefJson, Solution, SolutionPlacement
};
pub use solver::{SolveOptions, solve_exact_cover};

#[cfg(target_arch = "wasm32")]
mod wasm_api {
    use wasm_bindgen::prelude::*;

    use crate::{ExactCoverInput, SolveOptions, solve_exact_cover, ExactCoverResult};

    #[wasm_bindgen(js_name = solveExactCover)]
    pub fn solve_exact_cover_wasm(
        input: ExactCoverInput,
        options: SolveOptions
    ) -> Result<ExactCoverResult, JsValue> {
        console_error_panic_hook::set_once();

        solve_exact_cover(&input, options)
            .map_err(|e| JsValue::from_str(&format!("solver error: {e}")))
    }
}