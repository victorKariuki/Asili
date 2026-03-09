//! Semantic analysis: type parsing (types) and name/flow checks (analyzer).

mod analyzer;
mod types;

pub use types::parse_value_type;
pub(crate) use analyzer::run_semantic_check;
