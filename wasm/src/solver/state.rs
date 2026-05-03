// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Mutable search state during backtracking.
//!
//! Encapsulates all fields that change as placements are applied and undone.

use crate::base::BitSet;
use crate::domain::placement::Placement;

/// Mutable state during backtracking search.
/// - `covered`: which board cells are currently occupied by some placement.
///   `bits` set <-> cell covered.
/// - `remaining`: per piece-type, how many instances are still unplaced.
///   Indexed by `Placement::type_idx`.
/// - `covered_count`: cardinality of `covered`. Maintained alongside the bitset
///   to avoid recounting.
/// - `covered_black`: number of checkerboard-black cells in `covered`.
///   Used by parity prune.
/// - `has_center_mark`: `true` once any placed piece's mark cell falls
///   on the center region. Once `true`, never resets within a subtree
///   until that placement is undone.
/// - `center_mark_type_remaining`: count of piece types that (a) still have
///   remaining instances and (b) at least one of those instances could land a mark
///   on the center region. When this reaches 0 without `has_center_mark` being true,
///   the subtree is infeasible.
/// - `result`: stack of currently-applied placement indices, in apply order.
///   A complete solution is the contents of this stack.
#[derive(Debug)]
pub(crate) struct SearchState {
    pub(crate) covered: BitSet,
    pub(crate) remaining: Vec<u16>,
    pub(crate) covered_count: u16,
    pub(crate) covered_black: u16,
    pub(crate) has_center_mark: bool,
    pub(crate) center_mark_type_remaining: u16,
    pub(crate) result: Vec<usize>
}

impl SearchState {
    /// Creates an empty state for a problem with `num_types` piece types
    /// and `type_counts[i]` instances of each type.
    pub(crate) fn new(type_counts: Vec<u16>, center_mark_type_remaining: u16) -> Self {
        Self {
            covered: BitSet::new(),
            remaining: type_counts,
            covered_count: 0,
            covered_black: 0,
            has_center_mark: false,
            center_mark_type_remaining,
            result: Vec::with_capacity(64)
        }
    }

    /// Resets the state to "no placements applied".
    pub(crate) fn reset(&mut self, type_counts: &[u16], center_mark_type_remaining: u16) {
        self.covered.reset();
        self.remaining.clear();
        self.remaining.extend_from_slice(type_counts);
        self.covered_count = 0;
        self.covered_black = 0;
        self.has_center_mark = false;
        self.center_mark_type_remaining = center_mark_type_remaining;
        self.result.clear();
    }

    /// Returns `true` if no piece type with at least one remaining instance
    /// can satisfy the center-mark constraint.
    #[inline]
    pub(crate) fn center_mark_unreachable(&self) -> bool {
        !self.has_center_mark && self.center_mark_type_remaining == 0
    }

    /// Returns `true` if `count` cells are covered.
    #[inline]
    pub(crate) fn is_fully_covered(&self, target_count: u16) -> bool {
        self.covered_count == target_count
    }

    /// Applies a placement: marks its cells covered, decrements the remaining count
    /// for its type, updates parity and center-mark state, and pushes the placement
    /// index onto the result stack.
    ///
    /// # Preconditions
    /// - `pl.bits` must not overlap with `self.covered`.
    /// - `self.remaining[pl.type_idx]` must be > 0.
    /// - `placement_index` is the index of this placement in the solver's flat placement list,
    ///   used for solution reconstruction.
    #[inline]
    pub(crate) fn apply_placement(
        &mut self,
        pl: &Placement,
        placement_index: usize,
        center_mark_type_drop: bool
    ) -> PlacementUndo {
        debug_assert!(
            !self.covered.has_overlap(&pl.bits),
            "apply_placement: bits overlap with covered"
        );

        debug_assert!(
            self.remaining[pl.type_idx as usize] > 0,
            "apply_placement: type {} has no remaining instances",
            pl.type_idx
        );

        let prev_has_center_mark = self.has_center_mark;
        let prev_center_mark_type_drop = center_mark_type_drop;

        self.covered.apply(&pl.bits);
        self.remaining[pl.type_idx as usize] -= 1;
        self.covered_count += pl.cell_indices.len() as u16;
        self.covered_black += pl.b_count as u16;

        if pl.mark_on_center {
            self.has_center_mark = true;
        }

        if prev_center_mark_type_drop {
            self.center_mark_type_remaining -= 1;
        }

        self.result.push(placement_index);

        PlacementUndo {
            prev_has_center_mark,
            center_mark_type_drop: prev_center_mark_type_drop
        }
    }

    /// Undoes the most recent `apply_placement` call.
    pub(crate) fn undo_placement(&mut self, pl: &Placement, undo: PlacementUndo) {
        debug_assert!(
            self.result.last().is_some(),
            "undo_placement: result stack is unexpectedly empty"
        );

        self.result.pop();
        if undo.center_mark_type_drop {
            self.center_mark_type_remaining += 1;
        }

        self.has_center_mark = undo.prev_has_center_mark;
        self.covered_black -= pl.b_count as u16;
        self.covered_count -= pl.cell_indices.len() as u16;
        self.remaining[pl.type_idx as usize] += 1;
        self.covered.clear_bits(&pl.bits);
    }
}

