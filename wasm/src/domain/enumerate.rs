// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Cloudholic

//! Placement enumeration.
//!
//! Given a board layout (set of valid cells with assigned indices) and
//! a collection of piece definitions, produces every legal ['Placement']
//! along with all precomputed information the solver needs at decision time.

use std::collections::HashMap;

use crate::base::{BitSet, CAPACITY};
use crate::domain::piece::{Coord, PieceDef, all_variants};
use crate::domain::placement::Placement;
use crate::error::{Result, SolverError};

/// Board geometry shared by the solver and the enumerator.
pub struct BoardLayout {
    pub cells: Vec<Coord>,
    pub coord_to_idx: HashMap<Coord, u16>,
    pub center_cells: Vec<Coord>,
    pub cell_color: Vec<u8>
}

impl BoardLayout {
    /// Builds a board layout from a list of cell coordinates and the center region.
    pub fn new(cells: Vec<Coord>, center_cells: Vec<Coord>) -> Result<Self> {
        if cells.len() > CAPACITY {
            return Err(SolverError::BoardTooLarge {
                actual: cells.len(),
                capacity: CAPACITY
            });
        }

        let mut coord_to_idx = HashMap::with_capacity(cells.len());
        for (i, &coord) in cells.iter().enumerate() {
            coord_to_idx.insert(coord, i as u16);
        }

        let cell_color = cells
            .iter()
            .map(|&(r, c)| ((r + c).rem_euclid(2)) as u8)
            .collect();

        Ok(Self { cells, coord_to_idx, center_cells, cell_color })
    }
}

/// One instance of a piece in the puzzle input.
#[derive(Debug, Clone)]
pub struct PieceInstance {
    pub type_idx: u16,
    pub def_id: String
}

/// Enumerates all legal placements for every piece instance.
pub fn enumerate_all_placements(
    pieces: &[PieceInstance],
    piece_defs: &HashMap<String, PieceDef>,
    board: &BoardLayout
) -> Result<Vec<Placement>> {
    let mut variant_cache: HashMap<&str, Vec<crate::domain::PieceVariant>> = HashMap::new();
    let mut placements: Vec<Placement> = Vec::new();
    let center_set: std::collections::HashSet<Coord> =
        board.center_cells.iter().copied().collect();

    for piece in pieces {
        let def = piece_defs.get(&piece.def_id).ok_or_else(|| {
            SolverError::UnknownPieceDef {
                id: piece.def_id.clone()
            }
        })?;

        let variants = variant_cache
            .entry(piece.def_id.as_str())
            .or_insert_with(|| all_variants(def));

        for variant in variants.iter() {
            enumerate_variant(piece, variant, board, &center_set, &mut placements);
        }
    }

    Ok(placements)
}

/// Enumerate all anchor positions for a single variant and appends successful placements to 'out'.
fn enumerate_variant(
    piece: &PieceInstance,
    variant: &crate::domain::PieceVariant,
    board: &BoardLayout,
    center_set: &std::collections::HashSet<Coord>,
    out: &mut Vec<Placement>
) {
    let (anchor_r, anchor_c) = variant.cells[0];

    for &(br, bc) in &board.cells {
        let dr = br - anchor_r;
        let dc = bc - anchor_c;

        // Try mapping every cell of the variant under this offset to a board index.
        // Bail out the moment one cell falls off-board.
        let mut cell_indices: Vec<u16> = Vec::with_capacity(variant.cells.len());
        let mut cells: Vec<Coord> = Vec::with_capacity(variant.cells.len());
        let mut bits = BitSet::new();
        let mut b_count: u8 = 0;
        let mut all_in_board = true;

        for &(vr, vc) in &variant.cells {
            let coord = (vr + dr, vc + dc);
            match board.coord_to_idx.get(&coord) {
                Some(&idx) => {
                    cell_indices.push(idx);
                    cells.push(coord);
                    bits.set(idx as usize);
                    b_count += (board.cell_color[idx as usize] == 0) as u8;
                }
                None => {
                    all_in_board = false;
                    break;
                }
            }
        }

        if !all_in_board {
            continue;
        }

        // Compute neighbor_bits: cells adjacent to the placement but not inside it.
        let neighbor_bits = compute_neighbor_bits(&cells, board, &bits);
        let mark = (variant.mark.0 + dr, variant.mark.1 + dc);
        let mark_on_center = center_set.contains(&mark);

        out.push(Placement {
            type_idx: piece.type_idx,
            bits, neighbor_bits, b_count, mark_on_center,
            cell_indices, mark, cells
        });
    }
}

