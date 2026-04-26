# Contract Migration Runbook

Concrete, ordered steps to move from "three repos with overlapping
hand-rolled types" to "three repos sharing one generated contract."

Companion to [PLAN.md](./PLAN.md) — that document is the *why*; this one
is the *do*. Each step has a single acceptance gate. Do not start step
N+1 until step N's gate is green.

Repos referenced (by local path):

- **C** = `~/dev/qua-media-contracts` (this repo, the contract)
- **U** = `~/dev/qua-media-ui` (React client + Rust mock server)
- **R** = `~/dev/qua-media-rs` (real backend: api, worker, fsm, stages…)

## Audit snapshot (as of v0.1.0)

| Repo | State                                                                                       |
| ---- | ------------------------------------------------------------------------------------------- |
| C    | Phase 0 ✅ — schemas + TS gen + Rust gen + validate green; **published to Cloudsmith @ `v0.1.0`** |
| U    | Phase 1 ✅ — client + mock server consume contracts; CI green; `VITE_WS_URL` swap point wired |
| R    | Phase 1 partial — workspace dep declared (`qua-media-contracts = { workspace = true }` in `crates/api`); **no WS handler yet**; CI green |

Phases 1a–1d are done. Phase 2 (qua-api emits contract messages) is the
next piece of work. Steps 5–8 below remain open.

## Ordering principle

**Compiler-driven before human-driven.** The Rust mock is the strictest
consumer of the contract — wire it up first so any naming, discriminator,
or shape mismatch surfaces as a build error before it can become a UI
runtime mystery.

**Lowest blast radius first.** The mock can be broken and fixed in
minutes; the real backend cannot. The UI client sits between them and
must not regress visually during migration.

**Reconcile before depending.** Do not add `qua-media-contracts` to R
until R's `qua-domain` and C's `schemas/v1/domain.schema.json` agree on
shapes. Adding the dep across a disagreement creates a permanent patch
treadmill.

---

## Step 1 — Phase 1b: wire the UI mock server onto the Rust contract crate

**Why first:** the mock is the canonical reference implementation of the
contract. If C's generated types don't match what the mock currently
emits, that's a contract bug — fix it here before any other repo
consumes the broken types.

1. In **U**: add to `server/Cargo.toml`:
   ```toml
   qua-media-contracts = { path = "../../qua-media-contracts/crates/qua-media-contracts" }
   ```
   Use a path dep during migration; switch to `git = ..., tag = ...`
   only after Step 4 tags `v0.1.0`.
2. Delete the inline `enum WsMessage` and `enum ClientMessage` from
   `server/src/main.rs` (the ~825–1200 line block).
3. Replace with `use qua_media_contracts::{ServerMessage as WsMessage, ClientMessage};`.
4. Build. Fix every compile error. Expected categories:
   - PascalCase variant renames (e.g. `WorkerHeartbeat` vs
     `worker_heartbeat`).
   - `type_:` discriminator field on generated structs.
   - Numeric type widths (`u32` vs `u64`) on ids/timestamps.
5. **Each compile error is a decision:** is the mock wrong (fix the
   mock) or is the schema wrong (fix `schemas/v1/**` in C, regenerate,
   try again)? Prefer fixing the schema if the mock's shape is the one
   the UI already depends on.

**Acceptance gate:**
- `cargo build --manifest-path ~/dev/qua-media-ui/server/Cargo.toml` clean.
- `tilt up` in **U** brings the mock up; the client receives a snapshot.
- `tilt trigger e2e` in **U** passes 8/8.
- WS frames captured during e2e are byte-identical (modulo field order)
  to the pre-migration baseline. Capture once before Step 1, diff after.

**Commit boundary:** one commit in **C** if any schema changed; one
commit in **U** for the mock migration. Do not bundle.

---

## Step 2 — Phase 1a: wire the UI client onto the TS contract package

**Why second:** the mock now provably emits the contract's exact shapes.
Switching the client to import the same generated types is purely
type-level — runtime behavior cannot change.

