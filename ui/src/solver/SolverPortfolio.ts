// Runs N solver workers in parallel and returns the result from whichever one finishes first.
// The remaining workers are cancelled.

import type { ExactCoverInput, ExactCoverResult, SolveOptions } from "@solver/wasm";

import { SolverWorker } from "./SolverWorker.ts";

const MAX_WORKERS = 20;

function pickWorkerCount(): number {
	const cores = navigator.hardwareConcurrency ?? 4;
	return Math.max(1, Math.min(cores - 1, MAX_WORKERS));
}

type State = "idle" | "solving" | "done";

export class SolverPortfolio {
	private readonly workers: readonly SolverWorker[];
	private state: State = "idle";

	constructor() {
		const count = pickWorkerCount();
		this.workers = Array.from({ length: count }, () => new SolverWorker());
	}

	get workerCount(): number {
		return this.workers.length;
	}

	/**
	 * Run all workers concurrently on the same input.
	 * Resolves with the first worker's result; the rest are canceled.
	 */
	async solve(input: ExactCoverInput, options: SolveOptions): Promise<ExactCoverResult> {
		if (this.state !== "idle")
			throw new Error("SolverPortfolio is one=shot; create a new instance.");

		this.state = "solving";

		const promises = this.workers.map(w => w.solve(input, options));

		// Pre-attach no-op catches.
		for (const p of promises)
			p.catch(() => undefined);

		try {
			const result = await Promise.any(promises);
			for (const w of this.workers)
				w.cancel();

			return result;
		} catch (err) {
			throw new Error(
				`all ${this.workers.length} solver workers failed`,
				{ cause: err }
			);
		} finally {
			this.state = "done";
		}
	}

	/** Signal all workers to stop. */
	cancel(): void {
		for (const w of this.workers)
			w.cancel();
	}
}
