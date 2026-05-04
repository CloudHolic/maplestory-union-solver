import { create } from "zustand";

import { UNION_BOARD } from "@/domain/boardLayout.ts";
import type { GroupId } from "@/types/board.ts";

interface BoardState {
	/** Cells the user has explicitly selected. */
	selectedCells: ReadonlySet<string>;

	/**
	 * Per-group count constraint.
	 * 0 = no constraint, group accepts individual cell selection.
	 * >0 = group is in count mode (overlay shown, cells in that group can't be individually toggled).
	 * */
	groupCounts: Readonly<Record<GroupId, number>>;

	/**
	 * When true, clicking any cell selects/deselects its whole group instead of just that cell.
	 * UI checkbox state.
	 * */
	groupSelectMode: boolean;

	toggleCell: (key: string) => void;
	setCell: (key: string, selected: boolean) => void;
	toggleGroup: (groupId: GroupId) => void;
	setGroupCount: (groupId: GroupId, count: number) => void;
	setGroupSelectMode: (on: boolean) => void;
	loadSelection: (cells: ReadonlySet<string>) => void;
	clear: () => void;
}

const INITIAL_GROUP_COUNTS: Readonly<Record<GroupId, number>> = {
	outer_1: 0, outer_2: 0, outer_3: 0, outer_4: 0,
	outer_5: 0, outer_6: 0, outer_7: 0, outer_8: 0,
	inner_1: 0, inner_2: 0, inner_3: 0, inner_4: 0,
	inner_5: 0, inner_6: 0, inner_7: 0, inner_8: 0
};

const EMPTY_SELECTION: ReadonlySet<string> = new Set();

function cellsOfGroup(groupId: GroupId): readonly string[] {
	const group = UNION_BOARD.groups.find(g => g.id === groupId);
	if (group === undefined)
		return [];

	return group.cells.map(([r, c]) => `${r},${c}`);
}

function isCellSelectable(key: string, groupCounts: Readonly<Record<GroupId, number>>): boolean {
	const groupId = UNION_BOARD.cellToGroup.get(key);
	if (groupId === undefined)
		return false;

	return groupCounts[groupId] === 0;
}

export const useBoardStore = create<BoardState>((set, get) => ({
	selectedCells: EMPTY_SELECTION,
	groupCounts: INITIAL_GROUP_COUNTS,
	groupSelectMode: false,

	toggleCell: key => {
		const { groupCounts, selectedCells } = get();
		if (!isCellSelectable(key, groupCounts))
			return;

		const next = new Set(selectedCells);
		if (next.has(key))
			next.delete(key);
		else
			next.add(key);

		set({ selectedCells: next });
	},

	setCell: (key, selected) => {
		const { groupCounts, selectedCells } = get();
		if (!isCellSelectable(key, groupCounts))
			return;

		const has = selectedCells.has(key);
		if (has === selected)
			return;

		const next = new Set(selectedCells);
		if (selected)
			next.add(key);
		else
			next.delete(key);

		set({ selectedCells: next });
	},

	toggleGroup: groupId => {
		const { groupCounts, selectedCells } = get();
		if (groupCounts[groupId] > 0)
			return;

		const groupKeys = cellsOfGroup(groupId);
		if (groupKeys.length === 0)
			return;

		const allSelected = groupKeys.every(k => selectedCells.has(k));
		const next = new Set(selectedCells);

		for (const k of groupKeys)
			if (allSelected)
				next.delete(k);
			else
				next.add(k);

		set({ selectedCells: next });
	},

	setGroupCount: (groupId, count) => {
		const clamped = Math.max(0, Math.floor(count));
		const { groupCounts, selectedCells } = get();
		if (groupCounts[groupId] === clamped)
			return;

		const nextGroupCounts: Readonly<Record<GroupId, number>> = {
			...groupCounts,
			[groupId]: clamped
		};

		if (clamped === 0) {
			// Leaving count mode - only counts change.
			set({ groupCounts: nextGroupCounts });
			return;
		}

		// Entering count mode - clear any selected cells of this group.
		const groupKeys = cellsOfGroup(groupId);
		const nextSelected = new Set(selectedCells);

		for (const k of groupKeys)
			nextSelected.delete(k);

		set({ groupCounts: nextGroupCounts, selectedCells: nextSelected });
	},

	setGroupSelectMode: on => {
		set({ groupSelectMode: on });
	},

	loadSelection: (cells: ReadonlySet<string>) => {
		set({ selectedCells: new Set(cells) });
	},

	clear: () => {
		set({
			selectedCells: EMPTY_SELECTION,
			groupCounts: INITIAL_GROUP_COUNTS
		});
	}
}));
