// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Branch-trace collector for ML training-data generation.

use std::io::Write;
use std::mem::take;

use serde::Serialize;
use serde_big_array::BigArray;

use crate::domain::{Coord, PieceDef, Placement};
use crate::error::Result;
use crate::io::PieceDefJson;
use crate::ml::{BITMAP_SIZE, GRID_COLS, canonical_bitmap};
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

    /// Flat placement list (for index lookups during post-state computation).
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
#[derive(Serialize)]
struct BranchRecord {
    branch_id: u32,
    candidates: Vec<CandidateRecord>
}

/// One candidate within a [`BranchRecord`].
#[derive(Serialize)]
struct CandidateRecord {
    placement_idx: u32,
    post_state: PostState,
    tried: bool,
    succeeded: bool,
    subtree_nodes: u64
}

/// Solver state after virtually applying a single candidate placement.
#[derive(Serialize)]
struct PostState {
    /// Grid indices (row-major, `r * GRID_COLS + c`) of cells still empty
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

/// JSONL row for the per-instance header.
/// Carries the canonical bitmaps so per-branch records can omit them.
#[derive(Serialize)]
struct InstanceHeader<'a> {
    #[serde(rename = "_kind")]
    kind: &'static str,
    instance_id: &'a str,
    canonical_bitmaps: Vec<u8>
}

