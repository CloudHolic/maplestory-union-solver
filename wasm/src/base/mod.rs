// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Algorithm-agnostic primitives.

mod bitset;
mod connectivity;
mod rng;

pub(crate) use bitset::{BitSet, CAPACITY};
pub(crate) use connectivity::{BfsWorkspace, bfs_components};
pub(crate) use rng::{LubyIterator, SolverRng, make_rng, shuffle};

#[cfg(test)]
pub(crate) use connectivity::make_chain_adj;