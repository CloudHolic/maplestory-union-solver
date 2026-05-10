// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Random piece pool synthesis for one ML training instance.
//!
//! Samples a piece count and per-piece sizes, draws shapes from the polyomino catalog,
//! deduplicates by canonical bitmap, and assigns mark positions.
//! Output is a [`PiecePool`] suitable for embedding into an `ExactCoverInput`.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::iter::repeat_n;

use rand::{Rng, RngExt};
use rand::seq::IndexedRandom;
use rand_distr::{Beta, Distribution};

use crate::domain::{PieceDef, PieceInstance};
use crate::ml::{PolyominoCatalog, canonical_bitmap};

/// One synthesized instance's piece pool
pub(crate) struct PiecePool {
    /// Distinct piece definitions used by this instance.
    pub piece_defs: Vec<PieceDef>,

    /// Concrete piece instances - `pieces.len()` total, each pointing to
    /// `piece_defs` via `type_idx` and `def_id`.
    pub pieces: Vec<PieceInstance>
}

impl PiecePool {
    /// Total cell count summed over all pieces.
    pub(crate) fn total_cells(&self) -> usize {
        self.pieces.iter()
            .map(|p| self.piece_defs[p.type_idx as usize].cells.len())
            .sum()
    }
}

const PIECE_COUNT_MIN: usize = 20;
const PIECE_COUNT_MAX: usize = 43;
const SMALL_INCLUSION_PROB: f64 = 0.10;
const SIX_INCLUSION_PROB: f64 = 0.05;

/// Builds a random piece pool given an enumerated polyomino catalog.
pub(crate) fn build_piece_pool(
    catalog: &PolyominoCatalog,
    rng: &mut impl Rng
) -> PiecePool {
    // 1. Total piece count
    let piece_count = rng.random_range(PIECE_COUNT_MIN..=PIECE_COUNT_MAX);

    // 2.Special small pieces (size 1-3)
    let small_count = if rng.random_bool(SMALL_INCLUSION_PROB) {
        rng.random_range(1..=3).min(piece_count)
    } else {
        0
    };

    // 3. Size-6 pieces (future expansion)
    let six_count = if rng.random_bool(SIX_INCLUSION_PROB) {
        rng.random_range(1..=2).min(piece_count.saturating_sub(small_count))
    } else {
        0
    };

    // 4. Remaining = size 4 vs 5, ratio sampled.
    let remaining = piece_count - small_count - six_count;
    let r5 = sample_size5_ratio(rng);
    let size5_count = (remaining as f64 * r5).round() as usize;
    let size4_count = remaining - size5_count;

    // 5. Build the size sequence
    let mut size_sequence: Vec<u8> = Vec::with_capacity(piece_count);
    for _ in 0..small_count {
        size_sequence.push(rng.random_range(1u8..=3));
    }
    size_sequence.extend(repeat_n(4u8, size4_count));
    size_sequence.extend(repeat_n(5u8, size5_count));
    size_sequence.extend(repeat_n(6u8, six_count));

    // 6. Sample shapes and dedup by canonical bitmap
    let mut shape_to_def_idx: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut piece_defs: Vec<PieceDef> = Vec::new();
    let mut pieces: Vec<PieceInstance> = Vec::with_capacity(piece_count);

    for size in size_sequence {
        let candidates = catalog.of_size(size);
        if candidates.is_empty() {
            continue;
        }

        let shape = candidates.choose(rng).expect("non-empty checked above");
        let bitmap = canonical_bitmap(shape);

        let def_idx = match shape_to_def_idx.entry(bitmap) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let mark_index = rng.random_range(0..shape.cells.len());
                let id = format!("def_{}", piece_defs.len());

                piece_defs.push(PieceDef {
                    id,
                    cells: shape.cells.clone(),
                    mark_index
                });

                let idx = piece_defs.len() - 1;
                e.insert(idx);

                idx
            }
        };

        pieces.push(PieceInstance {
            type_idx: def_idx as u16,
            def_id: piece_defs[def_idx].id.clone()
        });
    }

    PiecePool { piece_defs, pieces }
}

