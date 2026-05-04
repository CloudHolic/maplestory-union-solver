import { Button, Checkbox } from "@heroui/react";
import { useCallback, useMemo } from "react";

import { ElapsedTimer } from "@/components/solver/ElapsedTimer.tsx";
import { SHAPES } from "@/domain/pieces.ts";
import { validateBoardSelection } from "@/domain/validation.ts";
import { useBoardStore } from "@/state/boardStore.ts";
import { useCharacterStore } from "@/state/characterStore.ts";
import { useSolverStore } from "@/state/solverStore.ts";

export function BoardControls() {
	const selectedCells = useBoardStore(s => s.selectedCells);
	const groupCounts = useBoardStore(s => s.groupCounts);
	const groupSelectMode = useBoardStore(s => s.groupSelectMode);
	const setGroupSelectMode = useBoardStore(s => s.setGroupSelectMode);
	const clearBoard = useBoardStore(s => s.clear);

	const shapeCounts = useCharacterStore(s => s.shapeCounts);

	const solverStatus = useSolverStore(s => s.status);
	const startSolve = useSolverStore(s => s.startSolve);
	const cancelSolve = useSolverStore(s => s.cancel);

	const isRunning = solverStatus === "running";
	const showTimer = solverStatus !== "idle";

	const stats = useMemo(() => {
		const groupArea = Object.values(groupCounts).reduce((a, b) => a + b, 0);
		const selectedArea = selectedCells.size + groupArea;
		const availableArea = shapeCounts.reduce(
			(sum, count, i) => sum + count * (SHAPES[i]?.cells.length ?? 0),
			0
		);
		const characterCount = shapeCounts.reduce((a, b) => a + b, 0);

		return { selectedArea, availableArea, characterCount };
	}, [selectedCells, groupCounts, shapeCounts]);

	const validationError = useMemo(
		() => validateBoardSelection(selectedCells, groupCounts, shapeCounts),
		[selectedCells, groupCounts, shapeCounts]
	);

	const actionDisabled = !isRunning && validationError !== null;

	const handleAction = useCallback(() => {
		if (isRunning)
			cancelSolve();
		else
			startSolve();
	}, [isRunning, cancelSolve, startSolve]);

	return (
		<div className="flex w-full items-start justify-between gap-4">
			<div className="flex flex-col gap-1 text-sm">
				<span>내가 선택한 영역 : {stats.selectedArea}</span>
				<span>선택 가능한 영역 : {stats.availableArea}</span>
				<span>등록 공격대원 수 : {stats.characterCount}</span>
				{showTimer && (
					<span className="mt-1">
						경과 시간: <ElapsedTimer />
					</span>
				)}
			</div>

			<div className="flex items-center gap-3">
				<Checkbox
					isSelected={groupSelectMode}
					onChange={setGroupSelectMode}
					isDisabled={isRunning}
					className="text-sm"
				>
					<Checkbox.Control className="border-gray-400 bg-white data-[selected=true]:border-gray-400 data-[selected=true]:bg-white">
						<Checkbox.Indicator className="text-black" />
					</Checkbox.Control>
					<Checkbox.Content>그룹 선택</Checkbox.Content>
				</Checkbox>

				<Button variant="danger" onPress={clearBoard} isDisabled={isRunning}>
					선택영역 초기화
				</Button>

				<Button
					variant={isRunning ? "danger" : "primary"}
					onPress={handleAction}
					isDisabled={actionDisabled}
				>
					{isRunning ? "중단" : "배치 시작"}
				</Button>
			</div>
		</div>
	);
}
