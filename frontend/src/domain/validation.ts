import type { GroupId } from "@/types/board.ts";
import { cellKey, parseKey } from "@/utils/coords.ts";

import { BOARD_CENTER_CELLS, UNION_BOARD } from "./boardLayout.ts";
import { SHAPES } from "./pieces.ts";

/** The 4 cells at the geometric center of the board. */
const CENTER_CELLS: ReadonlySet<string> = new Set(BOARD_CENTER_CELLS);

type ValidationError =
	| { kind: "no_pieces" }
	| { kind: "no_selection" }
	| { kind: "size_mismatch"; selectedArea: number; requiredArea: number }
	| { kind: "no_center" }
	| { kind: "not_connected"; islands: number }
	| { kind: "bridge_too_small"; groupId: GroupId; required: number; actual: number };

/** Validates that the user's board selection is solvable given their registered pieces. */
export function validateBoardSelection(
	selectedCells: ReadonlySet<string>,
	groupCounts: Readonly<Record<GroupId, number>>,
	shapeCounts: ReadonlyArray<number>
): ValidationError | null {
	const requiredArea = shapeCounts.reduce(
		(sum, count, i) => sum + count * (SHAPES[i]?.cells.length ?? 0),
		0
	);

	if (requiredArea === 0)
		return { kind: "no_pieces" };

	const groupArea = Object.values(groupCounts).reduce((a, b) => a + b, 0);
	const selectedArea = selectedCells.size + groupArea;

	if (selectedArea === 0)
		return { kind: "no_selection" };

	if (selectedArea !== requiredArea)
		return { kind: "size_mismatch", selectedArea, requiredArea };

	if (!hasCenterCoverage(selectedCells, groupCounts))
		return { kind: "no_center" };

	const components = computeComponents(selectedCells);
	const countModeGroups = UNION_BOARD.groups.filter(g => groupCounts[g.id] > 0);

	// Disjoint set over (component indices) ∪ (count-mode group indices).
	// Component i has node id `i`. Group j has node id `components.length + j`.
	const totalNodes = components.length + countModeGroups.length;
	const parent = Array.from({ length: totalNodes }, (_, i) => i);
	const find = (i: number): number => {
		while (parent[i] !== i) {
			parent[i] = parent[parent[i]!]!;
			i = parent[i]!;
		}

		return i;
	};
	const union = (a: number, b: number) => {
		const ra = find(a);
		const rb = find(b);
		if (ra !== rb)
			parent[ra] = rb;
	};

	for (let j = 0; j < countModeGroups.length; j++) {
		const group = countModeGroups[j]!;
		const groupNodeId = components.length + j;
		const groupCount = groupCounts[group.id]!;

		const touched = touchedComponents(group, components);

		// Check whether `groupCount` cells, chosen from this group's cells and connected within this group,
		// can simultaneously touch every component in `touched`.
		// If not, no solver can satisfy this; fail early.
		if (!hasFeasibleBridge(group.cells, groupCount, touched, components))
			return {
				kind: "bridge_too_small",
				groupId: group.id,
				required: Math.max(touched.size, 1),
				actual: groupCount
			};

		// Union the group node with every component it touches.
		for (const compIdx of touched)
			union(groupNodeId, compIdx);
	}

	// All components must end up in the same union root.
	if (components.length > 0) {
		const root = find(0);
		for (let i = 1; i < components.length; i++)
			if (find(i) !== root)
				return { kind: "not_connected", islands: components.length };
	}

	return null;
}

function hasCenterCoverage(
	selectedCells: ReadonlySet<string>,
	groupCounts: Readonly<Record<GroupId, number>>
): boolean {
	for (const cell of CENTER_CELLS) {
		if (selectedCells.has(cell))
			return true;

		const groupId = UNION_BOARD.cellToGroup.get(cell);
		if (groupId !== undefined && groupCounts[groupId] > 0)
			return true;
	}

	return false;
}

