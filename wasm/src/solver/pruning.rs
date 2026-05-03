// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Feasibility checks that prune infeasible subtrees during backtracking.
//!
//! 3 checks are provided:
//! - [`parity_check`]: validates that the remaining black/white cell counts
//!   are reachable from the remaining pieces' contribution range.
//! - [`island_check`]: validates that uncovered cells partition into components
//!   whose sizes are expressible as sums of remaining piece sizes.
//! - [`neighbor_check`]: after a placement, validates that no adjacent uncovered cell
//!   has been left with zero valid placements.

use crate::domain::placement::Placement;
use crate::solver::{SearchState, TypeGroup};

/// Workspace buffers reused across [`island_check`] cells.
///
/// `island_check` performs a BFS and a subset-sum DP. The buffers grow but never shrink,
/// so steady-state allocation is 0.
pub(crate) struct IslandWorkspace {
    /// `visited[i]` is non-zero when call `i` has been seen by BFS in the current call.
    pub visited: Vec<u8>,

    /// BFS frontier stack. Reused across components.
    pub stack: Vec<u32>,

    /// Sizes of discovered components, 1 entry per component.
    pub comp_sizes: Vec<u32>,

    /// Subset-sum DP buffer: `dp[k]` is non-zero iff some subset of the remaining pieces
    /// sums to exactly `k` cells.
    pub dp: Vec<u8>
}

impl IslandWorkspace {
    /// Allocates buffers sized for `total_cells` board cells.
    pub(crate) fn new(total_cells: usize) -> Self {
        Self {
            visited: vec![0; total_cells],
            stack: Vec::with_capacity(total_cells),
            comp_sizes: Vec::with_capacity(64),
            dp: vec![0; total_cells + 1]
        }
    }

    /// Resets the visited bitmap and reusable scratch space.
    /// `dp` is reset inside [`island_check`] because its zeroing is scoped to
    /// the active subset-sum range, not the full buffer.
    fn reset_for_traversal(&mut self) {
        self.visited.iter_mut().for_each(|v| *v = 0);
        self.stack.clear();
        self.comp_sizes.clear();
    }
}

/// Parity (checkerboard color) feasibility check.
///
/// - `total_black` and `total_white`: total black/white cells on the board.
/// - `type_min_black[t]` and `type_max_black[t]`: min and max black-cell contribution
///   of any single placement of type `t`.
///
/// Returns `true` if the remaining-pieces contribution range to black cells
/// overlaps the required range of black cells still to cover.
#[inline]
pub(crate) fn parity_check(
    state: &SearchState,
    total_black: u16,
    total_white: u16,
    type_min_black: &[u8],
    type_max_black: &[u8]
) -> bool {
    let need_black = total_black.saturating_sub(state.covered_black);
    let covered_white = state.covered_count.saturating_sub(state.covered_black);
    if total_white < covered_white {
        // covered more white than exist; bug or invariant violation
        return false;
    }

    let mut pieces_min_black: u32 = 0;
    let mut pieces_max_black: u32 = 0;
    for (ti, &count) in state.remaining.iter().enumerate() {
        if count == 0 {
            continue;
        }

        pieces_min_black += count as u32 * type_min_black[ti] as u32;
        pieces_max_black += count as u32 * type_max_black[ti] as u32;
    }

    let need_black = need_black as u32;
    need_black >= pieces_min_black && need_black <= pieces_max_black
}

