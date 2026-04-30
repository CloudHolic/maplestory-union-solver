import { useState } from "react";

import type {
	ExactCoverInput,
	ExactCoverResult,
	SolveOptions
} from "@solver/wasm";

import { Board } from "@/components/board/Board.tsx";

import { SolverWorker } from "./solver/SolverWorker";

// Sample input — same trivial 2x2 case as the original wasm-test/index.html.
// Four cells, one square piece, mark at corner. Trivially solvable.
const SAMPLE_INPUT: ExactCoverInput = {
	targetCells: ["0,0", "0,1", "1,0", "1,1"],
	pieces: [{ defId: "square", index: 0 }],
	pieceDefs: [
		[
			"square",
			{
				id: "square",
				cells: [
					[0, 0],
					[0, 1],
					[1, 0],
					[1, 1]
				],
				markIndex: 0
			}
		]
	],
	centerCells: ["0,0"]
};

const SAMPLE_OPTIONS: SolveOptions = {
	seed: 42,
	timeoutMs: 5000,
	lubyBase: 1024
};

type RunState =
	| { kind: "idle" }
	| { kind: "running"; worker: SolverWorker; startedAt: number }
	| { kind: "done"; result: ExactCoverResult; elapsedMs: number }
	| { kind: "error"; message: string };

function App() {
	const [inputJson, setInputJson] = useState(() =>
		JSON.stringify(SAMPLE_INPUT, null, 2)
	);
	const [optionsJson, setOptionsJson] = useState(() =>
		JSON.stringify(SAMPLE_OPTIONS, null, 2)
	);
	const [run, setRun] = useState<RunState>({ kind: "idle" });

	const handleSolve = async () => {
		let input: ExactCoverInput;
		let options: SolveOptions;
		try {
			input = JSON.parse(inputJson) as ExactCoverInput;
			options = JSON.parse(optionsJson) as SolveOptions;
		} catch (err) {
			setRun({
				kind: "error",
				message: `JSON parse error: ${err instanceof Error ? err.message : String(err)}`
			});
			return;
		}

		const worker = new SolverWorker();
		const startedAt = performance.now();
		setRun({ kind: "running", worker, startedAt });

		try {
			const result = await worker.solve(input, options);
			const elapsedMs = performance.now() - startedAt;
			setRun({ kind: "done", result, elapsedMs });
		} catch (err) {
			setRun({
				kind: "error",
				message: err instanceof Error ? err.message : String(err)
			});
		}
	};

	const handleCancel = () => {
		if (run.kind === "running")
			run.worker.cancel();

	};

	return (
		<div className="min-h-screen bg-background p-6 text-foreground">
			<Board />
			<hr className="border-default-300 my-8" />

			<h1 className="mb-4 text-2xl font-bold">Solver Smoke Test</h1>
			<p className="mb-6 text-sm text-foreground/60">
				Slice 1 verification — paste an ExactCoverInput and
				SolveOptions, run the solver via Worker, see the result. Cancel
				mid-solve to verify cooperative cancellation produces partial
				stats.
			</p>

			<div className="mb-4 grid grid-cols-2 gap-4">
				<label className="flex flex-col">
					<span className="mb-1 text-sm font-semibold">
						ExactCoverInput (JSON)
					</span>
					<textarea
						value={inputJson}
						onChange={e => setInputJson(e.target.value)}
						spellCheck={false}
						className="bg-content1 border-default-300 h-64 resize-y rounded border p-2 font-mono text-xs"
					/>
				</label>
				<label className="flex flex-col">
					<span className="mb-1 text-sm font-semibold">
						SolveOptions (JSON)
					</span>
					<textarea
						value={optionsJson}
						onChange={e => setOptionsJson(e.target.value)}
						spellCheck={false}
						className="bg-content1 border-default-300 h-64 resize-y rounded border p-2 font-mono text-xs"
					/>
				</label>
			</div>

			<div className="mb-4 flex gap-2">
				<button
					type="button"
					onClick={handleSolve}
					disabled={run.kind === "running"}
					className="bg-primary text-primary-foreground rounded px-4 py-2 disabled:opacity-50"
				>
					Solve
				</button>
				<button
					type="button"
					onClick={handleCancel}
					disabled={run.kind !== "running"}
					className="rounded bg-danger px-4 py-2 text-danger-foreground disabled:opacity-50"
				>
					Cancel
				</button>
			</div>

			<ResultPanel run={run} />
		</div>
	);
}

function ResultPanel({ run }: { run: RunState }) {
	switch (run.kind) {
		case "idle":
			return (
				<p className="text-foreground/60">
					Idle. Click Solve to start.
				</p>
			);

		case "running":
			return <p>Running… (cancel button is now active)</p>;

		case "error":
			return (
				<div>
					<p className="font-semibold text-danger">Error</p>
					<pre className="bg-content1 border-default-300 rounded border p-2 text-xs">
						{run.message}
					</pre>
				</div>
			);

		case "done": {
			const { result, elapsedMs } = run;
			const status = result.stats.cancelled
				? "cancelled"
				: result.stats.timedOut
					? "timed out"
					: result.solution
						? "solved"
						: "no solution";
			return (
				<div>
					<p className="font-semibold">
						Result: {status} — {elapsedMs.toFixed(1)}ms wall,{" "}
						{result.stats.elapsedMs}ms solver
					</p>
					<pre className="bg-content1 border-default-300 max-h-96 overflow-auto rounded border p-2 text-xs">
						{JSON.stringify(result, null, 2)}
					</pre>
				</div>
			);
		}
	}
}

export default App;