function computeComponents(cells: ReadonlySet<string>): Array<Set<string>> {
	const visited = new Set<string>();
	const components: Array<Set<string>> = [];

	for (const start of cells) {
		if (visited.has(start))
			continue;

		const component = new Set<string>();
		const queue: string[] = [start];

		visited.add(start);
		component.add(start);

		while (queue.length > 0) {
			const cell = queue.shift()!;
			const [r, c] = parseKey(cell);

			for (const [dr, dc] of [[-1, 0], [1, 0], [0, -1], [0, 1]] as const) {
				const nKey = cellKey(r + dr, c + dc);
				if (cells.has(nKey) && !visited.has(nKey)) {
					visited.add(nKey);
					component.add(nKey);
					queue.push(nKey);
				}
			}
		}

		components.push(component);
	}

	return components;
}

/** Indices of components 4-adjacent to any cell of the group. */
function touchedComponents(
	group: { cells: readonly (readonly [number, number])[] },
	components: ReadonlyArray<Set<string>>
): Set<number> {
	const touched = new Set<number>();

	for (const [gr, gc] of group.cells)
		for (const [dr, dc] of [[-1, 0], [1, 0], [0, -1], [0, 1]] as const) {
			const nKey = cellKey(gr + dr, gc + dc);
			for (let i = 0; i < components.length; i++)
				if (components[i]!.has(nKey))
					touched.add(i);
		}

	return touched;
}

const ENUMERATION_LIMIT = 16;

function hasFeasibleBridge(
	groupCells: readonly (readonly [number, number])[],
	K: number,
	touchedComps: ReadonlySet<number>,
	components: ReadonlyArray<Set<string>>
): boolean {
	if (touchedComps.size === 0)
		return true; // group floats alone, no bridge required.

	if (K === 0)
		return touchedComps.size === 0;

	if (groupCells.length > ENUMERATION_LIMIT)
		// Conservative fallback: K cells must be at least as many as distinct components to touch.
		return K >= touchedComps.size;

	// For each group cell, precompute which components it touches.
	const cellTouches: Array<Set<number>> = groupCells.map(([gr, gc]) => {
		const t = new Set<number>();
		for (const [dr, dc] of [[-1, 0], [1, 0], [0, -1], [0, 1]] as const) {
			const nKey = cellKey(gr + dr, gc + dc);
			for (let i = 0; i < components.length; i++)
				if (touchedComps.has(i) && components[i]!.has(nKey))
					t.add(i);
		}

		return t;
	});

	// Adjacency within the group (cell index → neighbor cell indices).
	const cellKeyof = (i: number) => cellKey(groupCells[i]![0], groupCells[i]![1]);
	const indexByKey = new Map<string, number>();
	groupCells.forEach((_, i) => indexByKey.set(cellKeyof(i), i));

	const groupAdj: Array<number[]> = groupCells.map(([gr, gc]) => {
		const adj: number[] = [];
		for (const [dr, dc] of [[-1, 0], [1, 0], [0, -1], [0, 1]] as const) {
			const idx = indexByKey.get(cellKey(gr + dr, gc + dc));
			if (idx !== undefined)
				adj.push(idx);
		}

		return adj;
	});

	// Enumerate connected size-K subsets via DFS-style growth.
	const seen = new Set<string>();

	const grow = (chosen: number[], frontier: Set<number>): boolean => {
		if (chosen.length === K) {
			const covered = new Set<number>();
			for (const i of chosen)
				for (const t of cellTouches[i]!)
					covered.add(t);

			for (const t of touchedComps)
				if (!covered.has(t))
					return false;

			return true;
		}

		for (const next of frontier) {
			if (chosen.includes(next))
				continue;

			const newChosen = [...chosen, next].sort((a, b) => a - b);
			const key = newChosen.join(",");
			if (seen.has(key))
				continue;
			seen.add(key);

			const newFrontier = new Set(frontier);
			newFrontier.delete(next);
			for (const n of groupAdj[next]!)
				if (!chosen.includes(n))
					newFrontier.add(n);

			if (grow(newChosen, newFrontier))
				return true;
		}

		return false;
	};

	for (let start = 0; start < groupCells.length; start++) {
		const frontier = new Set<number>([start]);
		if (grow([], frontier))
			return true;
	}

	return false;
}
