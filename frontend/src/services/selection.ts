// Effect.Service for the PUT /api/characters/:nickname/selection endpoint.

import { HttpBody, HttpClientRequest } from "@effect/platform";
import { Data, Effect } from "effect";

import { ApiClient } from "@/services/client.ts";

// Domain errors

export class SelectionCharacterNotFound extends Data.TaggedError("SelectionCharacterNotFound")<{
	readonly nickname: string;
}> {}

export class SelectionTooLarge extends Data.TaggedError("SelectionTooLarge")<Record<string, never>> {}

export class SelectionRateLimited extends Data.TaggedError("SelectionRateLimited")<Record<string, never>> {}

export class SelectionFailed extends Data.TaggedError("SelectionFailed")<{
	readonly reason: string;
}> {}

export type SelectionError =
	| SelectionCharacterNotFound
	| SelectionTooLarge
	| SelectionRateLimited
	| SelectionFailed;

// Service

export class Selection extends Effect.Service<Selection>()("ui/selection", {
	effect: Effect.gen(function*() {
		const client = yield* ApiClient;

		const saveSelection = (nickname: string, selection: unknown) => {
			const request = HttpClientRequest.put(
				`/api/characters/${encodeURIComponent(nickname)}/selection`
			).pipe(
				HttpClientRequest.setBody(
					HttpBody.text(JSON.stringify(selection), "application/json")
				)
			);

			return client.execute(request).pipe(
				Effect.asVoid,
				Effect.catchTags({
					ResponseError: err => {
						switch (err.response.status) {
							case 404:
								return Effect.fail(new SelectionCharacterNotFound({ nickname }));
							case 413:
								return Effect.fail(new SelectionTooLarge({}));
							case 429:
								return Effect.fail(new SelectionRateLimited({}));
							default:
								return Effect.fail(new SelectionFailed({
									reason: `HTTP ${err.response.status}`
								}));
						}
					},
					RequestError: err =>
						Effect.fail(new SelectionFailed({
							reason: `network: ${err.message}`
						}))
				})
			);
		};

		return { saveSelection };
	}),
	dependencies: [ApiClient.Default]
}) {}
