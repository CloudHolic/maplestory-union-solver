# maplestory-union-solver/server

Go backend for the MapleStory Union Solver. Proxies the NEXON
Open API, caches character data, and will record solve runs.

## Requirements

- Go 1.26.2
- A NEXON Open API key — register at
  https://openapi.nexon.com (region: KMS)

## Quick start

```powershell
cd server
copy .env.example .env
# edit .env and set NEXON_API_KEY

go run ./cmd/server
```

The server listens on `:8888` by default. SQLite database file
is created at `./data/union.db` on first run.

## Configuration

All settings are environment variables, optionally sourced from
a `.env` file in the working directory. OS env vars override
`.env` entries.

| Variable                | Required | Default                          | Notes                                          |
|-------------------------|----------|----------------------------------|------------------------------------------------|
| `NEXON_API_KEY`         | yes      | —                                | NEXON Open API key                             |
| `SERVER_ADDR`           | no       | `:8888`                          | listen address                                 |
| `DATABASE_URL`          | no       | `file:./data/union.db?_pragma=…` | modernc.org/sqlite DSN                         |
| `RATE_LIMIT_PER_MINUTE` | no       | `30`                             | per-IP cap on `/api`; burst is `rate/3`        |
| `TRUSTED_PROXIES`       | no       | _(empty)_                        | reserved for slice 6 (XFF header validation)   |
| `LOG_LEVEL`             | no       | `info`                           | `debug` / `info` / `warn` / `error`            |
| `LOG_FORMAT`            | no       | `json`                           | `json` for prod, `text` for human reading      |

## Endpoints

### `GET /api/characters/:nickname`

Resolves a MapleStory nickname to its character data.

```bash
curl http://localhost:8080/api/characters/{character-name}
```

200 response:
```json
{
  "nickname": "{character-name}",
  "ocid": "abc123...",
  "presets": [
    [{"type": "마법사", "class": "에반", "level": 290}, ...],
    [], [], [], []
  ],
  "usePresetNo": 1,
  "lastSelection": null,
  "lastSearchedAt": 1735000000
}
```

`presets` is always an array of length 5 (NEXON's preset slot
count). Empty preset slots are present as `[]`.

Error statuses:
- `400` — empty nickname
- `404` — NEXON could not resolve the nickname
- `429` — per-IP rate limit exceeded
- `503` — NEXON throttled or in maintenance
- `500` — server / NEXON config issue

## Building

Development:
```powershell
go run ./cmd/server
```

Production binary:
```powershell
go build -o bin/server.exe ./cmd/server
```

The binary is statically linked (cgo disabled — pure-Go SQLite
driver), so it can run on a `scratch` Docker base image without
glibc/musl dependencies.

## Project layout

```
cmd/server/main.go        entry: config → DB → Echo → listen
internal/
  config/                 env + .env loader
  db/                     sqlx open, embedded migrations, WAL
    migrations/0001_init.sql
  nexon/                  typed HTTP client + extraction
  characters/             repository / service / handler
  httpsrv/                Echo wiring, middleware, routing
```

## License

MIT. See the repository root `LICENSE` file.