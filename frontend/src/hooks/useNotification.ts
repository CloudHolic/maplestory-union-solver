// Fires a Notification when the tab is background.
// No-op when foregrounded, when permission isn't granted, or when the API is unsupported.

import { useCallback } from "react";

import { usePermission } from "./usePermission.ts";

interface UseNotification {
	notify: (title: string, body: string) => void;
	requestPermission: () => void;
}

export function useNotification(): UseNotification {
	const { permission, request } = usePermission();

	const notify = useCallback((title: string, body: string) => {
		if (typeof document === "undefined" || !document.hidden)
			return;
		if (permission !== "granted")
			return;
		if (typeof Notification === "undefined")
			return;

		try {
			const n = new Notification(title, { body });
			n.onclick = () => {
				window.focus();
				n.close();
			};
		} catch {
			// Permission revoked between check and instantiation - silently skip.
		}
	}, [permission]);

	return { notify, requestPermission: request };
}
