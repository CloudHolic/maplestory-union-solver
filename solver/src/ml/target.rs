// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Synthesizes target cell selections matching observed MapleStory player patterns.

use std::collections::HashSet;
use rand::Rng;
use rand::RngExt;
use rand::seq::IndexedRandom;

use crate::base::{DIRS, shuffle};
use crate::domain::Coord;
use crate::ml::{Group, UnionBoard};

/// Result of target cell selection for one ML training instance.
pub(crate) struct TargetSelection {
    /// All target cells, sorted in row-major order.
    pub all_cells: Vec<Coord>,

    /// Subset of `board.center_cells` included in the target. Always 1-4.
    pub center_cells: Vec<Coord>
}

/// Selects target cells totaling exactly `piece_total_cells`.
pub(crate) fn build_target_cells(
    board: &UnionBoard,
    piece_total_cells: usize,
    rng: &mut impl Rng
) -> TargetSelection {
    debug_assert!(
        piece_total_cells <= board.all_cells.len(),
        "piece_total_cells {} exceeds board capacity {}",
        piece_total_cells, board.all_cells.len()
    );

    // 1. Outer groups (priority list root)
    let n1_target = rng.random_range(3..=5);
    let outer_budget = piece_total_cells * 60 / 100;

    let mut all_outer: Vec<&Group> = board.outer_groups().collect();
    shuffle(&mut all_outer, rng);

    let mut n1 = 0;
    let mut outer_sum = 0;
    for g in &all_outer {
        if n1 >= n1_target {
            break;
        }

        let new_sum = outer_sum + g.cells.len();
        if new_sum > outer_budget && n1 >= 2 {
            break;
        }

        outer_sum = new_sum;
        n1 += 1;
    }

    let n1_take = n1.max(2).min(all_outer.len());
    let outer_picked: Vec<&Group> = all_outer[..n1_take].to_vec();
    let outer_rest: Vec<&Group> = all_outer[n1_take..].to_vec();

    let outer_quadrants: HashSet<Quadrant> = outer_picked.iter()
        .map(|g| group_quadrant(g))
        .collect();

    // 2. Center cells - prefer those in outer's quadrants
    let center_count = sample_center_count(rng);

    let mut centers_in: Vec<Coord> = board.center_cells.iter()
        .filter(|c| outer_quadrants.contains(&cell_quadrant(**c)))
        .copied()
        .collect();
    shuffle(&mut centers_in, rng);

    let mut centers_out: Vec<Coord> = board.center_cells.iter()
        .filter(|c| !outer_quadrants.contains(&cell_quadrant(**c)))
        .copied()
        .collect();
    shuffle(&mut centers_out, rng);

    let mut selected_centers: Vec<Coord> = centers_in.iter()
        .chain(centers_out.iter())
        .take(center_count)
        .copied()
        .collect();
    selected_centers.sort();

    // 3. Priority list
    let priority = build_priority_list(
        board, &outer_picked, &outer_rest, &outer_quadrants,
        &selected_centers, rng
    );

    // 4. Fill
    let mut target_cells: HashSet<Coord> = selected_centers.iter().copied().collect();
    for group in &priority {
        if target_cells.len() >= piece_total_cells {
            break;
        }

        let needed = piece_total_cells - target_cells.len();
        let available: Vec<Coord> = group.cells.iter()
            .copied()
            .filter(|c| !target_cells.contains(c))
            .collect();

        if available.is_empty() {
            continue;
        }
        if available.len() <= needed {
            target_cells.extend(available);
        } else {
            let partial = bfs_pick(&available, &target_cells, needed, rng);
            target_cells.extend(partial);
            break;
        }
    }

    let mut all_cells: Vec<Coord> = target_cells.into_iter().collect();
    all_cells.sort();

    TargetSelection { all_cells, center_cells: selected_centers }
}

/// One of the four board quadrants.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Quadrant { NW, NE, SW, SE }

fn cell_quadrant(cell: Coord) -> Quadrant {
    match (cell.0 < 10, cell.1 < 11) {
        (true, true) => Quadrant::NW,
        (true, false) => Quadrant::NE,
        (false, true) => Quadrant::SW,
        (false, false) => Quadrant::SE
    }
}

fn group_quadrant(group: &Group) -> Quadrant {
    cell_quadrant(group.cells[0])
}

/// Categorical([0.05, 0.50, 0.40, 0.05]) for [1, 2, 3, 4].
fn sample_center_count(rng: &mut impl Rng) -> usize {
    let r: f64 = rng.random();
    match r {
        x if x < 0.05 => 1,
        x if x < 0.55 => 2,
        x if x < 0.95 => 3,
        _ => 4
    }
}

