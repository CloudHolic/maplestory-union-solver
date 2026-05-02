import { NumberField } from "@heroui/react";
import { ChevronDown, ChevronUp } from "lucide-react";
import * as React from "react";
import { memo, useCallback } from "react";

import { shapeColor } from "@/components/shapePalette.ts";
import { SHAPES } from "@/domain/pieces.ts";
import { useCharacterStore } from "@/state/characterStore.ts";

import { ShapeThumbnail } from "./ShapeThumbnail.tsx";

/** 3x5 grid of all 15 shape inputs. */
export function ShapeGrid() {
	return (
		<div className="grid grid-cols-5 gap-3">
			{SHAPES.map((_, i) => (
				<ShapeInput key={i} shapeIndex={i} />
			))}
		</div>
	);
}

interface ShapeInputProps {
	shapeIndex: number;
}

function ShapeInputInner({ shapeIndex }: ShapeInputProps) {
	const shape = SHAPES[shapeIndex]!;
	const count = useCharacterStore(s => s.shapeCounts[shapeIndex] ?? 0);
	const updateShapeCount = useCharacterStore(s => s.updateShapeCount);

	const handleChange = useCallback((v: number) => {
		updateShapeCount(shapeIndex, Number.isNaN(v) ? 0 : v);
	}, [updateShapeCount, shapeIndex]);

	const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
		if (e.key === "Enter" || e.key === "Escape")
			e.currentTarget.blur();
	}, []);

	return (
		<div className="flex flex-col items-center gap-2">
			<div className="flex h-16 items-center justify-center">
				<ShapeThumbnail
					shape={shape}
					color={shapeColor(shapeIndex)}
					active={count > 0}
				/>
			</div>

			<NumberField
				value={count}
				onChange={handleChange}
				minValue={0}
				aria-label={`${shape.id} count`}
				className="group w-20"
			>
				<NumberField.Group className="relative flex h-12 items-center overflow-hidden rounded-xl border border-white/10 bg-zinc-900 transition-all focus-within:border-blue-500">
					<NumberField.Input
						className="h-full w-full [appearance:textfield] bg-transparent text-center text-lg font-medium text-white transition-[padding] duration-200 outline-none focus:pr-9 [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
						onKeyDown={handleKeyDown}
					/>

					<div className="absolute top-1.5 right-1.5 bottom-1.5 z-20 flex w-6 flex-col opacity-0 transition-opacity duration-200 group-focus-within:opacity-100">
						<NumberField.IncrementButton className="flex w-full min-w-0 flex-1 items-center justify-center rounded-t-lg bg-white/5 p-0 text-white hover:bg-white/10 active:bg-white/20">
							<ChevronUp size={14} className="shrink-0" />
						</NumberField.IncrementButton>

						<NumberField.DecrementButton className="flex w-full min-w-0 flex-1 items-center justify-center rounded-b-lg border-t border-zinc-800 bg-white/5 p-0 text-white hover:bg-white/10 active:bg-white/20">
							<ChevronDown size={14} className="shrink-0" />
						</NumberField.DecrementButton>
					</div>
				</NumberField.Group>
			</NumberField>
		</div>
	);
}

const ShapeInput = memo(ShapeInputInner);
