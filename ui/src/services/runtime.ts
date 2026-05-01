// Single Effect runtime for the page.

import { Layer, ManagedRuntime } from "effect";

import { Characters } from "./characters.ts";
import { Selection } from "./selection.ts";
import { Solver } from "./solver.ts";

const AppLayer = Layer.mergeAll(Characters.Default, Selection.Default, Solver.Default);

export const runtime = ManagedRuntime.make(AppLayer);
