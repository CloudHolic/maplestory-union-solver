// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Wire-format types shared by both solver variants.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use crate::domain::{Coord, PieceDef};
use crate::error::{Result, SolverError};

/// Wire format for [`PieceDef`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct PieceDefJson {
    pub id: String,
    pub cells: Vec<Coord>,
    pub mark_index: usize
}

impl From<PieceDefJson> for PieceDef {
    fn from(j: PieceDefJson) -> Self {
        PieceDef {
            id: j.id,
            cells: j.cells,
            mark_index: j.mark_index
        }
    }
}

/// One instance of a piece in the puzzle input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct PieceInstanceJson {
    pub def_id: String,
    pub index: u16
}

/// Fields shared by both [`ExactCoverInput`](crate::io::ExactCoverInput)
/// and [`GroupCountInput`](crate::io::GroupCountInput).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct SolverInput {
    pub pieces: Vec<PieceInstanceJson>,
    pub piece_defs: Vec<(String, PieceDefJson)>,
    pub center_cells: Vec<String>
}

impl SolverInput {
    /// Parses `center_cells` strings (`"r,c"` format) into coordinates.
    pub fn parse_center_cells(&self) -> Result<Vec<Coord>> {
        self.center_cells
            .iter()
            .map(|s| parse_cell_key(s))
            .collect()
    }

    /// Converts the on-wire `Vec<(String, PieceDefJson)>` to
    /// a `HashMap<String, PieceDef>` keyed by piece id.
    pub fn piece_defs_map(&self) -> HashMap<String, PieceDef> {
        self.piece_defs
            .iter()
            .map(|(id, def)| (id.clone(), def.clone().into()))
            .collect()
    }
}

/// Fields shared by both [`ExactCoverStats`](crate::io::ExactCoverStats)
/// and [`GroupCountStats`](crate::io::GroupCountStats).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify_next::Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi, large_number_types_as_bigints))]
#[serde(rename_all = "camelCase")]
pub struct SolverStats {
    pub node_count: u64,
    pub restarts: u32,
    pub unit_propagations: u64,
    pub island_prunes: u64,
    pub dead_cell_prunes: u64,
    pub neighbor_prunes: u64,
    pub parity_prunes: u64,
    pub seed: u64,
    pub timed_out: bool,
    pub elapsed_ms: u64,
    pub cancelled: bool
}

impl SolverStats {
    /// Constructs a stats object representing zero work.
    pub fn empty(seed: u64) -> Self {
        Self {
            node_count: 0,
            restarts: 0,
            unit_propagations: 0,
            island_prunes: 0,
            dead_cell_prunes: 0,
            neighbor_prunes: 0,
            parity_prunes: 0,
            seed,
            timed_out: false,
            elapsed_ms: 0,
            cancelled: false
        }
    }
}

/// Parses a cell key `"r,c"` into a [`Coord`].
pub(crate) fn parse_cell_key(s: &str) -> Result<Coord> {
    let mut parts = s.split(',');
    let r = parts.next().and_then(|x| x.trim().parse::<i8>().ok());
    let c = parts.next().and_then(|x| x.trim().parse::<i8>().ok());

    match (r, c, parts.next()) {
        (Some(r), Some(c), None) => Ok((r, c)),
        _ => Err(make_parse_error(s))
    }
}

fn make_parse_error(s: &str) -> SolverError {
    let msg = format!("malformed cell key: {s:?}");
    SolverError::Json(serde::de::Error::custom(msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cell_key_basic() {
        assert_eq!(parse_cell_key("3,5").unwrap(), (3, 5));
        assert_eq!(parse_cell_key("0,0").unwrap(), (0, 0));
        assert_eq!(parse_cell_key("21,21").unwrap(), (21, 21));
    }

    #[test]
    fn parse_cell_key_with_whitespace_is_lenient() {
        assert_eq!(parse_cell_key(" 3 , 5 ").unwrap(), (3, 5));
    }

    #[test]
    fn parse_cell_key_rejects_malformed() {
        assert!(parse_cell_key("3").is_err());
        assert!(parse_cell_key("3,5,7").is_err());
        assert!(parse_cell_key("a,b").is_err());
        assert!(parse_cell_key("").is_err());
        assert!(parse_cell_key("3, ").is_err());
    }

    #[test]
    fn empty_common_stats_is_zero() {
        let s = SolverStats::empty(42);
        assert_eq!(s.seed, 42);
        assert_eq!(s.node_count, 0);
        assert_eq!(s.restarts, 0);
        assert!(!s.timed_out);
        assert!(!s.cancelled);
    }

    #[test]
    fn solve_common_methods_work() {
        let common = SolverInput {
            pieces: vec![PieceInstanceJson {
                def_id: "p1".to_string(),
                index: 0,
            }],
            piece_defs: vec![(
                "p1".to_string(),
                PieceDefJson {
                    id: "p1".to_string(),
                    cells: vec![(0, 0), (0, 1)],
                    mark_index: 0,
                },
            )],
            center_cells: vec!["10,10".to_string(), "10,11".to_string()],
        };

        let centers = common.parse_center_cells().unwrap();
        assert_eq!(centers, vec![(10, 10), (10, 11)]);

        let defs = common.piece_defs_map();
        assert!(defs.contains_key("p1"));
        assert_eq!(defs["p1"].cells.len(), 2);
    }
}
