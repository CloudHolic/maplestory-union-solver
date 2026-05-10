# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Unit tests for poset.transforms."""

import pytest
import torch

from poset.schema import BOARD_SIZE, CANONICAL_SIZE, PostState
from poset.transforms import post_state_to_tensors


def test_basic_shapes():
	state = PostState(
		empty_target_indices=[0, 5, 100],
		center_mark=1,
		counts=[3, 2],
	)
	bitmaps = [[0] * CANONICAL_SIZE, [1] * CANONICAL_SIZE]

	out = post_state_to_tensors(state, bitmaps)

	assert out["empty_target"].shape == (BOARD_SIZE,)
	assert out["center_mark"].shape == (1,)
	assert out["pieces"].shape == (2, CANONICAL_SIZE)
	assert out["counts"].shape == (2,)


def test_count_zero_filter():
	"""Pieces with count=0 are filtered out (DeepSet sees only available)."""
	state = PostState(
		empty_target_indices=[],
		center_mark=0,
		counts=[3, 0, 2, 0, 1],
	)
	bitmaps = [[i] * CANONICAL_SIZE for i in range(5)]

	out = post_state_to_tensors(state, bitmaps)

	# Only counts [3, 2, 1] survive (indices 0, 2, 4).
	assert out["pieces"].shape == (3, CANONICAL_SIZE)
	assert out["counts"].tolist() == [3.0, 2.0, 1.0]
	assert out["pieces"][0, 0].item() == 0.0
	assert out["pieces"][1, 0].item() == 2.0
	assert out["pieces"][2, 0].item() == 4.0


def test_empty_target_indices_to_dense():
	indices = [0, 5, 100, BOARD_SIZE - 1]
	state = PostState(
		empty_target_indices=indices,
		center_mark=0,
		counts=[1],
	)
	bitmaps = [[0] * CANONICAL_SIZE]

	out = post_state_to_tensors(state, bitmaps)

	expected = torch.zeros(BOARD_SIZE)
	for i in indices:
		expected[i] = 1.0
	assert torch.equal(out["empty_target"], expected)


def test_empty_target_indices_empty():
	"""All target cells covered — bitmap is all zeros."""
	state = PostState(
		empty_target_indices=[],
		center_mark=1,
		counts=[1],
	)
	bitmaps = [[0] * CANONICAL_SIZE]

	out = post_state_to_tensors(state, bitmaps)

	assert torch.equal(out["empty_target"], torch.zeros(BOARD_SIZE))


def test_center_mark_values():
	for mark in [0, 1]:
		state = PostState(
			empty_target_indices=[0],
			center_mark=mark,
			counts=[1],
		)
		bitmaps = [[0] * CANONICAL_SIZE]
		out = post_state_to_tensors(state, bitmaps)
		assert out["center_mark"].tolist() == [float(mark)]


def test_counts_bitmaps_length_mismatch():
	state = PostState(
		empty_target_indices=[],
		center_mark=0,
		counts=[1, 2, 3],
	)
	bitmaps = [[0] * CANONICAL_SIZE]  # length 1, not 3

	with pytest.raises(ValueError, match="must align"):
		post_state_to_tensors(state, bitmaps)


def test_bitmap_wrong_size():
	state = PostState(
		empty_target_indices=[],
		center_mark=0,
		counts=[1],
	)
	bitmaps = [[0] * 25]  # 5×5, not 6×6

	with pytest.raises(ValueError, match="CANONICAL_SIZE"):
		post_state_to_tensors(state, bitmaps)


def test_all_counts_zero_raises():
	"""Solver invariant: at branch time at least one piece has count > 0."""
	state = PostState(
		empty_target_indices=[],
		center_mark=0,
		counts=[0, 0, 0],
	)
	bitmaps = [[0] * CANONICAL_SIZE for _ in range(3)]

	with pytest.raises(ValueError, match="count > 0"):
		post_state_to_tensors(state, bitmaps)


def test_dtype_float32():
	"""All tensors should be float32 — model input expectation."""
	state = PostState(
		empty_target_indices=[0],
		center_mark=1,
		counts=[3, 2],
	)
	bitmaps = [[0] * CANONICAL_SIZE, [1] * CANONICAL_SIZE]

	out = post_state_to_tensors(state, bitmaps)

	assert out["empty_target"].dtype == torch.float32
	assert out["center_mark"].dtype == torch.float32
	assert out["pieces"].dtype == torch.float32
	assert out["counts"].dtype == torch.float32