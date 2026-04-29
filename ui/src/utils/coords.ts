// Coordinate primitives for the 22x20 union board.

import type { Coord } from "@solver/wasm";

export type { Coord };

/** Stable string key for a cell position. */
export function cellKey(r: number, c: number): string {
	return `${r},${c}`;
}

/** Inverse of cellKey. */
export function parseKey(key: string): Coord {
	const [rStr, cStr] = key.split(",");
	if (rStr === undefined || cStr === undefined)
		throw new Error(`Invalid cell key: ${JSON.stringify(key)}`);

	return [Number(rStr), Number(cStr)];
}
