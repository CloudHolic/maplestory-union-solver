// Builds an ExactCoverInput from the UI's selection state.

import type { ExactCoverInput, PieceDefJson, PieceInstanceJson } from "@solver/wasm";

import { BOARD_CENTER_CELLS } from "@/domain/boardLayout.ts";
import { SHAPE_COUNT, SHAPES } from "@/domain/pieces.ts";
import type { Coord } from "@/utils/coords.ts";

// SHAPES are static.
const PIECE_DEFS: ReadonlyArray<readonly [string, PieceDefJson]> = SHAPES.map(s => [
	s.id,
	{
		id: s.id,
		cells: s.cells.map(([r, c]): Coord => [r, c]),
		markIndex: s.markIndex
	}
]);

export function buildExactCoverInput(
	shapeCounts: ReadonlyArray<number>,
	selectedCells: ReadonlySet<string>
): ExactCoverInput {
	const pieces: PieceInstanceJson[] = [];
	let totalPieceCells = 0;

	for (let i = 0; i < SHAPE_COUNT; i++) {
		const count = shapeCounts[i] ?? 0;
		const shape = SHAPES[i]!;
		totalPieceCells += count * shape.cells.length;

		for (let k = 0; k < count; k++)
			pieces.push({ defId: shape.id, index: k });
	}

	if (totalPieceCells !== selectedCells.size)
		throw new Error(
			`inputBuilder: piece-cell total ${totalPieceCells} ≠ selected-cell count ${selectedCells.size}`
		);

	return {
		pieces,
		pieceDefs: PIECE_DEFS.map(([id, def]): [string, PieceDefJson] => [id, def]),
		centerCells: [...BOARD_CENTER_CELLS],
		targetCells: [...selectedCells].sort()
	};
}
