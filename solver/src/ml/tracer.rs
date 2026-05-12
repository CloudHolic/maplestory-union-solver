// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Branch-trace collector for ML training-data generation.

use std::collections::HashMap;
use std::io::Write;
use std::mem::take;

use serde::{Serialize, Serializer};
use serde::ser::SerializeSeq;

use crate::domain::{Coord, PieceDef, Placement};
use crate::error::Result;
use crate::io::PieceDefJson;
use crate::ml::{BITMAP_SIZE, GRID_COLS, canonical_bitmap};
use crate::solver::SearchState;
use crate::SolverError;

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

    /// `total_nodes` value at the moment of the most recent on_branch
    /// or on_attempt call, used as the baseline for subtree node counting.
    nodes_at_branch_entry: u64,

    /// Whether the instance header row has been written to `writer`.
    /// Lazily set on the first finalized branch - trivial instances produce no output.
    header_emitted: bool,

    /// First I/O error encountered during streaming writes, deferred until
    /// `on_solve_complete` so the backtrack loop's signature stays clean.
    pending_io_error: Option<SolverError>,

    /// Most-recent branch's pending candidate records.
    pending_candidates: Vec<CandidateRecord>,

    /// Pre-state for the current pending branch.
    pending_pre_state: Option<PreState>,

    /// Compact placement table for the current instance.
    placements: Vec<PlacementJson>
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
    /// Grid indices (row-major, `r * GRID_COLS + c`) of cells still empty
    /// before applying the candidate.
    #[serde(serialize_with = "serialize_empty_bitmap")]
    empty_bitmap: [u8; 28],

    /// `true` if any placement marking the center has been applied
    /// (including the candidate itself).
    center_mark: bool,

    /// Remaining piece count per piece_def, in input piece_defs order.
    /// Length matches [`Tracer::canonical_bitmaps`].
    /// Defs not used by any solver type contribute `0`.
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

