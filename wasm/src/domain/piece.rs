// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Cloudholic

//! Piece shapes and their geometric variants.
//!
//! A `PieceDef` is the canonical description of a piece in the game data
//! (a polyomino shape with a designated mark cell).
//! A `PieceVariant` is a normalized rotation+reflection of that shape.
//!
//! `all_variants` returns the up-to-eight distinct variants of a piece,
//! each normalized so that the bounding box starts at (0, 0) and cells
//! are sorted in row-major order.
//! Pieces with rotational or reflective symmetry produce fewer than eight variants
//! because duplicates are collapsed.


use std::collections::HashSet;

/// Grid coordinate as `(row, column)`.
pub type Coord = (i8, i8);

/// Static definition of a piece shape.
#[derive(Debug, Clone)]
pub struct PieceDef {
    pub id: String,
    pub cells: Vec<Coord>,
    pub mark_index: usize
}

/// A single rotation+reflection of a piece, normalized so that:
/// - The minimum row and column are both 0.
/// - Cells are sorted in row-major order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PieceVariant {
    pub cells: Vec<Coord>,
    pub mark: Coord
}

/// Returns all distinct rotation+reflection variants of `def`.
pub fn all_variants(def: &PieceDef) -> Vec<PieceVariant> {
    debug_assert!(
        def.mark_index < def.cells.len(),
        "mark_index {} out of range for piece with {} cells",
        def.mark_index, def.cells.len()
    );

    let mut seen: HashSet<Vec<Coord>> = HashSet::new();
    let mut variants: Vec<PieceVariant> = Vec::with_capacity(8);

    let mut current = def.cells.clone();
    let mut mark = def.cells[def.mark_index];

    for _rot in 0..4 {
        for &do_reflect in &[false, true] {
            let cells = if do_reflect {
                reflect(&current)
            } else {
                current.clone()
            };

            let mark_xformed = if do_reflect { (mark.0, -mark.1) } else { mark };
            let (normalized_cells, normalized_mark) = normalize(&cells, mark_xformed);

            if seen.insert(normalized_cells.clone()) {
                variants.push(PieceVariant {
                    cells: normalized_cells,
                    mark: normalized_mark
                });
            }
        }

        current = rotate90(&current);
        mark = (mark.1, -mark.0);
    }

    variants
}

/// Rotates each cell 90 degrees clockwise: `(r, c) -> (c, -r)`
fn rotate90(cells: &[Coord]) -> Vec<Coord> {
    cells.iter().map(|&(r, c)| (c, -r)).collect()
}

/// Reflects each cell across the row axis: `(r, c) -> (r, -c)`
fn reflect(cells: &[Coord]) -> Vec<Coord> {
    cells.iter().map(|&(r, c)| (r, -c)).collect()
}

/// Translates `cells` so the minimum row and column are both 0,
/// applies the same translation to `mark`, and sorts cells row-major.
/// Returns `(normalized_cells, normalized_mark)`
fn normalize(cells: &[Coord], mark: Coord) -> (Vec<Coord>, Coord) {
    debug_assert!(!cells.is_empty(), "piece must have at least one cell");

    let min_r = cells.iter().map(|&(r, _)| r).min().expect("non-empty");
    let min_c = cells.iter().map(|&(_, c)| c).min().expect("non-empty");

    let mut shifted: Vec<Coord> = cells
        .iter()
        .map(|&(r, c)| (r - min_r, c - min_c))
        .collect();
    shifted.sort_unstable();

    let normalized_mark = (mark.0 - min_r, mark.1 - min_c);
    (shifted, normalized_mark)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_cell_piece_has_one_variant() {
        let def = PieceDef {
            id: "1x1".to_string(),
            cells: vec![(0, 0)],
            mark_index: 0
        };
        let variants = all_variants(&def);

        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].cells, vec![(0, 0)]);
        assert_eq!(variants[0].mark, (0, 0));
    }

    #[test]
    fn square_piece_has_one_variant() {
        let def = PieceDef {
            id: "2x2".to_string(),
            cells: vec![(0, 0), (0, 1), (1, 0), (1, 1)],
            mark_index: 0,
        };
        let variants = all_variants(&def);
        assert_eq!(variants.len(), 1);
    }

    #[test]
    fn horizontal_bar_has_two_variants() {
        let def = PieceDef {
            id: "1x3".to_string(),
            cells: vec![(0, 0), (0, 1), (0, 2)],
            mark_index: 1,
        };
        let variants = all_variants(&def);
        assert_eq!(variants.len(), 2);

        for v in &variants {
            let min_r = v.cells.iter().map(|&(r, _)| r).min().unwrap();
            let min_c = v.cells.iter().map(|&(_, c)| c).min().unwrap();
            assert_eq!(min_r, 0);
            assert_eq!(min_c, 0);
        }
    }

    #[test]
    fn l_tetromino_has_eight_variants() {
        let def = PieceDef {
            id: "L".to_string(),
            cells: vec![(0, 0), (1, 0), (2, 0), (2, 1)],
            mark_index: 0,
        };
        let variants = all_variants(&def);
        assert_eq!(variants.len(), 8);
    }

    #[test]
    fn t_tetromino_has_four_variants() {
        let def = PieceDef {
            id: "T".to_string(),
            cells: vec![(0, 0), (0, 1), (0, 2), (1, 1)],
            mark_index: 1,
        };
        let variants = all_variants(&def);
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn mark_is_a_member_of_cells() {
        let def = PieceDef {
            id: "L".to_string(),
            cells: vec![(0, 0), (1, 0), (2, 0), (2, 1)],
            mark_index: 3, // (2, 1)
        };
        let variants = all_variants(&def);
        for v in &variants {
            assert!(
                v.cells.contains(&v.mark),
                "mark {:?} not in cells {:?}",
                v.mark,
                v.cells
            );
        }
    }

    #[test]
    fn all_variants_are_normalized() {
        let def = PieceDef {
            id: "L".to_string(),
            cells: vec![(0, 0), (1, 0), (2, 0), (2, 1)],
            mark_index: 0,
        };
        let variants = all_variants(&def);
        for v in &variants {
            let min_r = v.cells.iter().map(|&(r, _)| r).min().unwrap();
            let min_c = v.cells.iter().map(|&(_, c)| c).min().unwrap();
            assert_eq!(min_r, 0, "variant not normalized: {:?}", v.cells);
            assert_eq!(min_c, 0, "variant not normalized: {:?}", v.cells);
        }
    }

    #[test]
    fn variant_cells_are_sorted() {
        let def = PieceDef {
            id: "L".to_string(),
            cells: vec![(0, 0), (1, 0), (2, 0), (2, 1)],
            mark_index: 0,
        };
        let variants = all_variants(&def);
        for v in &variants {
            let mut sorted = v.cells.clone();
            sorted.sort_unstable();
            assert_eq!(v.cells, sorted, "variant cells not sorted");
        }
    }

    #[test]
    fn rotate90_four_times_is_identity_after_normalization() {
        let original = vec![(0_i8, 0_i8), (1, 0), (2, 0), (2, 1)];
        let mut current = original.clone();
        for _ in 0..4 {
            current = rotate90(&current);
        }
        // After four rotations, shape is identical modulo translation.
        let (a, _) = normalize(&original, (0, 0));
        let (b, _) = normalize(&current, (0, 0));
        assert_eq!(a, b);
    }
}