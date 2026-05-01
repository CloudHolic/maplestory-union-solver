// SVG board grid.

import { useMemo, useState } from "react";

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
import { GroupCountOverlay } from "./GroupCountOverlay.tsx";

// GroupCount solver mode is temporarily disabled.
const GROUP_COUNT_ENABLED: boolean = false;

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
const OUTLINE_STROKE = 0.12;
// viewBox padding.
const PAD = OUTLINE_STROKE / 2;

export function Board() {
	const selectedCells = useBoardStore(s => s.selectedCells);
	const groupCounts = useBoardStore(s => s.groupCounts);
	const groupSelectMode = useBoardStore(s => s.groupSelectMode);
	const toggleCell = useBoardStore(s => s.toggleCell);
	const toggleGroup = useBoardStore(s => s.toggleGroup);
	const setGroupCount = useBoardStore(s => s.setGroupCount);

	const [activeGroupId, setActiveGroupId] = useState<GroupId | null>(null);

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
		const groupId = UNION_BOARD.cellToGroup.get(key);
		if (groupId === undefined)
			return;

		if (groupCounts[groupId] > 0) {
			setActiveGroupId(activeGroupId === groupId ? null : groupId);
			return;
		}

		if (activeGroupId !== null)
			setActiveGroupId(null);

		if (groupSelectMode)
			toggleGroup(groupId);
		else
			toggleCell(key);
	};

	const handleContextMenu = (key: string) => {
		const groupId = UNION_BOARD.cellToGroup.get(key);
		if (groupId === undefined)
			return;

		if (groupCounts[groupId] > 0) {
			setGroupCount(groupId, 0);
			if (activeGroupId === groupId)
				setActiveGroupId(null);

			return;
		}

		setGroupCount(groupId, 1);
		setActiveGroupId(groupId);
	};

	return (
		<div className="relative w-full">
			<svg
				viewBox={`${-PAD} ${-PAD} ${BOARD_WIDTH + 2 * PAD} ${BOARD_HEIGHT + 2 * PAD}`}
				className="block h-auto w-full bg-board-bg"
			>
				{ALL_CELLS.map(({ r, c, key }) => (
					<BoardCell
						key={key}
						row={r}
						col={c}
						isSelected={selectedCells.has(key)}
						onClick={handleClick}
						{...(GROUP_COUNT_ENABLED ? { onContextMenu: handleContextMenu } : {})}
					/>
				))}

				{GROUP_COUNT_ENABLED && [...countModeKeys].map(key => {
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
			</svg>

			{GROUP_COUNT_ENABLED && UNION_BOARD.groups
				.filter(g => groupCounts[g.id] > 0)
				.map(g => (
					<GroupCountOverlay
						key={g.id}
						groupId={g.id}
						count={groupCounts[g.id]!}
						boardPad={PAD}
						editing={activeGroupId === g.id}
						onEditStart={() => setActiveGroupId(g.id)}
						onEditEnd={() => setActiveGroupId(null)}
					/>
				))}
		</div>
	);
}
