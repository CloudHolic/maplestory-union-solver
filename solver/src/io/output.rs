// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Outputs for the solvers.

use serde::Serialize;

use crate::domain::Coord;
use crate::io::{PieceInstanceJson, SolverStats};

/// A complete solution: ordered list of placements.
pub type Solution = Vec<SolutionPlacement>;

/// One piece placement in a solution.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct SolutionPlacement {
    pub piece: PieceInstanceJson,
    pub cells: Vec<Coord>,
    pub mark: Coord
}

/// Result returned by the ExactCover solver.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, large_number_types_as_bigints))]
#[serde(rename_all = "camelCase")]
pub struct ExactCoverResult {
    pub solution: Option<Solution>,
    pub stats: ExactCoverStats
}

/// Statistics from an ExactCover solve.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, large_number_types_as_bigints))]
pub struct ExactCoverStats {
    #[serde(flatten)]
    pub common: SolverStats
}

impl ExactCoverStats {
    pub fn empty(seed: u64) -> Self {
        Self {
            common: SolverStats::empty(seed)
        }
    }
}

/// Result returned by the GroupCount solver.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, large_number_types_as_bigints))]
#[serde(rename_all = "camelCase")]
pub struct GroupCountResult {
    pub solution: Option<Solution>,
    pub stats: GroupCountStats
}

/// Statistics from a GroupCount solve.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, large_number_types_as_bigints))]
#[serde(rename_all = "camelCase")]
pub struct GroupCountStats {
    #[serde(flatten)]
    pub common: SolverStats,

    pub budget_prunes: u64,
    pub group_slack_prunes: u64
}

impl GroupCountStats {
    pub fn empty(seed: u64) -> Self {
        Self {
            common: SolverStats::empty(seed),
            budget_prunes: 0,
            group_slack_prunes: 0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_exact_cover_stats() {
        let s = ExactCoverStats::empty(42);
        assert_eq!(s.common.seed, 42);
        assert_eq!(s.common.node_count, 0);
    }

    #[test]
    fn empty_group_count_stats() {
        let s = GroupCountStats::empty(42);
        assert_eq!(s.common.seed, 42);
        assert_eq!(s.budget_prunes, 0);
        assert_eq!(s.group_slack_prunes, 0);
    }

    #[test]
    fn exact_cover_result_serializes_flat() {
        let result = ExactCoverResult {
            solution: None,
            stats: ExactCoverStats::empty(7),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains(r#""nodeCount":0"#));
        assert!(json.contains(r#""seed":7"#));
        assert!(json.contains(r#""timedOut":false"#));
        assert!(json.contains(r#""solution":null"#));
        // No nested "common" envelope:
        assert!(!json.contains(r#""common""#));
    }

    #[test]
    fn group_count_result_serializes_flat_with_extras() {
        let result = GroupCountResult {
            solution: None,
            stats: GroupCountStats::empty(99),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains(r#""seed":99"#));
        assert!(json.contains(r#""budgetPrunes":0"#));
        assert!(json.contains(r#""groupSlackPrunes":0"#));
        assert!(!json.contains(r#""common""#));
    }

    #[test]
    fn solution_placement_serializes() {
        let p = SolutionPlacement {
            piece: PieceInstanceJson {
                def_id: "archer_5".to_string(),
                index: 0,
            },
            cells: vec![(1, 0), (1, 1), (1, 2)],
            mark: (1, 1),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains(r#""defId":"archer_5""#));
        assert!(json.contains(r#""cells":[[1,0],[1,1],[1,2]]"#));
        assert!(json.contains(r#""mark":[1,1]"#));
    }
}