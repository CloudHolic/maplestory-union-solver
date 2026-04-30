// Static structure of the 22x20 union board.

import { cellKey, type Coord } from "@/utils/coords.ts";

export const BOARD_WIDTH = 22;
export const BOARD_HEIGHT = 20;

export type OuterGroupId =
	| "outer_1"
	| "outer_2"
	| "outer_3"
	| "outer_4"
	| "outer_5"
	| "outer_6"
	| "outer_7"
	| "outer_8";

export type InnerGroupId =
	| "inner_1"
	| "inner_2"
	| "inner_3"
	| "inner_4"
	| "inner_5"
	| "inner_6"
	| "inner_7"
	| "inner_8";

export type GroupId = OuterGroupId | InnerGroupId;

export interface BoardGroup {
	id: GroupId;
	cells: readonly Coord[];
	centroid: Coord;
}

export interface BoardLayout {
	width: number;
	height: number;

	/** As cell keys ("r,c"), in row-major order. */
	centerCells: readonly string[];

	/** O(1) center-cell membership counterpart. */
	centerCellSet: ReadonlySet<string>;

	/** All 16 groups in declaration order: outer_1..8, inner_1..8. */
	groups: readonly BoardGroup[];

	/** Reverse lookup from cell key to its group id. */
	cellToGroup: ReadonlyMap<string, GroupId>;
}

const CHAR_TO_GROUP: Record<string, GroupId> = {
	"0": "outer_1",
	"1": "outer_2",
	"2": "outer_3",
	"3": "outer_4",
	"4": "outer_5",
	"5": "outer_6",
	"6": "outer_7",
	"7": "outer_8",
	A: "inner_1",
	B: "inner_2",
	C: "inner_3",
	D: "inner_4",
	E: "inner_5",
	F: "inner_6",
	G: "inner_7",
	H: "inner_8"
};

// Iteration order for groups[].
const ALL_GROUP_IDS: readonly GroupId[] = [
	"outer_1",
	"outer_2",
	"outer_3",
	"outer_4",
	"outer_5",
	"outer_6",
	"outer_7",
	"outer_8",
	"inner_1",
	"inner_2",
	"inner_3",
	"inner_4",
	"inner_5",
	"inner_6",
	"inner_7",
	"inner_8"
];

const BOARD_MAP_ROWS: readonly string[] = [
	"0111111111122222222223",
	"0011111111122222222233",
	"0001111111122222222333",
	"0000111111122222223333",
	"0000011111122222233333",
	"00000ABBBBBCCCCCD33333",
	"00000AABBBBCCCCDD33333",
	"00000AAABBBCCCDDD33333",
	"00000AAAABBCCDDDD33333",
	"00000AAAAABCDDDDD33333",
	"44444EEEEEFGHHHHH55555",
	"44444EEEEFFGGHHHH55555",
	"44444EEEFFFGGGHHH55555",
	"44444EEFFFFGGGGHH55555",
	"44444EFFFFFGGGGGH55555",
	"4444466666677777755555",
	"4444666666677777775555",
	"4446666666677777777555",
	"4466666666677777777755",
	"4666666666677777777775"
];

// Center 4 cells. At least one piece's marked cell must land here.
const CENTER_COORDS: readonly Coord[] = [
	[9, 10],
	[9, 11],
	[10, 10],
	[10, 11]
];

export function isOuterGroup(id: GroupId): id is OuterGroupId {
	return id.startsWith("outer_");
}

export function isInnerGroup(id: GroupId): id is InnerGroupId {
	return id.startsWith("inner_");
}

/**
 * Cell of `cells` whose Euclidean distance to the arithmetic centroid is minimal.
 * For L-shaped or concave groups the raw centroid can fall outside the group;
 * this clamps it to a real member cell.
 * */
function nearestCellToCentroid(cells: readonly Coord[]): Coord {
	if (cells.length === 0)
		throw new Error("nearestCellToCentroid: empty cell list");

	let sumR = 0;
	let sumC = 0;
	for (const [r, c] of cells) {
		sumR += r;
		sumC += c;
	}
	const meanR = sumR / cells.length;
	const meanC = sumC / cells.length;

	let best: Coord = cells[0]!;
	let bestDist = Infinity;
	for (const cell of cells) {
		const dr = cell[0] - meanR;
		const dc = cell[1] - meanC;
		const d = dr * dr + dc * dc;

		if (d < bestDist) {
			bestDist = d;
			best = cell;
		}
	}

	return best;
}

export function parseBoardMap(rows: readonly string[]): BoardLayout {
	const firstRow = rows[0];
	if (firstRow === undefined)
		throw new Error("parseBoardMap: empty board");

	const width = firstRow.length;
	const height = rows.length;

	const cellsByGroup = new Map<GroupId, Coord[]>();
	const cellToGroup = new Map<string, GroupId>();

	for (const [r, row] of rows.entries()) {
		if (row.length !== width)
			throw new Error(
				`parseBoardMap: row ${r} length ${row.length}, expected ${width}`
			);

		for (let c = 0; c < width; c++) {
			const ch = row.charAt(c);
			const groupId = CHAR_TO_GROUP[ch];

			if (groupId === undefined)
				throw new Error(
					`parseBoardMap: unknown group char "${ch}" at (${r},${c})`
				);

			cellToGroup.set(cellKey(r, c), groupId);
			const list = cellsByGroup.get(groupId);
			if (list === undefined)
				cellsByGroup.set(groupId, [[r, c]]);
			else
				list.push([r, c]);

		}
	}

	const groups: BoardGroup[] = ALL_GROUP_IDS.map(id => {
		const cells = cellsByGroup.get(id);
		if (cells === undefined || cells.length === 0)
			throw new Error(
				`parseBoardMap: group ${id} has no cells in BOARD_MAP_ROWS`
			);

		return { id, cells, centroid: nearestCellToCentroid(cells) };
	});

	const centerCells = CENTER_COORDS.map(([r, c]) => cellKey(r, c));
	const centerCellSet = new Set(centerCells);

	return {
		width,
		height,
		centerCells,
		centerCellSet,
		groups,
		cellToGroup
	};
}

export const UNION_BOARD: BoardLayout = parseBoardMap(BOARD_MAP_ROWS);
