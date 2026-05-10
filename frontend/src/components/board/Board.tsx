// SVG board grid.

import * as React from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { BOARD_HEIGHT, BOARD_WIDTH, UNION_BOARD } from "@/domain/boardLayout.ts";
import { useBoardStore } from "@/state/boardStore.ts";
import { useSolverStore } from "@/state/solverStore.ts";
import type { GroupId } from "@/types/board.ts";
import { computeOutlinePath } from "@/utils/boardOutline.ts";
import { cellKey, parseKey } from "@/utils/coords.ts";

import { BoardCell } from "./BoardCell.tsx";
import { GroupCountOverlay } from "./GroupCountOverlay.tsx";
import { ResultOverlay } from "./ResultOverlay.tsx";
import { SolverNotice } from "./SolverNotice.tsx";

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

const OUTLINE_STROKE = 0.12;
const PAD = OUTLINE_STROKE / 2;
const VIEWBOX = `${-PAD} ${-PAD} ${BOARD_WIDTH + 2 * PAD} ${BOARD_HEIGHT + 2 * PAD}`;

const EMPTY_PLACEMENTS: readonly never[] = [];

export function Board() {
	const selectedCells = useBoardStore(s => s.selectedCells);
	const groupCounts = useBoardStore(s => s.groupCounts);
	const groupSelectMode = useBoardStore(s => s.groupSelectMode);
	const toggleCell = useBoardStore(s => s.toggleCell);
	const toggleGroup = useBoardStore(s => s.toggleGroup);
	const setCell = useBoardStore(s => s.setCell);
	const setGroupCount = useBoardStore(s => s.setGroupCount);

	const solverStatus = useSolverStore(s => s.status);
	const placements = useSolverStore(s => s.result?.solution ?? EMPTY_PLACEMENTS);

	const [activeGroupId, setActiveGroupId] = useState<GroupId | null>(null);

	const containerRef = useRef<HTMLDivElement>(null);
	const dragModeRef = useRef<boolean | null>(null);

	const isRunning = solverStatus === "running";
	const hasPlacements = placements.length > 0;

	const svgClassName = "block h-auto w-full bg-board-bg" +
		(isRunning ? " pointer-events-none" : "");

	// Global mouseup ends the drag.
	useEffect(() => {
		const handleMouseUp = () => dragModeRef.current = null;
		document.addEventListener("mouseup", handleMouseUp);
		return () => document.removeEventListener("mouseup", handleMouseUp);
	}, [groupCounts, groupSelectMode, toggleCell, toggleGroup]);

	// Cell keys covered by any count-mode group.
	const countModeKeys = useMemo(() => {
		const set = new Set<string>();
		for (const g of UNION_BOARD.groups)
			if (groupCounts[g.id] > 0)
				for (const [r, c] of g.cells)
					set.add(cellKey(r, c));

		return set;
	}, [groupCounts]);

	const handleContextMenu = useCallback((key: string) => {
		const groupId = UNION_BOARD.cellToGroup.get(key);
		if (groupId === undefined)
			return;

		if (groupCounts[groupId] > 0) {
			setGroupCount(groupId, 0);
			setActiveGroupId(prev => prev === groupId ? null : prev);
			return;
		}

		setGroupCount(groupId, 1);
		setActiveGroupId(groupId);
	}, [groupCounts, setGroupCount]);

	const handleMouseDown = useCallback((key: string, event: React.MouseEvent) => {
		// Left button only.
		if (event.button !== 0 || groupSelectMode)
			return;

		const groupId = UNION_BOARD.cellToGroup.get(key);
		if (groupId === undefined || groupCounts[groupId] > 0)
			return;

		const willSelect = !selectedCells.has(key);
		dragModeRef.current = willSelect;
		setCell(key, willSelect);
	}, [groupSelectMode, groupCounts, selectedCells, setCell]);

	const handleMouseEnter = useCallback((key: string) => {
		const mode = dragModeRef.current;
		if (mode === null)
			return;

		setCell(key, mode);
	}, [setCell]);

	const handleClick = useCallback((key: string) => {
		const groupId = UNION_BOARD.cellToGroup.get(key);
		if (groupId === undefined || groupCounts[groupId] > 0)
			return;

		toggleGroup(groupId);
	}, [groupCounts, toggleGroup]);

	const handleEditStart = useCallback((id: GroupId) => setActiveGroupId(id), []);
	const handleEditEnd = useCallback(() => setActiveGroupId(null), []);

	return (
		<div ref={containerRef} className="relative w-full">
			<svg viewBox={VIEWBOX} className={svgClassName}>
				{ALL_CELLS.map(({ r, c, key }) => (
					<BoardCell
						key={key}
						row={r}
						col={c}
						isSelected={selectedCells.has(key)}
						{...(GROUP_COUNT_ENABLED ? { onContextMenu: handleContextMenu } : {})}
						{...(groupSelectMode
							? { onClick: handleClick }
							: { onMouseDown: handleMouseDown, onMouseEnter: handleMouseEnter })}
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

				{hasPlacements && <ResultOverlay placements={placements} mode="fill" />}

				{GROUP_OUTLINES.map(({ id, d }) => (
					<path
						key={`outline-${id}`}
						d={d}
						strokeWidth={OUTLINE_STROKE}
						className="pointer-events-none fill-none stroke-board-outline"
					/>
				))}

				{hasPlacements && <ResultOverlay placements={placements} mode="outline" />}

				{isRunning && (
					<g className="pointer-events-none">
						{[...selectedCells].map(key => {
							const [r, c] = parseKey(key);
							return (
								<rect
									key={`dim-${key}`}
									x={c}
									y={r}
									width={1}
									height={1}
									fill="black"
									opacity={0.4}
								/>
							);
						})}
					</g>
				)}
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
						onEditStart={handleEditStart}
						onEditEnd={handleEditEnd}
					/>
				))}

			<SolverNotice dismissTriggerRef={containerRef} />
		</div>
	);
}
