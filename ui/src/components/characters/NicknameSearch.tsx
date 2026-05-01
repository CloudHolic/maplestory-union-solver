import { Input, TextField } from "@heroui/react";
import { useState } from "react";
import * as React from "react";

import { useCharacterStore } from "@/state/characterStore.ts";
import { useRecentSearchesStore } from "@/state/recentSearchesStore.ts";

/** Nickname input with a recent-searches dropdown. */
export function NicknameSearch() {
	const [input, setInput] = useState("");
	const [open, setOpen] = useState(false);
	const [highlightedIndex, setHighlightedIndex] = useState(-1);

	const recents = useRecentSearchesStore(s => s.entries);
	const search = useCharacterStore(s => s.search);

	const filtered = recents.filter(e =>
		input === "" || e.nickname.toLowerCase().includes(input.toLowerCase())
	);

	const submit = (nickname: string) => {
		const trimmed = nickname.trim();
		if (trimmed.length === 0)
			return;

		setInput(trimmed);
		setOpen(false);
		setHighlightedIndex(-1);
		search(trimmed);
	};

	const handleChange = (v: string) => {
		setInput(v);
		setHighlightedIndex(-1);
	};

	const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
		if (e.key === "ArrowDown") {
			e.preventDefault();
			if (filtered.length > 0)
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
	};

	return (
		<div className="relative w-44">
			<TextField
				value={input}
				onChange={handleChange}
			>
				<Input
					placeholder="닉네임 입력"
					onFocus={() => setOpen(true)}
					onBlur={() => setOpen(false)}
					onKeyDown={handleKeyDown}
					className="h-10 text-base"
				/>
			</TextField>

			{open && filtered.length > 0 && (
				<ul className="border-default-300 absolute z-10 mt-1 max-h-60 w-full overflow-y-auto rounded border bg-white shadow-lg">
					{filtered.map((entry, i) => (
						<li
							key={entry.nickname}
							className={`cursor-pointer px-3 py-2 text-sm text-black ${i === highlightedIndex ? "bg-gray-100" : ""}`}
							onMouseEnter={() => setHighlightedIndex(i)}
							onMouseDown={e => {
								e.preventDefault();
								submit(entry.nickname);
							}}
						>
							{entry.nickname}
						</li>
					))}
				</ul>
			)}
		</div>
	);
}
