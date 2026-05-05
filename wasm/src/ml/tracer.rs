// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Branch-trace collector for ML training-data generation.

use std::io::{Result, Write};
use std::mem::take;

use crate::domain::{Coord, PieceDef, Placement};
use crate::io::PieceDefJson;
use crate::ml::{BITMAP_SIZE, GRID_COLS, canonical_5x5_bitmap};
use crate::solver::SearchState;

/// Information passed to [`Tracer::on_branch`] at every branch point.
///
/// The tracer borrows everything it needs from the solver to compute
/// per-candidate post-state features.
pub(crate) struct BranchEvent<'a> {
    /// Placement indices the solver will try, in attempt order.
    /// Already filtered by overlap-free + neighbor-check.
    pub candidates: &'a [u32],

    /// Current solver state immediately before any candidate is applied.
    pub state: &'a SearchState,

    /// First placement list (for index lookups during post-state computation).
    pub placements: &'a [Placement]
}

/// Branch-trace collector.
///
/// Lifecycle (called by the solver under `--features tracing`):
/// 1. [`Tracer::set_instance_id`] — set the instance label.
/// 2. [`Tracer::start_instance`] — provide per-instance constants and
///    reset per-instance state.
/// 3. [`Tracer::on_branch`] — once per branch point. Records the
///    candidate set and computes post-state features for each.
/// 4. [`Tracer::on_attempt`] — after each candidate's recursion returns.
///    Records `tried`, `succeeded`, and `subtree_nodes`.
/// 5. [`Tracer::on_solve_complete`] — once per instance. On success,
///    finalizes the buffered records and writes them as JSONL.
pub struct Tracer {
    /// JSONL output sink. Used at [`Tracer::on_solve_complete`] on success.
    writer: Box<dyn Write>,

    // ─── Per-instance constants (set in start_instance) ───

    /// Canonical 5x5 bitmaps for every piece_def, in input piece_defs order.
    canonical_bitmaps: Vec<[u8; BITMAP_SIZE]>,

    /// Maps board cell index -> row-major grid index (`r * GRID_COLS + c`).
    cell_to_grid_idx: Vec<u16>,

    /// For each piece_def (in input order), the solver's internal type index.
    /// `None` if the def is not used by any piece instance.
    type_idx_of_def: Vec<Option<u16>>,

    /// Identifier for the current instance.
    instance_id: String,

    // ─── Mutable per-instance state ───

    /// Counter assigned to each branch within the current instance.
    next_branch_id: u32,

    /// `total_nodes` value at the moment of the most recent on_branch
    /// or on_attempt call, used as the baseline for subtree node counting.
    nodes_at_branch_entry: u64,

    /// Finalized branch records for the current instance.
    /// Dumped as JSONL on success, discarded on failure.
    instance_buffer: Vec<BranchRecord>,

    /// Most-recent branch's pending candidate records.
    pending_candidates: Vec<CandidateRecord>,
}

/// One row of the JSONL output.
struct BranchRecord {
    branch_id: u32,
    candidates: Vec<CandidateRecord>
}

/// One candidate within a [`BranchRecord`].
struct CandidateRecord {
    placement_idx: u32,
    post_state: PostState,
    tried: bool,
    succeeded: bool,
    subtree_nodes: u64
}

/// Solver state after virtually applying a single candidate placement.
struct PostState {
    /// Grid indices (row-major, `r & GRID_COLS + c`) of cells still empty
    /// after applying the candidate.
    empty_target_indices: Vec<u16>,

    /// `true` if any placement marking the center has been applied
    /// (including the candidate itself).
    center_mark: bool,

    /// Remaining piece count per piece_def, in input piece_defs order.
    /// Length matches [`Tracer::canonical_bitmaps`].
    /// Defs not used by any solver type contribute `0`.
    counts: Vec<u32>
}

impl Tracer {
    pub(crate) fn new(writer: Box<dyn Write>, total_cells: u16) -> Self {
        Self {
            writer,
            canonical_bitmaps: Vec::new(),
            cell_to_grid_idx: Vec::new(),
            type_idx_of_def: Vec::new(),
            instance_id: String::new(),
            next_branch_id: 0,
            nodes_at_branch_entry: 0,
            instance_buffer: Vec::new(),
            pending_candidates: Vec::new(),
        }
    }

    /// Sets the instance identifier embedded in subsequent JSONL output.
    pub(crate) fn set_instance_id(&mut self, id: String) {
        self.instance_id = id;
    }