1. In **U** `client/package.json`:
   ```jsonc
   "dependencies": {
     "@qua/media-contracts": "file:../../qua-media-contracts/packages/ts"
   }
   ```
   Path dep during migration; swap to `github:…#v0.1.0` after Step 4.
2. Run `npm install` in `client/`.
3. Edit `client/src/types/index.ts`:
   - Re-export wire types from `@qua/media-contracts`:
     ```ts
     export type {
       ServerMessage as WsMessage,
       ClientMessage,
       Snapshot,
       Project,
       Episode,
       Take,
       Artifact,
       AttemptRecord,
       // …everything currently hand-rolled that has a schema match
     } from "@qua/media-contracts";
     ```
   - Keep UI-only types local: `EventLog`, `STAGE_ORDER`, `STATUS_META`,
     any derived `StageState.history` field, etc. These are not on the
     wire.
4. `npm run build` in `client/`. Fix any consumer call sites the new
   types break (likely: optional vs required fields, string-literal
   union narrowing).

**Acceptance gate:**
- `npm run build` clean.
- `tilt trigger e2e` 8/8 pass.
- No visual regression smoke-checked in the running app.

**Commit boundary:** one commit in **U**.

---

## Step 3 — Phase 1c: add the `VITE_WS_URL` swap point

**Why now:** trivial 5-line change, but locks in the protocol-neutral
contract that the client will use to point at R later. Do it while the
mock-only context is still simple.

1. In **U** `client/src/lib/`, locate the WS connect site (the file that
   constructs `new WebSocket(...)` — likely a hook or connection
   manager; grep for `WebSocket` or `3001`).
2. Replace the hardcoded URL with:
   ```ts
   const WS_URL = import.meta.env.VITE_WS_URL ?? "ws://localhost:3001/ws";
   ```
3. Add a typed entry to `client/src/vite-env.d.ts` if `ImportMetaEnv` is
   declared there.
4. Document in `client/README.md` that `VITE_WS_URL` overrides the
   default to point at any contract-conformant server.

**Acceptance gate:**
- `tilt trigger e2e` 8/8 pass with no env var set (default mock).
- Manually: `VITE_WS_URL=ws://localhost:3001/ws npm run dev` works
  identically — proves the env path is wired.

**Commit boundary:** one commit in **U**.

---

## Step 4 — Phase 1d: tag `v0.1.0` on contracts and UI

1. In **C**: `git tag v0.1.0 && git push --tags`.
2. In **U**: switch path deps to pinned git deps:
   - `server/Cargo.toml`:
     ```toml
     qua-media-contracts = { git = "ssh://git@github.com/<org>/qua-media-contracts", tag = "v0.1.0" }
     ```
   - `client/package.json`:
     ```jsonc
     "@qua/media-contracts": "github:<org>/qua-media-contracts#v0.1.0"
     ```
3. Re-run both acceptance gates from Steps 1 and 2.
4. In **U**: `git tag v0.1.0 && git push --tags`.

**Confirmation required before pushing tags or switching to git deps.**
Tags are durable and downstream consumers will pin against them.

**Commit boundary:** one commit in **C** (none, just a tag); one commit
in **U** for the dep-source swap.

---

## Step 5 — Reconciliation pass: qua-media-rs `qua-domain` vs the contract

**Why before Phase 2:** R already has its own schemas, validators, and
event payloads in `qua-domain`. Adding the contract crate as a dep on
top of an inconsistent type model creates two parallel universes inside
one repo. Resolve disagreements first, dep second.

1. Pick a small set of types to compare first — start with `Take`,
   `Snapshot`, `StageState`, `Artifact`, `WorkerHeartbeat`. These are
   the Phase 2 vertical-slice surface.
2. For each type, diff:
   - Field names (snake_case discipline)
   - Field types (id widths, timestamp encoding, enum variants)
   - Required vs optional (`#[serde(default)]` vs schema `required`)
   - Tagged-union discriminator (`type` field literal values)