/// Token returned by [`SearchState::apply_placement`] and consumed by [`SearchState::undo_placement`].
/// Captures state values that cannot be recovered from the placement alone:
/// - `prev_has_center_mark`: whether any prior placement had already satisfied
///   the center-mark constraint.
/// - `center_mark_type_drop`: whether this applies caused the `center_mark_type_remaining`
///   counter to decrement.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PlacementUndo {
    pub(crate) prev_has_center_mark: bool,
    pub(crate) center_mark_type_drop: bool
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::piece::Coord;

    fn make_test_placement(type_idx: u16, mark_on_center: bool, cells: &[u16]) -> Placement {
        let mut bits = BitSet::new();
        for &i in cells {
            bits.set(i as usize);
        }

        let cells_owned: Vec<u16> = cells.to_vec();
        let coord_cells: Vec<Coord> = cells.iter().map(|&i| (0, i as i8)).collect();

        Placement {
            type_idx,
            bits,
            neighbor_indices: Vec::new(),
            b_count: 0,
            mark_on_center,
            cell_indices: cells_owned,
            mark: (0, 0),
            cells: coord_cells,
        }
    }

    #[test]
    fn new_state_is_empty() {
        let state = SearchState::new(vec![3, 5], 2);
        assert!(state.covered.is_empty());
        assert_eq!(state.remaining, vec![3, 5]);
        assert_eq!(state.covered_count, 0);
        assert_eq!(state.covered_black, 0);
        assert!(!state.has_center_mark);
        assert_eq!(state.center_mark_type_remaining, 2);
        assert!(state.result.is_empty());
    }

    #[test]
    fn apply_and_undo_restores_state() {
        let mut state = SearchState::new(vec![2, 1], 1);
        let pl = make_test_placement(0, false, &[5, 6, 7]);

        let snapshot = (
            state.covered,
            state.remaining.clone(),
            state.covered_count,
            state.has_center_mark,
            state.center_mark_type_remaining,
            state.result.clone(),
        );

        let undo = state.apply_placement(&pl, 42, false);
        assert_eq!(state.covered_count, 3);
        assert_eq!(state.remaining[0], 1);
        assert_eq!(state.result, vec![42]);
        assert!(state.covered.test(5));

        state.undo_placement(&pl, undo);
        assert_eq!(state.covered, snapshot.0);
        assert_eq!(state.remaining, snapshot.1);
        assert_eq!(state.covered_count, snapshot.2);
        assert_eq!(state.has_center_mark, snapshot.3);
        assert_eq!(state.center_mark_type_remaining, snapshot.4);
        assert_eq!(state.result, snapshot.5);
    }

    #[test]
    fn apply_with_center_mark_flips_flag() {
        let mut state = SearchState::new(vec![1], 1);
        let pl = make_test_placement(0, true, &[10]);

        let undo = state.apply_placement(&pl, 0, false);
        assert!(state.has_center_mark);

        state.undo_placement(&pl, undo);
        assert!(!state.has_center_mark);
    }

    #[test]
    fn nested_apply_undo_works() {
        let mut state = SearchState::new(vec![3], 1);
        let pl1 = make_test_placement(0, false, &[0, 1]);
        let pl2 = make_test_placement(0, true, &[2, 3]);

        let u1 = state.apply_placement(&pl1, 0, false);
        let u2 = state.apply_placement(&pl2, 1, false);

        assert_eq!(state.covered_count, 4);
        assert_eq!(state.remaining[0], 1);
        assert!(state.has_center_mark);
        assert_eq!(state.result, vec![0, 1]);

        state.undo_placement(&pl2, u2);
        assert_eq!(state.covered_count, 2);
        assert!(!state.has_center_mark);
        assert_eq!(state.result, vec![0]);

        state.undo_placement(&pl1, u1);
        assert_eq!(state.covered_count, 0);
        assert_eq!(state.remaining[0], 3);
        assert!(state.result.is_empty());
    }

    #[test]
    fn center_mark_type_drop_decrements_and_restores() {
        let mut state = SearchState::new(vec![1, 2], 2);
        let pl = make_test_placement(0, false, &[0]);

        // Simulate: applying this placement causes type 0 to drop to 0
        // remaining, and type 0 had center-mark placements, so the count
        // of "types that can still satisfy center-mark" drops by 1.
        let undo = state.apply_placement(&pl, 0, true);
        assert_eq!(state.center_mark_type_remaining, 1);

        state.undo_placement(&pl, undo);
        assert_eq!(state.center_mark_type_remaining, 2);
    }

    #[test]
    fn center_mark_unreachable_detects_dead_state() {
        let mut state = SearchState::new(vec![1], 0);
        assert!(state.center_mark_unreachable());

        // If a has_center_mark is true, it's not unreachable.
        state.has_center_mark = true;
        assert!(!state.center_mark_unreachable());

        // If a center-mark-capable type remains, it's not unreachable.
        state.has_center_mark = false;
        state.center_mark_type_remaining = 1;
        assert!(!state.center_mark_unreachable());
    }

    #[test]
    fn reset_clears_all_mutable_fields() {
        let mut state = SearchState::new(vec![2, 1], 2);
        let pl = make_test_placement(0, true, &[5, 6]);
        let _ = state.apply_placement(&pl, 0, true);

        state.reset(&[2, 1], 2);
        assert!(state.covered.is_empty());
        assert_eq!(state.remaining, vec![2, 1]);
        assert_eq!(state.covered_count, 0);
        assert!(!state.has_center_mark);
        assert_eq!(state.center_mark_type_remaining, 2);
        assert!(state.result.is_empty());
    }

    #[test]
    fn is_fully_covered_compares_count() {
        let mut state = SearchState::new(vec![2], 1);
        assert!(!state.is_fully_covered(5));

        let pl = make_test_placement(0, false, &[0, 1, 2, 3, 4]);
        let _ = state.apply_placement(&pl, 0, false);
        assert!(state.is_fully_covered(5));
        assert!(!state.is_fully_covered(6));
    }
}