// Computes SVG path data for the boundary of a set of cells.

import { parseKey } from "./coords.ts";

/** SVG path data tracing the boundary of `cellKeys`. */
export function computeOutlinePath(cellKeys: ReadonlySet<string>): string {
	const segments: string[] = [];

	for (const key of cellKeys) {
		const [r, c] = parseKey(key);

		// North neighbor missing -> top edge
		if (!cellKeys.has(`${r - 1},${c}`))
			segments.push(`M${c} ${r}L${c + 1} ${r}`);

		// South neighbor missing -> bottom edge
		if (!cellKeys.has(`${r + 1},${c}`))
			segments.push(`M${c} ${r + 1}L${c + 1} ${r + 1}`);

		// West neighbor missing -> left edge
		if (!cellKeys.has(`${r},${c - 1}`))
			segments.push(`M${c} ${r}L${c} ${r + 1}`);

		// East neighbor missing -> right edge
		if (!cellKeys.has(`${r},${c + 1}`))
			segments.push(`M${c + 1} ${r}L${c + 1} ${r + 1}`);
	}

	return segments.join("");
}
