// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Integration tests for the ExactCover solver.

use std::sync::atomic::AtomicI32;
use maplestory_union_solver_wasm::{
    CancelFlag, PieceDefJson, PieceInstanceJson,
    SolveOptions, SolverInput, SolverStats, ExactCoverInput,
    solve_exact_cover
};

/// Builds a 2x2 board input with one 2x2 piece. Center at (0, 0).
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
fn solve_returns_valid_result_for_trivial_input() {
    let input = make_2x2_input();
    let options = SolveOptions {
        seed: Some(1),
        ..Default::default()
    };

    let result = solve_exact_cover(&input, options, None,
                                   #[cfg(feature = "tracing")] None).expect("solver should not error");

    assert!(result.solution.is_some(), "trivial input should be solved");
    let solution = result.solution.unwrap();
    assert_eq!(solution.len(), 1);
    assert_eq!(solution[0].piece.def_id, "square");
}

#[test]
fn solve_records_seed_in_stats() {
    let input = make_2x2_input();
    let options = SolveOptions {
        seed: Some(0xDEADBEEF),
        ..Default::default()
    };

    let result = solve_exact_cover(&input, options, None,
                                   #[cfg(feature = "tracing")] None).unwrap();
    assert_eq!(result.stats.common.seed, 0xDEADBEEF);
}

#[test]
fn solve_with_default_options_works() {
    let input = make_2x2_input();
    let result = solve_exact_cover(&input, SolveOptions::default(), None,
                                   #[cfg(feature = "tracing")] None).unwrap();
    assert!(result.solution.is_some());
}

#[test]
fn input_round_trips_through_json() {
    let input = make_2x2_input();
    let json = serde_json::to_string(&input).expect("serialize");
    let parsed: ExactCoverInput = serde_json::from_str(&json).expect("deserialize");

    let opts1 = SolveOptions {
        seed: Some(42),
        ..Default::default()
    };
    let opts2 = opts1.clone();

    let r1 = solve_exact_cover(&input, opts1, None,
                               #[cfg(feature = "tracing")] None).unwrap();
    let r2 = solve_exact_cover(&parsed, opts2, None,
                               #[cfg(feature = "tracing")] None).unwrap();

    assert_eq!(
        r1.solution.is_some(),
        r2.solution.is_some(),
        "round-tripped input should have same solvability"
    );
}

#[test]
fn result_serializes_to_camel_case_json() {
    let input = make_2x2_input();
    let result = solve_exact_cover(&input, SolveOptions::default(), None,
                                   #[cfg(feature = "tracing")] None).unwrap();
    let json = serde_json::to_string(&result).unwrap();

    assert!(json.contains(r#""nodeCount""#), "should be camelCase");
    assert!(json.contains(r#""timedOut""#), "should be camelCase");
    assert!(!json.contains(r#""node_count""#), "should not be snake_case");
}

#[test]
fn solver_stats_has_all_expected_fields() {
    let input = make_2x2_input();
    let result = solve_exact_cover(&input, SolveOptions::default(), None,
                                   #[cfg(feature = "tracing")] None).unwrap();
    let stats: &SolverStats = &result.stats.common;

    let _ = stats.node_count;
    let _ = stats.restarts;
    let _ = stats.unit_propagations;
    let _ = stats.island_prunes;
    let _ = stats.dead_cell_prunes;
    let _ = stats.neighbor_prunes;
    let _ = stats.parity_prunes;
    let _ = stats.seed;
    let _ = stats.timed_out;
    let _ = stats.elapsed_ms;
    let _ = stats.cancelled;
}

#[test]
fn cancel_flag_stops_solve_and_serializes_to_camel_case() {
    let input = make_2x2_input();
    let flag = AtomicI32::new(1);  // pre-cancelled
    let cancel = CancelFlag::new(&flag);

    let result = solve_exact_cover(&input, SolveOptions::default(), Some(&cancel),
                                   #[cfg(feature = "tracing")] None).unwrap();

    assert!(result.solution.is_none());
    assert!(result.stats.common.cancelled);

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains(r#""cancelled":true"#), "wire format should be camelCase 'cancelled'");
}

#[test]
fn no_cancel_flag_means_solve_completes() {
    let input = make_2x2_input();
    let result = solve_exact_cover(&input, SolveOptions::default(), None,
                                   #[cfg(feature = "tracing")] None).unwrap();

    assert!(result.solution.is_some());
    assert!(!result.stats.common.cancelled);
}