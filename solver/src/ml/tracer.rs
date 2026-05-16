// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Branch-trace collector for ML training-data generation.

use std::collections::HashMap;
use std::io::Write;

use serde::{Serialize, Serializer};
use serde::ser::SerializeSeq;

use crate::domain::{Coord, PieceDef, Placement};
use crate::error::Result;
use crate::io::PieceDefJson;
use crate::ml::{BITMAP_SIZE, GRID_COLS, canonical_bitmap};
use crate::solver::SearchState;
use crate::SolverError;

/// Information passed to [`Tracer::on_branch`] at every branch point.
pub(crate) struct BranchEvent<'a> {
    /// Placement indices the solver will try, in attempt order.
    /// Already filtered by overlap-free + neighbor-check.
    pub candidates: &'a [u32],

    /// Current solver state immediately before any candidate is applied.
    pub state: &'a SearchState,
}

/// Branch-trace collector.
///
/// Lifecycle (called by the solver under `--features tracing`):
/// 1. [`Tracer::set_instance_id`] — set the instance label.
/// 2. [`Tracer::start_instance`] — provide per-instance constants and
///    reset per-instance state.
/// 3. [`Tracer::on_branch`] — once per branch point. Pushes a new pending
///    branch onto the stack with the candidate set and pre-state.
/// 4. [`Tracer::on_attempt`] — after each candidate's recursive solve
///    returns. Updates the top-of-stack record.
/// 5. [`Tracer::on_branch_complete`] — once the branch's last attempt has
///    been recorded (or on early success). Pops the top, emits it as JSONL.
/// 6. [`Tracer::reset_for_restart`] — at every restart boundary. Clears the
///    pending stack.
/// 7. [`Tracer::on_solve_complete`] — once per instance. On success, drains
///    any branches still on the stack and propagates any deferred I/O error.
///    On failure, just clears the stack.
pub struct Tracer {
    /// JSONL output sink. Branches are written as they finalize, not buffered.
    writer: Box<dyn Write>,

    // ─── Per-instance constants (set in start_instance) ───

    /// Canonical 6x6 bitmaps for every piece_def, in input piece_defs order.
    canonical_bitmaps: Vec<u8>,

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

    /// Whether the instance header row has been written to `writer`.
    /// Lazily set on the first finalized branch - trivial instances produce no output.
    header_emitted: bool,

    /// First I/O error encountered during streaming writes, deferred until
    /// `on_solve_complete` so the backtrack loop's signature stays clean.
    pending_io_error: Option<SolverError>,

    /// Stack of pending branches, one per active recursion level.
    pending_stack: Vec<PendingBranch>,

    /// Compact placement table for the current instance.
    placements: Vec<PlacementJson>
}

/// One in-flight branch, kept on `pending_stack` until its `on_branch_complete`.
struct PendingBranch {
    candidates: Vec<CandidateRecord>,
    pre_state: PreState,

    /// `total_nodes` baseline: most recent reference point for subtree size computation.
    nodes_at_entry: u64
}

/// One row of the JSONL output.
#[derive(Serialize)]
struct BranchRecord {
    branch_id: u32,
    pre_state: PreState,
    candidates: Vec<CandidateRecord>
}

/// One candidate within a [`BranchRecord`].
#[derive(Serialize)]
struct CandidateRecord {
    placement_idx: u32,
    tried: bool,
    succeeded: bool,
    subtree_nodes: u64
}

/// Solver state before virtually applying a single candidate placement.
#[derive(Serialize)]
struct PreState {
    #[serde(serialize_with = "serialize_empty_bitmap")]
    empty_bitmap: [u8; 28],
    center_mark: bool,
    counts: Vec<u8>
}

fn serialize_empty_bitmap<S>(bm: &[u8; 28], serializer: S)
    -> std::result::Result<S::Ok, S::Error>
    where S: Serializer {
    let mut seq = serializer.serialize_seq(Some(28))?;
    for &b in bm {
        seq.serialize_element(&b)?;
    }
    seq.end()
}

