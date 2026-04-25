# Qua Media Contracts — Plan & Roadmap

The architectural plan that motivated this repo, plus the phased migration
toward a real backend (`qua-media-rs`) sharing the wire contract with the
mock that lives in `qua-media-pipeline-ui/server`.

---

## The core problem

The UI repo iterates fast against a mock that produces convenient data. The
backend iterates against real concerns: Postgres schemas, Kafka transactions,
FFmpeg edge cases. **If they share nothing, they drift. If they share
everything, they collapse into one repo.**

The answer is a thin shared contract that both repos depend on, plus a swap
point that's smaller than you think.

## The architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   qua-media-contracts                        │
│  (separate repo, single source of truth)                     │
│                                                              │
│  - WebSocket message schemas (JSON Schema)                   │
│  - HTTP API OpenAPI spec                                     │
│  - Stage event contracts                                     │
│  - TypeScript types (generated)                              │
│  - Rust types (generated, serde-compatible)                  │
└─────────────┬─────────────────────────┬─────────────────────┘
              │                         │
              ▼                         ▼
┌──────────────────────────┐  ┌──────────────────────────────┐
│    qua-media-pipeline-ui │  │      qua-media-rs            │
│                          │  │                              │
│  client/ (React)         │  │  qua-api/                    │
│  ├─ uses contracts/ts    │  │  ├─ uses contracts/rust      │
│  └─ talks to ANY server  │  │  ├─ serves real WS           │
│                          │  │  └─ same shapes as mock      │
│  server/ (Rust mock)     │  │                              │
│  ├─ uses contracts/rust  │  │  qua-worker, qua-fsm, etc.   │
│  └─ produces fake data   │  │                              │
└──────────────────────────┘  └──────────────────────────────┘
```

The mock server lives in the UI repo. The real server lives in the backend
repo. **Both implement the same contract. The frontend doesn't know which
one it's talking to.**

---

## What goes in this repo

Three artifacts, all generated from one source of truth.

### 1. JSON Schema as the source of truth (`schemas/v1/`)

Each WebSocket message type and HTTP endpoint is a JSON Schema document.
Humans edit these; everything else is generated.

```
schemas/v1/
├── domain.schema.json       # all reusable domain types as $defs
├── ws/
│   ├── server.schema.json   # ServerMessage tagged union (oneOf on `type`)
│   └── client.schema.json   # ClientMessage tagged union (oneOf on `type`)
├── http/
│   └── openapi.yaml         # HTTP API surface
└── examples/
    ├── server/*.json        # conformance fixtures, run by `make validate`
    └── client/*.json
```

> **Original brief listed split files** (`snapshot.schema.json`,
> `state_change.schema.json`, etc.). The implementation merges them into one
> tagged-union document per direction (`ws/server.schema.json`,
> `ws/client.schema.json`) with each variant defined under `$defs/`. This
> makes the discriminator (`type`) literal-checkable in both `oneOf` and
> typify, and it removes a pile of cross-file `$ref`s. Adding a new message
> is a `$defs` entry plus a `oneOf` entry — same friction as a new file.

### 2. Generated TypeScript types (`packages/ts/`)

`json-schema-to-typescript` produces a single `src/index.ts` with every
type. Published as `@qua/media-contracts` (for now: git dep; later: private
npm registry).

### 3. Generated Rust types (`crates/qua-media-contracts/`)

`build.rs` uses [`typify`] to produce structs and tagged enums with
`#[derive(Serialize, Deserialize)]`. `src/generated.rs` is gitignored and
regenerated deterministically every `cargo build`. Published as
`qua-media-contracts` (for now: git dep; later: private cargo registry).

Both the mock server and the real backend depend on the Rust crate. Both
React clients depend on the TS package. **Schema changes break both at
compile time — that's the point.**

---

## Migration path

### Phase 0 — Extract contracts ✅ DONE

Bootstrap this repo. Move every WebSocket message shape into JSON Schema.
Set up codegen for TS + Rust.

**Status:** complete. Both pipelines verified end-to-end.

- 29 server message variants, 10 client message variants, ~60 domain types
- TS gen + validate + tsc all green; cargo build green (~22k LOC generated)
- 4 example messages validate against their schemas in CI

This phase was **risk-free and worth doing immediately** because it forced
formalization of what was previously implicit.

### Phase 1 — Wire contracts into the UI repo ⬅ NEXT

The UI repo depends on this contract; nothing observable changes.

#### 1a. Client (TypeScript)

```jsonc
// client/package.json
"dependencies": {
  "@qua/media-contracts": "github:danielmgzzg/qua-media-contracts#v0.1.0"
}
```

Replace the hand-rolled types in `client/src/types/index.ts` with re-exports
from `@qua/media-contracts`. Keep UI-only types (`EventLog`, `STAGE_ORDER`,
`STATUS_META`, derived `StageState.history`) local — they aren't on the wire.

```ts
// client/src/types/index.ts (after)
export type {
  WsMessage,
  ClientMessage,
  Snapshot,
  Project,
  Episode,
  // ... etc
} from "@qua/media-contracts";

// UI-only additions stay here:
export interface EventLog { /* ... */ }
export const STAGE_ORDER = [ /* ... */ ] as const;
```

**Acceptance gate:** `npm run build` clean, `tilt trigger e2e` 8/8 pass.

#### 1b. Server (Rust mock)

```toml
# server/Cargo.toml
[dependencies]
qua-media-contracts = { git = "ssh://git@github.com/danielmgzzg/qua-media-contracts", tag = "v0.1.0" }
```

Delete the inline `enum WsMessage` and `enum ClientMessage` from
`server/src/main.rs` (lines ~825–1200). Replace with:

```rust
use qua_media_contracts::{ServerMessage as WsMessage, ClientMessage};
```

**Watch-outs:**

- Generated variant names use PascalCase from the schema's `$defs` keys
  (e.g. `ServerMessage::WorkerHeartbeat`). Match arms may need renaming.
- Generated structs have a `pub type_:` field with `#[serde(rename = "type")]`
  for the discriminator. Constructors that fabricated `WsMessage::Foo { ... }`
  variant-style still work because typify emits tuple-variant enums.
