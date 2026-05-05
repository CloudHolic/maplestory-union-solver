// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Branch-trace collector for ML training-data generation.

use std::io::{Result, Write};
use std::mem::take;

use crate::base::{BfsWorkspace, BitSet};
use crate::domain::{Coord, Placement};
use crate::solver::SearchState;

/// Information passed to [`Tracer::on_branch`] at every branch point.
///
/// Contain references the tracer borrows from the solver to compute state
/// and candidate features. The solver's data is not modified.
pub(crate) struct BranchEvent<'a> {
    /// The branch cell selected by MRV: candidates all cover this cell.
    pub branch_cell: u16,

    /// Indices into `placements` of valid candidate placements for this branch.
    pub candidates: &'a [u32],

    /// Current solver state at branch entry.
    pub state: &'a SearchState,

    /// First placement list for the entire solve.
    pub placements: &'a [Placement],

    /// 4-connectivity adjacency list over board cells.
    pub adj_list: &'a [Vec<u16>],

    /// Total board cells.
    pub total_cells: u16,

    /// Center region cells (for "branch cell in center" feature).
    pub center_cells: &'a [Coord],

    /// Total piece instances at solve start (for progress feature).
    pub total_pieces: u16
}

/// Branch-trace collector.
pub struct Tracer {
    /// Where to flush JSONL on successful instance completion.
    writer: Box<dyn Write>,

    /// Reusable BFS workspace for region-size computations.
    bfs_ws: BfsWorkspace,

    /// Reusable buffer for cell -> component membership lookup.
    membership_buf: Vec<u16>,

    /// Scratch BitSet for post-placement simulation (covered -> candidate.bits).
    sim_covered: BitSet,

    /// `type_idx -> piece_defs index` mapping for the current instance.
    piece_def_idx_of_type: Vec<u16>,

    /// Number of distinct piece definitions in the current instance's input.
    num_piece_defs: u16,

    /// Buffer of branch records collected for the current instance.
    /// Dumped as JSONL on success, discarded on failure.
    instance_buffer: Vec<BranchRecord>,

    /// Identifier for the current instance (for JSONL `instance_id`).
    instance_id: String,

    /// Counter assigned to each branch within the current instance.
    next_branch_id: u32,

    /// Snapshot of `env.total_nodes` at the most recent `on_branch` call,
    /// used by `on_attempt` to compute subtree node counts.
    nodes_at_branch_entry: u64,

    /// Most-recent branch's pending candidate records.
    pending_candidates: Vec<CandidateRecord>,

    /// State features for the most-recent branch (computed in `on_branch`).
    pending_state_features: Vec<f32>
}

/// One row of the JSONL output.
struct BranchRecord {
    branch_id: u32,
    state_features: Vec<f32>,
    candidates: Vec<CandidateRecord>
}

/// One candidate within a branch record.
struct CandidateRecord {
    placement_idx: u32,
    features: Vec<f32>,
    tried: bool,
    succeeded: bool,
    subtree_nodes: u64
}

impl Tracer {
    pub(crate) fn new(writer: Box<dyn Write>, total_cells: u16) -> Self {
        Self {
            writer,
            bfs_ws: BfsWorkspace::new(total_cells as usize),
            membership_buf: vec![u16::MAX; total_cells as usize],
            sim_covered: BitSet::new(),
            piece_def_idx_of_type: Vec::new(),
            num_piece_defs: 0,
            instance_buffer: Vec::new(),
            instance_id: String::new(),
            next_branch_id: 0,
            nodes_at_branch_entry: 0,
            pending_candidates: Vec::new(),
            pending_state_features: Vec::new()
        }
    }

    pub(crate) fn set_instance_id(&mut self, id: String) {
        self.instance_id = id;
    }

    /// Resets all per-instance state and builds the type -> piece-def index mapping
    /// for this instance.
    ///
    /// `type_ids[ti]` is the piece type with internal index `ti`.
    /// `piece_def_ids[i]` is the piece-def at input position `i`.
    /// Each `type_ids[ti]` must appear in `piece_def_ids`.
    pub(crate) fn start_instance(
        &mut self,
        type_ids: &[String],
        piece_def_ids: &[&str]
    ) {
        self.num_piece_defs = piece_def_ids.len() as u16;
        self.piece_def_idx_of_type.clear();
        self.piece_def_idx_of_type.extend(type_ids.iter().map(|tid| {
            piece_def_ids
                .iter()
                .position(|id| *id == tid.as_str())
                .expect("type_id must appear in piece_def_ids") as u16
        }));
        self.instance_buffer.clear();
        self.next_branch_id = 0;
        self.nodes_at_branch_entry = 0;
        self.pending_candidates.clear();
        self.pending_state_features.clear();
    }

    /// Called by the solver at every branch point.
    /// Computes state and candidate features and records them in pending buffers;
    /// finalized on the next branch or solve completion.
    pub(crate) fn on_branch(&mut self, event: BranchEvent, current_total_nodes: u64) {
        // Finalize the previous branch's pending record (if any).
        self.finalize_pending();

        self.nodes_at_branch_entry = current_total_nodes;

        // TODO: Compute state features.
        // TODO: Compute per-candidate features.

        self.pending_state_features.clear();
        self.pending_candidates.clear();
        self.pending_candidates.extend(event.candidates.iter().map(|&p_idx| {
            CandidateRecord {
                placement_idx: p_idx,
                features: Vec::new(),
                tried: false,
                succeeded: false,
                subtree_nodes: 0
            }
        }));
    }

    /// Called by the solver after each candidate's recursive call returns.
    /// Records that the candidate was tried, whether it succeeded,
    /// and how many subtree nodes its recursion consumed.
    pub(crate) fn on_attempt(
        &mut self,
        placement_idx: u32,
        succeeded: bool,
        current_total_nodes: u64
    ) {
        let subtree = current_total_nodes - self.nodes_at_branch_entry;

        // Update the matching pending candidate record.
        if let Some(rec) = self
            .pending_candidates
            .iter_mut()
            .find(|c| c.placement_idx == placement_idx) {
            rec.tried = true;
            rec.succeeded = succeeded;
            rec.subtree_nodes = subtree;
        }

        // Reset baseline so the next attempt's subtree is measured correctly.
        self.nodes_at_branch_entry = current_total_nodes;
    }

    /// Called at instance solve completion.
    /// On `success`, finalizes any pending branch and
    /// writes the entire instance buffer as JSONL, one record per line.
    /// On failure, discards the buffer.
    pub(crate) fn on_solve_complete(&mut self, success: bool) -> Result<()> {
        if !success {
            self.instance_buffer.clear();
            self.pending_candidates.clear();
            self.pending_state_features.clear();
            return Ok(());
        }

        self.finalize_pending();

        // TODO: Serialize instance_buffer as JSONL.

        self.instance_buffer.clear();
        Ok(())
    }

    /// Moves pending state features and candidates into a finalized `BranchRecord`
    /// and pushes onto `instance_buffer`. No-op if no pending data.
    fn finalize_pending(&mut self) {
        if self.pending_candidates.is_empty() {
            return;
        }

        let branch_id = self.next_branch_id;
        self.next_branch_id += 1;

        self.instance_buffer.push(BranchRecord {
            branch_id,
            state_features: take(&mut self.pending_state_features),
            candidates: take(&mut self.pending_candidates),
        });
    }
}