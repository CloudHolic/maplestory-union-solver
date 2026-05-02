// Tracks the browser Notification permission state and exposes a request trigger.

import { useCallback, useEffect, useState } from "react";

type Permission = "default" | "granted" | "denied" | "unsupported";

function readCurrent(): Permission {
	if (typeof Notification === "undefined")
		return "unsupported";

	return Notification.permission;
}

interface UsePermission {
	permission: Permission;
	request: () => void;
}

export function usePermission(): UsePermission {
	const [permission, setPermission] = useState<Permission>(readCurrent);

	// Subscribe to permission changes.
	useEffect(() => {
		if (typeof navigator === "undefined" || navigator.permissions === undefined)
			return;

		let status: PermissionStatus | null = null;
		let cancelled = false;

		const handleChange = () => {
			if (status !== null)
				setPermission(status.state as Permission);
		};

		navigator.permissions
			.query({ name: "notifications" })
			.then(s => {
				if (cancelled)
					return;

				status = s;
				setPermission(s.state as Permission);
				s.addEventListener("change", handleChange);
			})
			.catch(() => {
				// Permissions API unsupported or query rejected.
			});

		return () => {
			cancelled = true;
			if (status !== null)
				status.removeEventListener("change", handleChange);
		};
	}, []);

	const request = useCallback(() => {
		if (typeof Notification === "undefined")
			return;
		if (Notification.permission === "default")
			return;

		void Notification.requestPermission().then(result => {
			setPermission(result);
		});
	}, []);

	return { permission, request };
}