/// JSONL row for a single branch - flattens [`BranchRecord`] alongside
/// the instance label and discriminator.
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
            nodes_at_branch_entry: 0,
            instance_buffer: Vec::new(),
            pending_candidates: Vec::new(),
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
            canonical_bitmap(&def)
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

        // Skip output entirely for trivial instances that never branched -
        // such instances carry no learning signal.
        if self.instance_buffer.is_empty() {
            return Ok(());
        }

        // Emit instance header row.
        let header = InstanceHeader {
            kind: "instance",
            instance_id: &self.instance_id,
            canonical_bitmaps: &self.canonical_bitmaps
        };
        serde_json::to_writer(&mut self.writer, &header)?;
        self.writer.write_all(b"\n")?;

        // Emit one branch row per record.
        for record in &self.instance_buffer {
            let line = BranchLine {
                kind: "branch",
                instance_id: &self.instance_id,
                record
            };
            serde_json::to_writer(&mut self.writer, &line)?;
            self.writer.write_all(b"\n")?;
        }

        self.writer.flush()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::BitSet;
    use crate::domain::Coord;

    /// Tiny placement constructor for tracer tests.
    /// Solver-side fields irrelevant to the tracer (b_count, neighbor_indices, mark, cells)
    /// are filled with zeroes.
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

    /// Single-cell piece def. The shape itself is irrelevant for these tests
    /// (canonical_5x5_bitmap correctness is covered in canonical.rs).
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

    /// Configures a tracer with the given board layout. `def_ids` may include
    /// ids absent from `type_ids` to exercise the `type_idx_of_def == None`
    /// branch.
    fn setup(t: &mut Tracer, type_ids: &[&str], def_ids: &[&str], board_cells: &[Coord]) {
        let type_ids_owned: Vec<String> = type_ids.iter().map(|s| s.to_string()).collect();
        let defs: Vec<(String, PieceDefJson)> = def_ids.iter().map(|id| def_json(id)).collect();
        t.start_instance(&type_ids_owned, &defs, board_cells);
    }

    /// Capture writer for JSONL inspection
    ///
    /// `Box<dyn Write>` -compatible writer that funnels into a shared `Vec<u8>`.
    /// Tracer requires `'static`-bounded boxed writers, so test buffers go through
    /// `Rc<RefCell<_>>` rather than direct `&mut Vec<u8>` references.
    struct SharedBuf(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn make_capture_writer() -> (Box<dyn Write>, std::rc::Rc<std::cell::RefCell<Vec<u8>>>) {
        let buf = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        (Box::new(SharedBuf(buf.clone())), buf)
    }

    #[test]
    fn empty_target_indices_use_grid_coord_translation() {
        let mut t = new_tracer();
        setup(&mut t, &["P0"], &["P0"], &[(2, 3), (5, 7), (1, 1)]);

        let mut state = SearchState::new(vec![1], 0);
        state.covered.set(1);
        state.covered_count = 1;

        let pl = placement(0, &[0], false);
        let post = t.compute_post_state(&state, &pl);

        assert_eq!(post.empty_target_indices, vec![21]);
    }

    #[test]
    fn center_mark_uses_or_semantics() {
        let mut t = new_tracer();
        setup(&mut t, &["P0"], &["P0"], &[(0, 0)]);

        let pl_no = placement(0, &[0], false);
        let pl_yes = placement(0, &[0], true);

        let mut state = SearchState::new(vec![2], 0);

        assert!(!t.compute_post_state(&state, &pl_no).center_mark);
        assert!(t.compute_post_state(&state, &pl_yes).center_mark);

        state.has_center_mark = true;
        assert!(t.compute_post_state(&state, &pl_no).center_mark);
        assert!(t.compute_post_state(&state, &pl_yes).center_mark);
    }

    #[test]
    fn counts_decrement_matching_type_only_unused_defs_zero() {
        let mut t = new_tracer();
        setup(&mut t, &["P0", "P1"], &["P0", "P1", "P2"], &[(0, 0)]);

        let state = SearchState::new(vec![3, 2], 0);
        let pl = placement(1, &[0], false);
        let post = t.compute_post_state(&state, &pl);

        assert_eq!(post.counts, vec![3, 1, 0]);
    }

    #[test]
    fn branch_lifecycle_finalizes_and_measures_subtree() {
        let mut t = new_tracer();
        setup(&mut t, &["P0"], &["P0"], &[(0, 0), (0, 1)]);

        let placements = vec![
            placement(0, &[0], false),
            placement(0, &[1], false)
        ];
        let state = SearchState::new(vec![1], 0);

        t.on_branch(
            BranchEvent { candidates: &[0], state: &state, placements: &placements },
            100
        );

        assert_eq!(t.instance_buffer.len(), 0);
        assert_eq!(t.pending_candidates.len(), 1);

        t.on_attempt(0, false, 150);
        assert!(t.pending_candidates[0].tried);
        assert!(!t.pending_candidates[0].succeeded);
        assert_eq!(t.pending_candidates[0].subtree_nodes, 50);

        t.on_branch(
            BranchEvent { candidates: &[1], state: &state, placements: &placements },
            150
        );

        assert_eq!(t.instance_buffer.len(), 1);
        assert_eq!(t.instance_buffer[0].branch_id, 0);
        assert_eq!(t.instance_buffer[0].candidates.len(), 1);
        assert_eq!(t.pending_candidates.len(), 1);
    }

    #[test]
    fn solve_complete_clears_buffers_on_both_outcomes() {
        // Failure case: discard buffers without writing.
        {
            let mut t = new_tracer();
            setup(&mut t, &["P0"], &["P0"], &[(0, 0)]);
            let placements = vec![placement(0, &[0], false)];
            let state = SearchState::new(vec![1], 0);

            t.on_branch(
                BranchEvent { candidates: &[0], state: &state, placements: &placements },
                0
            );
            assert!(!t.pending_candidates.is_empty());

            t.on_solve_complete(false).unwrap();
            assert!(t.pending_candidates.is_empty());
            assert!(t.instance_buffer.is_empty());
        }

        // Success case: write JSONL then clear.
        {
            let mut t = new_tracer();
            setup(&mut t, &["P0"], &["P0"], &[(0, 0)]);
            let placements = vec![placement(0, &[0], false)];
            let state = SearchState::new(vec![1], 0);

            t.on_branch(
                BranchEvent { candidates: &[0], state: &state, placements: &placements },
                0
            );

            t.on_solve_complete(true).unwrap();
            assert!(t.pending_candidates.is_empty());
            assert!(t.instance_buffer.is_empty());
        }
    }

    #[test]
    fn solve_complete_writes_jsonl_with_instance_header_and_branch_rows() {
        let (writer, buf) = make_capture_writer();
        let mut t = Tracer::new(writer);
        t.set_instance_id("synth_test_001".to_string());
        setup(&mut t, &["P0"], &["P0"], &[(0, 0), (0, 1)]);

        let placements = vec![placement(0, &[0], false)];
        let state = SearchState::new(vec![1], 0);

        t.on_branch(
            BranchEvent { candidates: &[0], state: &state, placements: &placements },
            0
        );
        t.on_attempt(0, false, 10);

        t.on_solve_complete(true).unwrap();

        let bytes = buf.borrow().clone();
        let text = std::str::from_utf8(&bytes).expect("output must be valid UTF-8");
        let lines: Vec<&str> = text.lines().collect();

        assert_eq!(lines.len(), 2, "expected 2 JSONL rows, got: {text:?}");

        // Row 1 — instance header.
        let header: serde_json::Value = serde_json::from_str(lines[0]).expect("row 1 valid JSON");
        assert_eq!(header["_kind"], "instance");
        assert_eq!(header["instance_id"], "synth_test_001");
        assert!(header["canonical_bitmaps"].is_array());
        assert_eq!(header["canonical_bitmaps"].as_array().unwrap().len(), 1);

        // Row 2 — branch record.
        let branch: serde_json::Value = serde_json::from_str(lines[1]).expect("row 2 valid JSON");
        assert_eq!(branch["_kind"], "branch");
        assert_eq!(branch["instance_id"], "synth_test_001");
        assert_eq!(branch["branch_id"], 0);

        let candidates = branch["candidates"].as_array().unwrap();
        assert_eq!(candidates.len(), 1);

        let post = &candidates[0]["post_state"];
        assert!(post["empty_target_indices"].is_array());
        assert!(post["center_mark"].is_boolean());
        assert!(post["counts"].is_array());
        assert!(post.get("pieces").is_none(), "pieces must live in instance header, not post_state");
    }

    #[test]
    fn solve_complete_writes_nothing_when_no_branches_recorded() {
        let (writer, buf) = make_capture_writer();
        let mut t = Tracer::new(writer);
        t.set_instance_id("trivial_instance".to_string());
        setup(&mut t, &["P0"], &["P0"], &[(0, 0)]);

        // No on_branch calls — instance_buffer stays empty.
        t.on_solve_complete(true).unwrap();

        assert!(buf.borrow().is_empty(), "no branches → no output");
    }
}