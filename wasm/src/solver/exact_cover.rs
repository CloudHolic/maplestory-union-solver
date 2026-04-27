// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! ExactCover solver: backtracking search for exact-cover packing.
//! See `docs/algorithms/exact-cover.md` for the algorithmic background.

use std::collections::HashMap;

use crate::base::{BitSet, CAPACITY, LubyIterator, SolverRng, make_rng, shuffle};
use crate::domain::{
    BoardLayout, PieceInstance, enumerate_all_placements,
    Coord, Placement
};
use crate::error::{Result, SolverError};
use crate::io::{ExactCoverInput, ExactCoverResult, ExactCoverStats, Solution};
use crate::solver::{IslandWorkspace, SearchState, PlacementUndo, island_check, neighbor_check, parity_check};


// ─── Solve options ──────────────────────────────────────────────────

/// Options controlling solver execution.
#[derive(Debug, Clone)]
pub struct SolveOptions {
    /// Wall-clock timeout. Solver returns without a solution if this elapses,
    /// with `timed_out: true` in stats.
    pub timeout_ms: Option<u64>,

    /// Random seed for the PRNG. If `None`, a seed is drawn from system entropy.
    pub seed: Option<u64>,

    /// Base node budget for the Luby restart sequence.
    /// Each restart gets `luby_base * t_i` nodes, where `t_i` is the i-th Luby term.
    pub luby_base: u64
}

impl Default for SolveOptions {
    fn default() -> Self {
        Self {
            timeout_ms: None,
            seed: None,
            luby_base: 100_000
        }
    }
}


// ─── Solve context (precomputed, immutable during search) ─────────────

/// All precomputed data for a single solve.
struct SolveContext {
    /// All cells of the target region, in index order.
    board: BoardLayout,

    /// Total target cell count (== `board.cells.len()`).
    total_cells: u16,

    /// Total black/white cells on the board (precomputed for parity).
    total_black: u16,
    total_white: u16,

    /// Type IDs in the order assigned by setup.
    type_ids: Vec<String>,

    /// Number of instances per type.
    type_counts: Vec<u16>,

    /// Cell count of each piece type.
    size_of_type: Vec<u8>,

    /// Min black-cell contribution across all variants of type `ti`.
    /// Used by parity check.
    type_min_black: Vec<u8>,

    /// Max black-cell contribution across all variants of type `ti`.
    type_max_black: Vec<u8>,

    /// Number of placements per type that land a mark on the center region.
    center_mark_counts: Vec<u16>,

    /// All valid placements, in a flat list.
    placements: Vec<Placement>,

    /// `cell_to_placements[c]` lists indices into `placements` of every placement
    /// that covers cell `c`.
    /// Used by the pruner's neighbor check and by the per-cell MRV scan.
    cell_to_placements: Vec<Vec<u32>>,

    /// 4-connectivity adjacency list over board cells.
    /// `adj_list[i]` is the list of board-cell indices orthogonally adjacent to `i`.
    /// Used by the island check.
    adj_list: Vec<Vec<u16>>
}