/// Island (connected component) feasibility check.
///
/// Partitions uncovered cells into 4-connected components via BFS.
/// For the subtree to be feasible, each component's size must be
/// expressible as a sum of some subset of the remaining piece sizes (subset-sum check).
///
/// - `adj_list[i]`: the precomputed adjacency list for cell `i`.
/// - `size_of_type[t]`: the number of cells in a placement of type `t`.
/// - `total_cells`: the total board cell count, used to bound the subset-sum DP range.
///
/// Returns `true` if the subtree may still be feasible.
pub(crate) fn island_check(
    state: &SearchState,
    adj_list: &[Vec<u16>],
    size_of_type: &[u8],
    total_cells: u16,
    workspace: &mut IslandWorkspace
) -> bool {
    // Trivial case: no cells covered yet, single component, no point checking.
    if state.covered_count == 0 {
        return true;
    }

    workspace.reset_for_traversal();

    // BFS over uncovered cells.
    for start in 0..total_cells as usize {
        if state.covered.test(start) || workspace.visited[start] != 0 {
            continue;
        }

        workspace.stack.push(start as u32);
        workspace.visited[start] = 1;
        let mut size: u32 = 0;

        while let Some(u) = workspace.stack.pop() {
            size += 1;
            let u_idx = u as usize;

            for &v_u16 in &adj_list[u_idx] {
                let v = v_u16 as usize;
                if !state.covered.test(v) && workspace.visited[v] == 0 {
                    workspace.visited[v] = 1;
                    workspace.stack.push(v_u16 as u32);
                }
            }
        }

        workspace.comp_sizes.push(size);
    }

    if workspace.comp_sizes.is_empty() {
        return true;
    }

    // Cheap pre-check: every component must accommodate at least one remaining piece,
    // so `min_remaining_size` must fit in every comp.
    let min_size = state.remaining
        .iter()
        .copied()
        .zip(size_of_type.iter().copied())
        .filter(|&(count, _)| count > 0)
        .map(|(_, sz)| sz as u32)
        .min();

    if let Some(min_size) = min_size {
        for &comp in &workspace.comp_sizes {
            if comp < min_size {
                return false;
            }
        }
    }

    // Subset-sum DP. Reachable sums of remaining pieces.
    let total_cap = (total_cells as usize) - state.covered_count as usize;
    workspace.dp[..=total_cap].fill(0);
    workspace.dp[0] = 1;

    for (ti, &count) in state.remaining.iter().enumerate() {
        if count == 0 {
            continue;
        }

        let sz = size_of_type[ti] as usize;
        for _ in 0..count {
            // Reverse iteration to avoid using a piece twice.
            for s in (sz..=total_cap).rev() {
                if workspace.dp[s - sz] != 0 {
                    workspace.dp[s] = 1;
                }
            }
        }
    }

    // Every component's size must appear in the DP table.
    for &comp in &workspace.comp_sizes {
        if workspace.dp[comp as usize] == 0 {
            return false;
        }
    }

    true
}

