// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Canonical 5x5 piece-bitmap representation.

use crate::domain::{PieceDef, all_variants};

/// Side length of the canonical bitmap.
const SIDE: usize = 6;

/// Total cells in the canonical bitmap (`SIDE x SIDE`)
pub(crate) const BITMAP_SIZE: usize = SIDE * SIDE;

/// Returns the canonical row-major bitmap for `def`.
pub(crate) fn canonical_bitmap(def: &PieceDef) -> Vec<u8> {
    let variants = all_variants(def);
    let canonical = variants
        .iter()
        .min_by(|a, b| a.cells.cmp(&b.cells))
        .expect("piece definition produces at least one variant");

    let mut bitmap = vec![0u8; BITMAP_SIZE];
    for &(r, c) in &canonical.cells {
        debug_assert!(
            (0..SIDE as i8).contains(&r) && (0..SIDE as i8).contains(&c),
            "canonical variant cell {:?} out of {}x{} bounds for {}",
            (r, c), SIDE, SIDE, def.id
        );

        let idx = (r as usize) * SIDE + (c as usize);
        bitmap[idx] = 1;
    }

    bitmap
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(id: &str, cells: Vec<(i8, i8)>) -> PieceDef {
        PieceDef { id: id.to_string(), cells, mark_index: 0 }
    }

    #[test]
    fn single_cell_is_top_left() {
        let bitmap = canonical_bitmap(&def("dot", vec![(0, 0)]));
        let mut expected = [0u8; BITMAP_SIZE];
        expected[0] = 1;
        assert_eq!(bitmap, expected);
    }

    #[test]
    fn horizontal_bar_picks_horizontal_canonical() {
        let bitmap = canonical_bitmap(&def("bar3", vec![(0, 0), (0, 1), (0, 2)]));
        let mut expected = [0u8; BITMAP_SIZE];
        expected[0] = 1;
        expected[1] = 1;
        expected[2] = 1;
        assert_eq!(bitmap, expected);
    }

    #[test]
    fn rotated_inputs_produce_identical_bitmap() {
        // Same shape written in two orientations canonicalizes to one bitmap.
        let l1 = def("l1", vec![(0, 0), (1, 0), (2, 0), (2, 1)]);
        let l2 = def("l2", vec![(0, 0), (0, 1), (0, 2), (1, 0)]);
        assert_eq!(canonical_bitmap(&l1), canonical_bitmap(&l2));
    }

    #[test]
    fn cell_count_is_preserved() {
        let pieces = [
            def("dot", vec![(0, 0)]),
            def("bar3", vec![(0, 0), (0, 1), (0, 2)]),
            def("L", vec![(0, 0), (1, 0), (2, 0), (2, 1)]),
        ];
        for p in &pieces {
            let bitmap = canonical_bitmap(p);
            let ones: u8 = bitmap.iter().sum();
            assert_eq!(ones as usize, p.cells.len(), "piece {}: cell count mismatch", p.id);
        }
    }
}