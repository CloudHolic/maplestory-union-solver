// One cell of the union board.

import { memo, useCallback } from "react";

import { cellKey } from "@/utils/coords.ts";

interface BoardCellProps {
	row: number;
	col: number;
	isSelected: boolean;

	onClick: (key: string) => void;
	onContextMenu?: (key: string) => void;
}

const CELL_STROKE = 0.03;

function BoardCellInner({
	row,
	col,
	isSelected,
	onClick,
	onContextMenu
}: BoardCellProps) {
	const key = cellKey(row, col);
	const fill = isSelected ? "fill-board-cell-selected" : "fill-board-cell";

	const handleClick = useCallback(() => onClick(key), [onClick, key]);

	const handleContextMenu = useCallback((e: React.MouseEvent) => {
		if (onContextMenu === undefined)
			return;

		e.preventDefault();
		onContextMenu(key);
	}, [onContextMenu, key]);

	return (
		<rect
			x={col}
			y={row}
			width={1}
			height={1}
			strokeWidth={CELL_STROKE}
			className={`${fill} cursor-pointer stroke-board-cell-border`}
			onClick={handleClick}
			onContextMenu={onContextMenu === undefined ? undefined : handleContextMenu}
		/>
	);
}

export const BoardCell = memo(BoardCellInner);
