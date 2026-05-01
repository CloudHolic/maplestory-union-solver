import { Effect, Either } from "effect";
import { create } from "zustand";

import { aggregatePresetCounts, SHAPE_COUNT } from "@/domain/pieces.ts";
import { type CharacterData, type CharacterError, Characters } from "@/services/characters.ts";
import { runtime } from "@/services/runtime.ts";

import { useRecentSearchesStore } from "./recentSearchesStore.ts";

type Status = "idle" | "loading" | "loaded" | "error";

interface CharacterState {
	status: Status;
	nickname: string;
	data: CharacterData | null;
	errorMessage: string | null;
	selectedPresetIndex: number;
	shapeCounts: ReadonlyArray<number>;

	search: (nickname: string) => void;
	selectPreset: (index: number) => void;
	updateShapeCount: (shapeIndex: number, value: number) => void;
	resetShapeCounts: () => void;
	clear: () => void;
}

const ZERO_COUNTS: ReadonlyArray<number> = new Array<number>(SHAPE_COUNT).fill(0);

let searchSeq = 0;

function formatError(err: CharacterError): string {
	switch (err._tag) {
		case "CharacterNotFound":
			return `'${err.nickname}' 캐릭터를 찾을 수 없어요.`;
		case "CharacterRateLimited":
			return "검색이 너무 잦아요. 잠시 후 다시 시도하세요.";
		case "CharacterUpstreamUnavailable":
			return "서버가 일시적으로 응답하지 않아요.";
		case "CharacterFetchFailed":
			return `검색에 실패했어요: ${err.reason}`;
	}
}

export const useCharacterStore = create<CharacterState>((set, get) => ({
	status: "idle",
	nickname: "",
	data: null,
	errorMessage: null,
	selectedPresetIndex: 0,
	shapeCounts: ZERO_COUNTS,

	search: rawNickname => {
		const nickname = rawNickname.trim();
		if (nickname.length === 0)
			return;

		const mySeq = ++searchSeq;
		set({ status: "loading", nickname, errorMessage: null });

		const program = Effect.either(
			Effect.flatMap(Characters, c => c.fetchByNickname(nickname))
		);

		void runtime.runPromise(program).then(result => {
			// Stale response - a newer search has superseded this one.
			if (mySeq !== searchSeq)
				return;

			if (Either.isRight(result)) {
				const data = result.right;
				const presetIndex = Math.max(0, Math.min(4, data.usePresetNo - 1));
				const counts = aggregatePresetCounts(data.presets[presetIndex] ?? []);

				set({
					status: "loaded",
					data,
					selectedPresetIndex: presetIndex,
					shapeCounts: counts,
					errorMessage: null
				});

				useRecentSearchesStore.getState().push(nickname);
			} else
				set({
					status: "error",
					errorMessage: formatError(result.left)
				});
		}).catch(() => {
			// Defects (Effect.Die) - shouldn't happen in normal operation.
			if (mySeq !== searchSeq)
				return;

			set({
				status: "error",
				errorMessage: "예상치 못한 오류가 발생했어요."
			});
		});
	},

	selectPreset: index => {
		const { data } = get();
		if (data === null)
			return;

		if (index < 0 || index >= data.presets.length)
			return;

		const counts = aggregatePresetCounts(data.presets[index] ?? []);
		set({ selectedPresetIndex: index, shapeCounts: counts });
	},

	updateShapeCount: (shapeIndex, value) => {
		if (shapeIndex < 0 || shapeIndex >= SHAPE_COUNT)
			return;

		const clamped = Math.max(0, Math.floor(value));
		const { shapeCounts } = get();
		if (shapeCounts[shapeIndex] === clamped)
			return;

		const next = [...shapeCounts];
		next[shapeIndex] = clamped;
		set({ shapeCounts: next });
	},

	resetShapeCounts: () => {
		set({ shapeCounts: ZERO_COUNTS });
	},

	clear: () => {
		// Invalidate any in-flight search so its resolution doesn't rest us.
		++searchSeq;
		set({
			status: "idle",
			nickname: "",
			data: null,
			errorMessage: null,
			selectedPresetIndex: 0,
			shapeCounts: ZERO_COUNTS
		});
	}
}));
