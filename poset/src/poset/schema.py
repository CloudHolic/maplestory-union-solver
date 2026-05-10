# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Input schema for the POSET branching policy.

Defines the dataclass for one post-candidate state - the model's input.
"""

from dataclasses import dataclass

# Board / Piece dimensions

CANONICAL_GRID = 6
CANONICAL_SIZE = CANONICAL_GRID * CANONICAL_GRID # 36

BOARD_ROWS = 20
BOARD_COLS = 22
BOARD_SIZE = BOARD_ROWS * BOARD_COLS # 440

# Schema dataclasses

@dataclass(slots=True, frozen=True)
class PostState:
    """Board state after virtually applying a candidate placement.

    The model's V(s) input.

    Fields:
        - empty_target_indices: row-major indices (r * BOARD_COLS + c) of empty target cells.
        - center_mark: 0 or 1. Whether the center 4-cell region still contains an unmarked target.
        - counts: remaining count per piece type, raw integers.
    """

    empty_target_indices: list[int]
    center_mark: int
    counts: list[int]
