//! Run the Mwalimu LSP server (stdio). Used by editors for diagnostics and hover.

use super::CliResult;

/// Run the LSP server. Blocks until the client disconnects.
#[allow(dead_code)] // used via dispatch when user runs `pata mwalimu`
pub fn run(_args: &[String]) -> CliResult {
    pata_lsp::run_stdio_blocking();
    Ok(())
}
