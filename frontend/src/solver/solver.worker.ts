/// <reference lib="webworker" />

// Solver worker entry point.

import init, { solveExactCover } from "@solver/wasm";

import type { SolveRequest, WorkerResponse } from "./messages.ts";

// Eager init.
const initPromise = init();

self.onmessage = async (event: MessageEvent<SolveRequest>): Promise<void> => {
	const request = event.data;

	if (request.kind !== "solve") {
		postResponse({
			kind: "error",
			message: `unexpected message kind: ${(request as { kind: string }).kind}`
		});

		return;
	}

	try {
		await initPromise;
		const result = solveExactCover(
			request.input,
			request.options,
			request.cancelBuffer
		);
		postResponse({ kind: "result", result });
	} catch (error) {
		postResponse({
			kind: "error",
			message: error instanceof Error ? error.message : String(error)
		});
	}
};

// A type-safe wrapper of postMessage.
function postResponse(response: WorkerResponse): void {
	self.postMessage(response);
}
