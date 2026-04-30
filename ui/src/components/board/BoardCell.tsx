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
			x={col + 0.04}
			y={row + 0.04}
			width={0.92}
			height={0.92}
			className={`${fill} ${cursor}`}
			onClick={() => onClick(key)}
			onContextMenu={e => {
				e.preventDefault();
				onContextMenu(key);
			}}
		/>
	);
}
