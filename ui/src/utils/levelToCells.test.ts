import { describe, expect, it } from "vitest";

import { blockSizeFromLevel } from "./levelToCells.ts";

describe("blockSizeFromLevelStandard", () => {
	it("clamps below 100 to size 1", () => {
		expect(blockSizeFromLevel(60)).toBe(1);
		expect(blockSizeFromLevel(99)).toBe(1);
	});

	it("returns size 2 for 100..139", () => {
		expect(blockSizeFromLevel(100)).toBe(2);
		expect(blockSizeFromLevel(139)).toBe(2);
	});

	it("returns size 3 for 140..199", () => {
		expect(blockSizeFromLevel(140)).toBe(3);
		expect(blockSizeFromLevel(199)).toBe(3);
	});

	it("returns size 4 for 200..249", () => {
		expect(blockSizeFromLevel(200)).toBe(4);
		expect(blockSizeFromLevel(249)).toBe(4);
	});

	it("returns size 5 for 250+", () => {
		expect(blockSizeFromLevel(250)).toBe(5);
		expect(blockSizeFromLevel(300)).toBe(5);
	});
});

describe("blockSizeFromLevelMapleM", () => {
	it("walks 30 / 50 / 70 / 120 / 250 thresholds", () => {
		expect(blockSizeFromLevel(30, true)).toBe(1);
		expect(blockSizeFromLevel(49, true)).toBe(1);
		expect(blockSizeFromLevel(50, true)).toBe(2);
		expect(blockSizeFromLevel(69, true)).toBe(2);
		expect(blockSizeFromLevel(70, true)).toBe(3);
		expect(blockSizeFromLevel(119, true)).toBe(3);
		expect(blockSizeFromLevel(120, true)).toBe(4);
		expect(blockSizeFromLevel(249, true)).toBe(4);
		expect(blockSizeFromLevel(250, true)).toBe(5);
	});

	it("disagrees with standard around level 130", () => {
		expect(blockSizeFromLevel(130, true)).toBe(4);
		expect(blockSizeFromLevel(130)).toBe(2);
	});
});
