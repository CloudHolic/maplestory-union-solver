// Main-thread wrapper around a single solver Worker.

import type {
	ExactCoverInput,
	ExactCoverResult,
	SolveOptions
} from "@/types/wasm";

import type { SolveRequest, WorkerResponse } from "./messages.ts";
import SolverWorkerCtor from "./solver.worker.ts?worker";

export class SolverWorker {
	private readonly worker: Worker;
	private readonly cancelBuffer: SharedArrayBuffer;
	private readonly cancelView: Int32Array;

	/**
	 * `true` once `solve()` has been called.
	 */
	private started = false;

	constructor() {
		this.cancelBuffer = new SharedArrayBuffer(4);
		this.cancelView = new Int32Array(this.cancelBuffer);
		this.worker = new SolverWorkerCtor();
	}

	/**
	 * Run the solver. Resolves with the result regardless of outcome.
	 * Rejects only on infrastructure errors.
	 */
	solve(
		input: ExactCoverInput,
		options: SolveOptions
	): Promise<ExactCoverResult> {
		if (this.started)
			return Promise.reject(
				new Error("SolverWorker is one-shot; create a new instance.")
			);

		this.started = true;

		const { promise, resolve, reject } =
			Promise.withResolvers<ExactCoverResult>();

		this.worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
			const response = event.data;
			switch (response.kind) {
				case "result":
					resolve(response.result);
					break;
				case "error":
					reject(new Error(`solver error: ${response.message}`));
					break;
			}
			this.worker.terminate();
		};

		this.worker.onerror = event => {
			reject(new Error(`worker crash: ${event.message}`));
			this.worker.terminate();
		};

		const request: SolveRequest = {
			kind: "solve",
			input,
			options,
			cancelBuffer: this.cancelBuffer
		};
		this.worker.postMessage(request);

		return promise;
	}

	cancel(): void {
		Atomics.store(this.cancelView, 0, 1);
	}
}