    /// Resets per-instance state and rebuilds lookup tables for a new instance.
    pub(crate) fn start_instance(
        &mut self,
        type_ids: &[String],
        piece_defs: &[(String, PieceDefJson)],
        board_cells: &[Coord]
    ) {
        // For each piece_def (in input order), find its solver type index.
        // None if the def is not used by any piece instance.
        self.type_idx_of_def.clear();
        self.type_idx_of_def.extend(piece_defs.iter().map(|(def_id, _)| {
            type_ids.iter()
                .position(|tid| tid == def_id)
                .map(|p| p as u16)
        }));

        self.canonical_bitmaps.clear();
        self.canonical_bitmaps.extend(piece_defs.iter().map(|(_, def_json)| {
            let def: PieceDef = def_json.clone().into();
            canonical_5x5_bitmap(&def)
        }));

        self.cell_to_grid_idx.clear();
        self.cell_to_grid_idx.extend(board_cells.iter().map(|&(r, c)| {
            (r as u16) * GRID_COLS + (c as u16)
        }));

        // Reset per-instance state.
        self.instance_buffer.clear();
        self.next_branch_id = 0;
        self.nodes_at_branch_entry = 0;
        self.pending_candidates.clear();
    }

    /// Called by the solver at every branch point.
    /// Computes post-state features for each candidate and stores them in `pending_candidates`.
    /// Finalized at the next branch or at solve completion.
    pub(crate) fn on_branch(&mut self, event: BranchEvent<'_>, current_total_nodes: u64) {
        // Finalize the previous branch's pending record (if any).
        self.finalize_pending();

        self.nodes_at_branch_entry = current_total_nodes;

        self.pending_candidates.clear();
        self.pending_candidates.reserve(event.candidates.len());

        for &placement_idx in event.candidates {
            let pl = &event.placements[placement_idx as usize];
            let post_state = self.compute_post_state(event.state, pl);
            self.pending_candidates.push(CandidateRecord {
                placement_idx,
                post_state,
                tried: false,
                succeeded: false,
                subtree_nodes: 0
            });
        }
    }

    /// Called after each candidate's recursive solve attempt returns.
    /// Records the outcome and resets the subtree-node baseline.
    pub(crate) fn on_attempt(
        &mut self,
        placement_idx: u32,
        succeeded: bool,
        current_total_nodes: u64
    ) {
        let subtree = current_total_nodes - self.nodes_at_branch_entry;

        if let Some(rec) = self.pending_candidates
            .iter_mut()
            .find(|c| c.placement_idx == placement_idx) {
            rec.tried = true;
            rec.succeeded = succeeded;
            rec.subtree_nodes = subtree;
        }

        // Reset baseline so the next attempt's subtree is measured correctly.
        self.nodes_at_branch_entry = current_total_nodes;
    }

    /// Called once per instance at solve completion.
    /// On success, finalizes any pending branch and emits the buffered records as JSONL.
    /// On failure, discards the buffer.
    pub(crate) fn on_solve_complete(&mut self, success: bool) -> Result<()> {
        if !success {
            self.instance_buffer.clear();
            self.pending_candidates.clear();
            return Ok(());
        }

        self.finalize_pending();

        // TODO: Serialize self.instance_buffer to JSONL via self.writer.

        self.instance_buffer.clear();
        Ok(())
    }

    /// Computes the post-state for a single candidate without mutating the solver's `SearchState`.
    fn compute_post_state(&self, state: &SearchState, pl: &Placement) -> PostState {
        // Empty target cells AFTER applying pl:
        // not currently covered AND not part of pl's footprint.
        let total_cells = self.cell_to_grid_idx.len();
        let mut empty_target_indices = Vec::new();

        for ci in 0..total_cells {
            if !state.covered.test(ci) && !pl.bits.test(ci) {
                empty_target_indices.push(self.cell_to_grid_idx[ci]);
            }
        }

        let center_mark = state.has_center_mark || pl.mark_on_center;

        // Counts in piece_defs order (matches canonical_bitmaps order).
        // Decrement the entry whose solver types matches pl.type_idx
        let mut counts = Vec::with_capacity(self.type_idx_of_def.len());

        for &maybe_ti in &self.type_idx_of_def {
            let count = match maybe_ti {
                Some(ti) => {
                    let base = state.remaining[ti as usize] as u32;
                    if pl.type_idx == ti {
                        base.saturating_sub(1)
                    } else {
                        base
                    }
                }
                None => 0
            };

            counts.push(count);
        }

        PostState {
            empty_target_indices,
            center_mark,
            counts
        }
    }

    /// Moves pending candidates into a finalized [`BranchRecord`] and
    /// pushes onto `instance_buffer`. No-op if no pending data.
    fn finalize_pending(&mut self) {
        if self.pending_candidates.is_empty() {
            return;
        }

        let branch_id = self.next_branch_id;
        self.next_branch_id += 1;

        self.instance_buffer.push(BranchRecord {
            branch_id,
            candidates: take(&mut self.pending_candidates),
        });
    }
}