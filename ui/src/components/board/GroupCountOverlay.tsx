import { Button, Input } from "@heroui/react";
import { useEffect, useEffectEvent, useRef, useState } from "react";

import { BOARD_HEIGHT, BOARD_WIDTH, type GroupId, UNION_BOARD } from "@/domain/boardLayout.ts";
import { useBoardStore } from "@/state/boardStore.ts";

interface CountInputProps {
	initialValue: number;
	max: number;
	groupId: GroupId;

	onCommit: () => void;
}

interface GroupCountOverlayProps {
	groupId: GroupId;
	count: number;
	boardPad: number;
	editing: boolean;

	onEditStart: () => void;
	onEditEnd: () => void;
}

export function GroupCountOverlay({
	groupId,
	count,
	boardPad,
	editing,
	onEditStart,
	onEditEnd
}: GroupCountOverlayProps) {
	const group = UNION_BOARD.groups.find(g => g.id === groupId);
	if (group === undefined)
		return null;

	const [centroidR, centroidC] = group.centroid;

	// viewBox dimensions including pad on both sides.
	const viewW = BOARD_WIDTH + 2 * boardPad;
	const viewH = BOARD_HEIGHT + 2 * boardPad;

	const xPercent = (centroidC + 0.5 + boardPad) / viewW * 100;
	const yPercent = (centroidR + 0.5 + boardPad) / viewH * 100;

	return (
		<div
			className="absolute -translate-x-1/2 -translate-y-1/2"
			style={{ left: `${xPercent}%`, top: `${yPercent}%` }}
		>
			{editing ? (
				<CountInput
					key={`${groupId}-${count}`}
					initialValue={count}
					max={group.cells.length}
					groupId={groupId}
					onCommit={onEditEnd}
				/>
			) : (
				<Button
					variant="tertiary"
					onPress={onEditStart}
					className="min-w-0 cursor-pointer bg-transparent px-1 text-3xl font-bold text-black data-hovered:bg-transparent data-pressed:bg-transparent"
				>
					{count}
				</Button>
			)}
		</div>
	);
}

function CountInput({
	initialValue,
	max,
	groupId,
	onCommit
}: CountInputProps) {
	const setGroupCount = useBoardStore(s => s.setGroupCount);
	const [draft, setDraft] = useState(String(initialValue));
	const inputRef = useRef<HTMLInputElement>(null);

	const stableOnCommit = useEffectEvent(() => onCommit());
	const onUnmountCommit = useEffectEvent(() => {
		if (useBoardStore.getState().groupCounts[groupId] === 0)
			return;

		const v = Number(draft);
		if (!Number.isNaN(v) && draft !== "") {
			const clamped = Math.max(1, Math.min(Math.floor(v), max));
			setGroupCount(groupId, clamped);
		}
	});

	useEffect(() => {
		inputRef.current?.focus();
		inputRef.current?.select();

		const handleDocMouseDown = (e: MouseEvent) => {
			const target = e.target as Element;
			if (inputRef.current?.contains(target))
				return;

			if (target.closest("svg"))
				return;

			stableOnCommit();
		};

		document.addEventListener("mousedown", handleDocMouseDown);

		return () => {
			document.removeEventListener("mousedown", handleDocMouseDown);
			onUnmountCommit();
		};
	}, []);

	return (
		<Input
			ref={inputRef}
			type="number"
			value={draft}
			min={1}
			max={max}
			onChange={e => setDraft(e.target.value)}
			onKeyDown={e => {
				if (e.key === "Enter" || e.key === "Escape")
					onCommit();
			}}
			className="w-16 text-center text-xl font-bold"
			aria-label="Group count"
		/>
	);
}
