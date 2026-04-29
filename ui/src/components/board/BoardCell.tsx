// One cell of the union board.

import { cellKey } from "@/utils/coords.ts";

interface BoardCellProps {
	row: number;
	col: number;
	isSelected: boolean;
	isCountMode: boolean;

	onClick: (key: string) => void;
	onContextMenu: (key: string) => void;
}

export function BoardCell({
	row,
	col,
	isSelected,
	isCountMode,
	onClick,
	onContextMenu
}: BoardCellProps) {
	const key = cellKey(row, col);
	const fill = isSelected ? "fill-board-cell-selected" : "fill-board-cell";
	const cursor = isCountMode ? "cursor-not-allowed" : "cursor-pointer";

	return (
		<rect
			x={col}
			y={row}
			width={1}
			height={1}
			className={`${fill} ${cursor}`}
			onClick={() => onClick(key)}
			onContextMenu={e => {
				e.preventDefault();
				onContextMenu(key);
			}}
		/>
	);
}
