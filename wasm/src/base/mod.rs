// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Algorithm-agnostic primitives.

mod bitset;
mod rng;

pub(crate) use bitset::{BitSet, CAPACITY};
pub(crate) use rng::{LubyIterator, SolverRng, make_rng, shuffle};

#[cfg(test)]
pub(crate) use bitset::WORDS;
