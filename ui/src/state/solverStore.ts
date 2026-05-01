// Owns the solve run: input snapshot, run state, elapsed timer, result.

import { Effect } from "effect";
import { create } from "zustand";

import type { ExactCoverResult } from "@solver/wasm";

import { runtime } from "@/services/runtime.ts";
import { Selection } from "@/services/selection.ts";
import { Solver } from "@/services/solver.ts";
import { buildExactCoverInput } from "@/solver/inputBuilder.ts";

import { useBoardStore } from "./boardStore.ts";
import { useCharacterStore } from "./characterStore.ts";

type Status = "idle" | "running" | "done" | "error";

interface InputSnapshot {
	readonly shapeCounts: ReadonlyArray<number>;
	readonly selectedCells: ReadonlySet<string>;
	readonly nickname: string;
}

interface SolverStoreState {
	status: Status;
	inputSnapshot: InputSnapshot | null;
	elapsedMs: number;
	result: ExactCoverResult | null;
	errorMessage: string | null;

	startSolve: () => void;
	cancel: () => void;
	reset: () => void;
}

let timerId: ReturnType<typeof setInterval> | null = null;

function clearTimer(): void {
	if (timerId !== null) {
		clearInterval(timerId);
		timerId = null;
	}
}

function buildSelectionBlob(snapshot: InputSnapshot): unknown {
	return {
		v: 1,
		shapeCounts: [...snapshot.shapeCounts],
		selectedCells: [...snapshot.selectedCells].sort(),
		groupCounts: {}
	};
}

export const useSolverStore = create<SolverStoreState>((set, get) => ({
	status: "idle",
	inputSnapshot: null,
	elapsedMs: 0,
	result: null,
	errorMessage: null,

	startSolve: () => {
		if (get().status === "running")
			return;

		const board = useBoardStore.getState();
		const characters = useCharacterStore.getState();

		const snapshot: InputSnapshot = {
			shapeCounts: characters.shapeCounts,
			selectedCells: new Set(board.selectedCells),
			nickname: characters.nickname
		};

		let input;
		try {
			input = buildExactCoverInput(snapshot.shapeCounts, snapshot.selectedCells);
		} catch (err) {
			set({
				status: "error",
				inputSnapshot: null,
				elapsedMs: 0,
				result: null,
				errorMessage: err instanceof Error
					? `입력이 잘못됐어요: ${err.message}`
					: "입력이 잘못됐어요"
			});

			return;
		}

		const startedAt = performance.now();
		set({
			status: "running",
			inputSnapshot: snapshot,
			elapsedMs: 0,
			result: null,
			errorMessage: null
		});

		// Elapsed timer (100ms tick).
		clearTimer();
		timerId = setInterval(() => {
			set({ elapsedMs: performance.now() - startedAt });
		}, 100);

		// Fire-and-forget selection save. Skip when nickname is empty.
		if (snapshot.nickname.length > 0) {
			const blob = buildSelectionBlob(snapshot);
			const program = Effect.flatMap(Selection, s =>
				s.saveSelection(snapshot.nickname, blob)
			);
			void runtime.runPromise(program).catch((err: unknown) => {
				console.warn("selection save failed:", err);
			});
		}

		// Run the solver.
		const solveProgram = Effect.flatMap(Solver, s =>
			s.runSolve(input, {
				timeoutMs: undefined,
				seed: undefined
			})
		);

		void runtime.runPromise(solveProgram)
			.then(result => {
				clearTimer();
				set({
					status: "done",
					result,
					elapsedMs: performance.now() - startedAt
				});
			})
			.catch((err: unknown) => {
				clearTimer();
				set({
					status: "error",
					elapsedMs: performance.now() - startedAt,
					errorMessage: err instanceof Error
						? `솔버 실행 실패: ${err.message}`
						: "솔버 실행 실패"
				});
			});
	},

	cancel: () => {
		if (get().status !== "running")
			return;

		const program = Effect.flatMap(Solver, s => s.cancel());
		void runtime.runPromise(program);
	},

	reset: () => {
		if (get().status === "running")
			return;

		clearTimer();
		set({
			status: "idle",
			inputSnapshot: null,
			elapsedMs: 0,
			result: null,
			errorMessage: null
		});
	}
}));
