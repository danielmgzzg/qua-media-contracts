# qua-media-contracts

Single source of truth for the wire contract between the Qua media pipeline
UI (`qua-media-pipeline-ui`) and the Rust backend (`qua-media-rs`).

## Why this exists

The UI repo iterates fast against a mock that produces convenient data. The
backend iterates against real concerns: Postgres, Kafka, FFmpeg edge cases.
If they share nothing, they drift. If they share everything, they collapse
into one repo.

This repo is the **thin shared contract** both sides depend on, and it is the
only place where coordination is required to evolve the protocol.

## Layout

```
schemas/v1/
  domain.schema.json       # all domain types as $defs (Take, StageState, ...)
  ws/
    server.schema.json     # server -> client WsMessage (tagged union)
    client.schema.json     # client -> server ClientMessage (tagged union)
  http/
    openapi.yaml           # HTTP API (currently: /health only)

packages/ts/               # @qua/media-contracts (TypeScript codegen)
crates/qua-media-contracts/ # qua-media-contracts (Rust codegen, serde + JsonSchema)
```

JSON Schema (draft 2020-12) is the canonical definition. Humans edit
`schemas/v1/**`. Everything else is generated.

## Codegen

```sh
make gen      # regenerate both TS and Rust types from schemas
make gen-ts   # TS only -> packages/ts/src/index.ts
make gen-rust # Rust only -> crates/qua-media-contracts/src/generated.rs
make validate # JSON Schema lint + sample-message conformance
```

The TS pipeline uses [`json-schema-to-typescript`]. The Rust pipeline uses
[`typify`] in a `build.rs` so the generated module is reproducible from
`schemas/v1/` at crate build time — no committed Rust output drift.

## Consuming this from each repo

### qua-media-pipeline-ui (TypeScript client + Rust mock)

```jsonc
// client/package.json
"dependencies": {
  "@qua/media-contracts": "github:quadricular/qua-media-contracts#v0.1.0"
}
```

```toml
# server/Cargo.toml
[dependencies]
qua-media-contracts = { git = "https://github.com/quadricular/qua-media-contracts", tag = "v0.1.0" }
```

The client replaces hand-rolled types in `client/src/types/index.ts` with
`import type { WsMessage, ClientMessage } from "@qua/media-contracts"`. The
mock server replaces the inline `enum WsMessage` in `server/src/main.rs` with
`use qua_media_contracts::WsMessage`.

### qua-media-rs (real backend)

Same Cargo dep as the mock. The `qua-api` crate's WebSocket handler
implements the same `WsMessage` enum the mock implements. Stages emit real
events using the same shapes.

## The swap point

In `client/src/lib/ws.ts` (or wherever the WS URL is set):

```ts
const WS_URL = import.meta.env.VITE_WS_URL || "ws://localhost:3001/ws";
```

`VITE_WS_URL=ws://localhost:8080/v1/ws` points the client at the real backend.
Both servers speak the same protocol; the client doesn't care which answers.

## Drift guardrails

1. **Schema-first PRs.** New WS messages start as a PR here. Until tagged,
   neither downstream repo can reference them.
2. **Runtime validation in dev.** Both sides validate inbound + outbound
   frames against the published schemas (Ajv on the TS side, `jsonschema`
   on Rust). Production strips this.
3. **Conformance tests in CI** of both downstream repos:
   - UI repo: spin up the mock, capture every WS frame, validate.
   - Backend repo: spin up `qua-api` against test DB, replay scripted
     scenario, validate every frame.
   Both run against the same schema source.

## Versioning

`schemas/v1/` is the v1 wire format. Breaking changes get a new directory
(`schemas/v2/`) and a new generated module/package alongside v1. Generated
artifacts are tagged together (`v1.x.y`) so a single git tag pins both
TS and Rust consumers.

## Phase 0 status

This repo currently mirrors the message catalog implemented by the mock in
`qua-media-pipeline-ui/server/src/main.rs` as of the v0 extraction. Both
downstream repos still hand-roll their types — switching them over to
import from this contract is the next migration step (Phase 1).
