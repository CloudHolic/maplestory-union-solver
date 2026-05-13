// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Random piece pool synthesis for one ML training instance.
//!
//! Samples a piece count and per-piece sizes, draws shapes from the polyomino catalog,
//! deduplicates by canonical bitmap, and assigns mark positions.
//! Output is a [`PiecePool`] suitable for embedding into an `ExactCoverInput`.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use rand::{Rng, RngExt};
use rand_distr::{Beta, Distribution, Gamma};
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

const PIECE_COUNT_MIN: usize = 35;
const PIECE_COUNT_MAX: usize = 43;
const SMALL_INCLUSION_PROB: f64 = 0.05;
const SIX_INCLUSION_PROB: f64 = 0.05;

/// Builds a random piece pool given an enumerated polyomino catalog.
pub(crate) fn build_piece_pool(
    catalog: &PolyominoCatalog,
    rng: &mut impl Rng
) -> PiecePool {
    // 1. Total piece count
    let piece_count = rng.random_range(PIECE_COUNT_MIN..=PIECE_COUNT_MAX);
    let mut size_counts: [usize; 7] = [0; 7]; // index 0 unused, 1..=6

    // 2.Special small pieces (size 1-3)
    let small_count = if rng.random_bool(SMALL_INCLUSION_PROB) {
        rng.random_range(1..=3).min(piece_count)
    } else {
        0
    };

    // 3. Size-6 pieces (future expansion)
    size_counts[6] = if rng.random_bool(SIX_INCLUSION_PROB) {
        rng.random_range(1..=2).min(piece_count.saturating_sub(small_count))
    } else {
        0
    };

    // 4. Remaining = size 4 vs 5, ratio sampled.
    let remaining = piece_count - small_count - size_counts[6];
    let r5 = sample_size5_ratio(rng);
    size_counts[5] = (remaining as f64 * r5).round() as usize;
    size_counts[4] = remaining - size_counts[5];

    // 5. Count pieces per size. Small pieces draw their individual size first;
    // size 4/5/6 counts come from the earlier breakdown.
    for _ in 0..small_count {
        let s = rng.random_range(1u8..=3) as usize;
        size_counts[s] += 1;
    }

    // 6. For each size, draw shapes via a symmetric Dirichlet-multinomial.
    // The per-instance Dirichlet weights concentrate piece-count mass on a handful
    // of shapes within each size, mirroring real player class distributions.
    let mut shape_to_def_idx: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut piece_defs: Vec<PieceDef> = Vec::new();
    let mut pieces: Vec<PieceInstance> = Vec::with_capacity(piece_count);

    for size in 1u8..=6u8 {
        let n = size_counts[size as usize];
        if n == 0 {
            continue;
        }

        let candidates = catalog.of_size(size);
        if candidates.is_empty() {
            continue;
        }

        let alpha = sample_alpha_for_size(size, rng);
        let weights = sample_symmetric_dirichlet(alpha, candidates.len(), rng);

        for _ in 0..n {
            let shape = weighted_choice(candidates, &weights, rng);
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
    }

    PiecePool { piece_defs, pieces }
}

/// `Beta(7, 2)` for the size-5 vs size-4 ratio.
fn sample_size5_ratio(rng: &mut impl Rng) -> f64 {
    let beta = Beta::new(7.0_f64, 2.0_f64)
        .expect("Beta(7, 2) params are valid");
    beta.sample(rng)
}

/// Per-instance concentration parameter for the symmetric Dirichlet over
/// `size`'s polyomino catalog.
fn sample_alpha_for_size(size: u8, rng: &mut impl Rng) -> f64 {
    match size {
        4 => 2.0,
        5 => sample_log_uniform(0.5, 1.5, rng),
        _ => 1.0
    }
}

/// Draws from a log-uniform distribution on `[lo, hi]`.
fn sample_log_uniform(lo: f64, hi: f64, rng: &mut impl Rng) -> f64 {
    debug_assert!(lo > 0.0 && hi > lo, "log_uniform requires 0 < lo < hi");

    let log_lo = lo.ln();
    let log_hi = hi.ln();
    let log_x: f64 = rng.random_range(log_lo..log_hi);
    log_x.exp()
}

/// Sample symmetric Dirichlet(alpha, alpha, ..., alpha) of length `k`.
fn sample_symmetric_dirichlet(alpha: f64, k: usize, rng: &mut impl Rng) -> Vec<f64> {
    debug_assert!(alpha > 0.0, "Dirichlet concentration must be positive");
    debug_assert!(k >= 1, "Dirichlet length must be at least 1");

    let gamma = Gamma::new(alpha, 1.0_f64)
        .expect("Gamma(Alpha, 1) is valid for Alpha > 0");

    let mut weights: Vec<f64> = (0..k).map(|_| gamma.sample(rng)).collect();
    let sum: f64 = weights.iter().sum();
    debug_assert!(sum > 0.0, "Dirichlet weight sum must be positive");

    for w in &mut weights {
        *w /= sum;
    }

    weights
}

/// Picks one element from `items` according to `weights`.
fn weighted_choice<'a, T>(items: &'a [T], weights: &[f64], rng: &mut impl Rng) -> &'a T {
    debug_assert_eq!(items.len(), weights.len());

    let r: f64 = rng.random();
    let mut acc = 0.0;
    for (i, &w) in weights.iter().enumerate() {
        acc += w;
        if r < acc {
            return &items[i];
        }
    }

    // Floating-point sum slightly < 1.0
    &items[items.len() - 1]
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