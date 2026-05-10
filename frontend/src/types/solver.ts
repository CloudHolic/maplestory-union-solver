// Solver outcome types.

export type SolverOutcome = "success" | "no_solution" | "error" | "cancelled";

export interface OutcomePayload {
	outcome: SolverOutcome;
	errorMessage: string | null;
}