/// `Beta(2, 2)` for the size-5 vs size-4 ratio.
fn sample_size5_ratio(rng: &mut impl Rng) -> f64 {
    let beta = Beta::new(2.0_f64, 2.0_f64)
        .expect("Beta(2, 2) params are valid");
    beta.sample(rng)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    use rand::SeedableRng;
    use rand_xoshiro::Xoshiro256PlusPlus;

    fn make_catalog() -> PolyominoCatalog {
        PolyominoCatalog::enumerate(6)
    }

    fn make_rng(seed: u64) -> Xoshiro256PlusPlus {
        Xoshiro256PlusPlus::seed_from_u64(seed)
    }

    #[test]
    fn piece_count_within_declared_bounds() {
        let catalog = make_catalog();
        let mut rng = make_rng(42);
        for _ in 0..200 {
            let pool = build_piece_pool(&catalog, &mut rng);
            assert!(
                pool.pieces.len() >= PIECE_COUNT_MIN,
                "piece count {} below MIN {}",
                pool.pieces.len(), PIECE_COUNT_MIN
            );
            assert!(
                pool.pieces.len() <= PIECE_COUNT_MAX,
                "piece count {} above MAX {}",
                pool.pieces.len(), PIECE_COUNT_MAX
            );
        }
    }

    #[test]
    fn piece_defs_have_unique_canonical_bitmaps() {
        let catalog = make_catalog();
        let mut rng = make_rng(42);
        for _ in 0..100 {
            let pool = build_piece_pool(&catalog, &mut rng);
            let bitmaps: HashSet<Vec<u8>> = pool.piece_defs.iter()
                .map(canonical_bitmap)
                .collect();
            assert_eq!(
                bitmaps.len(), pool.piece_defs.len(),
                "piece_defs should be deduplicated by canonical bitmap"
            );
        }
    }

    #[test]
    fn pieces_reference_valid_defs_and_ids_match() {
        let catalog = make_catalog();
        let mut rng = make_rng(42);
        for _ in 0..100 {
            let pool = build_piece_pool(&catalog, &mut rng);
            for piece in &pool.pieces {
                let ti = piece.type_idx as usize;
                assert!(
                    ti < pool.piece_defs.len(),
                    "type_idx {} out of range (defs len {})",
                    ti, pool.piece_defs.len()
                );
                assert_eq!(
                    piece.def_id, pool.piece_defs[ti].id,
                    "def_id mismatch for type_idx {}", ti
                );
            }
        }
    }

    #[test]
    fn mark_index_is_within_cells() {
        let catalog = make_catalog();
        let mut rng = make_rng(42);
        for _ in 0..100 {
            let pool = build_piece_pool(&catalog, &mut rng);
            for def in &pool.piece_defs {
                assert!(
                    def.mark_index < def.cells.len(),
                    "mark_index {} out of range for {}-cell piece {}",
                    def.mark_index, def.cells.len(), def.id
                );
            }
        }
    }

    #[test]
    fn deterministic_with_same_seed() {
        let catalog = make_catalog();

        let pool1 = build_piece_pool(&catalog, &mut make_rng(123));
        let pool2 = build_piece_pool(&catalog, &mut make_rng(123));

        assert_eq!(pool1.pieces.len(), pool2.pieces.len());
        assert_eq!(pool1.piece_defs.len(), pool2.piece_defs.len());
        for (a, b) in pool1.pieces.iter().zip(&pool2.pieces) {
            assert_eq!(a.type_idx, b.type_idx);
            assert_eq!(a.def_id, b.def_id);
        }
        for (a, b) in pool1.piece_defs.iter().zip(&pool2.piece_defs) {
            assert_eq!(a.cells, b.cells);
            assert_eq!(a.mark_index, b.mark_index);
        }
    }
}