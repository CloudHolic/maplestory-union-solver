// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Synthesizes a ML training instance.

use rand::Rng;
use crate::io::{ExactCoverInput, PieceDefJson, PieceInstanceJson, SolverInput};
use crate::ml::{build_piece_pool, build_target_cells, PolyominoCatalog, UnionBoard};

/// Builds one synthetic ML training instance.
pub fn build_instance(
    catalog: &PolyominoCatalog,
    board: &UnionBoard,
    rng: &mut impl Rng
) -> ExactCoverInput {
    let pool = build_piece_pool(catalog, rng);
    let target = build_target_cells(board, pool.total_cells(), rng);

    let piece_defs: Vec<(String, PieceDefJson)> = pool.piece_defs.iter()
        .map(|d| (d.id.clone(), PieceDefJson {
            id: d.id.clone(),
            cells: d.cells.clone(),
            mark_index: d.mark_index
        }))
        .collect();

    let pieces: Vec<PieceInstanceJson> = pool.pieces.iter()
        .enumerate()
        .map(|(i, p)| PieceInstanceJson {
            def_id: p.def_id.clone(),
            index: i as u16
        })
        .collect();

    let target_cells: Vec<String> = target.all_cells.iter()
        .map(|(r, c)| format!("{r},{c}"))
        .collect();

    let center_cells: Vec<String> = target.center_cells.iter()
        .map(|(r, c)| format!("{r},{c}"))
        .collect();

    ExactCoverInput {
        target_cells,
        common: SolverInput { pieces, piece_defs, center_cells }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashSet;
    use rand::SeedableRng;
    use rand_xoshiro::Xoshiro256PlusPlus;

    fn make_rng(seed: u64) -> Xoshiro256PlusPlus {
        Xoshiro256PlusPlus::seed_from_u64(seed)
    }

    fn make_catalog() -> PolyominoCatalog {
        PolyominoCatalog::enumerate(6)
    }

    #[test]
    fn target_total_matches_piece_cell_sum() {
        let board = UnionBoard::new();
        let catalog = make_catalog();
        for seed in 0..50 {
            let mut rng = make_rng(seed);
            let input = build_instance(&catalog, &board, &mut rng);

            let def_lookup: std::collections::HashMap<&str, &PieceDefJson> =
                input.common.piece_defs.iter()
                    .map(|(id, def)| (id.as_str(), def))
                    .collect();
            let piece_total: usize = input.common.pieces.iter()
                .map(|p| def_lookup[p.def_id.as_str()].cells.len())
                .sum();

            assert_eq!(
                input.target_cells.len(), piece_total,
                "target {} != piece total {} (seed={})",
                input.target_cells.len(), piece_total, seed
            );
        }
    }

    #[test]
    fn piece_instances_reference_known_defs() {
        let board = UnionBoard::new();
        let catalog = make_catalog();
        let mut rng = make_rng(42);

        for _ in 0..50 {
            let input = build_instance(&catalog, &board, &mut rng);
            let def_ids: HashSet<&str> = input.common.piece_defs.iter()
                .map(|(id, _)| id.as_str())
                .collect();
            for piece in &input.common.pieces {
                assert!(
                    def_ids.contains(piece.def_id.as_str()),
                    "piece def_id {:?} not in piece_defs", piece.def_id
                );
            }
        }
    }

    #[test]
    fn center_cells_are_subset_of_target() {
        let board = UnionBoard::new();
        let catalog = make_catalog();
        let mut rng = make_rng(42);

        for _ in 0..50 {
            let input = build_instance(&catalog, &board, &mut rng);
            let target_set: HashSet<&str> = input.target_cells.iter()
                .map(|s| s.as_str())
                .collect();
            for cc in &input.common.center_cells {
                assert!(
                    target_set.contains(cc.as_str()),
                    "center cell {:?} missing from target", cc
                );
            }
        }
    }

    #[test]
    fn piece_indices_are_unique() {
        let board = UnionBoard::new();
        let catalog = make_catalog();
        let mut rng = make_rng(42);

        for _ in 0..50 {
            let input = build_instance(&catalog, &board, &mut rng);
            let indices: HashSet<u16> = input.common.pieces.iter()
                .map(|p| p.index)
                .collect();
            assert_eq!(
                indices.len(), input.common.pieces.len(),
                "duplicate index in pieces"
            );
        }
    }

    #[test]
    fn synthesized_input_round_trips_through_solver_parse() {
        let board = UnionBoard::new();
        let catalog = make_catalog();
        let mut rng = make_rng(42);

        let input = build_instance(&catalog, &board, &mut rng);

        // All string-keyed cells parse cleanly — confirms they're in
        // the format the solver expects.
        let _ = input.parse_target_cells().expect("target cells parse");
        let _ = input.common.parse_center_cells().expect("center cells parse");
        let _ = input.common.piece_defs_map();
    }

    #[test]
    fn deterministic_with_same_seed() {
        let board = UnionBoard::new();
        let catalog = make_catalog();

        let r1 = build_instance(&catalog, &board, &mut make_rng(123));
        let r2 = build_instance(&catalog, &board, &mut make_rng(123));

        assert_eq!(r1.target_cells, r2.target_cells);
        assert_eq!(r1.common.center_cells, r2.common.center_cells);
        assert_eq!(r1.common.pieces.len(), r2.common.pieces.len());
        assert_eq!(r1.common.piece_defs.len(), r2.common.piece_defs.len());
    }
}