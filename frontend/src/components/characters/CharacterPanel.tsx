import { Button } from "@heroui/react";

import { useCharacterStore } from "@/state/characterStore.ts";
import { useSolverStore } from "@/state/solverStore.ts";

import { NicknameSearch } from "./NicknameSearch.tsx";
import { PresetTabs } from "./PresetTabs.tsx";
import { ShapeGrid } from "./ShapeGrid.tsx";

export function CharacterPanel() {
	const status = useCharacterStore(s => s.status);
	const nickname = useCharacterStore(s => s.nickname);
	const errorMessage = useCharacterStore(s => s.errorMessage);
	const resetShapeCounts = useCharacterStore(s => s.resetShapeCounts);

	const isRunning = useSolverStore(s => s.status === "running");

	return (
		<div className="flex flex-col gap-4">
			<NicknameSearch isDisabled={isRunning} />

			{status === "loading" && (
				<p className="text-sm text-foreground/60">
					{`'${nickname}' 검색 중...`}
				</p>
			)}

			{status === "error" && errorMessage !== null && (
				<p className="text-sm text-danger">
					{errorMessage}
				</p>
			)}

			{status === "loaded" && (
				<>
					<h3 className="text-sm">
						<span className="font-bold">{nickname}</span>의 공격대원 정보 (실시간)
					</h3>
					<PresetTabs isDisabled={isRunning} />
				</>
			)}

			<ShapeGrid isDisabled={isRunning} />

			<Button
				variant="danger"
				onPress={resetShapeCounts}
				isDisabled={isRunning}
				className="self-end"
			>
				블록 초기화
			</Button>
		</div>
	);
}
