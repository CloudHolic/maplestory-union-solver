// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Solver-specific logic.

pub(crate) mod pruning;
pub(crate) mod state;

pub(crate) use state::SearchState;