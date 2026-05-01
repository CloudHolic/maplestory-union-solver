import { Input, TextField } from "@heroui/react";
import { useState } from "react";

import { useCharacterStore } from "@/state/characterStore.ts";
import { useRecentSearchesStore } from "@/state/recentSearchesStore.ts";

/** Nickname input with a recent-searches dropdown. */
export function NicknameSearch() {
	const [input, setInput] = useState("");
	const [open, setOpen] = useState(false);

	const recents = useRecentSearchesStore(s => s.entries);
	const search = useCharacterStore(s => s.search);

	const filtered = recents.filter(e =>
		input === "" || e.nickname.toLocaleLowerCase().includes(input.toLowerCase())
	);

	const submit = (nickname: string) => {
		const trimmed = nickname.trim();
		if (trimmed.length === 0)
			return;

		setInput(trimmed);
		setOpen(false);
		search(trimmed);
	};

	return (
		<div className="relative w-40">
			<TextField value={input} onChange={setInput}>
				<Input
					placeholder="닉네임 입력"
					onFocus={() => setOpen(true)}
					onBlur={() => setOpen(false)}
					onKeyDown={e => {
						if (e.key === "Enter")
							submit(input);
						else if (e.key === "Escape")
							setOpen(false);
					}}
				/>
			</TextField>

			{open && filtered.length > 0 && (
				<ul className="bg-content1 border-default-300 absolute z-10 mt-1 max-h-60 w-full overflow-y-auto rounded border shadow-lg">
					{filtered.map(entry => (
						<li
							key={entry.nickname}
							className="hover:bg-default-200 cursor-pointer px-3 py-2 text-sm"
							onMouseEnter={e => {
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
