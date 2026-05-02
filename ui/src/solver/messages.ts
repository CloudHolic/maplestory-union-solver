// Worker message protocol for the solver.

import type { ExactCoverInput, ExactCoverResult, SolveOptions } from "@/types/wasm.ts";

export type SolveRequest = {
	kind: "solve";
	input: ExactCoverInput;
	options: SolveOptions;
	cancelBuffer: SharedArrayBuffer;
};

export type WorkerResponse =
	| { kind: "result"; result: ExactCoverResult }
	| { kind: "error"; message: string };
