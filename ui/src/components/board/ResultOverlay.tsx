// Renders solver placement result on top of the board's SVG.

import { memo, useMemo } from "react";

import { shapeColor } from "@/components/shapePalette";
import { SHAPES } from "@/domain/pieces.ts";
import type { SolutionPlacement } from "@/types/wasm";
import { computeOutlinePath } from "@/utils/boardOutline.ts";
import { cellKey } from "@/utils/coords.ts";

type OverlayMode = "fill" | "outline";

interface ResultOverlayProps {
	placements: ReadonlyArray<SolutionPlacement>;
	mode: OverlayMode;
}

const FILL_OPACITY = 0.85;
const PLACEMENT_STROKE = 0.14;
const PLACEMENT_STROKE_COLOR = "#0a0a0a";

function shapeIndexFor(defId: string): number {
	return SHAPES.findIndex(s => s.id === defId);
}

function ResultOverlayInner({ placements, mode }: ResultOverlayProps) {
	const fillCells = useMemo(() => {
		if (mode !== "fill")
			return [];

		return placements.flatMap((p, i) => {
			const idx = shapeIndexFor(p.piece.defId);
			const color = idx > 0 ? shapeColor(idx) : "#888";
			return p.cells.map(([r, c]) => ({
				key: `fill-${i}-${r}-${c}`,
				r,
				c,
				color
			}));
		});
	}, [placements, mode]);

	const outlinePaths = useMemo(() => {
		if (mode !== "outline")
			return [];

		return placements.map((p, i) => ({
			key: `placement-${i}-${p.piece.defId}-${p.piece.index}`,
			d: computeOutlinePath(new Set(p.cells.map(([r, c]) => cellKey(r, c))))
		}));
	}, [placements, mode]);

	if (mode === "fill")
		return (
			<g className="pointer-events-none">
				{fillCells.map(({ key, r, c, color }) => (
					<rect
						key={key}
						x={c}
						y={r}
						width={1}
						height={1}
						fill={color}
						opacity={FILL_OPACITY}
					/>
				))}
			</g>
		);

	return (
		<g className="pointer-events-none">
			{outlinePaths.map(({ key, d }) => (
				<path
					key={key}
					d={d}
					fill="none"
					stroke={PLACEMENT_STROKE_COLOR}
					strokeWidth={PLACEMENT_STROKE}
					strokeLinejoin="round"
				/>
			))}
		</g>
	);
}

export const ResultOverlay = memo(ResultOverlayInner);
