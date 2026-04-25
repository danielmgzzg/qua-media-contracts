# qua-media-contracts (Rust)

Generated, serde-compatible Rust types for the Qua media pipeline wire
protocol. Both the mock server in `qua-media-pipeline-ui/server` and the
real backend in `qua-media-rs` depend on this crate.

## Build

```sh
cargo build  # build.rs regenerates src/generated.rs from ../../schemas/v1
```

`src/generated.rs` is intentionally gitignored — it is rebuilt
deterministically from the schemas on every `cargo build`. There is no
"check committed output for drift" step because the output isn't
committed.

## Use

```rust
use qua_media_contracts::{ServerMessage, ClientMessage};

fn handle(msg: ServerMessage) {
    match msg {
        ServerMessage::Snapshot(snap) => { /* ... */ }
        ServerMessage::WorkerHeartbeat(hb) => { /* ... */ }
        _ => {}
    }
}
```

## Runtime validation (optional)

```toml
[dev-dependencies]
qua-media-contracts = { path = "...", features = ["validate"] }
```

```rust
#[cfg(feature = "validate")]
qua_media_contracts::validate::server_message(&json_value)?;
```

## Adding a message type

1. Edit `schemas/v1/**` (the canonical source).
2. `cargo build` — typify regenerates the Rust types.
3. Match on the new variant where needed; the compiler tells you what
   to update.