- The `take_id`/`run_id`/`timestamp` field types may differ slightly from
  the inline ones (`u32` vs `u64`, etc.). Adjust call sites — the compiler
  tells you what.

**Acceptance gate:** `cargo build --manifest-path server/Cargo.toml` clean,
`tilt up` snapshot reaches the client, `tilt trigger e2e` 8/8 pass.

#### 1c. Add the swap point

```ts
// client/src/lib/ws.ts
const WS_URL = import.meta.env.VITE_WS_URL || "ws://localhost:3001/ws";
```

`VITE_WS_URL=ws://localhost:8080/v1/ws` will later point at the real
backend. No-op today; trivial 5-line change. Tag this `v0.1.0` of the UI
once 1a+1b+1c land.

#### 1d. Tag the contract `v0.1.0`

Tag here, then tag UI repo with the matching contract version pinned.
From this point forward, **every contract change ships as a new tag**;
neither downstream repo can reference an unpinned message shape.

### Phase 2 — Backend builds against contracts (qua-media-rs)

The backend repo adds the same Cargo dep. The `qua-api` crate's WebSocket
handler implements the same `ServerMessage` enum the mock implements. As
stages get built (`semantic_frontend` first), they emit real events using
the same shapes the mock fabricates.

**Critical insight: vertical migration, not horizontal.** The backend
doesn't need feature parity to be useful. It can implement only:

- `snapshot`
- `state_change` / `stage_started` / `stage_finished`
- `worker_heartbeat`

…and still be testable end-to-end with the UI. Stages not yet implemented
send no events; UI shows them as pending. Migrate **one stage at a time**,
not all messages then no stages.

Suggested order (matches the pipeline):

1. `semantic_frontend` — script ingest → snapshot + script blocks
2. `extract_and_preview` — take ingest → takes + thumbnails
3. `audio_alignment` → alignment_attempt + alignment_edit_proposed
4. `subtitles` → subtitle cues
5. `compositor` → compositor_chroma_key, _background_replace, _preview
6. `eye_alignment` → face_detected, eye_line_analysis, alignment_applied
7. `color_grade` → color_analysis, look_match_computed, grade_applied
8. `audio_master` → loudness_measured, noise_profile, master_applied
9. `export` → render_started, render_progress, render_completed, export_summary
10. `qa` → qa state in snapshot only

### Phase 3 — The swap

Already wired in Phase 1c. Set `VITE_WS_URL=ws://localhost:8080/v1/ws`.
Run both simultaneously during transition: mock on `:3001` for UI iteration,
real backend on `:8080` for end-to-end testing. Switch via env var.

### Phase 4 — Mock becomes a contract test fixture

Once the real backend implements all messages, **don't delete the mock —
promote it to a contract conformance harness.** Two purposes:

1. **Offline UI development:** Frontend continues to develop against the
   mock for visual concerns (no Postgres + Kafka stack required for CSS work).
2. **Contract regression suite:** Test that connects to the real backend
   and asserts its messages conform to the same schemas the mock produces.
   Run against staging on every backend PR.

---

## How drift is prevented

### 1. Schema-first PRs

