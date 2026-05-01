import { Button, Checkbox } from "@heroui/react";

import { SHAPES } from "@/domain/pieces.ts";
import { useBoardStore } from "@/state/boardStore.ts";
import { useCharacterStore } from "@/state/characterStore.ts";

export function BoardControls() {
	const selectedCells = useBoardStore(s => s.selectedCells);
	const groupCounts = useBoardStore(s => s.groupCounts);
	const groupSelectMode = useBoardStore(s => s.groupSelectMode);
	const setGroupSelectMode = useBoardStore(s => s.setGroupSelectMode);
	const clearBoard = useBoardStore(s => s.clear);

	const shapeCounts = useCharacterStore(s => s.shapeCounts);

	const selectedArea = selectedCells.size +
		Object.values(groupCounts).reduce((a, b) => a + b, 0);

	const availableArea = shapeCounts.reduce((sum, count, i) =>
		sum + count * (SHAPES[i]?.cells.length ?? 0), 0);

	const characterCount = shapeCounts.reduce((a, b) => a + b, 0);

	const handleStart = () => {
		console.log("Starting...");
	};

	const canStart = characterCount > 0 && selectedArea > 0;

	return (
		<div className="flex w-full items-end justify-between gap-4">
			<div className="flex flex-col gap-0.5 text-sm">
				<span>내가 선택한 영역 : {selectedArea}</span>
				<span>선택 가능한 영역 : {availableArea}</span>
				<span>등록 공격대원 수 : {characterCount}</span>
			</div>

			<div className="flex items-center gap-2">
				<Checkbox isSelected={groupSelectMode} onChange={setGroupSelectMode}>
					<Checkbox.Indicator />
					<Checkbox.Content>그룹 선택</Checkbox.Content>
				</Checkbox>

				<Button variant="danger" size="sm" onPress={clearBoard}>
					선택영역 초기화
				</Button>

				<Button
					variant="primary"
					size="sm"
					onPress={handleStart}
					isDisabled={!canStart}
				>
					배치 시작
				</Button>
			</div>
		</div>
	);
}
