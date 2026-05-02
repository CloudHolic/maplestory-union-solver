import type { ShapeDef } from "@/types/pieces.ts";
import { type BlockSize, blockSizeFromLevel } from "@/utils/levelToCells.ts";

type ClassGroup =
	| "warrior"
	| "mage"
	| "archer"
	| "thief"
	| "pirate"
	| "xenon";

/**
 * The 15 distinct shapes that can appear on the union board.
 * For size >=4, each id means it's class - Warrior, Mage, Archer, Thief, Pirate, and Xenon.
 */
export const SHAPES: readonly ShapeDef[] = [
	{ id: "size1", cells: [[0, 0]], markIndex: 0 },

	{ id: "size2", cells: [[0, 0], [0, 1]], markIndex: 0 },

	{ id: "size3-L", cells: [[0, 0], [1, 0], [1, 1]], markIndex: 1 },
	{ id: "size3-I", cells: [[0, 0], [0, 1], [0, 2]], markIndex: 1 },

	{ id: "size4-W", cells: [[0, 0], [0, 1], [1, 0], [1, 1]], markIndex: 0 },
	{ id: "size4-M", cells: [[0, 1], [1, 0], [1, 1], [1, 2]], markIndex: 0 },
	{ id: "size4-A", cells: [[0, 0], [0, 1], [0, 2], [0, 3]], markIndex: 1 },
	{ id: "size4-T", cells: [[0, 0], [1, 0], [1, 1], [1, 2]], markIndex: 2 },
	{ id: "size4-P", cells: [[0, 0], [0, 1], [1, 1], [1, 2]], markIndex: 1 },

	{ id: "size5-W", cells: [[0, 0], [0, 1], [0, 2], [1, 1], [1, 2]], markIndex: 2 },
	{ id: "size5-M", cells: [[0, 1], [1, 0], [1, 1], [1, 2], [2, 1]], markIndex: 2 },
	{ id: "size5-A", cells: [[0, 0], [0, 1], [0, 2], [0, 3], [0, 4]], markIndex: 2 },
	{ id: "size5-T", cells: [[0, 2], [1, 0], [1, 1], [1, 2], [2, 2]], markIndex: 2 },
	{ id: "size5-P", cells: [[0, 0], [0, 1], [1, 1], [1, 2], [1, 3]], markIndex: 1 },
	{ id: "size5-X", cells: [[0, 0], [0, 1], [1, 1], [2, 1], [2, 2]], markIndex: 2 }
];

export const SHAPE_COUNT = 15;

/** Maps `(ClassGroup, BlockSize)` to an index in `SHAPES`. */
const CLASS_GROUP_SIZE_TO_SHAPE: Record<ClassGroup, Record<BlockSize, number>> = {
	warrior: { 1: 0, 2: 1, 3: 2, 4: 4, 5: 9 },
	mage:    { 1: 0, 2: 1, 3: 3, 4: 5, 5: 10 },
	archer:  { 1: 0, 2: 1, 3: 3, 4: 6, 5: 11 },
	thief:   { 1: 0, 2: 1, 3: 3, 4: 7, 5: 12 },
	pirate:  { 1: 0, 2: 1, 3: 2, 4: 8, 5: 13 },
	xenon:   { 1: 0, 2: 1, 3: 3, 4: 7, 5: 14 }
};

/** `block_type` → ClassGroup. */
const TYPE_TO_CLASS_GROUP: Record<string, ClassGroup> = {
	"전사": "warrior",
	"마법사": "mage",
	"궁수": "archer",
	"도적": "thief",
	"해적": "pirate"
};

/** Resolves a server-returned Block to an index in `SHAPES`. */
export function resolveShapeIndex(blockType: string, blockClass: string, level: number): number | null {
	if (blockClass === "제논")
		return CLASS_GROUP_SIZE_TO_SHAPE.xenon[blockSizeFromLevel(level)];

	if (blockType.includes("메이플 M"))
		return CLASS_GROUP_SIZE_TO_SHAPE.archer[blockSizeFromLevel(level, true)];

	const cg = TYPE_TO_CLASS_GROUP[blockType];
	if (cg === undefined)
		return null;

	return CLASS_GROUP_SIZE_TO_SHAPE[cg][blockSizeFromLevel(level)];
}

/** Aggregates a flat list of blocks (one preset's worth) into per-shape counts. */
export function aggregatePresetCounts(
	blocks: ReadonlyArray<{ type: string; class: string; level: number }>
): number[] {
	const counts = new Array<number>(SHAPE_COUNT).fill(0);

	for (const b of blocks) {
		const idx = resolveShapeIndex(b.type, b.class, b.level);
		if (idx === null) {
			console.warn("aggregatePresetCounts: unknown block, skipping", b);
			continue;
		}

		counts[idx]!++;
	}

	return counts;
}
