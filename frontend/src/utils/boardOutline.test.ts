import { describe, expect, it } from "vitest";

import { computeOutlinePath } from "./boardOutline.ts";

describe("computeOutlinePath", () => {
	it("returns empty string for empty set", () => {
		expect(computeOutlinePath(new Set())).toBe("");
	});

	it("traces 4 edges of a single cell", () => {
		const path = computeOutlinePath(new Set(["0,0"]));

		expect(path).toContain("M0 0L1 0");
		expect(path).toContain("M0 1L1 1");
		expect(path).toContain("M0 0L0 1");
		expect(path).toContain("M1 0L1 1");
	});

	it("omits the internal edge in a 1x2 horizontal pair", () => {
		const path = computeOutlinePath(new Set(["0,0", "0,1"]));
		expect(path).not.toContain("M1 0L1 1");
	});

	it("traces the perimeter of a 2x2 block as 8 segments", () => {
		const path = computeOutlinePath(new Set(["0,0", "0,1", "1,0", "1,1"]));
		const segmentCount = (path.match(/M/g) ?? []).length;
		expect(segmentCount).toBe(8);
	});
});
