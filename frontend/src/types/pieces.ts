// Piece types.

import type { Coord } from "./wasm.ts";

export interface ShapeDef {
	id: string;
	cells: readonly Coord[];
	markIndex: number;
}
