# GroupCount solver

*The design of the GroupCount solver is an open problem under active
exploration in a separate prototype repository. This document will be
written once the approach stabilizes.*

## Problem statement

The GroupCount solver generalizes ExactCover to the case where the
covered cell set is not fixed. Instead, the board is partitioned into
named groups, each with a target cell count. The solver must choose
*which* cells within each group to cover such that:

- The per-group cell counts are met.
- The resulting set of covered cells is 4-connected.
- At least one placed piece has its mark cell on the board center.

ExactCover is the special case where every group has `target = size`
(i.e., exact cover within each group).

## Why this is harder

The added degree of freedom — choosing which cells to cover within a
group — interacts badly with connectivity. A naive approach of "skip
decisions" explodes combinatorially: enumerating cell subsets of size
`target` within each group yields `C(size, target)` options per group,
multiplied across groups.

The PoC repository has explored several approaches, none fully
satisfactory yet:

- **DLX with skip pieces**: infeasible row-count blowup.
- **SAT encoding (MiniSat CDCL)**: cannot encode connectivity naturally,
  and does not exploit the geometric structure that makes ExactCover
  efficient.
- **Unified backtracking with dynamic branching**: branch on cells when a
  group becomes "tight" (slack = 0), on pieces otherwise. This preserves
  ExactCover's pruning but performance characteristics remain under
  investigation.

## Status

Not implemented in this repository. Design iteration continues in the
prototype.
