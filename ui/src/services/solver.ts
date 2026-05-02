// Effect.Service wrapper around SolverPortfolio.

import { Data, Effect } from "effect";

import { SolverPortfolio } from "@/solver/SolverPortfolio.ts";
import type { ExactCoverInput, ExactCoverResult, SolveOptions } from "@/types/wasm";

// Domain errors

/** Every worker failed with an infrastructure error. */
export class SolverInfraFailed extends Data.TaggedError("SolverInfraFailed")<{
	readonly reason: string;
}> {}

/** runSolve was called while another solve is still in flight. */
export class SolverBusy extends Data.TaggedError("SolverBusy")<Record<string, never>> {}

export type SolverError = SolverInfraFailed | SolverBusy;

// Service

/**
 * Drives a SolverPortfolio behind an Effect surface.
 */
export class Solver extends Effect.Service<Solver>()("ui/solver", {
	effect: Effect.sync(() => {
		let active: SolverPortfolio | null = null;

		const runSolve = (
			input: ExactCoverInput,
			options: SolveOptions
		): Effect.Effect<ExactCoverResult, SolverError> =>
			Effect.suspend((): Effect.Effect<ExactCoverResult, SolverError> => {
				if (active !== null)
					return Effect.fail(new SolverBusy({}));

				const portfolio = new SolverPortfolio();
				active = portfolio;

				return Effect.tryPromise({
					try: () => portfolio.solve(input, options),
					catch: err => new SolverInfraFailed({
						reason: err instanceof Error ? err.message : String(err)
					})
				}).pipe(
					Effect.ensuring(Effect.sync(() => {
						if (active === portfolio)
							active = null;
					}))
				);
			});

		const cancel = (): Effect.Effect<void> =>
			Effect.sync(() => {
				active?.cancel();
			});

		return { runSolve, cancel };
	})
}) {}
