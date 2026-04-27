// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Problem-domain types for the Union placement puzzle.
//!
//! [`PieceDef`] is part of the public API; the variant/placement
//! enumeration types are crate-internal and exist only to feed the
//! solver.

pub mod piece;
pub(crate) mod enumerate;
pub(crate) mod placement;

pub use piece::{Coord, PieceDef};
pub(crate) use enumerate::{BoardLayout, enumerate_all_placements};
pub(crate) use piece::{all_variants};
pub(crate) use placement::Placement;