3. For each disagreement, decide: **C wins** (the wire format is
   authoritative, R adapts) or **R wins** (R's shape is more correct,
   amend C's schema, regenerate, retag as `v0.2.0`).
4. Land the C-side schema fixes (if any) and tag a new patch/minor
   version. Re-run Steps 1–4 against the new tag in **U**.

**Acceptance gate:**
- A written reconciliation table (in `qua-media-rs/docs/CONTRACT.md`)
  enumerating each Phase 2 surface type and its decision.
- All Phase 2 surface types in C and R agree by name, type, and
  optionality.

**Commit boundary:** schema commits in **C** as needed; one doc commit
in **R**.

---

## Step 6 — Phase 2: qua-api implements the contract, vertical-slice

**Initial vertical slice landed** — commit `qua-media-pipeline@3ecf215`
adds [`crates/api/src/routes/ws.rs`](https://github.com/danielmgzzg/qua-media-pipeline/blob/main/crates/api/src/routes/ws.rs)
which mounts `GET /v1/ws` (outside `AuthLayer` during transition) and
bridges existing backend signals to contract `ServerMessage` frames:

- replays the last ~200 `workflow_events` rows on connect, then polls
  every ~750 ms; translates `stage_started` and `stage_finished`
  events into `StageStarted` / `StageFinished` contract frames;
- samples `WorkerRegistry` every ~2 s and emits one `WorkerHeartbeat`
  per known worker (cpu/memory placeholders, status flips alive/stale);
- validates every outbound frame against the bundled server schema in
  debug builds via `qua_media_contracts::validate::server_message`
  and drops drifted frames with a warning rather than panicking.

**Per stage**, in the order listed in [PLAN.md](./PLAN.md#phase-2--backend-builds-against-contracts-qua-media-rs):

1. In **R**: add `qua-media-contracts = { git = ..., tag = "vX.Y.Z" }`
   to `crates/api/Cargo.toml` (and any worker crate that emits events).
2. Implement the WS handler that emits `ServerMessage::Snapshot`,
   `StateChange`, `WorkerHeartbeat` for that stage.
3. Bridge real FSM events → contract messages. Where `qua-domain`
   types and contract types diverge, the bridge layer is the only
   adapter — keep it thin and one-directional.
4. **Validation in dev:** wire the `validate` feature of the contract
   crate (or a one-shot `serde_json::Value` round-trip through the JSON
   schema) into a `#[cfg(debug_assertions)]` middleware on the WS
   handler. Log on mismatch, never panic.
5. Manually point **U**'s client at this stage's running backend:
   `VITE_WS_URL=ws://localhost:8080/v1/ws npm run dev`. UI shows real
   data for the implemented stage; other stages remain pending.

**Acceptance gate per stage:**
- Backend integration test in **R** captures every WS frame for a
  scripted scenario and validates each against the contract schema.
- Manual end-to-end with **U** pointing at **R** works for that stage.

**Commit boundary:** one commit per stage in **R**, no bundling.

---

## Step 7 — Phase 3: switch the UI default to the real backend

Once enough stages exist in **R** that local dev against the real backend
is preferable to dev against the mock:

1. In **U** `client/src/lib/<ws-file>.ts`, change the default URL to
   point at the real backend port. The `VITE_WS_URL` override remains;
   developers wanting the mock set `VITE_WS_URL=ws://localhost:3001/ws`.
2. Document the inversion in `client/README.md` and
   `qua-media-ui/docs/DEVELOPMENT.md`.

**Acceptance gate:**
- `tilt up` in **U** + `tilt up` in **R** + e2e green against R.
- Mock still works when env override is set.

**Commit boundary:** one commit in **U**.

---

## Step 8 — Phase 4: promote the mock to a contract conformance harness

1. In **U**: add a Playwright (or vitest) test that connects to a
   configured `CONTRACT_TARGET_WS_URL` (defaults to mock; CI overrides
   to staging R), captures every WS frame, and validates each against
   the schema bundled in `@qua/media-contracts/schemas/v1/`.
2. Wire this test into **R**'s CI as a contract regression suite:
   nightly + on every backend PR.
3. The mock keeps living in **U** for offline UI dev — no Postgres, no
   Kafka, no FFmpeg required for CSS work.

**Acceptance gate:**
- Test passes against the mock (proves the harness works).
- Test passes against staging **R** (proves no drift).
- Failure mode demonstrated: introduce a deliberate schema violation in
  a throwaway branch of **R** and confirm the test fails.

**Commit boundary:** one commit in **U** (harness), one in **R** (CI
wiring).

---

## Status checklist

| Step | Description                                       | Status |
| ---- | ------------------------------------------------- | ------ |
| 1    | Mock server onto contract crate (Phase 1b)        | ✅ done |
| 2    | Client onto contract package (Phase 1a)           | ✅ done |
| 3    | `VITE_WS_URL` swap point (Phase 1c)               | ✅ done |
| 4    | Tag `v0.1.0` on C and U (Phase 1d)                | ✅ done — published to Cloudsmith |
| 5    | Reconcile `qua-domain` vs contract                | 🟡 de-facto done — reconciliation has happened per-bridge inline (see Phase 2 commits): `ScriptRole::Detail` → `practical`, `Decision::Reject` dropped, `Approved`/`Escalated` run-status folded into `Completed`/`Failed`. The standalone `qua-media-rs/docs/CONTRACT.md` table called for in the original plan was never written; remaining gaps are tracked in [PLAN.md Phase 2.5](./PLAN.md#phase-25--cas-reader-for-rich-payloads--done-reachable-scope) instead. |
| 6    | qua-api emits contract, per-stage (Phase 2)       | ✅ done (DB-reach + CAS-reach) — `/v1/ws[?run_id=]` slice landed (StageStarted/Finished/ApprovalRecorded/ArtifactPublished/RunCompleted/WorkerHeartbeat + Snapshot on connect); per-stage payloads bridged: `semantic_frontend` (take_set + episode), `asset_catalog.artifacts` (covers all stages), `campaign`+`project`, `review_queue`, `system_health`, `timeline` (from CAS), `timeline.subtitles` (from CAS). Remaining stubbed Snapshot fields are blocked on upstream pipeline work — see [PLAN.md Phase 2.5](./PLAN.md#phase-25--cas-reader-for-rich-payloads--done-reachable-scope). |
| 7    | UI default flips to real backend (Phase 3)        | ⬅ next |
| 8    | Mock becomes conformance harness (Phase 4)        |        |

## What was added beyond the original plan

- **Cloudsmith publishing** (`quadricular/qua-media`): both packages
  published on every `v*` tag via `.github/workflows/publish.yml`.
- **Multi-checkout CI pattern**: U and R workflows check out
  `qua-media-contracts` as a sibling directory so `path = "..."` deps
  resolve identically locally and in CI. `qua-media-contracts` is a
  **public** GitHub repo so this checkout works without a PAT.
- **`build.rs` writes to `OUT_DIR`** (not `src/generated.rs`) so the
  crate is publishable to a registry without uncommitted changes; the
  bundled copy of `schemas/v1/` lives under
  `crates/qua-media-contracts/schemas/` for registry installs.
- **TypeScript `dist/` step in CI**: `make gen-ts` is followed by
  `npx tsc -p tsconfig.json` so `@qua/media-contracts/dist/index.d.ts`
  exists before the client `npm ci` resolves types.
- **Cross-file `$ref` resolution in the `validate` feature**: `lib.rs`
  registers `domain.schema.json` via
  `JSONSchema::options().with_document(id, value)` so messages with
  refs like `CropApplied.crop → ../domain.schema.json#/$defs/CropRegion`
  validate correctly. Without this the mock server panicked on any
  message that referenced a domain type.
- **Postgres service in qua-media-rs CI** + libcurl4-openssl-dev for
  rdkafka cmake-build: the `sqlx::query!` macros need a live
  `DATABASE_URL` at compile time.
