import { Button } from "@heroui/react";
import { memo, useCallback} from "react";

import { useCharacterStore } from "@/state/characterStore.ts";

interface PresetTabsProps {
	isDisabled?: boolean;
}

/**
 * One button per preset. Clicking a button re-applies that preset's aggregation to the shape counts,
 * overwriting any manual edits.
 */
export function PresetTabs({ isDisabled = false }: PresetTabsProps) {
	const data = useCharacterStore(s => s.data);
	const selectedIndex = useCharacterStore(s => s.selectedPresetIndex);
	const selectPreset = useCharacterStore(s => s.selectPreset);

	if (data === null)
		return null;

	return (
		<div className="grid grid-cols-5 gap-2">
			{data.presets.map((_, i) => (
				<PresetButton
					key={i}
					index={i}
					selected={i === selectedIndex}
					isDisabled={isDisabled}
					onSelect={selectPreset}
				/>
			))}
		</div>
	);
}

interface PresetButtonProps {
	index: number;
	selected: boolean;
	isDisabled: boolean;

	onSelect: (index: number) => void;
}

function PresetButtonInner({ index, selected, isDisabled, onSelect }: PresetButtonProps) {
	const handlePress = useCallback(() => onSelect(index), [onSelect, index]);

	return (
		<Button
			variant={selected ? "primary" : "secondary"}
			onPress={handlePress}
			isDisabled={isDisabled}
			size="sm"
		>
			프리셋 {index + 1}
		</Button>
	);
}

const PresetButton = memo(PresetButtonInner);
