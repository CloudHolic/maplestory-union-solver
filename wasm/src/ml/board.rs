// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Static MapleStory Union board layout.

use std::collections::{HashMap, HashSet};

use crate::domain::Coord;

const BOARD_HEIGHT: i8 = 20;
const BOARD_WIDTH: i8 = 22;

/// Per-row group assignment.
const BOARD_MAP_ROWS: [&str; BOARD_HEIGHT as usize] = [
    "0111111111122222222223",
    "0011111111122222222233",
    "0001111111122222222333",
    "0000111111122222223333",
    "0000011111122222233333",
    "00000ABBBBBCCCCCD33333",
    "00000AABBBBCCCCDD33333",
    "00000AAABBBCCCDDD33333",
    "00000AAAABBCCDDDD33333",
    "00000AAAAABCDDDDD33333",
    "44444EEEEEFGHHHHH55555",
    "44444EEEEFFGGHHHH55555",
    "44444EEEFFFGGGHHH55555",
    "44444EEFFFFGGGGHH55555",
    "44444EFFFFFGGGGGH55555",
    "4444466666677777755555",
    "4444666666677777775555",
    "4446666666677777777555",
    "4466666666677777777755",
    "4666666666677777777775"
];

/// Identifies one of the 16 board groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum GroupId {
    Outer(u8),
    Inner(u8)
}

impl GroupId {
    pub(crate) fn is_outer(self) -> bool {
        matches!(self, GroupId::Outer(_))
    }

    fn from_char(ch: char) -> Self {
        match ch {
            '0'..='7' => GroupId::Outer((ch as u8 - b'0') + 1),
            'A'..='H' => GroupId::Inner((ch as u8 - b'A') + 1),
            _ => panic!("BOARD_MAP_ROWS contains invalid group char: {ch:?}")
        }
    }

    /// Sort key: outer before inner, then by group number.
    fn sort_key(self) -> (u8, u8) {
        match self {
            GroupId::Outer(n) => (0, n),
            GroupId::Inner(n) => (1, n)
        }
    }
}

/// One of the 16 groups.
pub(crate) struct Group {
    pub id: GroupId,
    pub cells: Vec<Coord>,
    pub contains_center: bool
}

/// Static board layout, fully parsed.
pub(crate) struct UnionBoard {
    pub all_cells: Vec<Coord>,
    pub center_cells: Vec<Coord>,
    pub groups: Vec<Group>
}

impl UnionBoard {
    /// Parses BOARD_MAP_ROWS into the structured layout.
    pub(crate) fn new() -> Self {
        let mut by_group: HashMap<GroupId, Vec<Coord>> = HashMap::new();
        let mut all_cells: Vec<Coord> = Vec::with_capacity((BOARD_HEIGHT as usize) * (BOARD_WIDTH as usize));

        for (r, row) in BOARD_MAP_ROWS.iter().enumerate() {
            assert_eq!(row.len(), BOARD_WIDTH as usize,
                       "BOARD_MAP_ROWS[{}] has length {}, expected {}",
                       r, row.len(), BOARD_WIDTH);

            for (c, ch) in row.chars().enumerate() {
                let coord = (r as i8, c as i8);
                let group = GroupId::from_char(ch);
                by_group.entry(group).or_default().push(coord);
                all_cells.push(coord);
            }
        }

        let center_cells: Vec<Coord> = vec![
            (BOARD_HEIGHT / 2 - 1, BOARD_WIDTH / 2 - 1),
            (BOARD_HEIGHT / 2 - 1, BOARD_WIDTH / 2),
            (BOARD_HEIGHT / 2, BOARD_WIDTH / 2 - 1),
            (BOARD_HEIGHT / 2, BOARD_WIDTH / 2),
        ];
        let center_set: HashSet<Coord> = center_cells.iter().copied().collect();

        let mut groups: Vec<Group> = by_group.into_iter()
            .map(|(id, cells)| {
                let contains_center = cells.iter().any(|c| center_set.contains(c));
                Group { id, cells, contains_center }
            })
            .collect();

        // HasMap iteration order is non-deterministic - sort for reproducibility
        groups.sort_by_key(|g| g.id.sort_key());

        Self { all_cells, center_cells, groups }
    }

    pub(crate) fn outer_groups(&self) -> impl Iterator<Item = &Group> {
        self.groups.iter().filter(|g| g.id.is_outer())
    }

    pub(crate) fn inner_groups(&self) -> impl Iterator<Item = &Group> {
        self.groups.iter().filter(|g| !g.id.is_outer())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_cell_count_is_440() {
        let board = UnionBoard::new();
        assert_eq!(board.all_cells.len(), 440, "20×22 fully tiled = 440 cells");
    }

    #[test]
    fn has_16_groups_8_outer_8_inner() {
        let board = UnionBoard::new();
        assert_eq!(board.groups.len(), 16);
        assert_eq!(board.outer_groups().count(), 8);
        assert_eq!(board.inner_groups().count(), 8);
    }

    #[test]
    fn group_cells_are_pairwise_disjoint_and_cover_board() {
        let board = UnionBoard::new();

        let mut all: Vec<Coord> = board.groups.iter()
            .flat_map(|g| g.cells.iter().copied())
            .collect();
        let total = all.len();
        all.sort();
        all.dedup();

        assert_eq!(all.len(), total, "groups overlap (duplicate cells)");
        assert_eq!(total, 440, "groups don't cover all 440 cells");
    }

    #[test]
    fn center_cells_are_at_expected_positions() {
        let board = UnionBoard::new();
        let expected = vec![(9, 10), (9, 11), (10, 10), (10, 11)];
        assert_eq!(board.center_cells, expected);
    }

    #[test]
    fn center_belongs_to_inner_groups_2_3_6_7() {
        let board = UnionBoard::new();
        let center_groups: Vec<&GroupId> = board.groups.iter()
            .filter(|g| g.contains_center)
            .map(|g| &g.id)
            .collect();

        // 4 inner groups should each contain one center cell.
        assert_eq!(center_groups.len(), 4);
        for g in &center_groups {
            assert!(matches!(g, GroupId::Inner(2 | 3 | 6 | 7)),
                    "center should be in Inner(2|3|6|7), got {:?}", g);
        }
    }

    #[test]
    fn group_ids_are_unique() {
        let board = UnionBoard::new();
        let ids: HashSet<GroupId> = board.groups.iter().map(|g| g.id).collect();
        assert_eq!(ids.len(), 16);
    }
}