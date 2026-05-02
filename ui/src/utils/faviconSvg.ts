import type { FaviconInput } from "@/types/favicon.ts";

const FILL_BY_STATE: Record<FaviconInput, string> = {
	idle: "#f5f5f5",
	running: "#06b6d4",
	success: "#22c55e",
	no_solution: "#ef4444",
	error: "#ef4444",
	cancelled: "#f5f5f5"
};

const STROKE = "#1a1a1a";
const STROKE_WIDTH = 1.5;

function buildSvg(fill: string): string {
	return [
		`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">`,
		`<rect x="4" y="4" width="12" height="12" fill="${fill}" stroke="${STROKE}" stroke-width="${STROKE_WIDTH}"/>`,
		`<rect x="4" y="16" width="12" height="12" fill="${fill}" stroke="${STROKE}" stroke-width="${STROKE_WIDTH}"/>`,
		`<rect x="16" y="16" width="12" height="12" fill="${fill}" stroke="${STROKE}" stroke-width="${STROKE_WIDTH}"/>`,
		`</svg>`
	].join("");
}

export function faviconDataUrl(state: FaviconInput): string {
	const svg = buildSvg(FILL_BY_STATE[state]);
	return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}
