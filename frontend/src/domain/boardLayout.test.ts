import { describe, expect, it } from "vitest";

import {
	BOARD_HEIGHT,
	BOARD_WIDTH,
	parseBoardMap,
	UNION_BOARD
} from "./boardLayout.ts";

describe("UNION_BOARD", () => {
	it("has 22x20 dimensions", () => {
		expect(UNION_BOARD.width).toBe(22);
		expect(UNION_BOARD.height).toBe(20);
		expect(BOARD_WIDTH).toBe(22);
		expect(BOARD_HEIGHT).toBe(20);
	});

	it("has 16 groups, 8 outer + 8 inner", () => {
		expect(UNION_BOARD.groups).toHaveLength(16);
		const outer = UNION_BOARD.groups.filter(g => g.id.startsWith("outer_"));
		const inner = UNION_BOARD.groups.filter(g => g.id.startsWith("inner_"));
		expect(outer).toHaveLength(8);
		expect(inner).toHaveLength(8);
	});

	it("groups partition all 22*20 cells exactly", () => {
		const total = UNION_BOARD.groups.reduce(
			(sum, g) => sum + g.cells.length,
			0
		);
		expect(total).toBe(BOARD_WIDTH * BOARD_HEIGHT);
		expect(UNION_BOARD.cellToGroup.size).toBe(BOARD_WIDTH * BOARD_HEIGHT);
	});

	it("each group's centroid is a member cell of that group", () => {
		for (const group of UNION_BOARD.groups) {
			const centroidKey = `${group.centroid[0]},${group.centroid[1]}`;
			const memberKeys = new Set(
				group.cells.map(([r, c]) => `${r},${c}`)
			);
			expect(memberKeys.has(centroidKey)).toBe(true);
		}
	});
});

describe("parseBoardMap", () => {
	it("rejects rows of inconsistent width", () => {
		expect(() => parseBoardMap(["0", "00"])).toThrow(/length/);
	});

	it("rejects unknown group chars", () => {
		expect(() => parseBoardMap(["X"])).toThrow(/unknown group char/);
	});

	it("rejects boards missing groups", () => {
		expect(() => parseBoardMap(["0"])).toThrow(/has no cells/);
	});
});
