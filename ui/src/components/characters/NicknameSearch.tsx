import { Input, TextField } from "@heroui/react";
import * as React from "react";
import { memo, useCallback, useMemo, useState } from "react";

import { useCharacterStore } from "@/state/characterStore.ts";
import { useRecentSearchesStore } from "@/state/recentSearchesStore.ts";

/** Nickname input with a recent-searches dropdown. */
export function NicknameSearch() {
	const [input, setInput] = useState("");
	const [open, setOpen] = useState(false);
	const [highlightedIndex, setHighlightedIndex] = useState(-1);

	const recents = useRecentSearchesStore(s => s.entries);
	const search = useCharacterStore(s => s.search);

	const filtered = useMemo(() => {
		if (input === "")
			return recents;

		const q = input.toLowerCase();
		return recents.filter(e => e.nickname.toLowerCase().includes(q));
	}, [recents, input]);

	const submit = useCallback((nickname: string) => {
		const trimmed = nickname.trim();
		if (trimmed.length === 0)
			return;

		setInput(trimmed);
		setOpen(false);
		setHighlightedIndex(-1);
		search(trimmed);
	}, [search]);

	const handleChange = useCallback((v: string) => {
		setInput(v);
		setHighlightedIndex(-1);
	}, []);

	const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
		if (e.key === "ArrowDown") {
			e.preventDefault();
			setHighlightedIndex(i => Math.min(i + 1, filtered.length - 1));
		} else if (e.key === "ArrowUp") {
			e.preventDefault();
			setHighlightedIndex(i => Math.max(i - 1, -1));
		} else if (e.key === "Enter") {
			const picked = highlightedIndex >= 0 ? filtered[highlightedIndex] : null;
			submit(picked?.nickname ?? input);
		} else if (e.key === "Escape") {
			setOpen(false);
			setHighlightedIndex(-1);
		}
	}, [filtered, highlightedIndex, input, submit]);

	const handleFocus = useCallback(() => setOpen(true), []);
	const handleBlur = useCallback(() => setOpen(false), []);

	return (
		<div className="relative w-44">
			<TextField
				value={input}
				onChange={handleChange}
			>
				<Input
					placeholder="닉네임 입력"
					onFocus={handleFocus}
					onBlur={handleBlur}
					onKeyDown={handleKeyDown}
					className="h-10 text-base"
				/>
			</TextField>

			{open && filtered.length > 0 && (
				<ul className="border-default-300 absolute z-10 mt-1 max-h-60 w-full overflow-y-auto rounded border bg-white shadow-lg">
					{filtered.map((entry, i) => (
						<RecentEntry
							key={entry.nickname}
							nickname={entry.nickname}
							index={i}
							highlighted={i === highlightedIndex}
							onHover={setHighlightedIndex}
							onPick={submit}
						/>
					))}
				</ul>
			)}
		</div>
	);
}

interface RecentEntryProps {
	nickname: string;
	index: number;
	highlighted: boolean;

	onHover: (index: number) => void;
	onPick: (nickname: string) => void;
}

function RecentEntryInner({ nickname, index, highlighted, onHover, onPick }: RecentEntryProps) {
	const handleEnter = useCallback(() => onHover(index), [onHover, index]);
	const handleMouseDown = useCallback((e: React.MouseEvent) => {
		e.preventDefault();
		onPick(nickname);
	}, [onPick, nickname]);

	return (
		<li
			className={`cursor-pointer px-3 py-2 text-sm text-black ${highlighted ? "bg-gray-100" : ""}`}
			onMouseEnter={handleEnter}
			onMouseDown={handleMouseDown}
		>
			{nickname}
		</li>
	);
}

const RecentEntry = memo(RecentEntryInner);
