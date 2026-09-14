//! Shared module-resolution core for `pata-cli` and `pata-lsp`.
//!
//! Extracted so both consumers share exactly one resolver/interface-registry implementation
//! instead of two independently-maintained ones — `pata-lsp` previously carried its own smaller
//! reimplementation (`pata/lsp/src/workspace.rs`, pre-extraction) with no version-dependency
//! resolution and no visibility into `.asi` trait tracking, which routinely fell behind whatever
//! `pata-cli`'s own resolver gained. `pata-cli` was binary-only (no `[lib]` target) before this
//! crate existed, which is why `pata-lsp` couldn't simply depend on it directly — the reverse
//! edge (`pata-cli` depends on `pata-lsp` to launch `pata mwalimu`) would have cycled.
//!
//! `pata-cli`'s own `CliError` (message + exit code, used throughout its command layer) is not
//! reused here — a shared library crate consumed by an LSP server has no concept of a process
//! exit code. `Error` below is the plain equivalent; `pata-cli`'s call sites convert via `From`.

pub mod builtin_modules;
pub mod dependency;
pub mod interface_registry;
pub mod resolve;

/// A resolution/interface-loading failure, with no CLI-specific concept of an exit code —
/// `pata-cli`'s own `CliError` converts from this via `From<Error> for CliError` at its call
/// sites (see `pata/cli/src/commands/mod.rs`), and `pata-lsp` can match on `.message` directly
/// or degrade silently, matching how every other fallible path in that crate already behaves.
#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

pub use dependency::Dependency;
pub use interface_registry::{InterfaceRegistry, ModuleInterface, StdlibEnv, TraitStub, TraitMethodStub};
pub use resolve::{
    build_export_table, check_duplicate_imports, dependency_order, find_module_file,
    merge_for_semantic, resolve_all, ExportTable, ResolvedModule, ResolvedProgram,
};