/// JSONL row for the per-instance header.
/// Carries the canonical bitmaps so per-branch records can omit them.
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
            header_emitted: false,
            pending_io_error: None,
            pending_candidates: Vec::new(),
            pending_pre_state: None,
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
        self.nodes_at_branch_entry = 0;
        self.header_emitted = false;
        self.pending_io_error = None;
        self.pending_candidates.clear();
        self.pending_pre_state = None;
    }

    /// Called by the solver at every branch point.
    /// Computes post-state features for each candidate and stores them in `pending_candidates`.
    /// Finalized at the next branch or at solve completion.
    pub(crate) fn on_branch(&mut self, event: BranchEvent<'_>, current_total_nodes: u64) {
        // Finalize the previous branch's pending record (if any).
        self.finalize_pending();

        self.nodes_at_branch_entry = current_total_nodes;

        self.pending_pre_state = Some(self.compute_pre_state(event.state));

        self.pending_candidates.clear();
        self.pending_candidates.reserve(event.candidates.len());

        for &placement_idx in event.candidates {
            self.pending_candidates.push(CandidateRecord {
                placement_idx,
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
    /// On success, finalizes any pending branch and propagates any deferred I/O error.
    /// On failure, only clears pending state.
    pub(crate) fn on_solve_complete(&mut self, success: bool) -> Result<()> {
        if !success {
            self.pending_candidates.clear();
            self.pending_pre_state = None;

            // Note: header_emitted, next_branch_id, and pending_io_error stay as_is.
            return Ok(());
        }

        self.finalize_pending();

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

    /// Moves pending candidates into a finalized [`BranchRecord`] and
    /// streams it directly to `writer`.
    fn finalize_pending(&mut self) {
        if self.pending_candidates.is_empty() {
            return;
        }

        // Skip further writes once an error has been recorded - repeated
        // attempts would just rediscover the same broken sink.
        if self.pending_io_error.is_some() {
            self.pending_candidates.clear();
            self.pending_pre_state = None;
            return;
        }

        if let Err(e) = self.try_emit_pending() {
            self.pending_io_error = Some(e);
            self.pending_candidates.clear();
            self.pending_pre_state = None;
        }
    }

    /// Inner helper for [`finalize_pending`] that may return an I/O / serde error.
    fn try_emit_pending(&mut self) -> Result<()> {
        if !self.header_emitted {
            let header = InstanceHeader {
                kind: "instance",
                instance_id: &self.instance_id,
                canonical_bitmaps: &self.canonical_bitmaps,
                cell_to_grid_idx: &self.cell_to_grid_idx,
                placements: &self.placements
            };

            serde_json::to_writer(&mut self.writer, &header)?;
            self.writer.write_all(b"\n")?;
            self.header_emitted = true;
        }

        let branch_id = self.next_branch_id;
        self.next_branch_id += 1;

        let record = BranchRecord {
            branch_id,
            pre_state: self.pending_pre_state.take()
                .expect("pending_pre_state must be set by on_branch before finalize"),
            candidates: take(&mut self.pending_candidates)
        };

        let line = BranchLine {
            kind: "branch",
            instance_id: &self.instance_id,
            record: &record
        };

        serde_json::to_writer(&mut self.writer, &line)?;
        self.writer.write_all(b"\n")?;

        Ok(())
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
    fn setup(t: &mut Tracer, type_ids: &[&str], def_ids: &[&str], board_cells: &[Coord], placements: &[Placement]) {
        let type_ids_owned: Vec<String> = type_ids.iter().map(|s| s.to_string()).collect();
        let defs: Vec<(String, PieceDefJson)> = def_ids.iter().map(|id| def_json(id)).collect();
        t.start_instance(&type_ids_owned, &defs, board_cells, placements);
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
    fn pre_state_empty_bitmap_marks_uncovered_board_cells_only() {
        let mut t = new_tracer();
        setup(&mut t, &["P0"], &["P0"], &[(2, 3), (5, 7), (1, 1)], &[]);

        let mut state = SearchState::new(vec![1], 0);
        state.covered.set(1);
        state.covered_count = 1;

        let pre = t.compute_pre_state(&state);

        let mut expected = [0u8; 28];
        expected[0] = 0b0000_0101;  // bits 0 and 2
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
    fn start_instance_builds_placement_lite_table_with_def_idx_reverse_mapping() {
        let mut t = new_tracer();
        let type_ids = ["P0", "P1"];
        let def_ids = ["P0", "P1", "P2"];
        let board = [(0, 0), (0, 1)];

        let placements = vec![
            placement(0, &[0], false),
            placement(1, &[1], true),
        ];

        setup(&mut t, &type_ids, &def_ids, &board, &placements);

        assert_eq!(t.placements.len(), 2);

        assert_eq!(t.placements[0].piece_def_idx, 0);
        assert_eq!(t.placements[0].cells, vec![0u16]);
        assert!(!t.placements[0].mark_on_center);

        assert_eq!(t.placements[1].piece_def_idx, 1);
        assert_eq!(t.placements[1].cells, vec![1u16]);
        assert!(t.placements[1].mark_on_center);
    }

    #[test]
    fn branch_lifecycle_captures_pre_state_and_streams_finalized_branches() {
        let (writer, buf) = make_capture_writer();
        let mut t = Tracer::new(writer);
        t.set_instance_id("test_instance".to_string());
        setup(&mut t, &["P0"], &["P0"], &[(0, 0), (0, 1)], &[]);

        let state = SearchState::new(vec![1], 0);

        // First branch enters pending state — nothing should be flushed yet.
        t.on_branch(BranchEvent { candidates: &[0], state: &state }, 100);
        assert!(buf.borrow().is_empty(),
                "no output until finalize_pending fires on next branch or solve complete");
        assert!(t.pending_pre_state.is_some());
        assert_eq!(t.pending_candidates.len(), 1);

        t.on_attempt(0, false, 150);
        assert!(t.pending_candidates[0].tried);
        assert!(!t.pending_candidates[0].succeeded);
        assert_eq!(t.pending_candidates[0].subtree_nodes, 50);

        // Second on_branch finalizes the first — header + branch 0 should hit the writer.
        t.on_branch(BranchEvent { candidates: &[1], state: &state }, 150);

        let text = String::from_utf8(buf.borrow().clone()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "expected header + 1 branch row, got: {text:?}");
        assert!(lines[0].contains("\"_kind\":\"instance\""));
        assert!(lines[1].contains("\"_kind\":\"branch\""));
        assert!(lines[1].contains("\"branch_id\":0"));

        // Branch 1 still pending.
        assert!(t.pending_pre_state.is_some());
        assert_eq!(t.pending_candidates.len(), 1);
    }

    #[test]
    fn solve_complete_resets_pending_state_on_both_success_and_failure() {
        // Failure path: pending dropped, anything already streamed stays on disk
        // (the caller's responsibility to unlink the temp file).
        {
            let (writer, _buf) = make_capture_writer();
            let mut t = Tracer::new(writer);
            setup(&mut t, &["P0"], &["P0"], &[(0, 0)], &[]);
            let state = SearchState::new(vec![1], 0);

            t.on_branch(BranchEvent { candidates: &[0], state: &state }, 0);
            assert!(!t.pending_candidates.is_empty());
            assert!(t.pending_pre_state.is_some());

            t.on_solve_complete(false).unwrap();
            assert!(t.pending_candidates.is_empty());
            assert!(t.pending_pre_state.is_none(),
                    "pending_pre_state must be reset on failure to prevent leakage into next instance");
        }

        // Success path: last pending finalizes and gets streamed.
        {
            let (writer, buf) = make_capture_writer();
            let mut t = Tracer::new(writer);
            t.set_instance_id("success_instance".to_string());
            setup(&mut t, &["P0"], &["P0"], &[(0, 0)], &[]);
            let state = SearchState::new(vec![1], 0);

            t.on_branch(BranchEvent { candidates: &[0], state: &state }, 0);

            t.on_solve_complete(true).unwrap();
            assert!(t.pending_candidates.is_empty());
            assert!(t.pending_pre_state.is_none());

            let text = String::from_utf8(buf.borrow().clone()).unwrap();
            let lines: Vec<&str> = text.lines().collect();
            assert_eq!(lines.len(), 2, "expected header + 1 branch on success, got: {text:?}");
        }
    }

    #[test]
    fn solve_complete_emits_pre_state_per_branch_and_placement_table_in_header() {
        let (writer, buf) = make_capture_writer();
        let mut t = Tracer::new(writer);
        t.set_instance_id("synth_test_001".to_string());

        let placements = vec![placement(0, &[0], false)];
        setup(&mut t, &["P0"], &["P0"], &[(0, 0), (0, 1)], &placements);

        let state = SearchState::new(vec![1], 0);

        t.on_branch(
            BranchEvent { candidates: &[0], state: &state },
            0
        );
        t.on_attempt(0, false, 10);

        t.on_solve_complete(true).unwrap();

        let bytes = buf.borrow().clone();
        let text = std::str::from_utf8(&bytes).expect("output must be valid UTF-8");
        let lines: Vec<&str> = text.lines().collect();

        assert_eq!(lines.len(), 2, "expected 2 JSONL rows, got: {text:?}");

        let header: serde_json::Value = serde_json::from_str(lines[0]).expect("row 1 valid JSON");
        assert_eq!(header["_kind"], "instance");
        assert_eq!(header["instance_id"], "synth_test_001");

        assert!(header["canonical_bitmaps"].is_array());

        assert!(header["cell_to_grid_idx"].is_array(),
                "header must include cell_to_grid_idx for reader-side grid mapping");
        let cell_to_grid = header["cell_to_grid_idx"].as_array().unwrap();
        assert_eq!(cell_to_grid.len(), 2, "board has 2 cells in setup");
        assert_eq!(cell_to_grid[0], 0);   // (0, 0) -> 0 * 22 + 0 = 0
        assert_eq!(cell_to_grid[1], 1);   // (0, 1) -> 0 * 22 + 1 = 1

        let placements_json = header["placements"].as_array()
            .expect("header must include placements table");
        assert_eq!(placements_json.len(), 1);
        assert_eq!(placements_json[0]["cells"], serde_json::json!([0]));
        assert_eq!(placements_json[0]["piece_def_idx"], 0);
        assert_eq!(placements_json[0]["mark_on_center"], false);

        let branch: serde_json::Value = serde_json::from_str(lines[1]).expect("row 2 valid JSON");
        assert_eq!(branch["_kind"], "branch");
        assert_eq!(branch["instance_id"], "synth_test_001");
        assert_eq!(branch["branch_id"], 0);

        let pre = &branch["pre_state"];
        assert!(pre.is_object(), "pre_state must live at branch level");

        let bitmap = pre["empty_bitmap"].as_array()
            .expect("empty_bitmap serialized as array of u8");
        assert_eq!(bitmap.len(), 28, "fixed-size 28-byte bitmap regardless of board size");
        assert!(pre["center_mark"].is_boolean());
        assert!(pre["counts"].is_array());

        let candidates = branch["candidates"].as_array().unwrap();
        assert_eq!(candidates.len(), 1);

        let cand = &candidates[0];
        assert!(cand.get("post_state").is_none(),
                "post_state must NOT appear in candidates — it's reconstructed by the reader");
        assert_eq!(cand["placement_idx"], 0);
        assert!(cand["tried"].is_boolean());
        assert!(cand["succeeded"].is_boolean());
        assert!(cand["subtree_nodes"].is_number());
    }

    #[test]
    fn solve_complete_writes_nothing_when_no_branches_recorded() {
        let (writer, buf) = make_capture_writer();
        let mut t = Tracer::new(writer);
        t.set_instance_id("trivial_instance".to_string());
        setup(&mut t, &["P0"], &["P0"], &[(0, 0)], &[]);

        // No on_branch calls — instance_buffer stays empty.
        t.on_solve_complete(true).unwrap();

        assert!(buf.borrow().is_empty(), "no branches → no output");
    }
}