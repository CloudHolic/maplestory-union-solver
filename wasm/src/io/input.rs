// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Input types for the solvers, deserialized from JSON.

use serde::{Deserialize, Serialize};

use crate::domain::Coord;
use crate::error::Result;
use crate::io::{SolverInput, parse_cell_key};

/// One group constraint for the GroupCount solver.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupConstraintJson {
    pub group_id: u32,
    pub cells: Vec<String>,
    pub count: u32
}

impl GroupConstraintJson {
    /// Parses this group's `cells` strings into coordinates.
    pub fn parse_cells(&self) -> Result<Vec<Coord>> {
        self.cells
            .iter()
            .map(|s| parse_cell_key(s))
            .collect()
    }
}

/// Input to the ExactCover solver.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExactCoverInput {
    pub target_cells: Vec<String>,

    #[serde(flatten)]
    pub common: SolverInput
}

impl ExactCoverInput {
    /// Parses `target_cells` strings into coordinates.
    pub fn parse_target_cells(&self) -> Result<Vec<Coord>> {
        self.target_cells
            .iter()
            .map(|s| parse_cell_key(s))
            .collect()
    }
}

/// Input to the GroupCount solver.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupCountInput {
    pub exact_cells: Vec<String>,
    pub group_constraints: Vec<GroupConstraintJson>,

    #[serde(flatten)]
    pub common: SolverInput
}

impl GroupCountInput {
    /// Parses `exact_cells` strings into coordinates.
    pub fn parse_exact_cells(&self) -> Result<Vec<Coord>> {
        self.exact_cells
            .iter()
            .map(|s| parse_cell_key(s))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_exact_cover_input() {
        let json = r#"{
            "targetCells": ["0,0", "0,1", "1,0"],
            "pieces": [
                { "defId": "archer_5", "index": 0 },
                { "defId": "archer_5", "index": 1 }
            ],
            "pieceDefs": [
                ["archer_5", { "id": "archer_5", "cells": [[0,0],[0,1],[1,0],[1,1],[1,2]], "markIndex": 2 }]
            ],
            "centerCells": ["10,10", "10,11"]
        }"#;

        let input: ExactCoverInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.target_cells.len(), 3);
        assert_eq!(input.common.pieces.len(), 2);
        assert_eq!(input.common.pieces[0].def_id, "archer_5");
        assert_eq!(input.common.piece_defs.len(), 1);
        assert_eq!(input.common.center_cells.len(), 2);

        let coords = input.parse_target_cells().unwrap();
        assert_eq!(coords, vec![(0, 0), (0, 1), (1, 0)]);

        let map = input.common.piece_defs_map();
        assert!(map.contains_key("archer_5"));
        assert_eq!(map["archer_5"].cells.len(), 5);
        assert_eq!(map["archer_5"].mark_index, 2);
    }

    #[test]
    fn deserialize_group_count_input() {
        let json = r#"{
            "exactCells": ["5,5"],
            "groupConstraints": [
                {
                    "groupId": 0,
                    "cells": ["0,0", "0,1", "0,2"],
                    "count": 2
                },
                {
                    "groupId": 1,
                    "cells": ["1,0", "1,1"],
                    "count": 1
                }
            ],
            "pieces": [{ "defId": "p1", "index": 0 }],
            "pieceDefs": [
                ["p1", { "id": "p1", "cells": [[0,0]], "markIndex": 0 }]
            ],
            "centerCells": ["10,10"]
        }"#;

        let input: GroupCountInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.exact_cells, vec!["5,5"]);
        assert_eq!(input.group_constraints.len(), 2);
        assert_eq!(input.group_constraints[0].group_id, 0);
        assert_eq!(input.group_constraints[0].count, 2);
        assert_eq!(input.group_constraints[1].cells.len(), 2);
        assert_eq!(input.common.pieces.len(), 1);

        let exact = input.parse_exact_cells().unwrap();
        assert_eq!(exact, vec![(5, 5)]);

        let group0_cells = input.group_constraints[0].parse_cells().unwrap();
        assert_eq!(group0_cells, vec![(0, 0), (0, 1), (0, 2)]);
    }

    #[test]
    fn flatten_means_no_common_envelope() {
        let json = r#"{
            "targetCells": [],
            "pieces": [],
            "pieceDefs": [],
            "centerCells": []
        }"#;
        let input: ExactCoverInput = serde_json::from_str(json).unwrap();
        
        assert_eq!(input.target_cells.len(), 0);
        assert_eq!(input.common.pieces.len(), 0);
    }

    #[test]
    fn serialize_round_trip() {
        let input = ExactCoverInput {
            target_cells: vec!["0,0".to_string()],
            common: SolverInput {
                pieces: vec![],
                piece_defs: vec![],
                center_cells: vec!["1,1".to_string()],
            },
        };
        let json = serde_json::to_string(&input).unwrap();
        
        assert!(json.contains(r#""targetCells":["0,0"]"#));
        assert!(json.contains(r#""centerCells":["1,1"]"#));
        assert!(!json.contains(r#""common""#));
    }
}
