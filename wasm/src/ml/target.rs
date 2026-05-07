// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Synthesizes target cell selections matching observed MapleStory player patterns.

use std::collections::{HashMap, HashSet};
use rand::Rng;
use rand::RngExt;
use rand::seq::IndexedRandom;

use crate::base::{DIRS, shuffle};
use crate::domain::Coord;
use crate::ml::{Group, GroupId, UnionBoard};

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

    // 1. Center cells.
    let center_count = sample_center_count(rng);
    let mut selected_centers: Vec<Coord> = board.center_cells
        .sample(rng, center_count)
        .copied()
        .collect();
    selected_centers.sort();

    // 2. Inner groups containing selected centers.
    let cell_to_group: HashMap<Coord, &Group> = board.groups.iter()
        .flat_map(|g| g.cells.iter().map(move |c| (*c, g)))
        .collect();
    let center_inner_ids: HashSet<GroupId> = selected_centers.iter()
        .map(|c| cell_to_group[c].id)
        .collect();

    // 3. Priority list of groups.
    let priority = build_priority_list(board, &center_inner_ids, rng);

    // 4. Fill cells.
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
    center_inner_ids: &HashSet<GroupId>,
    rng: &mut impl Rng
) -> Vec<&'a Group> {
    let mut priority: Vec<&'a Group> = Vec::new();

    // tier 1: outer N1 ∈ [2, 6]
    let n1 = rng.random_range(2..=6);
    let mut outer: Vec<&Group> = board.outer_groups().collect();
    shuffle(&mut outer, rng);
    let n1_take = n1.min(outer.len());
    priority.extend(outer[..n1_take].iter().copied());

    // tier 2: inner groups containing selected centers
    for g in board.inner_groups() {
        if center_inner_ids.contains(&g.id) {
            priority.push(g);
        }
    }

    // tier 3: other inner N2 ∈ [1, 4]
    let n2 = rng.random_range(1..=4);
    let mut inner_rest: Vec<&Group> = board.inner_groups()
        .filter(|g| !center_inner_ids.contains(&g.id))
        .collect();
    shuffle(&mut inner_rest, rng);
    let n2_take = n2.min(inner_rest.len());
    priority.extend(inner_rest[..n2_take].iter().copied());

    // tier 4: overflow (any remaining group)
    let mut tier4: Vec<&Group> = outer[n1_take..].iter().copied().collect();
    tier4.extend(inner_rest[n2_take..].iter().copied());
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