import { CloseButton as HCloseButton } from "@heroui/react";
import type { ComponentProps } from "react";

export type CloseButtonProps = ComponentProps<typeof HCloseButton>;

export function CloseButton(props: CloseButtonProps) {
	return <HCloseButton {...props} />;
}