#[derive(Serialize, Clone)]
struct PlacementJson {
    cells: Vec<u16>,
    piece_def_idx: u16,
    mark_on_center: bool
}

#[derive(Serialize)]
struct InstanceHeader<'a> {
    #[serde(rename = "_kind")]
    kind: &'static str,
    instance_id: &'a str,
    #[serde(serialize_with = "serialize_bitmaps_chunked")]
    canonical_bitmaps: &'a [u8],
    cell_to_grid_idx:  &'a [u16],
    placements: &'a [PlacementJson]
}

fn serialize_bitmaps_chunked<S>(bitmaps: &[u8], serializer: S)
    -> std::result::Result<S::Ok, S::Error>
    where S: Serializer {
    debug_assert!(bitmaps.len() % BITMAP_SIZE == 0,
                  "canonical_bitmaps length not a multiple of BITMAP_SIZE");

    let num_pieces = bitmaps.len() / BITMAP_SIZE;
    let mut seq = serializer.serialize_seq(Some(num_pieces))?;
    for chunk in bitmaps.chunks_exact(BITMAP_SIZE) {
        seq.serialize_element(chunk)?;
    }

    seq.end()
}

#[derive(Serialize)]
struct BranchLine<'a> {
    #[serde(rename = "_kind")]
    kind: &'static str,
    instance_id: &'a str,
    #[serde(flatten)]
    record: &'a BranchRecord
}

impl Tracer {
    pub fn new(writer: Box<dyn Write>) -> Self {
        Self {
            writer,
            canonical_bitmaps: Vec::new(),
            cell_to_grid_idx: Vec::new(),
            type_idx_of_def: Vec::new(),
            instance_id: String::new(),
            next_branch_id: 0,
            header_emitted: false,
            pending_io_error: None,
            pending_stack: Vec::new(),
            placements: Vec::new()
        }
    }

    /// Sets the instance identifier embedded in subsequent JSONL output.
    pub fn set_instance_id(&mut self, id: String) {
        self.instance_id = id;
    }

    /// Resets per-instance state and rebuilds lookup tables for a new instance.
    pub(crate) fn start_instance(
        &mut self,
        type_ids: &[String],
        piece_defs: &[(String, PieceDefJson)],
        board_cells: &[Coord],
        placements: &[Placement]
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
        for (_, def_json) in piece_defs {
            let def: PieceDef = def_json.clone().into();
            self.canonical_bitmaps.extend_from_slice(&canonical_bitmap(&def));
        }

        self.cell_to_grid_idx.clear();
        self.cell_to_grid_idx.extend(board_cells.iter().map(|&(r, c)| {
            (r as u16) * GRID_COLS + (c as u16)
        }));

        let mut def_idx_of_type: HashMap<u16, u16> = HashMap::new();
        for (def_idx, &maybe_ti) in self.type_idx_of_def.iter().enumerate() {
            if let Some(ti) = maybe_ti {
                def_idx_of_type.insert(ti, def_idx as u16);
            }
        }

        self.placements.clear();
        self.placements.reserve(placements.len());
        for pl in placements {
            self.placements.push(PlacementJson {
                cells: pl.cell_indices.clone(),
                piece_def_idx: *def_idx_of_type.get(&pl.type_idx)
                    .expect("every solver type must map to a piece_def"),
                mark_on_center: pl.mark_on_center,
            })
        }

        // Reset per-instance state.
        self.next_branch_id = 0;
        self.header_emitted = false;
        self.pending_io_error = None;
        self.pending_stack.clear();
    }

    /// Pushes a new pending branch onto the stack.
    pub(crate) fn on_branch(&mut self, event: BranchEvent<'_>, current_total_nodes: u64) {
        let pre_state = self.compute_pre_state(event.state);
        let candidates = event.candidates.iter().map(|&p| CandidateRecord {
            placement_idx: p,
            tried: false,
            succeeded: false,
            subtree_nodes: 0
        }).collect();

        self.pending_stack.push(PendingBranch {
            candidates,
            pre_state,
            nodes_at_entry: current_total_nodes
        });
    }

