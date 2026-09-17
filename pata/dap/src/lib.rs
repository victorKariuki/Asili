//! `pata-dap`: minimum viable Debug Adapter Protocol server for Asili — see
//! `docs/design/pata-implementation-spec.md` Section 20 for the full design rationale.
//!
//! A **new binary crate**, not a module inside `pata-lsp` — DAP and LSP are structurally
//! unrelated protocols (different message shapes, different lifecycle, different transport
//! conventions in practice even though both often run over stdio).
//!
//! `DebugHook` is defined in `asili_evaluator::debug_hook` (not in this crate — see that
//! module's own doc comment for why: `core/` never depends on `pata/`, so the trait a `core/`
//! type implements has to live on the `core/` side). `core/evaluator`'s `eval_stmt_impl` calls
//! into `Runtime::debug_hook` for real, driving actual step-through debugging via
//! `RealDebugHook`, not just a test fake — `MockHook` (this crate) remains useful independently
//! for exercising the DAP wire protocol without a real `.as` program on disk.

pub mod mock_hook;
pub mod server;

pub use asili_evaluator::debug_hook::DebugHook;
pub use mock_hook::MockHook;
pub use server::{run, DapSession};