impl SolveContext {
    /// Builds the context from parsed input.
    fn build(input: &ExactCoverInput) -> Result<Self> {
        // 1. Parse cell coordinates
        let target_cells = input.parse_target_cells()?;
        let center_cells = input.common.parse_center_cells()?;
        let board = BoardLayout::new(target_cells, center_cells)?;
        let total_cells = board.cells.len() as u16;

        if total_cells as usize > CAPACITY {
            return Err(SolverError::BoardTooLarge {
                actual: total_cells as usize,
                capacity: CAPACITY
            });
        }

        let mut total_black: u16 = 0;
        let mut total_white: u16 = 0;
        for &color in &board.cell_color {
            if color == 0 {
                total_black += 1;
            } else {
                total_white += 1;
            }
        }

        // 2. Build per-cell adjacency list
        let adj_list = build_adjacency(&board);

        // 3. Assign type indices to piece instances
        let mut type_id_to_idx: HashMap<String, u16> = HashMap::new();
        let mut type_ids: Vec<String> = Vec::new();
        let mut type_counts: Vec<u16> = Vec::new();
        let mut piece_instances: Vec<PieceInstance> = Vec::with_capacity(input.common.pieces.len());

        for piece in &input.common.pieces {
            let ti = match type_id_to_idx.get(&piece.def_id) {
                Some(&ti) => ti,
                None => {
                    let ti = type_ids.len() as u16;
                    type_id_to_idx.insert(piece.def_id.clone(), ti);
                    type_ids.push(piece.def_id.clone());
                    type_counts.push(0);
                    ti
                }
            };

            type_counts[ti as usize] += 1;
            piece_instances.push(PieceInstance {
                type_idx: ti,
                def_id: piece.def_id.clone()
            });
        }

        let num_types = type_ids.len();

        // 4. Compute size_of_type
        let piece_defs_map = input.common.piece_defs_map();
        let mut size_of_type: Vec<u8> = Vec::with_capacity(num_types);

        for id in &type_ids {
            let def = piece_defs_map
                .get(id)
                .ok_or_else(|| SolverError::UnknownPieceDef { id: id.clone() })?;
            size_of_type.push(def.cells.len() as u8);
        }

        // 5. Cell-count consistency: Σ size_of_type * type_counts == total_cells
        let total_piece_cells: u32 = (0..num_types)
            .map(|ti| size_of_type[ti] as u32 * type_counts[ti] as u32)
            .sum();

        if total_piece_cells != total_cells as u32 {
            return Err(SolverError::PieceCellMismatch {
                piece_cells: total_piece_cells as usize,
                target_cells: total_cells as usize,
            });
        }

        // 6. Enumerate all valid placements
        let placements = enumerate_all_placements(&piece_instances, &piece_defs_map, &board)?;

        // Reject inputs where no placement can satisfy the center-mark constraint.
        if !placements.iter().any(|p| p.mark_on_center) {
            return Err(SolverError::NoCenterMarkPossible);
        }

        // 7. Precompute parity tables (min/max black per type)
        let mut type_min_black: Vec<u8> = vec![u8::MAX; num_types];
        let mut type_max_black: Vec<u8> = vec![0; num_types];

        for pl in &placements {
            let ti = pl.type_idx as usize;

            if pl.b_count < type_min_black[ti] {
                type_min_black[ti] = pl.b_count;
            }

            if pl.b_count > type_max_black[ti] {
                type_max_black[ti] = pl.b_count;
            }
        }

        // 8. Center-mark counts per type
        let mut center_mark_counts: Vec<u16> = vec![0; num_types];
        for pl in &placements {
            if pl.mark_on_center {
                center_mark_counts[pl.type_idx as usize] += 1;
            }
        }

        // 9. Build per-cell and per-type placement indices
        let mut cell_to_placements: Vec<Vec<u32>> = (0..total_cells as usize).map(|_| Vec::new()).collect();
        for (pi, pl) in placements.iter().enumerate() {
            let pi32 = pi as u32;
            for &cell_idx in &pl.cell_indices {
                cell_to_placements[cell_idx as usize].push(pi32);
            }
        }

        Ok(Self {
            board,
            total_cells,
            total_black,
            total_white,
            type_ids,
            type_counts,
            size_of_type,
            type_min_black,
            type_max_black,
            center_mark_counts,
            placements,
            cell_to_placements,
            adj_list
        })
    }

    /// Computes the initial value of `center_mark_type_remaining`.
    fn initial_center_mark_type_remaining(&self) -> u16 {
        (0..self.type_ids.len())
            .filter(|&ti| {
                self.type_counts[ti] > 0 && self.center_mark_counts[ti] > 0
            })
            .count() as u16
    }
}

