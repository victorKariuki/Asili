//! `pata-dap` binary: runs the DAP server over stdio, launched directly by an editor's
//! debug-adapter configuration (not via `pata <subcommand>` — DAP servers are conventionally
//! launched by the client pointing at the adapter binary directly, matching how `pata-lsp`'s
//! standalone binary coexists with `pata mwalimu`).
//!
//! Real step-through debugging: a `launch` naming a `.as` file, followed by `configurationDone`,
//! compiles and runs that file with `asili_evaluator::debug_hook::RealDebugHook` attached (see
//! `pata_dap::runner::real_session`) — breakpoints genuinely pause the running program,
//! `variables` reflects real interpreter state, `continue` genuinely resumes it.

fn main() {
    let session = pata_dap::real_session();
    pata_dap::run_session(session, std::io::stdin(), std::io::stdout());
}
