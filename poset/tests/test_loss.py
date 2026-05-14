# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Unit tests for training.loss."""

import math

import pytest
import torch

from training.loss import compute_label, regression_loss
from training.schema import Candidate


def _make_candidate(*, tried: bool, succeeded: bool, subtree_nodes: int) -> Candidate:
	return Candidate(
		placement_idx=0,
		tried=tried,
		succeeded=succeeded,
		subtree_nodes=subtree_nodes,
	)


# compute_label

def test_label_succeeded():
	c = _make_candidate(tried=True, succeeded=True, subtree_nodes=42)
	assert compute_label(c) == 3.0


def test_label_failed_formula():
	c = _make_candidate(tried=True, succeeded=False, subtree_nodes=10)
	expected = 2.0 / (1.0 + math.log1p(10))
	assert compute_label(c) == pytest.approx(expected)


def test_label_failed_decreases_with_subtree():
	"""Bigger wasted subtree → smaller label."""
	small = _make_candidate(tried=True, succeeded=False, subtree_nodes=5)
	big = _make_candidate(tried=True, succeeded=False, subtree_nodes=500)
	assert compute_label(small) > compute_label(big)


def test_label_failed_subtree_zero():
	"""subtree_nodes=0 → log1p(0)=0 → label=2.0. No divide-by-zero."""
	c = _make_candidate(tried=True, succeeded=False, subtree_nodes=0)
	assert compute_label(c) == pytest.approx(2.0)


def test_label_untried_raises():
	c = _make_candidate(tried=False, succeeded=False, subtree_nodes=0)
	with pytest.raises(ValueError, match="untried"):
		compute_label(c)


# regression_loss

def test_regression_zero_when_match():
	"""Perfect predictions → loss = 0."""
	scores = torch.tensor([1.0, 2.0, 3.0])
	labels = torch.tensor([1.0, 2.0, 3.0])
	loss = regression_loss(scores, labels)
	assert loss.item() == pytest.approx(0.0)


def test_regression_grad_flows():
	"""Loss should produce gradients for scores."""
	scores = torch.tensor([0.0, 1.0], requires_grad=True)
	labels = torch.tensor([2.0, 0.0])
	loss = regression_loss(scores, labels)
	loss.backward()
	assert scores.grad is not None
	assert scores.grad[0] < 0   # scores[0] too low, gradient pulls up
	assert scores.grad[1] > 0   # scores[1] too high, gradient pulls down


def test_regression_huber_quadratic_region():
	"""Within |delta|=1 of label, huber is quadratic (like MSE/2)."""
	scores = torch.tensor([0.5])
	labels = torch.tensor([0.0])
	loss = regression_loss(scores, labels)
	# huber with delta=1, diff=0.5 → 0.5 * 0.5^2 = 0.125
	assert loss.item() == pytest.approx(0.125)


def test_regression_huber_linear_region():
	"""Beyond |delta|=1, huber is linear (less sensitive than MSE)."""
	scores = torch.tensor([5.0])
	labels = torch.tensor([0.0])
	loss = regression_loss(scores, labels)
	# huber with delta=1, diff=5 → delta * (|diff| - 0.5*delta) = 1 * 4.5 = 4.5
	assert loss.item() == pytest.approx(4.5)