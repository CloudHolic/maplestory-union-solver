import { HttpClientResponse } from "@effect/platform";
import { Data, Effect, Schema } from "effect";

import { ApiClient } from "./client.ts";

// Wire schema - matches CharacterView in server/internal/characters

const Block = Schema.Struct({
	type: Schema.String,
	class: Schema.String,
	level: Schema.Number
});

export type Block = typeof Block.Type;

const CharacterResponse = Schema.Struct({
	nickname: Schema.String,
	ocid: Schema.String,
	presets: Schema.Array(Schema.Array(Block)),
	usePresetNo: Schema.Number,
	lastSelection: Schema.NullOr(Schema.String),
	lastSearchedAt: Schema.Number
});

export type CharacterData = typeof CharacterResponse.Type;

// Domain errors

export class CharacterNotFound extends Data.TaggedError("CharacterNotFound")<{
	readonly nickname: string;
}> {}

export class CharacterRateLimited extends Data.TaggedError("CharacterRateLimited")<Record<string, never>> {}

export class CharacterUpstreamUnavailable
	extends Data.TaggedError("CharacterUpstreamUnavailable")<Record<string, never>> {}

export class CharacterFetchFailed extends Data.TaggedError("CharacterFetchFailed")<{
	readonly reason: string;
}> {}

export type CharacterError =
	| CharacterNotFound
	| CharacterRateLimited
	| CharacterUpstreamUnavailable
	| CharacterFetchFailed;

// Service

/**
 * Fetches character data from the union-solve API, mapping wire-level failures
 * (HTTP status, network, JSON shape) into a closed set of domain errors.
 */
export class Characters extends Effect.Service<Characters>()("ui/Characters", {
	effect: Effect.gen(function*() {
		const client = yield* ApiClient;

		const fetchByNickname = (nickname: string) =>
			client.get(`/api/characters/${encodeURIComponent(nickname)}`).pipe(
				Effect.flatMap(HttpClientResponse.schemaBodyJson(CharacterResponse)),
				Effect.catchTags({
					ResponseError: err => {
						switch (err.response.status) {
							case 404:
								return Effect.fail(new CharacterNotFound({ nickname }));
							case 429:
								return Effect.fail(new CharacterRateLimited({}));
							case 503:
								return Effect.fail(new CharacterUpstreamUnavailable({}));
							default:
								return Effect.fail(new CharacterFetchFailed({
									reason: `HTTP ${err.response.status}`
								}));
						}
					},
					RequestError: err =>
						Effect.fail(new CharacterFetchFailed({
							reason: `network: ${err.message}`
						})),
					ParseError: err =>
						Effect.fail(new CharacterFetchFailed({
							reason: `decode: ${err.message}`
						}))
				})
			);

		return { fetchByNickname };
	}),
	dependencies: [ApiClient.Default]
}) {}
