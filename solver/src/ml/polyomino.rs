// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Free polyomino enumeration via Redelmeier's  algorithm.

use std::collections::HashSet;

use crate::base::DIRS;
use crate::domain::{Coord, PieceDef};
use crate::ml::canonical_bitmap;

/// Anchor cell - every polyomino has its lex-min cell at this position.
const ROOT: Coord = (0, 0);

/// Free polyominoes of a given size, indexed by `polyominos[size]`.
pub struct PolyominoCatalog {
    by_size: Vec<Vec<PieceDef>>,
}

impl PolyominoCatalog {
    /// Enumerates all free polyominoes from size 1 up to `max_size`.
    pub fn enumerate(max_size: u8) -> Self {
        let max = max_size as usize;
        let mut by_size: Vec<Vec<PieceDef>> = vec![Vec::new(); max + 1];
        let mut seen: Vec<HashSet<Vec<u8>>> = vec![HashSet::new(); max + 1];

        // Seed: monomino at ROOT.
        let mut current: Vec<Coord> = vec![ROOT];
        let mut tried: HashSet<Coord> = HashSet::from([ROOT]);

        // Initial untried = ROOT's neighbors that pass the lex filter.
        let mut untried: Vec<Coord> = Vec::new();
        push_filtered_neighbors(ROOT, &tried, &mut untried);

        record(&current, &mut by_size, &mut seen);
        recurse(&mut current, untried, &mut tried, max_size, &mut by_size, &mut seen);

        // Assign synthetic ids.
        for (size, defs) in by_size.iter_mut().enumerate() {
            for (idx, def) in defs.iter_mut().enumerate() {
                def.id = format!("poly_{}_{}", size, idx);
            }
        }

        Self { by_size }
    }

    pub fn of_size(&self, size: u8) -> &[PieceDef] {
        &self.by_size[size as usize]
    }
}

/// Record `cells` as a polyomino if its canonical bitmap (under rotation/reflection)
/// hasn't been seen before at that size.
fn record(
    cells: &[Coord],
    by_size: &mut [Vec<PieceDef>],
    seen: &mut [HashSet<Vec<u8>>]
) {
    let size = cells.len();
    let def = PieceDef {
        id: String::new(),
        cells: cells.to_vec(),
        mark_index: 0
    };
    let bitmap = canonical_bitmap(&def);

    if seen[size].insert(bitmap) {
        by_size[size].push(def);
    }
}

fn push_filtered_neighbors(cell: Coord, tried: &HashSet<Coord>, out: &mut Vec<Coord>) {
    for (dr, dc) in DIRS {
        let n = (cell.0 + dr, cell.1 + dc);
        if n >= ROOT && !tried.contains(&n) {
            out.push(n);
        }
    }
}

/// Redelmeier inner loop with lex-filter on root cell.
fn recurse(
    current: &mut Vec<Coord>,
    untried: Vec<Coord>,
    tried: &mut HashSet<Coord>,
    max_size: u8,
    by_size: &mut [Vec<PieceDef>],
    seen: &mut [HashSet<Vec<u8>>]
) {
    if current.len() >= max_size as usize {
        return;
    }

    // Track cells this frame inserts into `tried`, for rollback at end.
    let mut added_to_tried: Vec<Coord> = Vec::with_capacity(untried.len());

    let mut idx = 0;
    while idx < untried.len() {
        let cell = untried[idx];
        idx += 1;

        if tried.contains(&cell) {
            continue;
        }

        // Add cell to polyomino.
        current.push(cell);
        tried.insert(cell);
        added_to_tried.push(cell);

        // Construct child's untried = parent's remaining + cell's new neighbors, filtered.
        let mut child_untried: Vec<Coord> = untried[idx..].to_vec();
        push_filtered_neighbors(cell, tried, &mut child_untried);

        record(current, by_size, seen);
        recurse(current, child_untried, tried, max_size, by_size, seen);

        current.pop();
    }

    // Rollback `tried`: remove every cell this frame added.
    for &cell in &added_to_tried {
        tried.remove(&cell);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(catalog: &PolyominoCatalog, size: u8) -> usize {
        catalog.of_size(size).len()
    }

    #[test]
    fn known_free_polyomino_counts() {
        let catalog = PolyominoCatalog::enumerate(6);

        // OEIS A000105: free polyomino counts.
        assert_eq!(count(&catalog, 1), 1, "monomino");
        assert_eq!(count(&catalog, 2), 1, "domino");
        assert_eq!(count(&catalog, 3), 2, "trominoes");
        assert_eq!(count(&catalog, 4), 5, "tetrominoes");
        assert_eq!(count(&catalog, 5), 12, "pentominoes");
        assert_eq!(count(&catalog, 6), 35, "hexominoes");
    }

    #[test]
    fn each_polyomino_has_correct_cell_count() {
        let catalog = PolyominoCatalog::enumerate(6);
        for size in 1..=6 {
            for def in catalog.of_size(size) {
                assert_eq!(def.cells.len(), size as usize,
                           "poly {}: cell count mismatch", def.id);
            }
        }
    }

    #[test]
    fn ids_are_assigned_and_unique() {
        let catalog = PolyominoCatalog::enumerate(5);
        let mut ids: Vec<&str> = Vec::new();
        for size in 1..=5 {
            for def in catalog.of_size(size) {
                assert!(!def.id.is_empty(), "id should be set");
                assert!(def.id.starts_with(&format!("poly_{}_", size)));
                ids.push(&def.id);
            }
        }
        let unique: HashSet<&&str> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len(), "ids must be unique");
    }

    #[test]
    fn canonical_bitmaps_are_pairwise_distinct() {
        let catalog = PolyominoCatalog::enumerate(6);
        for size in 1..=6 {
            let bitmaps: Vec<Vec<u8>> = catalog.of_size(size).iter()
                .map(canonical_bitmap)
                .collect();
            let unique: HashSet<&Vec<u8>> = bitmaps.iter().collect();
            assert_eq!(unique.len(), bitmaps.len(),
                       "size {} polyominoes must have distinct canonical bitmaps", size);
        }
    }
}