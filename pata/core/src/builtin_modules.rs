//! Re-export builtin module tables from asili-parser so every `pata-core` consumer (CLI, LSP)
//! shares one source of truth.
pub use asili_parser::builtins::*;
