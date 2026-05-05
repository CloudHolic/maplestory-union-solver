// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Machine-learning support for the solver.

#[cfg(feature = "tracing")]
pub mod tracer;

#[cfg(feature = "tracing")]
pub use tracer::{Tracer};

#[cfg(feature = "tracing")]
pub(crate) use tracer::{BranchEvent};