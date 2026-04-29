import { Checkbox as HCheckbox } from "@heroui/react";
import type { ComponentProps } from "react";

export type CheckboxProps = ComponentProps<typeof HCheckbox>;

export function Checkbox(props: CheckboxProps) {
	return <HCheckbox {...props} />;
}
