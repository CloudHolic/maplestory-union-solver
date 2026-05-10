// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Algorithm-agnostic primitives.

mod bitset;
mod rng;

pub(crate) use bitset::{BitSet, CAPACITY};
pub(crate) use rng::{LubyIterator, SolverRng, make_rng, shuffle};

/// 4-connected neighbor offsets (up, down, left, right) on a 2D grid.
pub(crate) const DIRS: [(i8, i8); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];