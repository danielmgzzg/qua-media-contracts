//! Generated Rust types for the Qua media pipeline wire protocol.
//!
//! Types are generated at build time from `../../schemas/v1/**` by
//! `build.rs` using [`typify`]. The output lives in `src/generated.rs`
//! (gitignored) and is `include!`d here.
//!
//! ## Usage
//!
//! ```ignore
//! use qua_media_contracts::{ServerMessage, ClientMessage};
//!
//! fn handle(msg: ServerMessage) {
//!     match msg {
//!         ServerMessage::Snapshot(snap) => { /* ... */ }
//!         ServerMessage::WorkerHeartbeat(hb) => { /* ... */ }
//!         _ => {}
//!     }
//! }
//! ```
//!
//! ## Optional runtime validation
//!
//! Enable the `validate` feature in dev/test builds to validate raw JSON
//! frames against the source schemas:
//!
//! ```ignore
//! #[cfg(feature = "validate")]
//! qua_media_contracts::validate::server_message(&json_value)?;
//! ```

#[allow(clippy::all, dead_code, non_camel_case_types, unused_imports)]
mod generated {
    include!("generated.rs");
}
pub use generated::*;

#[cfg(feature = "validate")]
pub mod validate {
    use jsonschema::JSONSchema;
    use serde_json::Value;
    use std::sync::OnceLock;

    static SERVER_SCHEMA_SRC: &str =
        include_str!("../../../schemas/v1/ws/server.schema.json");
    static CLIENT_SCHEMA_SRC: &str =
        include_str!("../../../schemas/v1/ws/client.schema.json");
    static DOMAIN_SCHEMA_SRC: &str =
        include_str!("../../../schemas/v1/domain.schema.json");

    fn server() -> &'static JSONSchema {
        static C: OnceLock<JSONSchema> = OnceLock::new();
        C.get_or_init(|| compile(SERVER_SCHEMA_SRC, "server"))
    }
    fn client() -> &'static JSONSchema {
        static C: OnceLock<JSONSchema> = OnceLock::new();
        C.get_or_init(|| compile(CLIENT_SCHEMA_SRC, "client"))
    }

    fn compile(src: &str, label: &str) -> JSONSchema {
        // Note: cross-file $refs to ../domain.schema.json are intentionally
        // unresolved here; for full validation in dev, bundle the three
        // schemas with a $RefParser-equivalent step before calling these.
        // The DOMAIN_SCHEMA_SRC is included so a future bundling helper has
        // it available without an extra fs read.
        let _ = DOMAIN_SCHEMA_SRC;
        let value: Value = serde_json::from_str(src)
            .unwrap_or_else(|e| panic!("parse {label} schema: {e}"));
        JSONSchema::compile(&value)
            .unwrap_or_else(|e| panic!("compile {label} schema: {e}"))
    }

    pub fn server_message(value: &Value) -> Result<(), Vec<String>> {
        let result = server().validate(value);
        match result {
            Ok(()) => Ok(()),
            Err(errs) => Err(errs.map(|e| e.to_string()).collect()),
        }
    }

    pub fn client_message(value: &Value) -> Result<(), Vec<String>> {
        let result = client().validate(value);
        match result {
            Ok(()) => Ok(()),
            Err(errs) => Err(errs.map(|e| e.to_string()).collect()),
        }
    }
}
