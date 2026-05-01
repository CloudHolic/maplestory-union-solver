// Single Effect runtime for the page.

import { ManagedRuntime } from "effect";

import { Characters } from "./characters.ts";

const AppLayer = Characters.Default;

export const runtime = ManagedRuntime.make(AppLayer);
