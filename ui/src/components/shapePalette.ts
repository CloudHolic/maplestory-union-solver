/** Per-shape color palette. Indices align with `SHAPES` in `domain/pieces.ts`. */
export const SHAPE_COLORS: readonly string[] = [
	"#fef3c7", // 0  size1
	"#fcd34d", // 1  size2
	"#f0d9dd", // 2  size3-L
	"#cce2ef", // 3  size3-I
	"#fca5a5", // 4  size4-W
	"#93c5fd", // 5  size4-M
	"#86efac", // 6  size4-A
	"#d8b4fe", // 7  size4-T
	"#cbd5e1", // 8  size4-P
	"#ef4444", // 9  size5-W
	"#3b82f6", // 10 size5-M
	"#22c55e", // 11 size5-A
	"#a855f7", // 12 size5-T
	"#64748b", // 13 size5-P
	"#8664c1"  // 14 size5-X
];

export function shapeColor(shapeIndex: number): string {
	const c = SHAPE_COLORS[shapeIndex];
	if (c === undefined)
		throw new Error(`shapeColor: index ${shapeIndex} out of range`);

	return c;
}
