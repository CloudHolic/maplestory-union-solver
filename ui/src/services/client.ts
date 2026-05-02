import { FetchHttpClient, HttpClient, HttpClientRequest } from "@effect/platform";
import { Effect } from "effect";

const BASE_URL = import.meta.env.VITE_API_BASE_URL ?? "http://localhost:8888";

/** Configured HTTP client for the union-solver API server. */
export class ApiClient extends Effect.Service<ApiClient>()("ui/ApiClient", {
	effect: Effect.gen(function*() {
		const base = yield* HttpClient.HttpClient;
		return base.pipe(
			HttpClient.mapRequest(HttpClientRequest.prependUrl(BASE_URL)),
			HttpClient.mapRequest(HttpClientRequest.acceptJson),
			HttpClient.filterStatusOk
		);
	}),
	dependencies: [FetchHttpClient.layer]
}) {}