/// Builds the group fill priority list.
fn build_priority_list<'a>(
    board: &'a UnionBoard,
    outer_picked: &[&'a Group],
    outer_rest: &[&'a Group],
    outer_quadrants: &HashSet<Quadrant>,
    _selected_centers: &[Coord],
    rng: &mut impl Rng
) -> Vec<&'a Group> {
    let mut priority: Vec<&'a Group> = Vec::new();

    // tier 1: chosen outer
    priority.extend(outer_picked.iter().copied());

    // tier 2: path inner (same quadrants as `outer_picked`)
    let mut path_inner: Vec<&Group> = board.inner_groups()
        .filter(|g| outer_quadrants.contains(&group_quadrant(g)))
        .collect();
    shuffle(&mut path_inner, rng);
    priority.extend(path_inner.iter().copied());

    // tier 3: other inner (different quadrants), N2 ∈ [1, 4]
    let n2 = rng.random_range(1..=4);
    let mut other_inner: Vec<&Group> = board.inner_groups()
        .filter(|g| !outer_quadrants.contains(&group_quadrant(g)))
        .collect();
    shuffle(&mut other_inner, rng);
    let n2_take = n2.min(other_inner.len());
    priority.extend(other_inner[..n2_take].iter().copied());

    // tier 4: overflow - remaining outer + remaining inner
    let mut tier4: Vec<&Group> = outer_rest.iter().copied().collect();
    tier4.extend(other_inner[n2_take..].iter().copied());
    shuffle(&mut tier4, rng);
    priority.extend(tier4);

    priority
}

/// Picks `n` connected cells from `available`, biased toward those adjacent to `already_filled`.
/// Falls back to random if BFS exhausted.
fn bfs_pick(
    available: &[Coord],
    already_filled: &HashSet<Coord>,
    n: usize,
    rng: &mut impl Rng
) -> Vec<Coord> {
    let available_set: HashSet<Coord> = available.iter().copied().collect();

    let mut frontier: Vec<Coord> = available.iter()
        .filter(|&&c| has_adjacent(c, already_filled))
        .copied()
        .collect();
    if frontier.is_empty() {
        frontier = vec![*available.choose(rng).expect("available is non-empty")];
    }

    let mut picked: HashSet<Coord> = HashSet::new();
    while picked.len() < n && !frontier.is_empty() {
        let idx = rng.random_range(0..frontier.len());
        let cell = frontier.swap_remove(idx);
        if !picked.insert(cell) {
            continue;
        }

        for (dr, dc) in DIRS {
            let neighbor = (cell.0 + dr, cell.1 + dc);
            if available_set.contains(&neighbor) && !picked.contains(&neighbor) {
                frontier.push(neighbor);
            }
        }
    }

    // Fallback: BFS exhausted but we still need more.
    if picked.len() < n {
        let mut leftover: Vec<Coord> = available.iter()
            .copied()
            .filter(|c| !picked.contains(c))
            .collect();
        shuffle(&mut leftover, rng);
        for cell in leftover {
            if picked.len() >= n {
                break;
            }
            picked.insert(cell);
        }
    }

    picked.into_iter().collect()
}

fn has_adjacent(cell: Coord, set: &HashSet<Coord>) -> bool {
    DIRS.iter().any(|&(dr, dc)| set.contains(&(cell.0 + dr, cell.1 + dc)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_xoshiro::Xoshiro256PlusPlus;

    fn make_rng(seed: u64) -> Xoshiro256PlusPlus {
        Xoshiro256PlusPlus::seed_from_u64(seed)
    }

    #[test]
    fn target_total_matches_piece_total_exactly() {
        let board = UnionBoard::new();
        for seed in 0..50 {
            for &piece_total in &[80usize, 150, 200, 215] {
                let mut rng = make_rng(seed);
                let result = build_target_cells(&board, piece_total, &mut rng);
                assert_eq!(
                    result.all_cells.len(), piece_total,
                    "expected {} cells, got {} (seed={})",
                    piece_total, result.all_cells.len(), seed
                );
            }
        }
    }

    #[test]
    fn selected_centers_are_valid_subset() {
        let board = UnionBoard::new();
        let mut rng = make_rng(42);
        for _ in 0..100 {
            let result = build_target_cells(&board, 200, &mut rng);
            assert!(
                (1..=4).contains(&result.center_cells.len()),
                "center count {} out of [1, 4]",
                result.center_cells.len()
            );
            for c in &result.center_cells {
                assert!(
                    board.center_cells.contains(c),
                    "selected center {:?} not in board.center_cells", c
                );
            }
        }
    }

    #[test]
    fn target_includes_all_selected_centers() {
        let board = UnionBoard::new();
        let mut rng = make_rng(42);
        for _ in 0..100 {
            let result = build_target_cells(&board, 200, &mut rng);
            for c in &result.center_cells {
                assert!(
                    result.all_cells.contains(c),
                    "selected center {:?} missing from target", c
                );
            }
        }
    }

    #[test]
    fn target_cells_are_within_board() {
        let board = UnionBoard::new();
        let board_set: HashSet<Coord> = board.all_cells.iter().copied().collect();
        let mut rng = make_rng(42);
        for _ in 0..50 {
            let result = build_target_cells(&board, 200, &mut rng);
            for c in &result.all_cells {
                assert!(board_set.contains(c), "target cell {:?} not on board", c);
            }
        }
    }

    #[test]
    fn target_cells_have_no_duplicates() {
        let board = UnionBoard::new();
        let mut rng = make_rng(42);
        for _ in 0..50 {
            let result = build_target_cells(&board, 200, &mut rng);
            let unique: HashSet<Coord> = result.all_cells.iter().copied().collect();
            assert_eq!(unique.len(), result.all_cells.len());
        }
    }

    #[test]
    fn deterministic_with_same_seed() {
        let board = UnionBoard::new();
        let r1 = build_target_cells(&board, 200, &mut make_rng(123));
        let r2 = build_target_cells(&board, 200, &mut make_rng(123));
        assert_eq!(r1.all_cells, r2.all_cells);
        assert_eq!(r1.center_cells, r2.center_cells);
    }
}