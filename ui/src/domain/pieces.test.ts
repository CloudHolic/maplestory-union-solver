import { describe, expect, it, vi } from "vitest";

import { aggregatePresetCounts, resolveShapeIndex, SHAPE_COUNT, SHAPES } from "./pieces";

describe("SHAPES catalog", () => {
	it("has exactly 15 entries", () => {
		expect(SHAPES).toHaveLength(SHAPE_COUNT);
	});

	it("each shape's cell count matches its id prefix", () => {
		for (const shape of SHAPES) {
			const sizePrefix = shape.id.match(/^size(\d)/)?.[1];
			expect(sizePrefix).toBeDefined();
			expect(shape.cells).toHaveLength(Number(sizePrefix));
		}
	});

	it("each shape's markIndex points at a valid cell", () => {
		for (const shape of SHAPES) {
			expect(shape.markIndex).toBeGreaterThanOrEqual(0);
			expect(shape.markIndex).toBeLessThan(shape.cells.length);
		}
	});

	it("ids are unique", () => {
		const ids = SHAPES.map(s => s.id);
		expect(new Set(ids).size).toBe(ids.length);
	});
});

describe("resolveShapeIndex", () => {
	it("maps standard block_type via classGroup + standard thresholds", () => {
		expect(resolveShapeIndex("전사", "히어로", 200)).toBe(4);
		expect(resolveShapeIndex("마법사", "비숍", 275)).toBe(10);
		expect(resolveShapeIndex("궁수", "보우마스터", 140)).toBe(3);
	});

	it("matches xenon by block_class regardless of block_type value", () => {
		expect(resolveShapeIndex("하이브리드", "제논", 250)).toBe(14);
		expect(resolveShapeIndex("하이브리드", "제논", 200)).toBe(7);
	});

	it("treats Maple M as archer with adjusted thresholds", () => {
		expect(resolveShapeIndex("메이플 M 캐릭터", "모바일 캐릭터", 130)).toBe(6);
		expect(resolveShapeIndex("메이플 M 캐릭터", "모바일 캐릭터", 70)).toBe(3);
	});

	it("returns null for unknown block_type", () => {
		expect(resolveShapeIndex("알 수 없는 그룹", "X", 200)).toBeNull();
	});

	it("size 1 and 2 collapse to indices 0 / 1 across all groups", () => {
		expect(resolveShapeIndex("전사", "히어로", 60)).toBe(0);
		expect(resolveShapeIndex("마법사", "비숍", 60)).toBe(0);
		expect(resolveShapeIndex("전사", "히어로", 100)).toBe(1);
		expect(resolveShapeIndex("해적", "바이퍼", 139)).toBe(1);
	});
});

describe("aggregatePresetCounts", () => {
	it("returns all-zero counts for empty input", () => {
		const counts = aggregatePresetCounts([]);
		expect(counts).toHaveLength(SHAPE_COUNT);
		expect(counts.every(c => c === 0)).toBe(true);
	});

	it("counts blocks at the indices resolveShapeIndex maps to", () => {
		const counts = aggregatePresetCounts([
			{ type: "전사", class: "히어로", level: 200 },     // idx 4
			{ type: "전사", class: "팔라딘", level: 220 },     // idx 4
			{ type: "마법사", class: "비숍", level: 275 },     // idx 10
			{ type: "하이브리드", class: "제논", level: 250 }, // idx 14
			{ type: "메이플 M 캐릭터", class: "모바일 캐릭터", level: 130 } // idx 6
		]);

		expect(counts[4]).toBe(2);
		expect(counts[10]).toBe(1);
		expect(counts[14]).toBe(1);
		expect(counts[6]).toBe(1);
		expect(counts.reduce((a, b) => a + b, 0)).toBe(5);
	});

	it("silently drops unresolvable blocks", () => {
		const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
		const counts = aggregatePresetCounts([
			{ type: "전사", class: "히어로", level: 200 },
			{ type: "알 수 없는 그룹", class: "X", level: 200 }
		]);
		expect(counts[4]).toBe(1);
		expect(counts.reduce((a, b) => a + b, 0)).toBe(1);
		expect(warn).toHaveBeenCalledTimes(1);
		warn.mockRestore();
	});
});
