import { beforeEach, describe, expect, it } from "vitest";

import { UNION_BOARD } from "@/domain/boardLayout.ts";

import { selectSolverMode, useBoardStore } from "./boardStore.ts";

describe("boardStore", () => {
	beforeEach(() => {
		useBoardStore.getState().clear();
	});

	it("starts empty", () => {
		const state = useBoardStore.getState();
		expect(state.selectedCells.size).toBe(0);
		expect(Object.values(state.groupCounts).every(n => n === 0)).toBe(true);
		expect(state.groupSelectMode).toBe(false);
	});

	it("toggleCell adds and removes cells", () => {
		const { toggleCell } = useBoardStore.getState();
		toggleCell("0,0");
		expect(useBoardStore.getState().selectedCells.has("0,0")).toBe(true);
		toggleCell("0,0");
		expect(useBoardStore.getState().selectedCells.has("0,0")).toBe(false);
	});

	it("toggleCell ignores unknown keys", () => {
		useBoardStore.getState().toggleCell("99,99");
		expect(useBoardStore.getState().selectedCells.size).toBe(0);
	});

	it("toggleCell is a no-op for cells in a count-mode group", () => {
		const { toggleCell, setGroupCount } = useBoardStore.getState();
		setGroupCount("outer_1", 5);
		toggleCell("0,0"); // (0,0) belongs to outer_1
		expect(useBoardStore.getState().selectedCells.has("0,0")).toBe(false);
	});

	it("toggleGroup selects all then deselects all on second call", () => {
		const { toggleGroup } = useBoardStore.getState();
		const outer1 = UNION_BOARD.groups.find(g => g.id === "outer_1");
		expect(outer1).toBeDefined();
		const groupKeys = outer1!.cells.map(([r, c]) => `${r},${c}`);

		toggleGroup("outer_1");
		const after1 = useBoardStore.getState().selectedCells;
		for (const k of groupKeys)
			expect(after1.has(k)).toBe(true);

		toggleGroup("outer_1");
		expect(useBoardStore.getState().selectedCells.size).toBe(0);
	});

	it("toggleGroup is a no-op for a count-mode group", () => {
		const { toggleGroup, setGroupCount } = useBoardStore.getState();
		setGroupCount("outer_1", 5);
		toggleGroup("outer_1");
		expect(useBoardStore.getState().selectedCells.size).toBe(0);
	});

	it("setGroupCount > 0 clears selected cells of that group only", () => {
		const { toggleCell, setGroupCount } = useBoardStore.getState();
		toggleCell("0,0"); // outer_1
		toggleCell("0,21"); // outer_4
		setGroupCount("outer_1", 3);

		const state = useBoardStore.getState();
		expect(state.selectedCells.has("0,0")).toBe(false);
		expect(state.selectedCells.has("0,21")).toBe(true);
		expect(state.groupCounts.outer_1).toBe(3);
	});

	it("setGroupCount clamps negative to 0 and floors floats", () => {
		const { setGroupCount } = useBoardStore.getState();
		setGroupCount("inner_1", -5);
		expect(useBoardStore.getState().groupCounts.inner_1).toBe(0);
		setGroupCount("inner_1", 3.7);
		expect(useBoardStore.getState().groupCounts.inner_1).toBe(3);
	});

	it("solver mode is exact_cover when all counts are 0, mixed otherwise", () => {
		expect(selectSolverMode(useBoardStore.getState())).toBe("exact_cover");
		useBoardStore.getState().setGroupCount("outer_1", 2);
		expect(selectSolverMode(useBoardStore.getState())).toBe("mixed");
	});
});
