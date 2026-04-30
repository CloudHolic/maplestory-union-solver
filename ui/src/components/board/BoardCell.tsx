// One cell of the union board.

import { cellKey } from "@/utils/coords.ts";

interface BoardCellProps {
	row: number;
	col: number;
	isSelected: boolean;

	onClick: (key: string) => void;
	onContextMenu: (key: string) => void;
}

const CELL_STROKE = 0.03;

export function BoardCell({
	row,
	col,
	isSelected,
	onClick,
	onContextMenu
}: BoardCellProps) {
	const key = cellKey(row, col);
	const fill = isSelected ? "fill-board-cell-selected" : "fill-board-cell";

	return (
		<rect
			x={col}
			y={row}
			width={1}
			height={1}
			strokeWidth={CELL_STROKE}
			className={`${fill} cursor-pointer stroke-board-cell-border`}
			onClick={() => onClick(key)}
			onContextMenu={e => {
				e.preventDefault();
				onContextMenu(key);
			}}
		/>
	);
}
