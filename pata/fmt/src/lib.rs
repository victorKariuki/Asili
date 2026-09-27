//! Library entry point for `pata-fmt`'s canonical formatter, so other crates (`pata-lsp`'s
//! format-on-save) can depend on the real token-stream printer instead of maintaining their own
//! copy. Previously `pata-fmt` was a `[[bin]]`-only crate with no `[lib]` target, which is why
//! `pata/lsp/src/format.rs` carried a literal copy-paste — see that file's own doc comment,
//! updated alongside this change to point here instead.

pub mod config;
pub mod format;
pub mod walk;

pub use config::FormatterConfig;
pub use format::{canonical_format, canonical_format_with_indent};
