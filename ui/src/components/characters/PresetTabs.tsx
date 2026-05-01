import { Button } from "@heroui/react";

import { useCharacterStore } from "@/state/characterStore.ts";

/**
 * One button per preset. Clicking a button re-applies that preset's aggregation to the shape counts,
 * overwriting any manual edits.
 */
export function PresetTabs() {
	const data = useCharacterStore(s => s.data);
	const selectedIndex = useCharacterStore(s => s.selectedPresetIndex);
	const selectPreset = useCharacterStore(s => s.selectPreset);

	if (data === null)
		return null;

	return (
		<div className="grid grid-cols-5 gap-2">
			{data.presets.map((_, i) => {
				const isSelected = i === selectedIndex;
				return (
					<Button
						key={i}
						variant={isSelected ? "primary" : "secondary"}
						onPress={() => selectPreset(i)}
						size="sm"
					>
						프리셋 {i + 1}
					</Button>
				);
			})}
		</div>
	);
}