    /// Updates the top-of-stack branch with the outcome of one attempt.
    pub(crate) fn on_attempt(
        &mut self,
        placement_idx: u32,
        succeeded: bool,
        current_total_nodes: u64
    ) {
        let top = self.pending_stack.last_mut()
            .expect("on_attempt called without matching on_branch");

        let subtree = current_total_nodes - top.nodes_at_entry;

        if let Some(rec) = top.candidates
            .iter_mut()
            .find(|c| c.placement_idx == placement_idx) {
            rec.tried = true;
            rec.succeeded = succeeded;
            rec.subtree_nodes = subtree;
        }

        // Advance baseline so the next attempt's subtree is measured from here.
        top.nodes_at_entry = current_total_nodes;
    }

    /// Pops the top branch and emits it as JSONL.
    pub(crate) fn on_branch_complete(&mut self) {
        let done = self.pending_stack.pop()
            .expect("on_branch_complete called without matching on_branch");

        _ = self.emit_branch(done)
    }

    /// Clears the pending stack at a restart boundary.
    pub(crate) fn reset_for_restart(&mut self) {
        self.pending_stack.clear();
    }

    /// Called once per instance at solve completion.
    pub(crate) fn on_solve_complete(&mut self, success: bool) -> Result<()> {
        if !success {
            self.pending_stack.clear();
            return Ok(());
        }

        // Success path: drain any remaining branches.
        while let Some(branch) = self.pending_stack.pop() {
            self.emit_branch(branch)?;
        }

        if let Some(e) = self.pending_io_error.take() {
            return Err(e);
        }

        self.writer.flush()?;
        Ok(())
    }

    /// Computes the pre-state for a single candidate without mutating the solver's `SearchState`.
    fn compute_pre_state(&self, state: &SearchState) -> PreState {
        let total_cells = self.cell_to_grid_idx.len();
        let mut empty_bitmap = [0u8; 28];

        for ci in 0..total_cells {
            if !state.covered.test(ci) {
                let byte = ci / 8;
                let bit = ci % 8;
                empty_bitmap[byte] |= 1u8 << bit;
            }
        }

        let mut counts = Vec::with_capacity(self.type_idx_of_def.len());
        for &maybe_ti in &self.type_idx_of_def {
            let count = match maybe_ti {
                Some(ti) => state.remaining[ti as usize] as u8,
                None => 0
            };

            counts.push(count);
        }

        PreState {
            empty_bitmap,
            center_mark: state.has_center_mark,
            counts
        }
    }

    /// Emits one completed branch to the writer.
    fn emit_branch(&mut self, branch: PendingBranch) -> Result<()> {
        if self.pending_io_error.is_some() {
            return Ok(());
        }

        if let Err(e) = self.try_emit_branch(branch) {
            self.pending_io_error = Some(e);
        }

        Ok(())
    }

