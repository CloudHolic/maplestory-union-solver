// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Concrete piece placements on the board.
//!
//! A [`Placement`] is the result of anchoring a [`PieceVariant`] to a specific board location.
//! Unlike a `PieceVariant`, a `Placement` carries all information the solver needs at decision time:
//! the cell footprint as a bitset, the adjacent-cell mask for connectivity checks,
//! the mark coordinate, and various precomputed counts.
//!
//! Placements are produced by [`crate::domain::enumerate`] and stored in
//! a single flat `Vec<Placement>` for the duration of a solve.
//! They are immutable thereafter.

use crate::base::BitSet;
use crate::domain::Coord;

/// A pice variant placed at a specific board location.
#[derive(Debug, Clone)]
pub(crate) struct Placement {
    pub type_idx: u16,
    pub bits: BitSet,
    pub neighbor_indices: Vec<u16>,
    pub b_count: u8,
    pub mark_on_center: bool,
    pub cell_indices: Vec<u16>,
    pub mark: Coord,
    pub cells: Vec<Coord>
}

impl Placement {
    /// Returns the number of cells this placement covers.
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn size(&self) -> usize {
        self.cell_indices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_placement() -> Placement {
        let mut bits = BitSet::new();
        bits.set(10);
        bits.set(11);
        bits.set(12);

        Placement {
            type_idx: 0,
            bits,
            neighbor_indices: vec![9, 13],
            b_count: 2,
            mark_on_center: false,
            cell_indices: vec![10, 11, 12],
            mark: (1, 1),
            cells: vec![(1, 0), (1, 1), (1, 2)],
        }
    }

    #[test]
    fn size_matches_cell_count() {
        let pl = make_test_placement();
        assert_eq!(pl.size(), 3);
        assert_eq!(pl.size(), pl.cells.len());
        assert_eq!(pl.size(), pl.bits.count_ones() as usize);
    }

    #[test]
    fn placement_is_clonable() {
        let pl = make_test_placement();
        let pl2 = pl.clone();
        assert_eq!(pl.type_idx, pl2.type_idx);
        assert_eq!(pl.cell_indices, pl2.cell_indices);
        assert_eq!(pl.bits, pl2.bits);
    }
}