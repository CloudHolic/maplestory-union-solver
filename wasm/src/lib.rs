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
pub use solver::{CancelFlag, SolveOptions, solve_exact_cover};

#[cfg(target_arch = "wasm32")]
mod wasm_api {
    use js_sys::SharedArrayBuffer;
    use wasm_bindgen::prelude::*;

    use crate::{ExactCoverInput, SolveOptions, solve_exact_cover, ExactCoverResult, CancelFlag};

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(typescript_custom_section)]
    const TS_APPEND_CONTENT: &'static str = r#"
export type Coord = [number, number];
export type Solution = SolutionPlacement[];
"#;

    #[wasm_bindgen(js_name = solveExactCover)]
    pub fn solve_exact_cover_wasm(
        input: ExactCoverInput,
        options: SolveOptions,
        #[wasm_bindgen(js_name = "cancelBuffer")]
        cancel_buffer: Option<SharedArrayBuffer>
    ) -> Result<ExactCoverResult, JsValue> {
        console_error_panic_hook::set_once();

        let cancel_flag = cancel_buffer.as_ref().map(CancelFlag::from_sab);
        solve_exact_cover(&input, options, cancel_flag.as_ref())
            .map_err(|e| JsValue::from_str(&format!("solver error: {e}")))
    }
}