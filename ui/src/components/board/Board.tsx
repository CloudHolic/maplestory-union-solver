// SVG board grid.

import { useMemo } from "react";

import {
	BOARD_HEIGHT,
	BOARD_WIDTH,
	type GroupId,
	UNION_BOARD
} from "@/domain/boardLayout.ts";
import { useBoardStore } from "@/state/boardStore.ts";
import { computeOutlinePath } from "@/utils/boardOutline.ts";
import { cellKey, parseKey } from "@/utils/coords.ts";

import { BoardCell } from "./BoardCell.tsx";

interface CellRenderInfo {
	r: number;
	c: number;
	key: string;
	groupId: GroupId;
}

const ALL_CELLS: readonly CellRenderInfo[] = [
	...UNION_BOARD.cellToGroup.entries()
].map(([key, groupId]) => {
	const [r, c] = parseKey(key);
	return { r, c, key, groupId };
});

const GROUP_OUTLINES: readonly { id: GroupId; d: string }[] =
	UNION_BOARD.groups.map(g => ({
		id: g.id,
		d: computeOutlinePath(new Set(g.cells.map(([r, c]) => cellKey(r, c))))
	}));

// Stroke widths in viewBox units.
const OUTLINE_STROKE = 0.06;
const CENTER_RING_STROKE = 0.1;
// viewBox padding.
const PAD = OUTLINE_STROKE / 2;

export function Board() {
	const selectedCells = useBoardStore(s => s.selectedCells);
	const groupCounts = useBoardStore(s => s.groupCounts);
	const groupSelectMode = useBoardStore(s => s.groupSelectMode);
	const toggleCell = useBoardStore(s => s.toggleCell);
	const toggleGroup = useBoardStore(s => s.toggleGroup);

	// Cell keys covered by any count-mode group.
	const countModeKeys = useMemo(() => {
		const set = new Set<string>();
		for (const g of UNION_BOARD.groups)
			if (groupCounts[g.id] > 0)
				for (const [r, c] of g.cells)
					set.add(cellKey(r, c));

		return set;
	}, [groupCounts]);

	const handleClick = (key: string) => {
		if (groupSelectMode) {
			const groupId = UNION_BOARD.cellToGroup.get(key);
			if (groupId !== undefined)
				toggleGroup(groupId);
		} else
			toggleCell(key);
	};

	const handleContextMenu = (key: string) => {
		const groupId = UNION_BOARD.cellToGroup.get(key);
		if (groupId === undefined)
			return;

		// TODO: Open GroupCountOverlay for this groupId.
		console.log("right-click on group", groupId);
	};

	return (
		<div className="relative inline-block">
			<svg
				viewBox={`${-PAD} ${-PAD} ${BOARD_WIDTH + 2 * PAD} ${BOARD_HEIGHT + 2 * PAD}`}
				className="block h-auto w-full max-w-3xl bg-board-bg"
			>
				{ALL_CELLS.map(({ r, c, key, groupId }) => (
					<BoardCell
						key={key}
						row={r}
						col={c}
						isSelected={selectedCells.has(key)}
						isCountMode={groupCounts[groupId] > 0}
						onClick={handleClick}
						onContextMenu={handleContextMenu}
					/>
				))}

				{[...countModeKeys].map(key => {
					const [r, c] = parseKey(key);
					return (
						<rect
							key={`cover-${key}`}
							x={c}
							y={r}
							width={1}
							height={1}
							className="pointer-events-none fill-board-count-overlay"
						/>
					);
				})}

				{GROUP_OUTLINES.map(({ id, d }) => (
					<path
						key={`outline-${id}`}
						d={d}
						strokeWidth={OUTLINE_STROKE}
						className="pointer-events-none fill-none stroke-board-outline"
					/>
				))}

				{UNION_BOARD.centerCells.map(key => {
					const [r, c] = parseKey(key);
					return (
						<rect
							key={`center-${key}`}
							x={c}
							y={r}
							width={1}
							height={1}
							strokeWidth={CENTER_RING_STROKE}
							className="pointer-events-none fill-none stroke-board-center-ring"
						/>
					);
				})}
			</svg>
		</div>
	);
}
