// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Integration tests for the ExactCover solver with tracing feature.

#![cfg(feature = "tracing")]

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::sync::atomic::AtomicI32;

use solver::{
    CancelFlag, ExactCoverInput, PieceDefJson, PieceInstanceJson,
    SolveOptions, SolverInput, Tracer,
    solve_exact_cover
};

/// Capture writer
///
/// Box<dyn Write>-compatible writer that funnels into a shared Vec<u8>.
/// Tracer takes ownership of its writer; sharing through Rc<RefCell> lets
/// the test inspect the buffer after the solve completes.
struct SharedBuf(Rc<RefCell<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn make_capture_writer() -> (Box<dyn Write>, Rc<RefCell<Vec<u8>>>) {
    let buf = Rc::new(RefCell::new(Vec::<u8>::new()));
    (Box::new(SharedBuf(buf.clone())), buf)
}

/// Builds a 2x2 board input with one 2x2 piece. Center at (0, 0).
fn make_2x2_dominoes_input() -> ExactCoverInput {
    ExactCoverInput {
        target_cells: vec![
            "0,0".to_string(),
            "0,1".to_string(),
            "1,0".to_string(),
            "1,1".to_string(),
        ],
        common: SolverInput {
            pieces: vec![
                PieceInstanceJson { def_id: "domino".to_string(), index: 0 },
                PieceInstanceJson { def_id: "domino".to_string(), index: 1 },
            ],
            piece_defs: vec![(
                "domino".to_string(),
                PieceDefJson {
                    id: "domino".to_string(),
                    cells: vec![(0, 0), (0, 1)],
                    mark_index: 0,
                },
            )],
            center_cells: vec!["0,0".to_string()],
        },
    }
}

#[test]
fn tracer_captures_branches_during_solve() {
    let (writer, buf) = make_capture_writer();
    let mut tracer = Tracer::new(writer);
    tracer.set_instance_id("integration_test_001".to_string());

    let input = make_2x2_dominoes_input();
    let options = SolveOptions {
        seed: Some(7),
        ..Default::default()
    };

    let result = solve_exact_cover(
        &input,
        options,
        None,
        Some(&mut tracer)
    ).expect("solver should not error");

    // Sanity: the solve actually ran and succeeded.
    assert!(result.solution.is_some(), "expected a solution");
    assert!(result.stats.common.node_count > 0, "expected nodes visited");

    // Decode the captured JSONL.
    let bytes = buf.borrow().clone();
    let text = std::str::from_utf8(&bytes).expect("output must be valid UTF-8");
    let lines: Vec<&str> = text.lines().collect();

    assert!(
        lines.len() >= 2,
        "expected ≥2 rows (header + ≥1 branch), got {}: {text:?}",
        lines.len()
    );

    // Row 0: instance header
    let header: serde_json::Value =
        serde_json::from_str(lines[0]).expect("row 0 must be valid JSON");
    assert_eq!(header["_kind"], "instance");
    assert_eq!(header["instance_id"], "integration_test_001");

    let bitmaps = header["canonical_bitmaps"].as_array().expect("canonical_bitmaps array");
    assert_eq!(bitmaps.len(), 1, "one piece def → one bitmap");

    let bitmap = bitmaps[0].as_array().expect("bitmap is array");
    assert_eq!(bitmap.len(), 36, "canonical bitmap is 6x6 = 36 cells");

    // Row 1+: branch records
    for (i, line) in lines[1..].iter().enumerate() {
        let branch: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("row {} must be valid JSON: {}", i + 1, e));

        assert_eq!(branch["_kind"], "branch");
        assert_eq!(branch["instance_id"], "integration_test_001");
        assert!(branch["branch_id"].is_u64());

        let candidates = branch["candidates"].as_array().expect("candidates array");
        assert!(!candidates.is_empty(), "branch must have candidates");

        for cand in candidates {
            assert!(cand["placement_idx"].is_u64());
            assert!(cand["tried"].is_boolean());
            assert!(cand["succeeded"].is_boolean());
            assert!(cand["subtree_nodes"].is_u64());

            let post = &cand["post_state"];
            assert!(
                post["empty_target_indices"].is_array(),
                "post_state.empty_target_indices must be array"
            );
            assert!(
                post["center_mark"].is_boolean(),
                "post_state.center_mark must be boolean"
            );
            assert!(post["counts"].is_array(), "post_state.counts must be array");

            assert!(
                post.get("pieces").is_none(),
                "post_state must NOT contain pieces (it lives in the instance header)"
            );
        }
    }
}

#[test]
fn tracer_writes_nothing_when_solve_cancelled() {
    let (writer, buf) = make_capture_writer();
    let mut tracer = Tracer::new(writer);
    tracer.set_instance_id("cancelled_instance".to_string());

    let input = make_2x2_dominoes_input();
    let flag = AtomicI32::new(1); // cancelled before solve starts
    let cancel = CancelFlag::new(&flag);

    let result = solve_exact_cover(
        &input,
        SolveOptions::default(),
        Some(&cancel),
        Some(&mut tracer)
    ).expect("solver should not error");

    assert!(result.solution.is_none(), "cancelled solve must not return a solution");
    assert!(result.stats.common.cancelled);

    assert!(
        buf.borrow().is_empty(),
        "cancelled solve must not write JSONL output"
    );
}