import type { ShapeDef } from "@/domain/pieces.ts";

interface Props {
	shape: ShapeDef;
	color: string;
	active: boolean;
}

const CELL = 14;
const GAP = 1.5;

/** Renders a single shape as an SVG thumbnail. */
export function ShapeThumbnail({ shape, color, active }: Props) {
	const maxRow = Math.max(...shape.cells.map(([r]) => r));
	const maxCol = Math.max(...shape.cells.map(([, c]) => c));
	const w = (maxCol + 1) * CELL + GAP;
	const h = (maxRow + 1) * CELL + GAP;

	return (
		<svg
			width={w}
			height={h}
			viewBox={`0 0 ${w} ${h}`}
			className={active ? "opacity-100" : "opacity-30"}
		>
			{shape.cells.map(([r, c], i) => (
				<rect
					key={i}
					x={c * CELL + GAP}
					y={r * CELL + GAP}
					width={CELL - GAP}
					height={CELL - GAP}
					fill={color}
					rx={1.5}
				/>
			))}
		</svg>
	);
}
