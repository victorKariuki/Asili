//! `pata-dap`: minimum viable Debug Adapter Protocol server for Asili — see
//! `docs/design/pata-implementation-spec.md` Section 20 for the full design rationale.
//!
//! A **new binary crate**, not a module inside `pata-lsp` — DAP and LSP are structurally
//! unrelated protocols (different message shapes, different lifecycle, different transport
//! conventions in practice even though both often run over stdio).
//!
//! This crate is buildable and testable today against `MockHook` — a real, working fake — but
//! genuine step-through debugging needs `core/evaluator` to implement the `DebugHook` trait
//! (see `hook.rs`), which does not exist yet and is out of this crate's scope (it needs a hook
//! inside the interpreter's own statement-execution loop). `pata-dap` depends only on the
//! trait's shape, not on any real evaluator integration.

pub mod hook;
pub mod mock_hook;
pub mod server;

pub use hook::DebugHook;
pub use mock_hook::MockHook;
pub use server::{run, DapSession};
