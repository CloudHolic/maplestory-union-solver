import { create } from "zustand";
import { persist } from "zustand/middleware";

export interface RecentSearchEntry {
	nickname: string;
	searchedAt: number;
}

interface RecentSearchesState {
	entries: ReadonlyArray<RecentSearchEntry>;

	push: (nickname: string) => void;
	clear: () => void;
}

const MAX_ENTRIES = 10;

/**
 * Persisted history of nicknames the user has successfully searched.
 * Each browser/profile gets its own list automatically via localStorage origin scoping.
 */
export const useRecentSearchesStore = create<RecentSearchesState>()(
	persist(
		set => ({
			entries: [],

			push: nickname => set(state => {
				const trimmed = nickname.trim();
				if (trimmed.length === 0)
					return state;

				const deduped = state.entries.filter(e => e.nickname !== trimmed);
				const next: RecentSearchEntry[] = [
					{ nickname: trimmed, searchedAt: Date.now() },
					...deduped
				];

				return { entries: next.slice(0, MAX_ENTRIES) };
			}),

			clear: () => set({ entries: [] })
		}),
		{
			name: "ums:recent-searches",
			version: 1
		}
	)
);
