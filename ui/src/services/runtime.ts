// Single Effect runtime for the page.

import { Layer, ManagedRuntime } from "effect";

import { Characters } from "./characters.ts";
import { Solver } from "./solver.ts";

const AppLayer = Layer.mergeAll(Characters.Default, Solver.Default);

export const runtime = ManagedRuntime.make(AppLayer);