    fn try_emit_branch(&mut self, branch: PendingBranch) -> Result<()> {
        if !self.header_emitted {
            let header = InstanceHeader {
                kind: "instance",
                instance_id: &self.instance_id,
                canonical_bitmaps: &self.canonical_bitmaps,
                cell_to_grid_idx: &self.cell_to_grid_idx,
                placements: &self.placements
            };

            serde_json::to_writer(&mut self.writer, &header)?;
            _ = self.writer.write_all(b"\n");
            self.header_emitted = true;
        }

        let branch_id = self.next_branch_id;
        self.next_branch_id += 1;

        let record = BranchRecord {
            branch_id,
            pre_state: branch.pre_state,
            candidates: branch.candidates
        };

        let line = BranchLine {
            kind: "branch",
            instance_id: &self.instance_id,
            record: &record
        };

        serde_json::to_writer(&mut self.writer, &line)?;
        _ = self.writer.write_all(b"\n");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::BitSet;
    use crate::domain::Coord;

    fn placement(type_idx: u16, cells: &[u16], mark_on_center: bool) -> Placement {
        let mut bits = BitSet::new();
        for &i in cells {
            bits.set(i as usize);
        }
        Placement {
            type_idx,
            bits,
            neighbor_indices: Vec::new(),
            b_count: 0,
            mark_on_center,
            cell_indices: cells.to_vec(),
            mark: (0, 0),
            cells: cells.iter().map(|&i| (0, i as i8)).collect()
        }
    }

    fn def_json(id: &str) -> (String, PieceDefJson) {
        (id.to_string(), PieceDefJson {
            id: id.to_string(),
            cells: vec![(0, 0)],
            mark_index: 0
        })
    }

    fn new_tracer() -> Tracer {
        Tracer::new(Box::new(std::io::sink()))
    }

    fn setup(t: &mut Tracer, type_ids: &[&str], def_ids: &[&str], board_cells: &[Coord], placements: &[Placement]) {
        let type_ids_owned: Vec<String> = type_ids.iter().map(|s| s.to_string()).collect();
        let defs: Vec<(String, PieceDefJson)> = def_ids.iter().map(|id| def_json(id)).collect();
        t.start_instance(&type_ids_owned, &defs, board_cells, placements);
    }

    struct SharedBuf(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }

    fn make_capture_writer() -> (Box<dyn Write>, std::rc::Rc<std::cell::RefCell<Vec<u8>>>) {
        let buf = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        (Box::new(SharedBuf(buf.clone())), buf)
    }

    #[test]
    fn pre_state_empty_bitmap_marks_uncovered_board_cells_only() {
        let mut t = new_tracer();
        setup(&mut t, &["P0"], &["P0"], &[(2, 3), (5, 7), (1, 1)], &[]);

        let mut state = SearchState::new(vec![1], 0);
        state.covered.set(1);
        state.covered_count = 1;

        let pre = t.compute_pre_state(&state);

        let mut expected = [0u8; 28];
        expected[0] = 0b0000_0101;
        assert_eq!(pre.empty_bitmap, expected);
    }

    #[test]
    fn pre_state_center_mark_reflects_state_only_not_candidate() {
        let mut t = new_tracer();
        setup(&mut t, &["P0"], &["P0"], &[(0, 0)], &[]);

        let mut state = SearchState::new(vec![2], 0);
        assert!(!t.compute_pre_state(&state).center_mark);

        state.has_center_mark = true;
        assert!(t.compute_pre_state(&state).center_mark);
    }

    #[test]
    fn pre_state_counts_are_raw_remaining_without_candidate_decrement() {
        let mut t = new_tracer();
        setup(&mut t, &["P0", "P1"], &["P0", "P1", "P2"], &[(0, 0)], &[]);

        let state = SearchState::new(vec![3, 2], 0);
        let pre = t.compute_pre_state(&state);
        assert_eq!(pre.counts, vec![3u8, 2, 0]);
    }

    #[test]
    fn start_instance_builds_placement_table_with_def_idx_reverse_mapping() {
        let mut t = new_tracer();
        let placements = vec![
            placement(0, &[0], false),
            placement(1, &[1], true),
        ];
        setup(&mut t, &["P0", "P1"], &["P0", "P1", "P2"], &[(0, 0), (0, 1)], &placements);

        assert_eq!(t.placements.len(), 2);
        assert_eq!(t.placements[0].piece_def_idx, 0);
        assert_eq!(t.placements[1].piece_def_idx, 1);
        assert!(t.placements[1].mark_on_center);
    }

    #[test]
    fn nested_branches_emit_in_completion_order_with_correct_subtree_counts() {
        let (writer, buf) = make_capture_writer();
        let mut t = Tracer::new(writer);
        t.set_instance_id("test".to_string());
        setup(&mut t, &["P0"], &["P0"], &[(0, 0)], &[]);
        let state = SearchState::new(vec![1], 0);

        t.on_branch(BranchEvent { candidates: &[10, 20], state: &state }, 100);
        t.on_branch(BranchEvent { candidates: &[30], state: &state }, 101);

        t.on_attempt(30, false, 105);
        t.on_branch_complete();

        t.on_attempt(10, false, 105);
        t.on_attempt(20, false, 106);

        t.on_branch_complete();
        t.on_solve_complete(true).unwrap();

        let text = String::from_utf8(buf.borrow().clone()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "header + 2 branches");

        let b0: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(b0["branch_id"], 0);
        let b0_cands = b0["candidates"].as_array().unwrap();
        assert_eq!(b0_cands.len(), 1);
        assert_eq!(b0_cands[0]["placement_idx"], 30);
        assert_eq!(b0_cands[0]["subtree_nodes"], 4);

        let b1: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(b1["branch_id"], 1);
        let b1_cands = b1["candidates"].as_array().unwrap();
        assert_eq!(b1_cands.len(), 2);
        assert_eq!(b1_cands[0]["placement_idx"], 10);
        assert_eq!(b1_cands[0]["subtree_nodes"], 5);
        assert_eq!(b1_cands[1]["placement_idx"], 20);
        assert_eq!(b1_cands[1]["subtree_nodes"], 1);
    }

    #[test]
    fn reset_for_restart_clears_in_flight_branches() {
        let mut t = new_tracer();
        setup(&mut t, &["P0"], &["P0"], &[(0, 0)], &[]);
        let state = SearchState::new(vec![1], 0);

        t.on_branch(BranchEvent { candidates: &[0], state: &state }, 100);
        t.on_branch(BranchEvent { candidates: &[1], state: &state }, 101);
        assert_eq!(t.pending_stack.len(), 2);

        t.reset_for_restart();
        assert!(t.pending_stack.is_empty(),
                "restart must clear the pending stack to prevent leaking abandoned-attempt state");
    }

    #[test]
    fn solve_complete_failure_clears_stack_without_emitting() {
        let (writer, buf) = make_capture_writer();
        let mut t = Tracer::new(writer);
        t.set_instance_id("failed".to_string());
        setup(&mut t, &["P0"], &["P0"], &[(0, 0)], &[]);
        let state = SearchState::new(vec![1], 0);

        t.on_branch(BranchEvent { candidates: &[0], state: &state }, 0);
        t.on_attempt(0, false, 5);

        t.on_solve_complete(false).unwrap();
        assert!(t.pending_stack.is_empty());
        assert!(buf.borrow().is_empty(),
                "failed instance must not emit (caller deletes the temp file)");
    }

    #[test]
    fn solve_complete_success_drains_remaining_stack() {
        // If the solve succeeds while branches are still on the stack (because
        // the success path doesn't call on_branch_complete for outer levels),
        // on_solve_complete must drain them.
        let (writer, buf) = make_capture_writer();
        let mut t = Tracer::new(writer);
        t.set_instance_id("ok".to_string());
        setup(&mut t, &["P0"], &["P0"], &[(0, 0)], &[]);
        let state = SearchState::new(vec![1], 0);

        t.on_branch(BranchEvent { candidates: &[0], state: &state }, 100);
        t.on_attempt(0, true, 110);
        // success path may not call on_branch_complete — on_solve_complete handles it
        t.on_solve_complete(true).unwrap();

        let text = String::from_utf8(buf.borrow().clone()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "header + 1 branch from drained stack");
    }

    #[test]
    fn solve_complete_writes_nothing_when_no_branches_recorded() {
        let (writer, buf) = make_capture_writer();
        let mut t = Tracer::new(writer);
        t.set_instance_id("trivial".to_string());
        setup(&mut t, &["P0"], &["P0"], &[(0, 0)], &[]);

        t.on_solve_complete(true).unwrap();
        assert!(buf.borrow().is_empty(), "no branches → no output");
    }
}