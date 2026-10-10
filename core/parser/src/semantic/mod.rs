//! Semantic analysis: type parsing (types) and name/flow checks (analyzer).

mod analyzer;
mod types;

pub(crate) use analyzer::{run_semantic_check, run_semantic_check_with_modules};
pub use types::{format_value_type, parse_value_type, parse_value_type_with, split_generic_args};
