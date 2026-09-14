use asili_diagnostics::Diagnostic;
use asili_lexer::Token;
use std::collections::{HashMap, HashSet};

mod ast;
pub mod attrs;
mod cursor;
mod module_merge;
mod parse;
mod semantic;
pub mod builtins;
pub use ast::*;
pub use attrs::{item_survives, parse_sharti_predicate, ShartiPredicate, Target};
pub use module_merge::merge_modules;
pub use semantic::parse_value_type;

use cursor::Parser;

pub fn parse_tokens(tokens: &[Token]) -> Result<Module, Vec<Diagnostic>> {
    let mut parser = Parser::new(tokens);
    let module = parser.parse_module();
    if parser.errors.is_empty() {
        Ok(module)
    } else {
        Err(parser.errors)
    }
}

pub fn discover_tests(module: &Module) -> Vec<Function> {
    module.functions.iter().filter(|f| f.is_test).cloned().collect()
}

pub fn semantic_check(module: &Module) -> Result<(), Vec<Diagnostic>> {
    semantic_check_with_options(module, true)
}

pub fn semantic_check_with_options(module: &Module, require_main: bool) -> Result<(), Vec<Diagnostic>> {
    semantic_check_with_env(module, require_main, HashMap::new(), HashMap::new())
}

/// Build extern function and constant maps for semantic analysis.
/// Always seeds with the msingi prelude (jozi, orodha, kamusi, etc. are always in scope),
/// then adds functions from each explicitly imported module.
pub fn extern_env_from_imports(module: &Module) -> (HashMap<String, FnContract>, HashMap<String, ValueType>) {
    let prelude = builtins::msingi_exports();
    let mut functions = prelude.functions;
    let mut constants = prelude.constants;
    for imp in &module.imports {
        let mod_name = match &imp.path {
            ImportPath::Full(s) => s.as_str(),
            ImportPath::Selective { module, .. } => module.as_str(),
        };
        if let Some(table) = builtins::builtin_module_exports(mod_name) {
            functions.extend(table.functions);
            constants.extend(table.constants);
        }
    }
    (functions, constants)
}

pub fn semantic_check_with_env(
    module: &Module,
    require_main: bool,
    extern_functions: HashMap<String, FnContract>,
    extern_constants: HashMap<String, ValueType>,
) -> Result<(), Vec<Diagnostic>> {
    let errors = semantic::run_semantic_check(module, require_main, extern_functions, extern_constants);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Like `semantic_check_with_env`, but also accepts `resolved_modules`: `leta` targets to allow
/// beyond the builtin-module whitelist (SEM007), because a project resolver already confirmed
/// they exist (user modules, path/vendored dependencies). Used by `pata jenga`'s pipeline, which
/// resolves imports before running semantic checks; single-file/no-resolver callers should keep
/// using `semantic_check_with_env`.
pub fn semantic_check_with_env_and_modules(
    module: &Module,
    require_main: bool,
    extern_functions: HashMap<String, FnContract>,
    extern_constants: HashMap<String, ValueType>,
    resolved_modules: HashSet<String>,
) -> Result<(), Vec<Diagnostic>> {
    let errors = semantic::run_semantic_check_with_modules(
        module,
        require_main,
        extern_functions,
        extern_constants,
        resolved_modules,
    );
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
