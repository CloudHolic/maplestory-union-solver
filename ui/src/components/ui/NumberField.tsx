import { NumberField as HNumberField } from "@heroui/react";
import type { ComponentProps } from "react";

export type NumberFieldProps = ComponentProps<typeof HNumberField>;

export function NumberField(props: NumberFieldProps) {
	return (
		<HNumberField {...props}>
			<HNumberField.Group>
				<HNumberField.DecrementButton />
				<HNumberField.Input />
				<HNumberField.IncrementButton />
			</HNumberField.Group>
		</HNumberField>
	);
}
