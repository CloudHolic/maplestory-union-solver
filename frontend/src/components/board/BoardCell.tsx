// One cell of the union board.

import * as React from "react";
import { memo, useCallback } from "react";

import { cellKey } from "@/utils/coords.ts";

interface BoardCellProps {
	row: number;
	col: number;
	isSelected: boolean;

	onClick?: (key: string) => void;
	onContextMenu?: (key: string) => void;
	onMouseDown?: (key: string, event: React.MouseEvent) => void;
	onMouseEnter?: (key: string) => void;
}

const CELL_STROKE = 0.03;

function BoardCellInner({
	row,
	col,
	isSelected,
	onClick,
	onContextMenu,
	onMouseDown,
	onMouseEnter
}: BoardCellProps) {
	const key = cellKey(row, col);
	const fill = isSelected ? "fill-board-cell-selected" : "fill-board-cell";

	const handleClick = useCallback(() => {
		if (onClick !== undefined)
			onClick(key);
	}, [onClick, key]);

	const handleContextMenu = useCallback((e: React.MouseEvent) => {
		if (onContextMenu === undefined)
			return;

		e.preventDefault();
		onContextMenu(key);
	}, [onContextMenu, key]);

	const handleMouseDown = useCallback((e: React.MouseEvent) => {
		if (onMouseDown !== undefined)
			onMouseDown(key, e);
	}, [onMouseDown, key]);

	const handleMouseEnter = useCallback(() => {
		if (onMouseEnter !== undefined)
			onMouseEnter(key);
	}, [onMouseEnter, key]);

	return (
		<rect
			x={col}
			y={row}
			width={1}
			height={1}
			strokeWidth={CELL_STROKE}
			className={`${fill} cursor-pointer stroke-board-cell-border`}
			data-cell-key={key}
			onClick={onClick === undefined ? undefined : handleClick}
			onContextMenu={onContextMenu === undefined ? undefined : handleContextMenu}
			onMouseDown={onMouseDown === undefined ? undefined : handleMouseDown}
			onMouseEnter={onMouseEnter === undefined ? undefined : handleMouseEnter}
		/>
	);
}

export const BoardCell = memo(BoardCellInner);
