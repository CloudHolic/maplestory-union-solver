// Transient board overlay that surfaces solver outcomes.

import { AlertTriangle, Info } from "lucide-react";
import * as React from "react";
import { useEffect, useEffectEvent, useRef, useState } from "react";

import { useNotification } from "@/hooks/useNotification.ts";
import { useSolverOutcome } from "@/hooks/useSolverOutcome.ts";
import { useSolverStore } from "@/state/solverStore.ts";
import type { OutcomePayload, SolverOutcome } from "@/types/solver.ts";

type NoticeKind = "no_solution" | "error";

interface NoticeMessage {
	kind: NoticeKind,
	title: string,
	body: string
}

const AUTO_DISMISS_MS = 30_000;

function messageFor(kind: NoticeKind, errorMessage: string | null): NoticeMessage {
	switch (kind) {
		case "no_solution":
			return { kind: kind, title: "배치 실패", body: "배치를 찾지 못했어요" };
		case "error":
			return {
				kind: kind,
				title: "도우미 오류",
				body: errorMessage ?? "도우미 실행에 실패했어요"
			};
	}
}

function isSurfaceable(outcome: SolverOutcome): outcome is NoticeKind {
	return outcome === "no_solution" || outcome === "error";
}

interface SolverNoticeProps {
	dismissTriggerRef: React.RefObject<HTMLElement | null>;
}

export function SolverNotice({ dismissTriggerRef }: SolverNoticeProps) {
	const status = useSolverStore(s => s.status);
	const [notice, setNotice] = useState<NoticeMessage | null>(null);
	const dismissTimerRef = useRef<number | null>(null);
	const { notify, requestPermission } = useNotification();

	function clearDismissTimer(): void {
		const id = dismissTimerRef.current;
		if (id !== null) {
			clearTimeout(id);
			dismissTimerRef.current = null;
		}
	}

	function dismiss(): void {
		clearDismissTimer();
		setNotice(null);
	}

	function showNotice(kind: NoticeKind, errorMessage: string | null): void {
		clearDismissTimer();

		const message = messageFor(kind, errorMessage);
		setNotice(message);
		notify(message.title, message.body);

		dismissTimerRef.current = setTimeout(() => {
			setNotice(null);
			dismissTimerRef.current = null;
		}, AUTO_DISMISS_MS);
	}

	// Permission request piggybacks on the first "running" entry.
	const requestedRef = useRef(false);
	const requestOnce = useEffectEvent(() => {
		if (!requestedRef.current) {
			requestedRef.current = true;
			requestPermission();
		}
	});

	useEffect(() => {
		if (status === "running")
			requestOnce();
	}, [status]);

	useSolverOutcome(({ outcome, errorMessage }: OutcomePayload) => {
		if (isSurfaceable(outcome))
			showNotice(outcome, errorMessage);
	});

	// Dismiss on any board-container click.
	useEffect(() => {
		if (notice === null)
			return;

		const target = dismissTriggerRef.current;
		if (target === null)
			return;

		const handler = () => {
			clearDismissTimer();
			setNotice(null);
		};
		target.addEventListener("mousedown", handler);
		return () => target.removeEventListener("mousedown", handler);
	}, [notice, dismissTriggerRef]);

	// Cleanup on unmount.
	useEffect(() => () => clearDismissTimer(), []);

	if (notice === null)
		return null;

	const Icon = notice.kind === "error" ? AlertTriangle : Info;
	const palette = notice.kind === "error"
		? "bg-red-900/95 border-red-700 text-red-50"
		: "bg-zinc-800/95 border-zinc-600 text-zinc-100";

	return (
		<div
			className="pointer-events-none absolute inset-0 z-30 flex items-center justify-center"
			role="status"
			aria-live="polite"
		>
			<button
				type="button"
				onClick={dismiss}
				className={`pointer-events-auto flex items-center gap-3 rounded-xl border px-6 py-4 text-base font-medium shadow-2xl backdrop-blur ${palette}`}
			>
				<Icon size={20} className="shrink-0" aria-hidden />
				<div className="flex flex-col gap-1">
					<span className="text-base font-semibold">{notice.title}</span>
					<span className="text-sm opacity-90">{notice.body}</span>
				</div>
			</button>
		</div>
	);
}