Adding a new WS message starts with a PR **here** (qua-media-contracts).
That PR generates new TS + Rust types. Until those types are tagged,
neither downstream repo can reference the new message. **The contract
leads.**

### 2. Schema validation at runtime in dev

Both the UI client and the mock server run every outbound + inbound
message through Ajv (TS) or `jsonschema` (Rust) and log a warning on
mismatch. Production strips this. Drift is caught the first time the
dev server runs.

**TS dev-mode wiring (sketch):**

```ts
import Ajv2020 from "ajv/dist/2020.js";
import serverSchema from "@qua/media-contracts/schemas/v1/ws/server.schema.json";

const ajv = new Ajv2020({ strict: false });
const validate = ajv.compile(serverSchema);

ws.addEventListener("message", (ev) => {
  const data = JSON.parse(ev.data);
  if (import.meta.env.DEV && !validate(data)) {
    console.warn("[contract drift]", validate.errors, data);
  }
  // ...
});
```

**Rust dev-mode wiring (sketch):**

```rust
#[cfg(debug_assertions)]
{
    let v: serde_json::Value = serde_json::from_str(&frame_text)?;
    if let Err(errs) = qua_media_contracts::validate::server_message(&v) {
        eprintln!("[contract drift] {errs:?}");
    }
}
```

### 3. Contract tests in CI for both repos

- **UI repo CI:** spin up the mock, run a Playwright test that captures
  every WS frame, validate each against its schema. Fails if mock drifts.
- **Backend repo CI:** spin up `qua-api` against a test database, replay
  a scripted scenario, capture every WS frame, validate each. Fails if
  backend drifts.

Both repos validate independently against the same source of truth.

---

## Why this works for AI-assisted iteration

- **UI agent has a complete runnable system locally.** Mock + real-shaped
  data + full WebSocket flow. Iterates on visual design, interactions,
  layouts without touching backend code or running Postgres. Fast loop.
- **Backend agent has a complete testable contract.** Knows exactly what
  messages to emit, what shapes the API must serve. No React knowledge
  required.
- **The contract repo is the only coordination point.** A new feature is
  a 3-PR sequence: contract → backend → UI (or contract → UI → backend).
  Each agent only needs one PR's worth of context.
- **Schema-driven prompts work better.** "Implement the `compositor_adjust`
  handler — see `schemas/v1/ws/client.schema.json#/$defs/CompositorAdjust`
  for the input shape and `server.schema.json#/$defs/CompositorPreview` for
  the response." The contract IS the spec. No ambiguity.

---

## What stays in each repo

### qua-media-contracts (this repo)

- JSON Schemas (`schemas/v1/**`)
- OpenAPI spec (`schemas/v1/http/openapi.yaml`)
- Codegen config (`packages/ts/scripts/`, `crates/qua-media-contracts/build.rs`)
- Type packages (TS, Rust)
- Schema validation (`packages/ts/scripts/validate.mjs`, optional Rust
  `validate` feature)
- Documentation: "what does message X mean semantically" (this folder)

### qua-media-pipeline-ui

- React client (`client/`) — uses `@qua/media-contracts`
- Mock server (`server/`) — uses `qua-media-contracts` Rust crate
- Visual design system, components
- E2E tests against the mock
- Storybook for components
- Contract conformance test that connects to a real backend URL when one
  is provided (Phase 4)

### qua-media-rs (future)

- `qua-api`, `qua-worker`, `qua-fsm`, etc.
- Real Postgres + Kafka + FFmpeg integrations
- All depend on `qua-media-contracts` Rust crate
- Backend conformance test in CI (Phase 2 onward)

---

## Versioning

`schemas/v1/` is the v1 wire format. Breaking changes get a new directory
(`schemas/v2/`) and a new generated module/package alongside v1. Generated
artifacts are tagged together (`v1.x.y`) so a single git tag pins both
TS and Rust consumers.

Today: `v0.x.y` — pre-1.0, the contract is allowed to break with each tag
during initial migration. Promote to `v1.0.0` once Phase 1 lands and the
backend has implemented at least the snapshot + state_change + heartbeat
core (start of Phase 2).

---

## Status

| Phase | Description                                              | Status |
| ----- | -------------------------------------------------------- | ------ |
| 0     | Extract contracts (this repo)                            | ✅ done |
| 1a    | Wire contracts into UI client                            | ⬅ next |
| 1b    | Wire contracts into UI mock server                       |        |
| 1c    | Add `VITE_WS_URL` swap point                             |        |
| 1d    | Tag `v0.1.0` on both repos                               |        |
| 2     | Real backend implements core, then per-stage             |        |
| 3     | Switch UI to real backend via env var                    |        |
| 4     | Mock becomes contract conformance harness                |        |

[`typify`]: https://crates.io/crates/typify
