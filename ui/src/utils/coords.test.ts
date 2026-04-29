import { describe, expect, it } from "vitest";

import { cellKey, parseKey } from "./coords.ts";

describe("cellKey", () => {
	it("formats (row, col) as 'r,c'", () => {
		expect(cellKey(0, 0)).toBe("0,0");
		expect(cellKey(9, 10)).toBe("9,10");
		expect(cellKey(19, 21)).toBe("19,21");
	});
});

describe("parseKey", () => {
	it("inverts cellKey across the board range", () => {
		for (const r of [0, 9, 19])
			for (const c of [0, 10, 21])
				expect(parseKey(cellKey(r, c))).toEqual([r, c]);
	});

	it("throws on malformed input", () => {
		expect(() => parseKey("3")).toThrow();
		expect(() => parseKey("")).toThrow();
	});
});