/// Builds the 4-connectivity adjacency list for a `BoardLayout`.
fn build_adjacency(board: &BoardLayout) -> Vec<Vec<u16>> {
    const DIRS: [(i8, i8); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    let mut adj: Vec<Vec<u16>> = (0..board.cells.len()).map(|_| Vec::with_capacity(4)).collect();

    for (i, &(r, c)) in board.cells.iter().enumerate() {
        for (dr, dc) in DIRS {
            let nb = (r + dr, c + dc);
            if let Some(&nb_idx) = board.coord_to_idx.get(&nb) {
                adj[i].push(nb_idx);
            }
        }
    }

    adj
}

// ─── Backtracking environment ─────────────────────────────────────────

/// Mutable per-restart environment for the backtracking search.
struct BacktrackEnv {
    /// Search state mutated during backtracking.
    state: SearchState,

    /// Workspace buffers reused across island_check cells.
    island_ws: IslandWorkspace,

    /// Cell ordering used by the MRV scan. Shuffled at each restart;
    /// during a single solve, traversed in this order.
    cell_order: Vec<u16>,

    /// Number of recursion-tree nodes visited in the current restart.
    /// Compared against `node_budget` for early termination.
    nodes_this_restart: u64,

    /// Maximum nodes allowed in the current restart. From the Luby schedule.
    node_budget: u64,

    /// Set to `true` when `nodes_this_restart` reaches `node_budget`.
    /// Each backtrack call observes this and unwinds promptly.
    budget_exhausted: bool
}

impl BacktrackEnv {
    fn new(ctx: &SolveContext) -> Self {
        let cell_order: Vec<u16> = (0..ctx.total_cells).collect();
        let initial_cmtr = ctx.initial_center_mark_type_remaining();

        Self {
            state: SearchState::new(ctx.type_counts.clone(), initial_cmtr),
            island_ws: IslandWorkspace::new(ctx.total_cells as usize),
            cell_order,
            nodes_this_restart: 0,
            node_budget: 0,
            budget_exhausted: false
        }
    }

    /// Resets state and counters before a new restart, but reuses allocations.
    fn reset_for_restart(&mut self, ctx: &SolveContext, node_budget: u64, rng: &mut SolverRng) {
        let initial_cmtr = ctx.initial_center_mark_type_remaining();

        self.state.reset(&ctx.type_counts, initial_cmtr);
        self.nodes_this_restart = 0;
        self.node_budget = node_budget;
        self.budget_exhausted = false;

        shuffle(&mut self.cell_order, rng);
    }
}

// ─── Backtracking core ────────────────────────────────────────────────

/// Result of a single MRV scan over uncovered cells.
enum MrvOutcome {
    /// Some cell has no valid placements. The subtree is dead.
    DeadCell,

    /// Some cell has exactly one valid placement; that placement is forced.
    /// The unit-propagation cascade applies it and rescans.
    Unit { placement_idx: u32 },

    /// All uncovered cells have ≥ 2 valid placements.
    /// Branch on the cell with the fewest placements.
    Branch { cell_idx: u16, count: u32 },

    /// No uncovered cells remain.
    AllCovered
}

/// Recursive backtracking. Returns `true` when a solution is found.
///
/// At each node:
///
/// 1. Budget and termination checks.
/// 2. Pruning checks (parity, island).
/// 3. Unit propagation cascade: while some uncovered cell
///    has exactly one valid placement, apply it and rescan.
/// 4. Branch on the MRV cell: try its candidate placements one by one,
///    recursing into each.
fn backtrack(ctx: &SolveContext, env: &mut BacktrackEnv) -> bool {
    env.nodes_this_restart += 1;
    if env.nodes_this_restart >= env.node_budget {
        env.budget_exhausted = true;
        return false;
    }

    // Termination: all target cells covered.
    if env.state.is_fully_covered(ctx.total_cells) {
        return env.state.has_center_mark;
    }

    // Quick infeasibility: no remaining type can satisfy center-mark.
    if env.state.center_mark_unreachable() {
        return false;
    }

    // Global pruning checks.
    if !parity_check(&env.state, ctx.total_black, ctx.total_white, &ctx.type_min_black, &ctx.type_max_black) {
        return false;
    }
    if !island_check(&env.state, &ctx.adj_list, &ctx.size_of_type, ctx.total_cells, &mut env.island_ws) {
        return false;
    }

    // Unit propagation cascade. Each iteration scans for unit/dead/branch cells;
    // on a unit, applies it and continues;
    // on dead or branch, proceeds out of the cascade.
    let mut cascade_depth: u32 = 0;
    let branch = loop {
        match scan_uncovered(ctx, env) {

        }
    }
}

/// Undoes the last `depth` placements pushed on to the result stack during a unit-propagation cascade.
///
///
fn undo_cascade(ctx: &SolveContext, env: &mut BacktrackEnv, depth: u32) {
    for _ in 0..depth {
        // The most recently pushed result entry is the placement we applied.
        // Pop it and reconstruct the undo token.
        let p_idx = match env.state.result.last().copied() {
            Some(idx) => idx,
            None => return
        };
        let pl = &ctx.placements[p_idx];
        
        // Reconstruct undo: prev_has_center_mark must be recomputed from currently-applied placements.
        let pl_is_center = pl.mark_on_center;
        let prev_has_center_mark = if pl_is_center {
            // Was there a previously-applied center-mark placement?
            env.state.result[..env.state.result.len() - 1]
                .iter()
                .any(|&p| ctx.placements[p].mark_on_center)
        } else {
            // This placement didn't change has_center_mark; preserve it.
            env.state.has_center_mark
        };
        
        // Reconstruct center_mark_type_drop: applying pl decrements the counter
        // iff after this apply, type[pl.type_idx]'s remaining hit zero
        // AND that type had any center-mark-eligible placements.
        let ti = pl.type_idx as usize;
        let center_mark_type_drop = env.state.remaining[ti] == 0 && ctx.center_mark_counts[ti] > 0;
        
        let undo_token = PlacementUndo {
            prev_has_center_mark,
            center_mark_type_drop
        };
        
        env.state.undo_placement(pl, undo_token);
    }
}

/// Scans uncovered cells once. Returns the strongest outcome found:
/// `DeadCell` > `Unit` > `Branch` > `AllCovered`.
fn scan_uncovered(ctx: &SolveContext, env: &mut BacktrackEnv) -> MrvOutcome {

}

/// Returns `true` if applying `pl` will cause the `center_mark_type_remaining` counter to drop.
#[inline]
fn will_drop_center_mark_type(ctx: &SolveContext, state: &SearchState, pl: &Placement) -> bool {
    let ti = pl.type_idx as usize;
    state.remaining[ti] == 1 && ctx.center_mark_counts[ti] > 0
}

// ─── Public entry point ───────────────────────────────────────────────

pub fn solve_exact_cover(
    input: &ExactCoverInput,
    options: SolveOptions
) -> Result<ExactCoverResult> {
    let _ctx = SolveContext::build(input)?;
    let seed = options.seed.unwrap_or_else(rand::random::<u64>);

    Ok(ExactCoverResult {
        solution: None,
        stats: ExactCoverStats::empty(seed)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::{PieceDefJson, PieceInstanceJson, SolverInput};

    /// Build a tiny test input: 2x2 board, one 2x2 piece, center cell
    /// at (0,0). Used to exercise SolveContext::build end-to-end.
    fn make_2x2_input() -> ExactCoverInput {
        ExactCoverInput {
            target_cells: vec![
                "0,0".to_string(),
                "0,1".to_string(),
                "1,0".to_string(),
                "1,1".to_string(),
            ],
            common: SolverInput {
                pieces: vec![PieceInstanceJson {
                    def_id: "square".to_string(),
                    index: 0,
                }],
                piece_defs: vec![(
                    "square".to_string(),
                    PieceDefJson {
                        id: "square".to_string(),
                        cells: vec![(0, 0), (0, 1), (1, 0), (1, 1)],
                        mark_index: 0,
                    },
                )],
                center_cells: vec!["0,0".to_string()],
            },
        }
    }

    #[test]
    fn build_context_succeeds_for_well_formed_input() {
        let input = make_2x2_input();
        let ctx = SolveContext::build(&input).unwrap();

        assert_eq!(ctx.total_cells, 4);
        assert_eq!(ctx.type_ids, vec!["square".to_string()]);
        assert_eq!(ctx.type_counts, vec![1]);
        assert_eq!(ctx.size_of_type, vec![4]);
        // 2x2 board: 2 black cells (0,0), (1,1) and 2 white cells (0,1), (1,0)
        assert_eq!(ctx.total_black, 2);
        assert_eq!(ctx.total_white, 2);
        // The square has only one variant; b_count = 2.
        assert_eq!(ctx.type_min_black, vec![2]);
        assert_eq!(ctx.type_max_black, vec![2]);
        // It should produce one placement (the one on the board), and
        // since (0,0) is on center, mark_on_center is true.
        assert_eq!(ctx.placements.len(), 1);
        assert_eq!(ctx.center_mark_counts, vec![1]);
    }

    #[test]
    fn build_rejects_piece_cell_mismatch() {
        let mut input = make_2x2_input();
        // Add a duplicate piece, doubling total_piece_cells but not target_cells.
        input.common.pieces.push(PieceInstanceJson {
            def_id: "square".to_string(),
            index: 1,
        });

        let result = SolveContext::build(&input);
        assert!(matches!(
            result,
            Err(SolverError::PieceCellMismatch { .. })
        ));
    }

    #[test]
    fn build_rejects_no_center_mark_possible() {
        let mut input = make_2x2_input();
        // Move center to a cell off the board.
        input.common.center_cells = vec!["99,99".to_string()];

        let result = SolveContext::build(&input);
        assert!(matches!(result, Err(SolverError::NoCenterMarkPossible)));
    }

    #[test]
    fn initial_center_mark_type_remaining_counts_correctly() {
        let input = make_2x2_input();
        let ctx = SolveContext::build(&input).unwrap();
        assert_eq!(ctx.initial_center_mark_type_remaining(), 1);
    }

    #[test]
    fn solve_exact_cover_stub_returns_empty_result() {
        // The stub returns no solution. Verifies plumbing is wired up.
        let input = make_2x2_input();
        let result = solve_exact_cover(&input, SolveOptions::default()).unwrap();
        assert!(result.solution.is_none());
    }
}