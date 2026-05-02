// Keeps `<link rel="icon">` synced with solver status.

import { useEffect, useState } from "react";

import { useSolverOutcome } from "@/hooks/useSolverOutcome.ts";
import { useSolverStore } from "@/state/solverStore.ts";
import type { FaviconInput } from "@/types/favicon.ts";
import { faviconDataUrl } from "@/utils/faviconSvg.ts";

const FAVICON_LINK_SELECTION = "link[rel=\"icon\"]";

function ensureFaviconLink(): HTMLLinkElement | null {
	const existing = document.querySelector<HTMLLinkElement>(FAVICON_LINK_SELECTION);
	if (existing !== null)
		return existing;

	const link = document.createElement("link");
	link.rel = "icon";
	link.type = "image/svg+xml";
	document.head.appendChild(link);
	return link;
}

export function useFavicon(): void {
	const status = useSolverStore(s => s.status);

	const [lastOutcome, setLastOutcome] = useState<FaviconInput>("idle");

	useSolverOutcome(({ outcome }) => setLastOutcome(outcome));

	useEffect(() => {
		const link = ensureFaviconLink();
		if (link === null)
			return;

		const input: FaviconInput = (status === "idle" || status === "running") ? status : lastOutcome;
		link.href = faviconDataUrl(input);
	}, [status, lastOutcome]);
}
