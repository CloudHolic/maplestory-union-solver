// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Machine-learning support for the solver.

#[cfg(feature = "tracing")]
pub mod tracer;

#[cfg(feature = "tracing")]
pub(crate) mod canonical;

#[cfg(feature = "tracing")]
pub use tracer::Tracer;

#[cfg(feature = "tracing")]
pub(crate) use canonical::{BITMAP_SIZE, canonical_5x5_bitmap};

#[cfg(feature = "tracing")]
pub(crate) use tracer::{BranchEvent};

#[cfg(feature = "tracing")]
pub(crate) const GRID_COLS: u16 = 20;