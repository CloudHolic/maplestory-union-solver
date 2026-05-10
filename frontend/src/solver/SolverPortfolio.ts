// Runs N solver workers in parallel and returns the result from whichever one finishes first.
// The remaining workers are cancelled.

import type { ExactCoverInput, ExactCoverResult, SolveOptions } from "@/types/wasm.ts";

import { SolverWorker } from "./SolverWorker.ts";

function pickWorkerCount(): number {
	const cores = navigator.hardwareConcurrency ?? 4;
	return Math.max(2, cores - 1);
}

type PortfolioState = "idle" | "solving" | "done";

export class SolverPortfolio {
	private readonly workers: readonly SolverWorker[];
	private state: PortfolioState = "idle";

	constructor() {
		const count = pickWorkerCount();
		this.workers = Array.from({ length: count }, () => new SolverWorker());
	}

	get workerCount(): number {
		return this.workers.length;
	}

	/**
	 * Run all workers concurrently on the same input.
	 * Resolves with the first worker's result; the rest are cancelled.
	 */
	async solve(input: ExactCoverInput, options: SolveOptions): Promise<ExactCoverResult> {
		if (this.state !== "idle")
			throw new Error("SolverPortfolio is one-shot; create a new instance.");

		this.state = "solving";

		const baseSeed = crypto.getRandomValues(new Uint32Array(1))[0]!;
		const promises = this.workers.map((w, i) => {
			const seed = (baseSeed + i * 0x9E3779B9) >>> 0;
			return w.solve(input, { ...options, seed });
		});

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