/// Neighbor (dead-cell) check after applying a placement.
///
/// For each cell adjacent to the placement (i.e., each bit set in `pl.neighbor_bits`),
/// checks that at least one valid placement of some remaining piece type still covers it.
///
/// - `cell_to_placements[c]`: a precomputed list of placement indices that cover cell `c`.
///   The check iterates these and tests overlap with `state.covered`.
/// - `placements`: the flat placement list (used to look up bits via index).
///
/// If any neighbor cell has zero such coverings, returns `false`.
#[inline]
pub(crate) fn neighbor_check(
    pl: &Placement,
    state: &SearchState,
    cell_pl_by_type: &[Vec<TypeGroup>],
    placements: &[Placement]
) -> bool {
    for &idx in &pl.neighbor_indices {
        let cell_idx = idx as usize;

        // Skip cells already covered by other placements.
        if state.covered.test(cell_idx) {
            continue;
        }

        let mut found = false;
        'groups: for group in &cell_pl_by_type[cell_idx] {
            // Group-level skip
            if state.remaining[group.type_idx as usize] == 0 {
                continue;
            }
            
            for &p_idx in &group.placements {
                let candidate = &placements[p_idx as usize];
                if !state.covered.has_overlap(&candidate.bits) {
                    found = true;
                    break 'groups;
                }
            }
        }

        if !found {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::BitSet;
    use crate::domain::piece::Coord;

    fn make_test_placement(
        type_idx: u16,
        cells: &[u16],
        b_count: u8,
        mark_on_center: bool,
    ) -> Placement {
        let mut bits = BitSet::new();
        for &i in cells {
            bits.set(i as usize);
        }
        Placement {
            type_idx,
            bits,
            neighbor_indices: Vec::new(),
            b_count,
            mark_on_center,
            cell_indices: cells.to_vec(),
            mark: (0, 0),
            cells: cells.iter().map(|&i| (0, i as i8)).collect::<Vec<Coord>>(),
        }
    }

    // ─── parity_check tests ───

    #[test]
    fn parity_check_passes_with_room_to_spare() {
        let state = SearchState::new(vec![2], 1);

        assert!(!parity_check(&state, 5, 5, &[1], &[2]));
    }

    #[test]
    fn parity_check_passes_when_in_range() {
        let state = SearchState::new(vec![2], 1);

        assert!(parity_check(&state, 3, 3, &[1], &[2]));
    }

    #[test]
    fn parity_check_rejects_too_few_black_pieces() {
        let state = SearchState::new(vec![2], 1);

        assert!(!parity_check(&state, 5, 1, &[1], &[2]));
    }

    // ─── island_check tests ───

    /// Build a tiny linear-graph adjacency list (chain of cells).
    fn make_chain_adj(n: usize) -> Vec<Vec<u16>> {
        (0..n)
            .map(|i| {
                let mut nb = vec![];
                if i > 0 {
                    nb.push((i - 1) as u16);
                }
                if i < n - 1 {
                    nb.push((i + 1) as u16);
                }
                nb
            })
            .collect()
    }

    #[test]
    fn island_check_trivially_passes_when_nothing_covered() {
        let state = SearchState::new(vec![1], 1);

        let adj = make_chain_adj(10);
        let mut ws = IslandWorkspace::new(10);

        assert!(island_check(&state, &adj, &[3], 10, &mut ws));
    }

    #[test]
    fn island_check_detects_unreachable_subset_sum() {
        let mut state = SearchState::new(vec![1, 1], 0);
        let pl = make_test_placement(0, &[3], 0, false);
        let _ = state.apply_placement(&pl, 0, false);

        let adj = make_chain_adj(6);
        let mut ws = IslandWorkspace::new(6);

        assert!(!island_check(&state, &adj, &[3, 1], 6, &mut ws));
    }

    #[test]
    fn island_check_passes_when_subset_sums_match() {
        let mut state = SearchState::new(vec![1, 1], 0);
        state.covered.set(3);
        state.covered_count = 1;

        let adj = make_chain_adj(6);
        let mut ws = IslandWorkspace::new(6);

        assert!(island_check(&state, &adj, &[3, 2], 6, &mut ws));
    }

    #[test]
    fn island_check_rejects_component_smaller_than_minimum_piece() {
        let mut state = SearchState::new(vec![2], 0);
        let pl1 = make_test_placement(0, &[0, 1], 0, false);
        let pl2 = make_test_placement(0, &[4], 0, false);
        let _ = state.apply_placement(&pl1, 0, false);
        let _ = state.apply_placement(&pl2, 1, false);

        let adj = make_chain_adj(5);
        let mut ws = IslandWorkspace::new(5);

        assert!(!island_check(&state, &adj, &[3], 5, &mut ws));
    }

    // ─── neighbor_check tests ───

    fn make_placement_with_neighbors(cells: &[u16], neighbors: &[u16]) -> Placement {
        let mut p = make_test_placement(0, cells, 0, false);
        p.neighbor_indices = neighbors.to_vec();
        p
    }
    
    fn singleton_group(type_idx: u16, placement_idx: u32) -> TypeGroup {
        TypeGroup { type_idx, placements: vec![placement_idx] }
    }

    #[test]
    fn neighbor_check_passes_when_neighbors_have_coverings() {
        let state = SearchState::new(vec![1], 1);
        let pl = make_placement_with_neighbors(&[10], &[5]);
        let other_placement = make_test_placement(0, &[5, 6], 0, false);
        let placements = vec![pl.clone(), other_placement];
        let cell_pl_by_type: Vec<Vec<TypeGroup>> = vec![
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![singleton_group(0, 1)], // cell 5 covered by placement 1, type 0
            vec![singleton_group(0, 1)], // cell 6 covered by placement 1, type 0
            vec![],
            vec![],
            vec![],
            vec![singleton_group(0, 0)], // cell 10 covered by placement 0, type 0
        ];

        assert!(neighbor_check(&pl, &state, &cell_pl_by_type, &placements));
    }

    #[test]
    fn neighbor_check_fails_when_neighbor_has_no_remaining_coverings() {
        let mut state = SearchState::new(vec![0], 0);
        state.remaining[0] = 0;

        let pl = make_placement_with_neighbors(&[10], &[5]);
        let other = make_test_placement(0, &[5], 0, false);
        let placements = vec![pl.clone(), other];
        let cell_pl_by_type: Vec<Vec<TypeGroup>> = vec![
            vec![], vec![], vec![], vec![], vec![],
            vec![singleton_group(0, 1)],
            vec![], vec![], vec![], vec![],
            vec![singleton_group(0, 0)]
        ];

        assert!(!neighbor_check(&pl, &state, &cell_pl_by_type, &placements));
    }

    #[test]
    fn neighbor_check_skips_already_covered_neighbors() {
        let mut state = SearchState::new(vec![0], 0);
        state.covered.set(5);

        let pl = make_placement_with_neighbors(&[10], &[5]);
        let placements = vec![pl.clone()];
        let cell_to_placements = vec![
            vec![], vec![], vec![], vec![], vec![],
            vec![],
            vec![], vec![], vec![], vec![], vec![],
        ];

        assert!(neighbor_check(&pl, &state, &cell_to_placements, &placements));
    }
}