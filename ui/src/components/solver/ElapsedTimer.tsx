// Displays solver run elapsed time.

import { useSolverStore } from "@/state/solverStore.ts";

function formatElapsed(ms: number): string {
	const totalDeci = Math.floor(ms / 100);
	const deci = totalDeci % 10;
	const totalSec = Math.floor(totalDeci / 10);
	const sec = totalSec % 60;
	const min = Math.floor(totalSec / 60);

	const mm = min.toString().padStart(2, "0");
	const ss = sec.toString().padStart(2, "0");
	return `${mm}:${ss}.${deci}`;
}

export function ElapsedTimer() {
	const elapsedMs = useSolverStore(s => s.elapsedMs);
	const status = useSolverStore(s => s.status);

	const muted = status === "done" || status === "error";
	const className = "font-mono text-sm tabular-nums " +
		(muted ? "text-zinc-500" : "text-zinc-200");

	return (
		<span
			className={className}
			aria-live="polite"
			aria-label="경과 시간"
		>
			{formatElapsed(elapsedMs)}
		</span>
	);
}