/// Builds the bitset of board cells adjacent to the placement.
///
/// A cell is in 'neighbor_bits' iff:
/// - It is a valid board cell, and
/// - It is orthogonally adjacent to at least one placement cell, and
/// - It is not itself a placement cell.
fn compute_neighbor_bits(
    placement_cells: &[Coord],
    board: &BoardLayout,
    placement_bits: &BitSet
) -> BitSet {
    const DIRS: [(i8, i8); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    let mut neighbor_bits = BitSet::new();

    for &(r, c) in placement_cells {
        for (dr, dc) in DIRS {
            let nb = (r + dr, c + dc);
            if let Some(&nb_idx) = board.coord_to_idx.get(&nb) {
                let idx = nb_idx as usize;
                if !placement_bits.test(idx) {
                    neighbor_bits.set(idx);
                }
            }
        }
    }

    neighbor_bits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_3x3_board() -> BoardLayout {
        let cells: Vec<Coord> = (0..3)
            .flat_map(|r| (0..3).map(move |c| (r, c)))
            .collect();
        BoardLayout::new(cells, vec![]).unwrap()
    }

    #[test]
    fn single_cell_piece_on_3x3() {
        let board = make_3x3_board();

        let mut piece_defs = HashMap::new();
        piece_defs.insert(
            "1x1".to_string(),
            PieceDef {
                id: "1x1".to_string(),
                cells: vec![(0, 0)],
                mark_index: 0,
            },
        );

        let pieces = vec![PieceInstance {
            type_idx: 0,
            def_id: "1x1".to_string(),
        }];

        let placements = enumerate_all_placements(&pieces, &piece_defs, &board).unwrap();
        assert_eq!(placements.len(), 9);
        for pl in &placements {
            assert_eq!(pl.size(), 1);
            assert_eq!(pl.b_count + (1 - pl.b_count), 1); // sanity
        }
    }

    #[test]
    fn bar_piece_on_3x3() {
        let board = make_3x3_board();

        let mut piece_defs = HashMap::new();
        piece_defs.insert(
            "bar".to_string(),
            PieceDef {
                id: "bar".to_string(),
                cells: vec![(0, 0), (0, 1), (0, 2)],
                mark_index: 1,
            },
        );

        let pieces = vec![PieceInstance {
            type_idx: 0,
            def_id: "bar".to_string(),
        }];

        let placements = enumerate_all_placements(&pieces, &piece_defs, &board).unwrap();
        // 3 horizontal positions (one per row) + 3 vertical positions (one per column) = 6
        assert_eq!(placements.len(), 6);
        for pl in &placements {
            assert_eq!(pl.size(), 3);
        }
    }

    #[test]
    fn off_board_placements_are_skipped() {
        let board = make_3x3_board();

        let mut piece_defs = HashMap::new();
        piece_defs.insert(
            "L".to_string(),
            PieceDef {
                id: "L".to_string(),
                cells: vec![(0, 0), (1, 0), (2, 0), (2, 1)],
                mark_index: 0,
            },
        );

        let pieces = vec![PieceInstance {
            type_idx: 0,
            def_id: "L".to_string(),
        }];

        let placements = enumerate_all_placements(&pieces, &piece_defs, &board).unwrap();
        assert_eq!(placements.len(), 16);

        // Every placement must lie entirely within the board.
        for pl in &placements {
            for &(r, c) in &pl.cells {
                assert!(
                    (0..3).contains(&r) && (0..3).contains(&c),
                    "placement cell {:?} is off-board",
                    (r, c)
                );
            }
        }

        for pl in &placements {
            assert_eq!(pl.size(), 4);
        }
    }

    #[test]
    fn mark_on_center_is_correctly_flagged() {
        let cells: Vec<Coord> = (0..3)
            .flat_map(|r| (0..3).map(move |c| (r, c)))
            .collect();
        let board = BoardLayout::new(cells, vec![(1, 1)]).unwrap();

        let mut piece_defs = HashMap::new();
        piece_defs.insert(
            "1x1".to_string(),
            PieceDef {
                id: "1x1".to_string(),
                cells: vec![(0, 0)],
                mark_index: 0,
            },
        );

        let pieces = vec![PieceInstance {
            type_idx: 0,
            def_id: "1x1".to_string(),
        }];

        let placements = enumerate_all_placements(&pieces, &piece_defs, &board).unwrap();
        let center_count = placements.iter().filter(|p| p.mark_on_center).count();
        assert_eq!(center_count, 1);
    }

    #[test]
    fn neighbor_bits_exclude_placement() {
        let board = make_3x3_board();

        let mut piece_defs = HashMap::new();
        piece_defs.insert(
            "1x1".to_string(),
            PieceDef {
                id: "1x1".to_string(),
                cells: vec![(0, 0)],
                mark_index: 0,
            },
        );

        let pieces = vec![PieceInstance {
            type_idx: 0,
            def_id: "1x1".to_string(),
        }];

        let placements = enumerate_all_placements(&pieces, &piece_defs, &board).unwrap();

        for pl in &placements {
            // Placement cells should not be in neighbor_bits.
            for &idx in &pl.cell_indices {
                assert!(
                    !pl.neighbor_bits.test(idx as usize),
                    "placement cell {idx} should not be in neighbor_bits"
                );
            }
            // Neighbor count for a 1x1 on a corner is 2, edge is 3, center is 4.
            let center_pl = pl.cells[0] == (1, 1);
            let on_edge = pl.cells[0].0 == 1 || pl.cells[0].1 == 1;
            let expected = if center_pl { 4 } else if on_edge { 3 } else { 2 };
            assert_eq!(
                pl.neighbor_bits.count_ones() as usize,
                expected,
                "wrong neighbor count for cell {:?}",
                pl.cells[0]
            );
        }
    }

    #[test]
    fn unknown_piece_def_returns_error() {
        let board = make_3x3_board();
        let piece_defs: HashMap<String, PieceDef> = HashMap::new();
        let pieces = vec![PieceInstance {
            type_idx: 0,
            def_id: "nonexistent".to_string(),
        }];

        let result = enumerate_all_placements(&pieces, &piece_defs, &board);
        assert!(matches!(result, Err(SolverError::UnknownPieceDef { .. })));
    }

    #[test]
    fn oversized_board_returns_error() {
        let cells: Vec<Coord> = (0..16)
            .flat_map(|r| (0..15).map(move |c| (r as i8, c as i8)))
            .collect(); // 240 cells > CAPACITY (224)
        let result = BoardLayout::new(cells, vec![]);
        assert!(matches!(result, Err(SolverError::BoardTooLarge { .. })));
    }
}