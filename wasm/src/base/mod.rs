// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Cloudholic

//! Algorithm-agnostic primitives.

pub mod bitset;
pub mod rng;

pub use bitset::{BitSet, CAPACITY, WORDS};
pub use rng::{LubyIterator, SolverRng, shuffle};