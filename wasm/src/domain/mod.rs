// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Cloudholic

//! Problem-domain types for the Union placement puzzle.

pub mod enumerate;
pub mod piece;
pub mod placement;

pub use enumerate::{BoardLayout, enumerate_all_placements};
pub use piece::{Coord, PieceDef, PieceVariant, all_variants};
pub use placement::Placement;