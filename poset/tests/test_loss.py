# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Unit tests for training.loss."""

import math

import pytest
import torch
from training.loss import SUCCESS_LABEL, compute_label, pairwise_margin_loss
from training.schema import Candidate

from poset.schema import PostState


def _make_candidate(*, tried: bool, succeeded: bool, subtree_nodes: int) -> Candidate:
	return Candidate(
		placement_idx=0,
		post_state=PostState(empty_target_indices=[], center_mark=0, counts=[]),
		tried=tried,
		succeeded=succeeded,
		subtree_nodes=subtree_nodes,
	)


# compute_label

def test_label_succeeded():
	c = _make_candidate(tried=True, succeeded=True, subtree_nodes=42)
	assert compute_label(c) == SUCCESS_LABEL


def test_label_failed_formula():
	c = _make_candidate(tried=True, succeeded=False, subtree_nodes=10)
	expected = 2.0 / (1.0 + math.log(10))
	assert compute_label(c) == pytest.approx(expected)


def test_label_failed_decreases_with_subtree():
	"""Bigger wasted subtree → smaller label."""
	small = _make_candidate(tried=True, succeeded=False, subtree_nodes=5)
	big = _make_candidate(tried=True, succeeded=False, subtree_nodes=500)
	assert compute_label(small) > compute_label(big)


def test_label_failed_subtree_one():
	"""subtree_nodes=1 → log(1)=0 → label=2.0. No division-by-zero."""
	c = _make_candidate(tried=True, succeeded=False, subtree_nodes=1)
	assert compute_label(c) == pytest.approx(2.0)


def test_label_untried_raises():
	c = _make_candidate(tried=False, succeeded=False, subtree_nodes=0)
	with pytest.raises(ValueError, match="tried=False"):
		compute_label(c)


# pairwise_margin_loss

def test_pairwise_basic():
	"""One branch, two candidates, correct order → loss = 0 (within margin)."""
	scores = torch.tensor([2.0, 0.5], requires_grad=True)
	labels = torch.tensor([3.0, 1.0])
	groups = torch.tensor([0, 0])

	loss = pairwise_margin_loss(scores, labels, groups, margin=1.0)
	assert loss.item() == pytest.approx(0.0)


def test_pairwise_violation():
	"""Wrong order → positive loss."""
	scores = torch.tensor([0.0, 1.0], requires_grad=True)
	labels = torch.tensor([3.0, 1.0])
	groups = torch.tensor([0, 0])

	loss = pairwise_margin_loss(scores, labels, groups, margin=1.0)
	assert loss.item() == pytest.approx(2.0)


def test_pairwise_groups_isolate():
	"""Pairs across different branches don't contribute."""
	scores = torch.tensor([0.0, 1.0, 1.0, 0.0], requires_grad=True)
	labels = torch.tensor([3.0, 1.0, 3.0, 1.0])
	groups = torch.tensor([0, 0, 1, 1])

	loss = pairwise_margin_loss(scores, labels, groups, margin=1.0)
	assert loss.item() == pytest.approx(1.0)


def test_pairwise_equal_labels_skipped():
	"""Pairs with same label are not eligible."""
	scores = torch.tensor([0.0, 5.0], requires_grad=True)
	labels = torch.tensor([3.0, 3.0])
	groups = torch.tensor([0, 0])

	loss = pairwise_margin_loss(scores, labels, groups)
	assert loss.item() == 0.0
	loss.backward()


def test_pairwise_no_eligible_pairs():
	"""Singleton branches → loss = 0 with grad."""
	scores = torch.tensor([1.0, 2.0, 3.0], requires_grad=True)
	labels = torch.tensor([3.0, 3.0, 3.0])
	groups = torch.tensor([0, 1, 2])

	loss = pairwise_margin_loss(scores, labels, groups)
	assert loss.item() == 0.0
	loss.backward()


def test_pairwise_grad_flows():
	"""Loss should produce gradients for scores."""
	scores = torch.tensor([0.0, 1.0], requires_grad=True)
	labels = torch.tensor([3.0, 1.0])
	groups = torch.tensor([0, 0])

	loss = pairwise_margin_loss(scores, labels, groups)
	loss.backward()
	assert scores.grad is not None
	assert scores.grad[0] < 0
	assert scores.grad[1] > 0


def test_pairwise_shape_mismatch():
	scores = torch.tensor([1.0, 2.0])
	labels = torch.tensor([3.0])
	groups = torch.tensor([0, 0])
	with pytest.raises(ValueError, match="shape mismatch"):
		pairwise_margin_loss(scores, labels, groups)