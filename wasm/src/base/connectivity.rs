// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Connected-component analysis on bit-encoded graphs.

use crate::base::BitSet;

/// Reusable scratch buffers for [`bfs_components`].
pub(crate) struct BfsWorkspace {
    /// `visited[i]` is non-zero when cell `i` has been seen by BFS in the current cell.
    pub visited: Vec<u8>,

    /// BFS frontier stack. Reused across components.
    pub stack: Vec<u32>,

    /// Sizes of discovered components, one entry per component.
    pub comp_sizes: Vec<u32>
}

impl BfsWorkspace {
    /// Allocates buffers sized for `total_cells` cells.
    pub(crate) fn new(total_cells: usize) -> Self {
        Self {
            visited: vec![0; total_cells],
            stack: Vec::with_capacity(total_cells),
            comp_sizes: Vec::with_capacity(64)
        }
    }

    /// Resets the visited bitmap and reusable scratch space.
    fn reset(&mut self) {
        self.visited.iter_mut().for_each(|v| *v = 0);
        self.stack.clear();
        self.comp_sizes.clear();
    }
}

/// Partition uncovered cells into 4-connected components via BFS.
///
/// Component sizes are written to `worksapce.comp_sizes` (cleared and refilled).
/// Returns the number of components found.
///
/// If `out_membership` is `Some`, also writes per-cell component membership:
/// cell `i` belongs to component `out_membership[i]`;
/// covered cells receive `u16::MAX` as a sentinel.
/// Caller must ensure `out_membership.len() == total_cells as usize`.
pub(crate) fn bfs_components(
    covered: &BitSet,
    adj_list: &[Vec<u16>],
    total_cells: u16,
    workspace: &mut BfsWorkspace,
    mut out_membership: Option<&mut [u16]>
) -> u16 {
    if let Some(m) = out_membership.as_deref_mut() {
        debug_assert_eq!(m.len(), total_cells as usize);
        m.fill(u16::MAX);
    }

    workspace.reset();
    let mut comp_idx: u16 = 0;

    for start in 0..total_cells as usize {
        if covered.test(start) || workspace.visited[start] != 0 {
            continue;
        }

        workspace.stack.push(start as u32);
        workspace.visited[start] = 1;

        if let Some(m) = out_membership.as_deref_mut() {
            m[start] = comp_idx;
        }

        let mut size: u32 = 0;

        while let Some(u) = workspace.stack.pop() {
            size += 1;
            let u_idx = u as usize;

            for &v_u16 in &adj_list[u_idx] {
                let v = v_u16 as usize;
                if !covered.test(v) && workspace.visited[v] == 0 {
                    workspace.visited[v] = 1;
                    if let Some(m) = out_membership.as_deref_mut() {
                        m[v] = comp_idx;
                    }

                    workspace.stack.push(v_u16 as u32);
                }
            }
        }

        workspace.comp_sizes.push(size);
        comp_idx += 1;
    }

    comp_idx
}

/// Build a tiny linear-graph adjacency list (chain of cells).
#[cfg(test)]
pub(crate) fn make_chain_adj(n: usize) -> Vec<Vec<u16>> {
    (0..n)
        .map(|i| {
            let mut nb = vec![];
            if i > 0 { nb.push((i - 1) as u16); }
            if i < n - 1 { nb.push((i + 1) as u16); }
            nb
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_singleton_when_nothing_covered() {
        let covered = BitSet::new();
        let adj = make_chain_adj(5);
        let mut ws = BfsWorkspace::new(5);

        let n = bfs_components(&covered, &adj, 5, &mut ws, None);
        assert_eq!(n, 1);
        assert_eq!(ws.comp_sizes, vec![5]);
    }

    #[test]
    fn splits_when_middle_covered() {
        let mut covered = BitSet::new();
        covered.set(2);
        let adj = make_chain_adj(5);
        let mut ws = BfsWorkspace::new(5);

        let n = bfs_components(&covered, &adj, 5, &mut ws, None);
        assert_eq!(n, 2);
        let mut sizes = ws.comp_sizes.clone();
        sizes.sort_unstable();
        assert_eq!(sizes, vec![2, 2]);
    }

    #[test]
    fn membership_assigns_correct_components() {
        let mut covered = BitSet::new();
        covered.set(2);
        let adj = make_chain_adj(5);
        let mut ws = BfsWorkspace::new(5);
        let mut membership = vec![0_u16; 5];

        let n = bfs_components(&covered, &adj, 5, &mut ws, Some(&mut membership));
        assert_eq!(n, 2);
        assert_eq!(membership[0], membership[1]);
        assert_eq!(membership[3], membership[4]);
        assert_ne!(membership[0], membership[3]);
        assert_eq!(membership[2], u16::MAX);
    }
}