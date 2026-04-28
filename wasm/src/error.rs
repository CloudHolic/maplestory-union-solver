// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Error types for the solver library.
//!
//! All public functions return 'Result<T, SolverError>'.
//! The variants below cover the failure modes that callers may need to discriminate.
//! Internal invariants (which should never fail) use 'debug_assert!' instead of error variants.

use thiserror::Error;

/// Errors that can occur during solver setup or execution.
#[derive(Debug, Error)]
pub enum SolverError {
    /// The input contains more cells than the bitset can represent.
    /// The current capacity is 'core::bitset::CAPACITY' (=224).
    #[error("board has {actual} cells, exceed capacity {capacity}")]
    BoardTooLarge {
        actual: usize,
        capacity: usize
    },

    /// A piece references a definition ID that wasn't supplied in 'piece_defs'.
    #[error("piece references unknown definition: {id}")]
    UnknownPieceDef {
        id: String
    },

    /// The total cell count of all pieces does not match the target cell count.
    /// This is a fast-fail check before running the solver.
    #[error("piece cell total {piece_cells} does not match target cell count {target_cells}")]
    PieceCellMismatch {
        piece_cells: usize,
        target_cells: usize
    },

    /// No placement of any piece can land its mark on the center region.
    /// The puzzle is unsatisfiable by construction.
    #[error("no piece placement can satisfy the center-mark constraint")]
    NoCenterMarkPossible,

    /// JSON (de)serialization failed.
    #[error("JSON I/O error: {0}")]
    Json(#[from] serde_json::Error)
}

pub type Result<T> = core::result::Result<T, SolverError>;