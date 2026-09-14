//! `pata-dap` binary: runs the DAP server over stdio, launched directly by an editor's
//! debug-adapter configuration (not via `pata <subcommand>` — DAP servers are conventionally
//! launched by the client pointing at the adapter binary directly, matching how `pata-lsp`'s
//! standalone binary coexists with `pata mwalimu`).
//!
//! Runs against `MockHook` today — real step-through debugging needs `core/evaluator` to
//! implement `DebugHook` first (see `pata_dap::hook`'s doc comment), which is out of this
//! crate's scope. This still lets a real DAP client exercise the protocol layer end-to-end
//! (breakpoints, stack trace, variables) against canned data.

use std::sync::Arc;

fn main() {
    let hook = Arc::new(pata_dap::MockHook::new());
    pata_dap::run(hook, std::io::stdin(), std::io::stdout());
}
