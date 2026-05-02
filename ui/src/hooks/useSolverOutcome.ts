// Subscribes to terminal solve transitions.
// The callback fires once per transition into a final state (`done` or `error`),
// with an outcome enum derived from `status` + `result`.

import { useEffect, useEffectEvent } from "react";

import { useSolverStore } from "@/state/solverStore.ts";
import type { OutcomePayload, SolverOutcome } from "@/types/solver.ts";
import type { SolverStatus } from "@/types/status.ts";

function describe(status: SolverStatus, hasSolution: boolean, wasCancelled: boolean): SolverOutcome | null {
	if (status === "done") {
		if (wasCancelled)
			return "cancelled";

		return hasSolution ? "success" : "no_solution";
	}

	if (status === "error")
		return "error";

	return null;
}

export function useSolverOutcome(onOutcome: (payload: OutcomePayload) => void): void {
	const stableHandler = useEffectEvent(onOutcome);

	useEffect(() => {
		let lastOutcome: SolverOutcome | null = null;

		return useSolverStore.subscribe(state => {
			const hasSolution = state.result?.solution !== undefined;
			const wasCancelled = state.result?.stats.cancelled === true;
			const next = describe(state.status, hasSolution, wasCancelled);

			if (next === null || next === lastOutcome)
				return;

			lastOutcome = next;
			stableHandler({
				outcome: next,
				errorMessage: state.errorMessage
			});
		});
	}, []);
}
